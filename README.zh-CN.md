# mdx_util - MDX/MDD 数据库命令行工具

`mdx_util` 是一个用于处理 MDict 格式 `.mdx` 和 `.mdd` 文件的命令行工具。它可以测试、搜索、建立全文索引、转换词典数据，并从目录打包 MDD 资源文件。

## 概览

`mdx_util` 基于 `mdx` 库提供命令行接口，面向词典制作、调试和数据处理场景。它可以处理 `.mdx` 文本词典文件，也可以处理 `.mdd` 图片、音频等二进制资源文件。

当前 fork 默认使用相邻目录中的本地 `mdx` 依赖：

```toml
mdx = { path = "../mdx", default-features = false }
```

因此推荐目录结构为：

```text
D:\0Rust\mdx
D:\0Rust\mdx_util
```

## 功能

### 核心能力

- **数据库测试与校验**：读取并解析 MDX/MDD 文件，验证基本结构和内容可访问性
- **关键词查询**：按关键词查找词条，支持前缀匹配、部分匹配和内容预览
- **全文搜索**：基于 Tantivy 创建并查询全文索引
- **数据库转换**：通过 JSON 配置文件将源数据转换为 `.mdx` 输出
- **资源打包**：将目录中的图片、音频等文件打包为 `.mdd`
- **授权码生成**：为带密码的词典生成密钥和注册码

## 命令

### `test` - 测试数据库结构

读取词典文件并验证词条内容是否可正常访问。

```bash
# 测试单个 MDX 文件
mdx_util test /path/to/file.mdx

# 使用 mdx 模式测试目录中的文件，并限制测试数量
mdx_util test --mode mdx --count 100 /path/to/directory

# 测试 MDD 或 ZDB 风格文件
mdx_util test --mode zdb --count 50 /path/to/file.mdd

# 随机抽样测试，而不是顺序读取
mdx_util test --random /path/to/file.mdx
```

常用选项：

- `--mode`：测试模式，支持 `zdb` 或 `mdx`，默认是 `zdb`
- `--count`：限制测试词条数量，默认测试全部词条
- `--random`：随机抽样测试

### `list` - 查询词条

按关键词搜索词条，并显示匹配词条及其后续若干词条。

```bash
# 基本查询
mdx_util list /path/to/file.mdx "apple"

# 显示 HTML 转文本后的内容预览
mdx_util list --preview /path/to/file.mdx "search term"

# 前缀匹配
mdx_util list --start-with-match /path/to/file.mdx "prefix"

# 部分匹配
mdx_util list --partial-match /path/to/file.mdx "pattern"

# 使用 mdx 模式查询
mdx_util list --mode mdx --preview /path/to/file.mdx "apple"
```

常用选项：

- `--mode`：查询模式，支持 `zdb` 或 `mdx`，默认是 `zdb`
- `--preview`：显示内容预览；纯文本内容在 HTML 提取为空时会回退显示原文
- `--start-with-match`：前缀匹配
- `--partial-match`：部分匹配

### `create-index` - 创建全文索引

为指定 MDX 文件创建 Tantivy 全文搜索索引。

```bash
mdx_util create-index /path/to/file.mdx
```

该命令会生成索引文件，用于后续快速全文搜索。

### `fts-search` - 全文搜索

使用已经创建的全文索引进行搜索。

```bash
mdx_util fts-search /path/to/file.mdx "search term"

# 搜索短语
mdx_util fts-search /path/to/file.mdx "exact phrase"
```

默认返回相关性最高的搜索结果。

### `convert-db` - 转换数据库

通过 JSON 配置文件把源数据转换为 `.mdx` 输出。

```bash
# 只生成示例配置文件
mdx_util convert-db config.json --generate-config-only

# 使用已有配置执行转换
mdx_util convert-db config.json
```

配置文件示例：

```json
{
  "input_path": "/path/to/source.txt",
  "output_file": "/path/to/output.mdx",
  "data_source_format": "MdictHtml",
  "content_type": "Html",
  "default_sorting_locale": "en_US",
  "preferred_content_block_size": 65536,
  "preferred_key_block_size": 16384,
  "password": ""
}
```

注意：当前 writer 生成的是 ZDB v3 风格文件，虽然扩展名通常为 `.mdx`，但不是标准 MDict v2 writer。更多细节见 `FIX_SUMMARY.md`。

### `build-mdd` - 打包 MDD 资源文件

将目录中的资源文件打包为 MDD。

```bash
mdx_util build-mdd /path/to/resources output.mdd "password"
```

适用于图片、音频、样式文件等词典资源。

### `export` - 还原源文本或资源文件

将 MDX/ZDB 文本词典导出回 MDict 源文本格式，或将 MDD 二进制资源解包到目录。

