use std::cmp::min;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Component, Path, PathBuf};

use mdx::Result;
use mdx::readers::{MdxReader, ZdbReader};
use mdx::storage::key_block::KeyIndex;
use mdx::utils::progress_report::ProgressState;
use url::Url;

use crate::report::print_progress;

pub fn run_export(
    file: &str,
    output: &str,
    mdx_mode: bool,
    count: Option<usize>,
    with_mdd: bool,
) -> Result<()> {
    let input_path = mdx::utils::io_utils::fix_windows_path_buf(PathBuf::from(
        shellexpand::tilde(file).to_string(),
    ));
    let output_path = PathBuf::from(shellexpand::tilde(output).to_string());

    if input_path.is_dir() {
        return export_batch(&input_path, &output_path, count, with_mdd);
    }

    if mdx_mode {
        export_mdx_text(&input_path, &output_path, count)?;
        if with_mdd {
            export_associated_mdd(&input_path, &output_path, count)?;
        }
        Ok(())
    } else {
        export_zdb(&input_path, &output_path, count)
    }
}

fn export_batch(
    input_dir: &Path,
    output_dir: &Path,
    count: Option<usize>,
    with_mdd: bool,
) -> Result<()> {
    println!("Batch exporting: {} -> {}", input_dir.display(), output_dir.display());
    fs::create_dir_all(output_dir)?;

    for entry in walkdir::WalkDir::new(input_dir) {
        let entry = entry.map_err(|e| mdx::ZdbError::general_error(e.to_string()))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or_default();
        if !extension.eq_ignore_ascii_case("mdx") {
            continue;
        }

        let relative = path.strip_prefix(input_dir).map_err(|e| {
            mdx::ZdbError::invalid_path(format!("failed to create relative path: {e}"))
        })?;
        let mut output_file = output_dir.join(relative);
        output_file.set_extension("txt");

        export_mdx_text(path, &output_file, count)?;
        if with_mdd {
            export_associated_mdd(path, &output_file, count)?;
        }
    }

    println!("Batch export completed");
    Ok(())
}

fn export_mdx_text(input_path: &Path, output_file: &Path, count: Option<usize>) -> Result<()> {
    let absolute_input_path = input_path.canonicalize()?;
    let mdx_url = Url::from_file_path(&absolute_input_path)
        .map_err(|()| mdx::ZdbError::invalid_path(format!("{}", absolute_input_path.display())))?;
    let mut reader = MdxReader::from_url(&mdx_url, "")?;
    let entry_count = reader.get_entry_count();
    let export_count = limited_count(entry_count, count);

    println!("Exporting mdx text: {} -> {}", input_path.display(), output_file.display());

    let has_stylesheet = !reader.content_db.meta.db_info.style_sheet.is_empty();
    if has_stylesheet {
        write_stylesheet_sidecar(output_file, &reader.content_db.meta.db_info.style_sheet)?;
    }

    let mut writer = create_text_writer(output_file)?;
    let mut progress = ProgressState::new("export_mdx_text", export_count, 5, Some(print_progress));
    for entry_no in 0..export_count {
        let key_index = reader.get_index(entry_no as mdx::storage::key_block::EntryNo)?;
        let content = reader.content_db.get_string(&key_index, false)?;
        write_source_record(&mut writer, &key_index.key, &content)?;
        progress.report(entry_no);
    }
    writer.flush()?;
    println!("Export completed: {} of {} entries", export_count, entry_count);
    Ok(())
}

fn export_associated_mdd(
    mdx_path: &Path,
    mdx_output_file: &Path,
    count: Option<usize>,
) -> Result<()> {
    let mdd_paths = find_associated_mdd_files(mdx_path)?;
    if mdd_paths.is_empty() {
        println!("Associated mdd not found for: {}", mdx_path.display());
        return Ok(());
    }

    for mdd_path in mdd_paths {
        let mdd_output_dir = mdd_output_dir(mdx_output_file, &mdd_path);
        export_zdb(&mdd_path, &mdd_output_dir, count)?;
    }
    Ok(())
}

fn find_associated_mdd_files(mdx_path: &Path) -> Result<Vec<PathBuf>> {
    let parent = mdx_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = mdx_path.file_stem().and_then(|value| value.to_str()).unwrap_or_default();
    let mut matches = Vec::new();

    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if let Some(order) = associated_mdd_order(stem, file_name) {
            matches.push((order, path));
        }
    }

    matches.sort_by_key(|(order, _)| *order);
    Ok(matches.into_iter().map(|(_, path)| path).collect())
}

fn associated_mdd_order(stem: &str, file_name: &str) -> Option<u32> {
    let lower_name = file_name.to_ascii_lowercase();
    let lower_stem = stem.to_ascii_lowercase();
    let base_name = format!("{}.mdd", lower_stem);

    if lower_name == base_name {
        return Some(0);
    }

    let prefix = format!("{}.", lower_stem);
    let suffix = ".mdd";
    if !lower_name.starts_with(&prefix) || !lower_name.ends_with(suffix) {
        return None;
    }

    let number = &lower_name[prefix.len()..lower_name.len() - suffix.len()];
    number.parse::<u32>().ok().filter(|value| *value > 0)
}

fn mdd_output_dir(mdx_output_file: &Path, mdd_path: &Path) -> PathBuf {
    let parent = mdx_output_file.parent().unwrap_or_else(|| Path::new(""));
    let stem = mdd_path.file_stem().and_then(|value| value.to_str()).unwrap_or("resources");
    parent.join(format!("{}_mdd", stem))
}

