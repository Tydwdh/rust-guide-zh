# 35 进阶篇图示：第 13 章（dyn 兼容性与 trait 对象）

Type: task
Status: open

## Question

给 `src/ch13-dyn-trait-objects.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 24 的「图示需求」（三张全做）。

- **图清单**：
  1. **胖指针 + vtable 布局**（**最高优先**；正文「vtable 与三种持有形式」一节）：`&dyn Shape` 两个机器字 —— 数据指针指向具体值、vtable 指针指向「头部元数据 + 每方法一格」；两个具体类型各一张 vtable。
  2. **泛型 vs dyn 代码路径对照**（正文「两条多态路线的账」）：单态化 = 3 份代码 + 直接调用（可内联）；dyn = 1 份代码 + vtable 取址 + 间接调用（不可内联）。
  3. **向上转型图**（`### 向上转型（1.86 起）`）：`dyn Pet → dyn Animal`：数据指针不变、只换 vtable 视图；旁标「横向（`dyn Shape` → `dyn Named`）不合法」。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若有 ASCII 草图被新图**完整覆盖**可删（逐条论证并记票）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch13-dyn-trait-objects.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch13-dyn-trait-objects.md` 绿；SVG 自查项同票 34；栅格化目视 `.scratch/tmp-35/`（收尾删）；票尾 `## Comments` 逐图记录（位置行号 / aria-label / 引出句 / 自查 / 目视 / 遗留）。

## Comments
