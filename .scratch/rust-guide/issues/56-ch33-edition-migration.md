# 56 第 33 章 Edition 与迁移

Type: task
Status: open

## Question

写第五篇第七章：**Edition 与迁移**。

- **范围**：Edition 是什么（与版本、crate 级 opt-in 的关系，各 crate 可不同 edition）；2024 的全部关键改动清单——**逐条以 Edition Guide 为准核对**（RPIT 生命周期捕获规则、`unsafe_op_in_unsafe_fn` 默认 warn、`static_mut_refs` deny、`gen` 关键字、macro fragment specifiers 变化、match ergonomics 的保留、`unsafe extern`、`unsafe attributes`、prelude 新增 `Future`/`IntoFuture`）；`cargo fix --edition` 的流程与 `rust-2024-compatibility` lint 组；一次真实迁移演练（探针 crate 从 2021 修到 2024，逐条修掉诊断）。
- **素材**：`research/02` 的 I23 行（`edition-guide/rust-2024/`）；本机 1.98 实测（两版 edition 对照编译）。
- **边界**：各改动的机制细节散见前文（RPIT → 第 14 章、`unsafe extern` → 第 26 章、`static_mut_refs` → 第 23 章等，回指不重讲）；Cargo/lints → 第 30/32 章。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch33-edition-migration.md` + `snippets/ch33/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch33-edition-migration.md` 绿；探针 `.scratch/tmp-56/`（迁移演练的全套文件）；收尾 `claimed` + `## Comments`。

## Comments
