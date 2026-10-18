# 29 第 18 章 错误处理与 API 设计

Type: task
Status: open

## Question

写第 18 章：**错误处理与 API 设计** —— 第 7 章的深化，面向「库作者」的取舍。

- **范围**：库 vs 应用的错误策略（具体错误 enum vs `Box<dyn Error>`/`anyhow` 式；「库不该强推日志/全局状态」）；`From` 转换策略与 `source()` 链的构建（第 7 章的深化）；`#[non_exhaustive]`（错误枚举的演进兼容）；builder 模式与 typestate（编译期状态机）；参数取舍（`&str` vs `String`、`AsRef`/`impl Into` 的边界）；SemVer 与破坏性变更清单（错误类型变大算不算破坏？）；`#[must_use]`、`#[deprecated]` 等 API 防呆手段。
- **素材**：`research/02`（I8）；`rust-lang/api-guidelines`（一手规范；注意它不覆盖 2024 后的语法风格，引用时说明）；std 的 `Error`/`From`/`TryFrom` 页；`reference` 的 attributes 页。
- **边界**：错误处理基础 → 7；`dyn` → 13；文档与 lint 工程 → 32；Edition 迁移 → 33。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch18-error-api-design.md`）。
