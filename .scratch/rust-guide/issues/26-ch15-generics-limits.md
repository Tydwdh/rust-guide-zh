# 26 第 15 章 泛型的边界：const 泛型与 GAT

Type: task
Status: open

## Question

写第 15 章：**泛型的两个「边界外」工具** —— const 泛型与 GAT。

- **范围**：const 泛型（`const N: usize`，`[T; N]` 的泛型化；与「类型参数」的心智模型对照）；为什么 **const 泛型算术仍不稳定**（`generic_const_exprs` / `min_generic_const_args` 的现状按 1.98 写，nightly 标注）；GAT（`type Item<'a>` 语法、解决什么问题 —— 例如 `LendingIterator`/`Iterator` 的借用迭代；与关联类型基础（第 12 章）的衔接）；两者各自「什么时候值得用」的判断清单与替代方案。
- **素材**：`research/02`（I4/I5 + §4.3 不稳定清单）；reference 的 const-generics / associated-items 页；std 现有 GAT 用例（如 `std::iter::Iterator` 之外的 `LendingIterator` 讨论按官方 issue/RFC 现状写）。
- **边界**：关联类型基础 → 12；async 迭代 → 21/22；手写集合（GAT 实战）→ 34。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 必须 `> [!WARNING]` 标注。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch15-generics-limits.md`）。
