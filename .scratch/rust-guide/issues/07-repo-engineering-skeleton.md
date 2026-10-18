# 07 仓库工程骨架

Type: task
Status: resolved

## Question

把仓库从当前最小骨架（mdbook init + git）变成能支撑长期写作的工程结构。

要做的事（AFK 为主）：

1. 目录结构定稿：`src/` 书籍源码、示例代码目录（先留位，具体布局待 06）、`.scratch/` 工作区。
2. `book.toml` 配置：标题、输出、链接检查、（如有需要）KaTeX 等；依赖 03 结论的部分先留位。
3. 本地工作流：`mdbook serve` 等常用命令写进 README；复核 `.gitignore`。
4. CI：GitHub Actions（构建 + 校验）先写好但不出网（无远端），留待公开时启用。
5. README：项目是什么、怎么本地预览、怎么贡献。

注意：示例工程布局取决于 06 的结论；本票先做占位与通用部分，示例布局待 06 resolved 后补（可另开小票）。

产出：可 `mdbook serve` 的完整骨架 + README + CI 配置。

## Answer

骨架建成并冒烟通过（2026-10-18 本机实测，Windows / mdbook 0.5.4 / rustc 1.98）：

- **结构**：`src/`（书源，`SUMMARY.md` 暂只有「前言」占位）、`examples/`（占位，布局待 06）、`.scratch/rust-guide/`（地图 + 票）、`.github/workflows/book.yml`、根 `README.md`、`.gitattributes`（统一 LF）。
- **book.toml**：`language = "zh-CN"` 已实测生效（输出 `<html lang="zh-CN">`，标题「前言 - Rust 进阶指南（暂定）」）；`[output.html]` 渲染 / 搜索 / 链接检查配置留给 03 → 06。
- **本地命令实测**：`mdbook build` exit 0（输出 `book/`）；`mdbook test` exit 0；`mdbook serve --port 3000` 返回 HTTP 200 —— 注意只监听 `localhost`，`127.0.0.1` 不通（readiness 探测别打 127.0.0.1）。
- **CI**：`cargo install mdbook --version 0.5.4 --locked` → `mdbook build` → `mdbook test`；无远端，公开到 GitHub 后自动生效。
- **坑（已转给 06）**：mdbook 0.5.4 对非法 `book.toml` 字段打印 ERROR 但 **exit 0**；`[output.html.search].use-unicode-segmentation` 即非法（0.5.4 合法字段：enable / limit-results / teaser-word-count / use-boolean-and / boost-title / boost-hierarchy / boost-paragraph / expand / heading-split-level / copy-js / chapter）。中文搜索分词问题已转 03 核实。
- **留待**：示例工程布局（06 定稿后补）；CI 校验链（06）；远端与 Pages（公开时）；MSRV / toolchain 文件（06）。
