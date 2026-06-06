use mdx::utils::progress_report::ProgressState;

pub fn print_progress(progress_state: &mut ProgressState) -> bool {
    log::info!(
        "Progress: {}%",
        (progress_state.current * 100).checked_div(progress_state.total).unwrap_or(0)
    );
    false
}
