# 38 进阶篇图示：第 16 章（variance、HRTB 与 drop check）

Type: task
Status: open

## Question

给 `src/ch16-lifetimes-advanced.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 27 的「图示需求」（三张都要）。

- **图清单**：
  1. **三种型变的赋值方向**（16.2 节）：三个包装类型 + 可通过/不可通过箭头 —— 协变（长的能当短的用，过）、逆变（反方向过）、不变（都不过）。
  2. **借用区间 vs 析构点时间轴**（16.4 节）：`Inspector`/`World` 形态，说明为什么必须「严格」outlives（持有者的析构点要落在借用区间里）。
  3. **省略规则决策树**（16.6 节）：输入里生命周期个数（0 / 1 / 多个）→ 输出怎么填（三条规则）。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若 ASCII 草图被完整覆盖可删（逐条论证）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch16-lifetimes-advanced.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch16-lifetimes-advanced.md` 绿；SVG 自查项同票 34；栅格化目视 `.scratch/tmp-38/`（收尾删）；票尾 `## Comments` 逐图记录。

## Comments
