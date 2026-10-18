# 49 第三批图示：第 22 章（Pin 与自引用）

Type: task
Status: open

## Question

给 `src/ch22-pin-self-reference.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 33 的「图示需求」。

- **图清单**：
  1. **`Pin<&mut T>` 的三层结构 + 移动前后对照**（必做）：左侧值/指针/`Pin` 包装三层，`ptr` 指向自身字段成立；右侧值被搬到新地址后 `ptr` 仍指旧地址 → 正好承载 22.1 实测的 `true → false`。
  2. **`Unpin` 的字段传播**（候选，做）：一个 `PhantomPinned` 字段如何关掉整个类型的 `Unpin`（字段级向内传播的示意）。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若某 `text` 草图被新图**完整覆盖**可删（逐条论证并记票）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动（本章刚经审校定稿）。
- **DoD**：§8 协议 —— 独占 `src/ch22-pin-self-reference.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch22-pin-self-reference.md` 绿；SVG 自查同票 47 口径；栅格化目视 `.scratch/tmp-49/`（收尾删）；收尾 `Status: claimed` + `## Comments`。

## Comments
