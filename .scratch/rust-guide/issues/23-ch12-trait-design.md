# 23 第 12 章 trait 设计

Type: task
Status: open

## Question

写进阶篇第一章：**trait 作为设计工具**（第 9 章只建立了「行为契约」的直觉）。

- **范围**：关联类型 vs 泛型参数的取舍（何时用哪个，用 `Iterator`/`Add` 做例）；supertrait（`trait A: B`）；sealed trait 模式（`pub trait X: private::Sealed`）；扩展 trait（给外部类型加方法）；coherence 与孤儿规则的细节（第 9 章点到为止处展开）；对象安全只留一句预告（第 13 章）。
- **素材**：`research/02-knowledge-structure.md`（I1 条目与 §5 易踩坑）；官方快照 `reference/items/traits.html`、`reference/items/associated-items.html`、std 的 `Iterator`/`Add` 页。
- **边界**：`dyn`/对象安全 → 13；`impl Trait`/RPIT → 14；GAT → 15；错误处理与 API 设计 → 18。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only（如 specialization）只标注不进正文；可选「语言对照角」。
- DoD：按 `WRITING.md` §8 批次协议（独占本章文件 + `snippets/ch12/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch12-trait-design.md`；收尾 `claimed` + `## Comments`）。
