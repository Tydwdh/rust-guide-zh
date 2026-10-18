# 34 进阶篇图示：第 12 章（trait 设计）

Type: task
Status: open

## Question

给 `src/ch12-trait-design.md` 补内嵌 SVG（规范 = `WRITING.md` §9；体系与样板见票 22 已完成批次）。需求出处：票 23 的「图示需求」。

- **图清单**（1、2 必做；3 可选）：
  1. **一致性 2×2**（小节 `### 一致性（coherence）与孤儿规则的细节`，现为四行表）：trait 本地/外部 × 类型本地/外部四格，标「允许 / 允许 / E0117 / E0210」，角落注「本地类型由 newtype 提供」。
  2. **密封 trait 三层**（小节 `### 密封 trait（sealed trait）：让外部不能实现你的 trait`，现为 `text` 树）：外部可命名（`pub trait Greet`）／外部不可命名（`mod private` → `trait Sealed`）／可实现区与不可实现区之间的墙。
  3. （可选）**关联类型 vs 泛型参数**（小节 `### 关联类型还是泛型参数`）：左 `type Item` 只有一格、右 `<Rhs>` 可多份。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若某 ASCII 草图被新图**完整覆盖**可删（逐条论证并记票）；否则一律保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch12-trait-design.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch12-trait-design.md` 绿；SVG 自查（块内无空行 / 无脚本外链 / `viewBox`+`width="100%"` 无 height / 仅主题变量 / `role="img"`+`aria-label` / `&amp;` 转义 / 列对齐加 `xml:space="preserve"`）；栅格化目视放 `.scratch/tmp-34/`（收尾删）；收尾 `Status: claimed` + `## Comments`（每图：位置行号 / aria-label / 引出句 / 自查结果 / 目视结论 / 遗留；HTML 渲染留给协调者收尾 build）。

## Comments
