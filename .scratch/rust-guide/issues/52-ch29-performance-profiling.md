# 52 第 29 章 性能与剖析

Type: task
Status: open

## Question

写第五篇第三章：**性能与剖析**。

- **范围**：构建档位与优化（dev/release、`opt-level`/`lto`/`codegen-units`、`profile` 表）；`cargo build --timings`；测量方法学（`Instant` 手写 micro-bench 与 `black_box`；**`#[bench]` 已属 nightly**；criterion 仅作说明性对照、外部依赖 `ignore`）；剖析工具版图（Linux `perf`/flamegraph；Windows 上的等价做法只给方向、不虚构）；PGO（`rustc` 文档流程，说明性）；覆盖率（`-C instrument-coverage`，说明性）；分配与克隆成本（`size_of`、`Vec` 容量增长实测、`clone` vs `Arc::clone`）。
- **素材**：`research/02` 的 I19 行（`cargo/guide/build-performance.html`、`cargo/reference/profiles.html`、`rustc/profile-guided-optimization.html`、`rustc/instrument-coverage.html`、`std/hint/index.html`）；本机 1.98 实测。
- **边界**：并发性能（`loom`/Miri）→ 第 31 章；内存布局 → 第 25 章（回指）；PGO/覆盖率只到「怎么跑」。
- **要求**：所有耗时数字随机器浮动——正文必须写明，不写「必须小于 X ms」式断言；可复现测量放 `snippets/ch29/`（真跑一次粘输出并注明浮动）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch29-performance-profiling.md` + `snippets/ch29/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch29-performance-profiling.md` 绿；探针 `.scratch/tmp-52/`；收尾 `claimed` + `## Comments`。

## Comments
