# 42 第 24 章 指针、provenance 与未初始化内存

Type: task
Status: open

## Question

写第四篇第二章：**指针、provenance 与未初始化内存**。

- **范围**：`NonNull`（与裸指针的取舍）；strict provenance（`ptr.addr()`、`with_addr`、`map_addr` 等 —— **先核对 1.98 快照的实际名字/稳定状态**，unstable 的标 nightly）；`&raw const`/`&raw mut`；`MaybeUninit`（`assume_init` 的安全条件、`assume_init_read`、`zeroed` 的坑）；`ManuallyDrop`；别名模型的直觉（Stacked/Tree Borrows 一小节，形式化 → 第 35 章）。
- **素材**：`research/02-knowledge-structure.md` 的 I14 行（依赖 I13；`nomicon/aliasing.html`、`nomicon/checked-uninit.html`、`nomicon/unchecked-uninit.html`、`std/ptr/`）；本机 1.98 快照逐条核对（provenance API 的稳定面变化快，逐条查 `std/ptr` 与 releases）。
- **边界**：别名模型形式化/UCG → 第 35 章；布局 → 第 25 章；FFI → 第 26 章；unsafe 基础 → 第 23 章（回指）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；unstable 逐条标注；Miri 演示标 nightly。
- **DoD**：§8 —— 独占 `src/ch24-pointers-provenance.md` + `snippets/ch24/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch24-pointers-provenance.md` 绿；探针 `.scratch/tmp-42/`；收尾 `claimed` + `## Comments`。

## Comments
