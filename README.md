# Rust 指南（暂名）

从零起步的中文 Rust 指南：假定读者会一门主流语言，**不假定会 Rust** —— 以「理解 Rust 的设计」为主线、诊断训练贯穿全书，用 [mdBook](https://rust-lang.github.io/mdBook/) 写。

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
| `examples/` | 实战项目的示例 crate（workspace；约定见 `WRITING.md` §1） |
| `.scratch/rust-guide/` | wayfinder 地图与决策票 |
| `WRITING.md` | 写作规范（章节模板、校验链、DoD、风格） |
| `GLOSSARY.md` | 术语表（写作一致性依据，将来派生书末附录） |
| `book/` | 构建产物（已 gitignore） |

## 状态

骨架阶段：目录结构（「05 内容版图与章节结构」）与写作规范（「06 写作与验收规范」）未定稿，正文未开始。
