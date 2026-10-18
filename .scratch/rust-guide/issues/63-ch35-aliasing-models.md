# 63 第 35 章 别名模型与形式化

Type: task
Status: open

## Question

写第六篇第二章：**别名模型与形式化**（素材 = D2：UCG 站点、Nomicon `aliasing.html`）。

- **范围**：Stacked Borrows 与 Tree Borrows 的机制对照（栈式借出/回收 vs 树式许可）；UCG 的定位（**进行中的非规范工作**，引用必须标注）；形式化与编译器现状的差距（1.98 是否默认启用任一所列模型——以官方现状核实后写）；provenance 细节深化（承接 ch24：严格/暴露两层结论、`&raw` 的意义）；Miri 的角色与使用方式（命令级文档化；**本机无 nightly，不得伪造输出**，用 `> [!WARNING]` 标注）。
- **要求**：概念引用逐字对官方页（快照/在线），RFC/issue 编号逐条核；所有「实测」必须是 stable 面证据。
- **边界**：Miri 用法细节 → ch23/24 回指；provenance API → ch24 回指（不重讲）；unsafe 基础 → ch23。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 内嵌 1–2 张 SVG（如两种模型对照）。
- **DoD**：§8 —— 独占 `src/ch35-aliasing-models.md` + `snippets/ch35/`（若有可编译演示）+ 本票；禁 `git`/禁 build；自验 `mdbook test -c ch35-aliasing-models.md` exit 0；探针 `.scratch/tmp-63/`；收尾 `Status: claimed` + `## Comments`。

## Comments