fn export_zdb(input_path: &Path, output_path: &Path, count: Option<usize>) -> Result<()> {
    let file = File::open(input_path)?;
    let mut reader = ZdbReader::<BufReader<File>>::from_reader(BufReader::new(file), "", "")?;
    let entry_count = reader.get_entry_count();

    if reader.is_binary_content() {
        export_zdb_resources(&mut reader, entry_count, input_path, output_path, count)
    } else {
        export_zdb_text(&mut reader, entry_count, input_path, output_path, count)
    }
}

fn export_zdb_text(
    reader: &mut ZdbReader<BufReader<File>>,
    entry_count: u64,
    input_path: &Path,
    output_file: &Path,
    count: Option<usize>,
) -> Result<()> {
    println!("Exporting zdb text: {} -> {}", input_path.display(), output_file.display());

    let has_stylesheet = !reader.meta.db_info.style_sheet.is_empty();
    if has_stylesheet {
        write_stylesheet_sidecar(output_file, &reader.meta.db_info.style_sheet)?;
    }

    let mut writer = create_text_writer(output_file)?;
    let export_count = limited_count(entry_count, count);
    let mut progress = ProgressState::new("export_zdb_text", export_count, 5, Some(print_progress));
    for entry_no in 0..export_count {
        let key_index = reader.get_index(entry_no as mdx::storage::key_block::EntryNo)?;
        let content = reader.get_string(&key_index, false)?;
        write_source_record(&mut writer, &key_index.key, &content)?;
        progress.report(entry_no);
    }
    writer.flush()?;
    println!("Export completed: {} of {} entries", export_count, entry_count);
    Ok(())
}

fn export_zdb_resources(
    reader: &mut ZdbReader<BufReader<File>>,
    entry_count: u64,
    input_path: &Path,
    output_dir: &Path,
    count: Option<usize>,
) -> Result<()> {
    println!("Exporting zdb resources: {} -> {}", input_path.display(), output_dir.display());
    fs::create_dir_all(output_dir)?;

    let export_count = limited_count(entry_count, count);
    let mut progress =
        ProgressState::new("export_zdb_resources", export_count, 5, Some(print_progress));
    for entry_no in 0..export_count {
        let key_index = reader.get_index(entry_no as mdx::storage::key_block::EntryNo)?;
        let data = reader.get_data(&key_index, false)?;
        let resource_path = resource_output_path(output_dir, &key_index)?;
        if let Some(parent) = resource_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resource_path, data)?;
        progress.report(entry_no);
    }
    println!("Export completed: {} of {} resources", export_count, entry_count);
    Ok(())
}

fn limited_count(entry_count: u64, count: Option<usize>) -> u64 {
    count.map_or(entry_count, |value| min(value as u64, entry_count))
}

fn create_text_writer(output_file: &Path) -> Result<BufWriter<File>> {
    if let Some(parent) = output_file.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(BufWriter::new(File::create(output_file)?))
}

fn write_stylesheet_sidecar(output_file: &Path, stylesheet: &str) -> Result<()> {
    let mut sidecar = output_file.as_os_str().to_os_string();
    sidecar.push(".stylesheet.txt");
    let sidecar = PathBuf::from(sidecar);
    if let Some(parent) = sidecar.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let stylesheet = decode_stylesheet_for_export(stylesheet);
    fs::write(&sidecar, stylesheet)?;
    println!("Exported compact stylesheet: {}", sidecar.display());
    Ok(())
}

fn decode_stylesheet_for_export(stylesheet: &str) -> String {
    warn_unknown_html_entities(stylesheet);
    decode_common_html_entities(&html_escape::decode_html_entities(stylesheet))
}

fn warn_unknown_html_entities(value: &str) {
    let bytes = value.as_bytes();
    let mut pos = 0;

    while let Some(relative_start) = value[pos..].find('&') {
        let start = pos + relative_start;
        let Some(relative_end) = value[start..].find(';') else {
            break;
        };
        let end = start + relative_end + 1;
        let entity = &value[start..end];

        if looks_like_html_entity(entity)
            && !is_common_html_entity(entity)
            && html_escape::decode_html_entities(entity).as_ref() == entity
        {
            log::warn!("Unknown HTML entity in compact stylesheet, keeping raw entity: {}", entity);
        }

        pos = end;
        if pos >= bytes.len() {
            break;
        }
    }
}

fn decode_common_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", "\u{00A0}")
        .replace("&ensp;", "\u{2002}")
        .replace("&emsp;", "\u{2003}")
        .replace("&thinsp;", "\u{2009}")
}

fn is_common_html_entity(entity: &str) -> bool {
    matches!(entity, "&nbsp;" | "&ensp;" | "&emsp;" | "&thinsp;")
}

fn looks_like_html_entity(entity: &str) -> bool {
    let inner = entity.trim_start_matches('&').trim_end_matches(';');
    !inner.is_empty()
        && inner.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '#' | 'x' | 'X'))
}

fn write_source_record<W: Write>(writer: &mut W, key: &str, content: &str) -> Result<()> {
    let content = content.trim_end_matches('\0');
    writeln!(writer, "{}", key)?;
    write!(writer, "{}", content)?;
    if !content.ends_with('\n') {
        writeln!(writer)?;
    }
    writeln!(writer, "</>")?;
    Ok(())
}

fn resource_output_path(output_dir: &Path, key_index: &KeyIndex) -> Result<PathBuf> {
    let normalized = key_index.key.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');
    let mut relative = PathBuf::new();

    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(mdx::ZdbError::invalid_path(format!(
                    "unsafe resource path: {}",
                    key_index.key
                )));
            }
        }
    }

    if relative.as_os_str().is_empty() {
        relative.push(format!("entry_{}", key_index.entry_no));
    }

    Ok(output_dir.join(relative))
}
