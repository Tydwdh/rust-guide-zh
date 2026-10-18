# 55 第 32 章 文档与 lint 工程

Type: task
Status: open

## Question

写第五篇第六章：**文档与 lint 工程**。

- **范围**：rustdoc（文档注释、`#![warn(missing_docs)]`、doctest 的隐藏行 `#`、`compile_fail` 标注、文档里的 `should_panic` 关联）；intra-doc link（`[Type]`/`[fn]` 写法与 `-D rustdoc::broken_intra_doc_links`）；`#[doc(cfg)]`（nightly）与 `#[cfg_attr(docsrs, doc(cfg(...)))]` 技巧（docs.rs 场景，说明性）；clippy 分组（correctness/style/complexity/perf）与常用 lint 取舍；`-D warnings` 的策略与**禁止 `deny(warnings)` 进库**的理由（下游编译被卡）；`#[expect]`（核对稳定版本）与 `[lints]` 表（衔接第 30 章）。
- **素材**：`research/02` 的 I22 行（`rustdoc/`、`clippy/`、`rustc/lints/index.html`）；本机 1.98 实测（探针 crate 真跑 `cargo doc` + `RUSTDOCFLAGS="-D warnings"` 与 `cargo clippy`）。
- **边界**：`[lints]` 表/workspace 继承 → 第 30 章（回指）；Edition 迁移 lint 组 → 第 33 章；测试面 → 第 31 章。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch32-docs-and-lints.md` + `snippets/ch32/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch32-docs-and-lints.md` 绿；探针 `.scratch/tmp-55/`（探针 crate 里跑 doc/clippy 取真实输出）；收尾 `claimed` + `## Comments`。

## Comments
