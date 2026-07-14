# 修复总结

本文档记录本次为 `mdx_util` 在 Windows 环境下编译、转换和查询测试过程中做的主要修复。

## 1. 使用本地 `mdx` 依赖

当前 `mdx_util` fork 使用相邻目录中的本地 `mdx` fork：

```toml
mdx = { path = "../mdx", default-features = false }
```

这样 `mdx_util` 构建时会直接使用 `D:\0Rust\mdx` 中的修复版库代码，方便两个 fork 联动开发和测试。

## 2. 修复 Windows 路径编译错误

仓库：`D:\0Rust\mdx`

文件：`src/utils/io_utils.rs`

原代码在 `fix_windows_path(path: &str)` 中尝试重新赋值不可变参数：

```rust
path = path.strip_prefix("/").unwrap();
```

当前 Rust 版本会报错：

```text
cannot assign to immutable argument `path`
```

已改为直接返回修正后的字符串：

```rust
return path.strip_prefix("/").unwrap().to_string();
```

## 3. 修复 ICU locale 兼容问题

仓库：`D:\0Rust\mdx`

文件：`src/utils/icu_wrapper.rs`

运行时遇到：

```text
Invalid BCP-47 locale string: en_US
```

原因是新版 ICU 接受 BCP-47 格式，例如 `en-US`，而部分配置或词典元数据使用旧式 ICU locale，例如 `en_US`。

已在 collator 创建入口统一规范化 locale：

```text
en_US -> en-US
zh_CN -> zh-CN
```

同时增加了回归测试，确认 `en_US` 可以正常创建 collator。

## 4. 修复 MDict 文本源内容为空

仓库：`D:\0Rust\mdx`

文件：`src/builder/mdict_source_loader.rs`

原代码使用：

```rust
let mut data = Vec::<u8>::with_capacity(entry.content_len as usize);
self.input_reader.read_exact(&mut data)?;
```

`with_capacity()` 只分配容量，不改变 Vec 长度，因此 `read_exact(&mut data)` 实际读取 0 字节，导致生成文件只有词条，没有内容。

已改为：

```rust
let mut data = vec![0; entry.content_len as usize];
self.input_reader.read_exact(&mut data)?;
```

## 5. 修正 `</>` 结束标记处理

仓库：`D:\0Rust\mdx`

文件：`src/builder/mdict_source_loader.rs`

MDict 文本源中，`</>` 是一个词条的结束标记，不应作为正文写入 record。目标 record 应保存正文内容，并以 `\0` 作为结束符。

原逻辑没有把 `</>` 转成 `\0`，而是把内容结束位置算到了读取 `</>` 之后：

```rust
let content_length = input_reader.stream_position()? - content_start_pos;
```

这样会把 `</>` 这一行也计入内容长度。已调整为记录读取结束标记前的位置，只把真实正文写入 record。

## 6. 为每个 record 追加 `\0` 终止符

仓库：`D:\0Rust\mdx`

文件：`src/builder/mdict_source_loader.rs`

对比已有工具生成的 ZDB v3 文件后发现，每条 record 内容末尾都会带一个 NUL 终止符：

```text
thitthhttt\r\n\0
thhessrotretttt\r\n\0
ksrrhhhhh\r\n\0
```

结合第 5 条，正确处理应是：源文本中的 `</>` 只用于分隔词条，写入 record 时用 `\0` 表示内容结束。

本项目原先生成的内容缺少末尾 `\0`，外部查询时可能从当前词条一路读到后续内容末尾。

已在 `load_data()` 读取内容后追加：

```rust
data.push(0);
```

修复后测试文件内容长度与参考文件一致：

```text
aa: 13 bytes
bb: 18 bytes
cc: 12 bytes
```

## 7. 修复 preview 为空

仓库：`D:\0Rust\mdx_util`

文件：`src/search.rs`

`list --preview` 原先总是把内容当 HTML 提取文本。对于纯文本内容，HTML 提取结果可能为空。

已在 zdb 和 mdx 两个分支中增加回退逻辑：

```text
先尝试 HTML 转文本；如果结果为空，则显示原始内容。
```

修复后：

```text
mdx_util.exe list "D:\Dicts\EN\COD\test.mdx" aa --preview
mdx_util.exe list "D:\Dicts\EN\COD\test.mdx" aa --mode mdx --preview
```

均可以显示 preview 内容。

## 8. 关于输出格式的结论

本项目当前 writer 生成的是 ZDB v3 风格文件，虽然扩展名可以是 `.mdx`，但文件头是：

```text
<ZDB GeneratedByEngineVersion="3.0" ... />
```

已有工具生成的标准 v2 MDX 文件头通常是 UTF-16LE 的：

```text
<Dictionary GeneratedByEngineVersion="2.0" ... />
```

源码中存在 v1/v2 的读取兼容逻辑，但没有 v2 writer。也就是说：

```text
能读 v2，但不能写 v2。
```

本次修复没有新增标准 MDX v2 写入器，而是在现有 ZDB v3 writer 路径上修复了 Windows 编译、locale、内容读取、preview 和 record 终止符问题。

## 9. 已验证命令

```powershell
cargo test test_create_legacy_underscore_locale_collator
cargo check
cargo run -- convert-db "example_test/test_config.json"
cargo run -- test "example_test/test.mdx"
cargo run -- list "example_test/test.mdx" aa --preview
cargo run -- list "D:\0Rust\mdx_util\example_test\test.mdx" aa --mode mdx --preview
cargo build --release
```

以上命令均已通过验证。
