# 22 基础篇图示补全

Type: task
Status: open

## Question

按已定体系把基础篇的图示补进各章：**内嵌 SVG**（手写、无脚本、`currentColor` 随主题变色；**单块内不得有空行** —— CommonMark 的 HTML 块会在空行处截断，实测）。

- **需求清单**：约 35 处，逐条在 `.scratch/rust-guide/batch-notes.md` 各票的「图示需求」（内存布局 / 借用区间 / 模块树 / 流程 / 包含关系）。
- **样例（画风与密度的参照）**：`src/ch06-collections-strings.md` UTF-8 一节的「你好」字节布局图。
- **约束**：不引入 mermaid / 外部资产 / 脚本；图不参与 `mdbook test`（SVG 不是代码块）；`mdbook build` 后扫 ERROR/WARN；每章补图后复跑 `mdbook test -c <章>`；主题变量可用 `currentColor`、`var(--links)`、`var(--warning-border)`（mdbook 主题里都有定义）。
- **排期**：等用户看过样例、确认画风与密度后再分批开写（建议每批 3–5 章）。

产出：各章内嵌 SVG；另外给 `WRITING.md` 补一节「图示规范」（尺寸/字号/配色变量/`role="img"` + `aria-label` 无障碍约定/「单块无空行」规则），使后继篇章（进阶篇、补充章节）能照此画图。
