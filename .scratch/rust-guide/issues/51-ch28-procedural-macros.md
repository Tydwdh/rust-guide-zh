# 51 第 28 章 过程宏

Type: task
Status: open

## Question

写第五篇第二章：**过程宏**。

- **范围**：三种形态（derive / attribute / function-like）；`proc_macro` 的 API 面（`TokenStream`/`Punct`/`Ident`/`Literal`/`Group`、`Span`、`TokenStream::from_str`）；在 `examples/macro-demo` 里实现一个**零依赖、手写解析**的最小 derive 宏（如 `#[derive(Describe)]` 生成 `fn describe(&self) -> String`，只支持命名结构体——不引 syn/quote）；在 `examples/macro-demo-use` 里消费并写测试；`syn`/`quote` 的取舍作为说明性对照（外部依赖，`ignore` 块）；错误报告（`compile_error!` 与 span 归属；`proc_macro::Diagnostic` 不稳定标注）。
- **素材**：`research/02-knowledge-structure.md` 的 I18 行（依赖 I17/I1；`reference/procedural-macros.html`、书 ch20.5、`std/proc_macro/`）；本机 1.98 实测（`cargo test -p macro-demo-use`）。
- **脚手架已就位**（协调者）：`examples/macro-demo`（`proc-macro = true` stub）与 `examples/macro-demo-use`（path 依赖）；workspace members 已配 —— **勿改 `examples/Cargo.toml`**；只写这两个 crate 的 `src/`/测试与它们自己的 `Cargo.toml`（如需 feature）。
- **章内代码**：过程宏实现代码一律 `rust,ignore`（`proc_macro` 只在 proc-macro crate 可用，doctest 编不了）；真实验证走 workspace 两 crate；消费端示例尽量用 `{{#include}}` 引 `macro-demo-use` 真实代码。
- **边界**：声明宏 → 第 27 章（回指）；宏在构建系统里的位置 → 第 30 章；诊断/span 的 rustc 内部 → 第 37 章（一句）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch28-procedural-macros.md` + `examples/macro-demo/**` + `examples/macro-demo-use/**` + 本票；禁 `git`/禁 build；自验：`CARGO_TARGET_DIR=.scratch/tmp-51/target cargo test --manifest-path examples/Cargo.toml -p macro-demo-use` 全绿 + `mdbook test -c ch28-procedural-macros.md` exit 0；探针 `.scratch/tmp-51/`；收尾 `claimed` + `## Comments`。

## Comments
