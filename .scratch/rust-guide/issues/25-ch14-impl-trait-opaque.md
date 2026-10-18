# 25 第 14 章 impl Trait 与 opaque type

Type: task
Status: open

## Question

写第 14 章：**`impl Trait` 与不透明类型** —— 参数位置 vs 返回位置、2024 的捕获规则。

- **范围**：参数位置 `impl Trait`（= 泛型语法糖，与 `<T: Trait>` 的异同）；返回位置 RPIT（opaque type：调用者只见 trait、不见具体类型；不能同一函数返回两种具体类型 → `Box<dyn>` 或 enum 的取舍）；**2024 的捕获规则**（`impl Trait` 隐式捕获作用域内全部泛型参数与生命周期，`use<..>` 精确控制）；RPITIT（trait 里的 RPIT）与 AFIT（`async fn in trait`），以及 **AFIT 的 future 不保证 `Send`** 这个头号坑；类型泄露与「不透明」的边界。
- **素材**：`research/02`（I3 + §4.1 edition 2024 条目 + §5.2#12）；`reference` 的 impl-trait / 生命周期捕获页；edition-guide 的 `rust-2024/` 相关页。
- **边界**：`dyn` → 13；GAT → 15；async 运行时 → 21。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注；可选「语言对照角」。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch14-impl-trait-opaque.md`）。
