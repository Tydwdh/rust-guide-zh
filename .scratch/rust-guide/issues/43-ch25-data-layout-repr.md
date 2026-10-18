# 43 第 25 章 数据布局与 repr

Type: task
Status: open

## Question

写第四篇第三章：**数据布局与 repr**。

- **范围**：`repr(Rust)` 无保证（字段重排、填充）与「别依赖它」；`repr(C)` 的字段顺序契约；`repr(transparent)`；niche 优化（`Option<&T>`/`Option<Box<T>>`/`Option<NonZeroU32>`，与第 17 章 `List` 例子呼应）；enum 布局（判别式、payload 与 niche 的合流）；DST 与胖指针（`str`/`[T]`/`dyn Trait`，`size_of_val`/`align_of_val`）。
- **素材**：`research/02-knowledge-structure.md` 的 I15 行（依赖 I13；`reference/type-layout.html`、`nomicon/repr-rust.html`、`nomicon/other-reprs.html`、`nomicon/exotic-sizes.html`）；实测工具：`size_of`/`align_of`/`offset_of!`/`size_of_val`（1.98 全稳定）。
- **边界**：FFI 的 ABI 契约 → 第 26 章；vtable 细节 → 第 13 章（回指）；别名模型 → 第 35 章。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；布局数字必须本机实测（注明平台：Windows x86-64），不跨平台硬断言。
- **DoD**：§8 —— 独占 `src/ch25-data-layout-repr.md` + `snippets/ch25/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch25-data-layout-repr.md` 绿；探针 `.scratch/tmp-43/`；收尾 `claimed` + `## Comments`。

## Comments
