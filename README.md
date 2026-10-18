# Rust 进阶指南（暂名）

面向**有编程经验的读者**、假定 Rust 基础已过的中文 Rust 指南，用 [mdBook](https://rust-lang.github.io/mdBook/) 写。

写作路线 —— 地图与全部决策票 —— 在 `.scratch/rust-guide/`：先读 `map.md`。

## 本地预览与校验

需要 mdBook 0.5.4（`cargo install mdbook --version 0.5.4 --locked`）。

```bash
mdbook serve --open   # 本地预览（默认 http://localhost:3000，改动自动重建）
mdbook build          # 构建到 book/
mdbook test           # 用 rustdoc 校验书里的代码块
```

## 目录结构

| 路径 | 用途 |
| --- | --- |
| `src/` | 书籍源码（`SUMMARY.md` 即目录） |
| `examples/` | 示例代码（占位，布局待「06 写作与验收规范」定稿） |
| `.scratch/rust-guide/` | wayfinder 地图与决策票 |
| `book/` | 构建产物（已 gitignore） |

## 状态

骨架阶段：目录结构（「05 内容版图与章节结构」）与写作规范（「06 写作与验收规范」）未定稿，正文未开始。
