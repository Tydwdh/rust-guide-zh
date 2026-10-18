# 37 进阶篇图示：第 15 章（const 泛型与 GAT）

Type: task
Status: open

## Question

给 `src/ch15-generics-limits.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 26 的「图示需求」。

- **图清单**：
  1. **GAT 借用区间/时间轴**（**想要、高优先**；小节 `### GAT 的语法与 where Self: 'a`）：把 `next(&'a1 mut self) -> Option<Item<'a1>>` 的三次调用 `'a1`/`'a2`/`'a3` 画成时间轴/借用区间，标出「`Item<'a1>` 在下一次借 `self` 之前失效」。现有三行 `text` 块只能并列，画不出区间与失效关系。
  2. （低优先，可略）`Buf<const N>` 实例化成 `Buf<4>`/`Buf<8>` 两个类型 → 各自单态化产物的示意。
- **放置**：插在对应小节内；图前一句话引出；不写图注编号。若 1 号图完整覆盖了对应 `text` 块可删（论证并记票）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch15-generics-limits.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch15-generics-limits.md` 绿；SVG 自查项同票 34；栅格化目视 `.scratch/tmp-37/`（收尾删）；票尾 `## Comments` 逐图记录。

## Comments
