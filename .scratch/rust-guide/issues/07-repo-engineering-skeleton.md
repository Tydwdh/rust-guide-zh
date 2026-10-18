# 07 仓库工程骨架

Type: task
Status: open

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