```bash
# 导出 ZDB/MDX 文本为源文本格式
mdx_util export /path/to/file.mdx output.txt

# 使用 mdx reader 模式导出
mdx_util export --mode mdx /path/to/file.mdx output.txt

# 导出 MDX 文本，并同时导出同名 MDD 资源
mdx_util export --mode mdx --with-mdd /path/to/file.mdx output.txt

# 批量导出目录中的 MDX 文件
mdx_util export /path/to/dictionaries output_directory

# 使用显式选项指定批量导出目录
mdx_util export /path/to/dictionaries --output-dir output_directory

# 批量导出目录中的 MDX 文件，并同时导出同名 MDD 资源
mdx_util export /path/to/dictionaries output_directory --with-mdd

# 解包 MDD 资源到目录
mdx_util export /path/to/file.mdd output_resources

# 只导出前 N 个词条或资源，适合抽样测试
mdx_util export /path/to/file.mdd output_resources --count 100
```

文本导出格式为：词条名、原始内容、`</>` 结束标记。Compact HTML 词典会导出 compact 源文本，并额外生成 `output.txt.stylesheet.txt` 这类 stylesheet 文件。资源导出会把 MDD 里的资源 key 映射为输出目录下的相对路径；同名 MDD 资源会导出到 `<词典名>_mdd` 目录。

### `keygen` - 生成授权信息

为加密或带密码词典生成相关授权信息。

```bash
mdx_util keygen "password" "user@example.com"

# 使用 UUID 标识
mdx_util keygen "password" "006F0050-0063-006B-0065-4444556494345454D00"
```

输出内容包括密码、数据库 cipher、标识信息和终端用户注册码。

## 安装与构建

### 前置条件

- Rust 和 Cargo
- 本地 `mdx` fork，路径建议为 `../mdx`

### 从源码构建

```bash
cargo build --release
```

构建完成后，可执行文件位于：

```text
target/release/mdx_util.exe
```

Linux/macOS 下文件名通常为：

```text
target/release/mdx_util
```

## 日志配置

可以通过环境变量或命令行参数控制日志级别。

```bash
# 通过环境变量
RUST_LOG=debug mdx_util test /path/to/file.mdx

# 通过命令行参数
mdx_util --log-level debug test /path/to/file.mdx
```

支持的日志级别：`error`、`warn`、`info`、`debug`、`trace`。默认级别是 `info`。

## 路径展开

命令中的路径支持 `~` 展开。

```bash
mdx_util test ~/dictionaries/my-dict.mdx
```

Windows 下也修复了从 file URL 得到 `/C:/...` 这类路径时的处理问题。

## 主要依赖

- **mdx**：MDict/ZDB 读写库
- **tantivy**：全文搜索引擎
- **clap**：命令行参数解析
- **serde / serde_json**：JSON 配置解析
- **snafu**：错误处理
- **lol_html**：HTML 处理和文本提取
- **zip**：MDD 资源打包
- **rand**：随机抽样测试
- **log / fern**：日志输出

完整依赖列表见 `Cargo.toml`。

## 使用场景

1. **词典开发**：制作过程中测试和验证词典文件
2. **质量检查**：检查词条、编码和内容是否可正常读取
3. **数据转换**：将文本源转换为当前 writer 支持的 `.mdx` 输出
4. **搜索优化**：为词典创建全文索引
5. **资源管理**：将图片、音频等资源打包为 MDD
6. **授权测试**：生成加密词典所需的授权信息

## 完整流程示例

```bash
# 1. 将源数据转换为 MDX/ZDB 风格文件
mdx_util convert-db config.json

# 2. 创建全文搜索索引
mdx_util create-index output.mdx

# 3. 测试数据库
mdx_util test output.mdx --count 100

# 4. 查询词条并显示预览
mdx_util list output.mdx "example" --preview

# 5. 执行全文搜索
mdx_util fts-search output.mdx "search term"
```

## 带资源词典示例

```bash
# 1. 创建资源包
mdx_util build-mdd ~/images dict.mdd "secret_password"

# 2. 在词典正文中通过路径引用资源

# 3. 测试词典和资源文件
mdx_util test output.mdx
mdx_util test dict.mdd --mode zdb
```

## 常见错误

- **Invalid file path**：文件不存在或路径不正确
- **Corrupted database**：文件损坏或格式不兼容
- **Missing index**：执行全文搜索前没有创建索引
- **Permission denied**：当前用户没有文件访问权限
- **Invalid BCP-47 locale string**：旧式 locale 已在本 fork 中做兼容处理，例如 `en_US` 会转为 `en-US`

## 许可证

GNU Affero General Public License v3.0 (AGPL-3.0)

## 作者

Rayman Zhang

## 贡献

问题和贡献可以提交到当前 fork，或参考上游 `mdx` / `mdx_util` 项目。
