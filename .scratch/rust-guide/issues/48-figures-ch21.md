# 48 第三批图示：第 21 章（async 基础）

Type: task
Status: open

## Question

给 `src/ch21-async-basics.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 32 的「图示需求」。

- **图清单**：
  1. **`async fn` → 状态机**：写法 ↔ 编译器生成的状态机（字段 = 跨 `.await` 存活的局部变量；切点 = 每个 `.await`）。
  2. **poll → `Pending` → 登记 waker → `wake` → 再 poll 循环**：执行器与 future 之间的往复（谁调用谁、`Waker` 在哪一步被登记/调用）。
  建议单图宽 ~600×220 或两张同规格；若两张信息重叠过多可合并为一张并说明。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若某 `text` 块被新图**完整覆盖**可删（逐条论证并记票）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动（本章刚经审校定稿）。
- **DoD**：§8 协议 —— 独占 `src/ch21-async-basics.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch21-async-basics.md` 绿；SVG 自查同票 47 口径；栅格化目视 `.scratch/tmp-48/`（收尾删）；收尾 `Status: claimed` + `## Comments`。

## Comments
