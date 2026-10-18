# 36 进阶篇图示：第 14 章（impl Trait 与 opaque type）

Type: task
Status: open

## Question

给 `src/ch14-impl-trait-opaque.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 25 的「图示需求」（1、2 必做；3 可选）。

- **图清单**：
  1. **APIT vs RPIT 角色方向**（正文「同一处语法、两处相反的角色」）：APIT（调用者选类型 → 函数）与 RPIT（函数选类型 → 调用者）的左右方向对照。
  2. **捕获集合 2021 vs 2024**（`捕获：隐藏类型能用到哪些泛型参数` 小节）：`fn evens(xs: &'a [u32]) -> impl Iterator<Item = u32>` 的作用域内参数与 `use<..>` 筛选结果，2021/2024 两列勾/叉；可附「多捕 → 调用者约束更紧」箭头。
  3. （可选）RPIT / `Box<dyn>` / enum 三条路的分派方式与分配代价（表格已能说清；代价高可略，票里说明）。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若 ASCII 草图被完整覆盖可删（逐条论证）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch14-impl-trait-opaque.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch14-impl-trait-opaque.md` 绿；SVG 自查项同票 34；栅格化目视 `.scratch/tmp-36/`（收尾删）；票尾 `## Comments` 逐图记录。

## Comments
