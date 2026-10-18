# 41 第 23 章 unsafe 与 UB

Type: task
Status: open

## Question

写第四篇第一章：**unsafe 与 UB**。

- **范围**：`unsafe` 的四个超能力（解引用裸指针、调用 `unsafe fn`、访问可变静态、实现 `unsafe trait`）；`unsafe fn` 的契约与 2024 的 `unsafe_op_in_unsafe_fn`（unsafe fn 体内也要显式 `unsafe {}`）；`unsafe trait` 与「安全承诺」；UB 清单（Reference 版逐条）与「危险但非 UB」的区分；用 Miri 抓 UB（nightly，命令给全）。
- **素材**：`research/02-knowledge-structure.md` 的 I13 行（依赖 B1/I7；`reference/unsafety.html`、`reference/behavior-considered-undefined.html`、`nomicon/meet-safe-and-unsafe.html`、书 ch20.1）；本机 1.98 快照逐条核对。
- **边界**：指针/provenance/`MaybeUninit` → 第 24 章；布局/`repr` → 第 25 章；FFI → 第 26 章；别名模型 → 第 35 章。第 16/22 章的既有概念只回指。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；Miri 全部标 nightly；`unsafe_op_in_unsafe_fn` 标 2024 变化。
- **DoD**：按 `WRITING.md` §8 —— 独占 `src/ch23-unsafe-and-ub.md` + `snippets/ch23/` + 本票；禁 `git`/禁 `mdbook build`；自验 `mdbook test -c ch23-unsafe-and-ub.md` 绿；探针 `.scratch/tmp-41/`（收尾删）；运行输出真跑真粘；英文引文逐字核对；收尾 `Status: claimed` + `## Comments`（同 §8 清单）。

## Comments
