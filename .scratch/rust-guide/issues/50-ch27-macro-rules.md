# 50 第 27 章 宏：macro_rules!

Type: task
Status: open

## Question

写第五篇第一章：**宏：`macro_rules!`**。

- **范围**：声明宏的基本形态（匹配臂与展开）；片段说明符（`expr`/`ty`/`ident`/`tt`/`pat`/`stmt`/`item`/`path`/`literal`/`meta`）与选择原则；**Edition 2024 的片段匹配变化**（`expr`/`pat`/`stmt` 片段在新 edition 的匹配差异——以 Edition Guide 逐条核对）；卫生性（值/类型/生命周期卫生的边界）；`$crate` 与跨 crate 展开；`#[macro_export]` 与作用域（`#[macro_use]` vs 路径调用）；递归宏；调试（`cargo expand` 作为外部工具说明性引用）。
- **素材**：`research/02-knowledge-structure.md` 的 I17 行（依赖 B2/I3；`reference/macros-by-example.html`、书 ch20.5、`rust-by-example`）与 Edition Guide；本机 1.98 实测。
- **边界**：过程宏 → 第 28 章；宏与 Cargo/build → 第 30 章；`cfg` 条件编译 → 第 30 章（回指）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；nightly-only 只标注。
- **DoD**：§8 协议 —— 独占 `src/ch27-macro-rules.md` + `snippets/ch27/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch27-macro-rules.md` 绿；探针 `.scratch/tmp-50/`；收尾 `Status: claimed` + `## Comments`（§8 清单）。

## Comments
