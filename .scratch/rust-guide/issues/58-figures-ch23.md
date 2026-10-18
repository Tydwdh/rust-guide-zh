# 58 第四批图示：第 23 章（unsafe 与 UB）

Type: task
Status: open

## Question

给 `src/ch23-unsafe-and-ub.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 41 的「图示需求」。

- **图清单**：
  1. **`unsafe` 的两种用法**（必做）：左栏 `unsafe fn` / `unsafe trait`（**声明契约**，箭头指向调用方/实现方）；右栏 `unsafe {}` / `unsafe impl`（**兑现契约**，箭头指向被调用的操作 / trait 的不变量）；中间一条竖线 = 「编译器检查的边界」。
  2. （可选）「UB 清单 vs 危险但非 UB」两栏对照（左：能触发 UB 的 11 类；右：死锁、泄漏、整数溢出、逻辑错误——承载 23.6/23.7）。与正文表格重复度高可略并说明。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。`text` 草图仅在「被新图完整覆盖」时可删（逐条论证）；否则保留。
- **边界**：只加图与引出句；正文一字不动（本章刚定稿、修回后）。
- **DoD**：§8 协议 —— 独占 `src/ch23-unsafe-and-ub.md` + 本票；禁 `git`/禁 build；加图后 `mdbook test -c ch23-unsafe-and-ub.md` 绿；SVG 自查（块内无空行 / 无脚本外链 / `viewBox`+`width="100%"` 无 height / 仅主题变量 / `role="img"`+`aria-label` / 实体转义 / 列对齐 `xml:space="preserve"`）；栅格化目视 `.scratch/tmp-58/`（收尾删）；收尾 `Status: claimed` + `## Comments`（每图：行号 / aria-label / 引出句 / 自查 / 目视 / 遗留）。

## Comments
