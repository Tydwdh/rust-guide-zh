# 67 第 39 章 宏系统极限

Type: task
Status: open

## Question

写第六篇第六章：**宏系统极限**（素材 = D6：reference `procedural-macros.html`、unstable-book）。

- **范围**：`macro_metavar_expr`（`${count}`/`${index}` 等，nightly 标注 + 只文档化）；声明宏与过程宏的组合形态（函数式宏生成属性宏调用、属性宏叠加顺序——stable 部分真跑）；`#[track_caller]`（stable，真跑 `Location::caller`）；诊断消息设计（`compile_error!` 的 span 与布局、`proc_macro::Diagnostic` nightly 标注）；卫生性/`$crate` 的极限情形（回指 ch27，不重讲）。
- **注意**：**不要碰 `examples/macro-demo` / `macro-demo-use`**（ch28 章内 include 依赖其逐行布局）；需要过程宏就在 `snippets/ch39/` 里自建独立小 crate（零依赖）。
- **边界**：宏基础 → ch27/28 回指；不提供 nightly 特性的伪造输出。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch39-macro-frontiers.md` + `snippets/ch39/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch39-macro-frontiers.md` exit 0（snippet crate 测试私有 target）；探针 `.scratch/tmp-67/`；收尾 `Status: claimed` + `## Comments`。

## Comments
