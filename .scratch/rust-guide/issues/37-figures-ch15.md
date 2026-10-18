# 37 进阶篇图示：第 15 章（const 泛型与 GAT）

Type: task
Status: claimed

## Question

给 `src/ch15-generics-limits.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 26 的「图示需求」。

- **图清单**：
  1. **GAT 借用区间/时间轴**（**想要、高优先**；小节 `### GAT 的语法与 where Self: 'a`）：把 `next(&'a1 mut self) -> Option<Item<'a1>>` 的三次调用 `'a1`/`'a2`/`'a3` 画成时间轴/借用区间，标出「`Item<'a1>` 在下一次借 `self` 之前失效」。现有三行 `text` 块只能并列，画不出区间与失效关系。
  2. （低优先，可略）`Buf<const N>` 实例化成 `Buf<4>`/`Buf<8>` 两个类型 → 各自单态化产物的示意。
- **放置**：插在对应小节内；图前一句话引出；不写图注编号。若 1 号图完整覆盖了对应 `text` 块可删（论证并记票）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch15-generics-limits.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch15-generics-limits.md` 绿；SVG 自查项同票 34；栅格化目视 `.scratch/tmp-37/`（收尾删）；票尾 `## Comments` 逐图记录。

## Comments

2026-10-19 图 1 落地（图 2 略，理由见下）；仅动第 317 行引出句 + 新增图，正文/代码块/表格一字未动。

**交付**：`src/ch15-generics-limits.md` 新增 1 张内嵌 SVG（清单项 1，必做）。

**图 1：GAT 借用区间时间轴**

- 位置：`src/ch15-generics-limits.md:319-343`，小节 `### GAT 的语法与 where Self: 'a`，紧随 `type Item<'a>` 那段讲解（`fn next` 签名代码块与其后段落之间）。
- 引出句（第 317 行，只在原句末尾补了加粗部分）：`……与**这一次**借用的 `'a` 绑定。下图把「每次调用一个新类型」**摊开成时间轴——每轮借一段新的 `'a`，产出的 `Item` 只在这一段里有效**：`
- aria-label：`GAT 借用区间时间轴：三次 next 各借一段互不重叠的 'a（'a1、'a2、'a3），产出的 Item<'a1>、Item<'a2>、Item<'a3> 是三个不同的类型，且都必须在下一次借 self 之前用完`（文件里 `<>` 写作 `&lt;`/`&gt;`）。
- 图内结构（`viewBox="0 0 780 266"`）：三行 `next(&'aN mut self) -> Option<Item<'aN>>` 签名（**取自被删文本块，逐字保留**）作行标签；每行右侧一条借用区间条，x = 310–460 / 460–610 / 610–760，构成互不重叠的阶梯时间轴，条内 `'a1`/`'a2`/`'a3`（`var(--links)`）；三处 `第 N 次 next` 对齐各自区间起点；两条 `var(--warning-border)` 虚线（x=460、610）＝「下一次借 self 的时刻」，虚线下方 `Item<'a1>`/`Item<'a2>` 到此失效；底部两行说明：「实线＝一次 next 借 self 的区间；虚线＝下次 next 的时刻：上一次的 Item 必须在此之前用完」／「继续持有它（跨过虚线）就借不出第二次 next（E0499）；三段区间互不重叠，所以三个 Item 是三个不同的类型」。
- 结构自查（票 34 口径，逐项脚本跑）：块内无空行 ✓｜无 `<script>`/外链/`<image>`/mermaid ✓｜`viewBox` 有、`width="100%"` 有、**无 `height`** ✓｜只有 `currentColor` + `var(--links)` + `var(--warning-border)`，无写死颜色（`fill="none"` 除外）✓｜`role="img"` + `aria-label` ✓｜`&amp;`/`&lt;`/`&gt;` 转义、块内无裸 `&` ✓｜`xml:space="preserve"` **不适用**（未复刻工具输出、无列对齐）｜线宽统一 1.5、`rx=4`、font-size 12/14、`opacity="0.75"` 只用于 3 处次要标字；无渐变/阴影/`<defs>` ✓。
- 目视（栅格化在 `.scratch/tmp-37/`，已删）：standalone SVG 里 `currentColor`/主题变量无定义，故栅格化前把三者换成实色（浅、深各一份）。结论：行标签最右约 x≈292 < 区间条起点 310，不重叠；最右元素 x=760 < viewBox 780，无裁切；四条标注互不遮挡；阶梯区间 + 两条虚线 + 三处 `第 N 次 next` 对齐，读得出「三段互不重叠、到期即失效」。
- **删掉原 `text` 块**（原 319–323 行三行）并论证完整覆盖：三行签名逐字成为图的行标签；原 `← 第 N 次调用` 的序号信息由图内 `第 N 次 next` 承载；原「（`'a1` 已结束）」由图中的虚线 + 「`Item<'a1>` 到此失效」标出（比纯文本更明确）；紧随其后的正文（第 345 行）所引用的 `'a1`/`'a2`/`'a3`、`Item<'a1>`/`Item<'a2>` 在图中均可直接看到 → 该块无剩余信息，故删。

**图 2（`Buf<4>`/`Buf<8>` 单态化示意）：略。** 理由：票 26 的图示需求原文即写「（低优先）…现用表格覆盖」；`### const 泛型的语法` 的「三条参数对照」表末行已是「单态化后 → 换成具体类型 / 擦除 / 换成具体值」，同节正文另有「`Buf<3>` 与 `Buf<4>` 是不同的类型，判定依据是 `3 != 4`」。单态化产物是两份并排的具体代码副本，没有位置/包含/流程关系要画，补图只会重复表格结论，故按低优先略去。

**事实核对（图底注的 E0499）**：`.scratch/tmp-37/hold.rs`（`let first = it.next().unwrap();` 之后直接再 `it.next()`）`rustc --edition 2024` →

```text
error[E0499]: cannot borrow `it` as mutable more than once at a time
  --> hold.rs:19:18
   |
18 |     let first = it.next().unwrap();
   |                 -- first mutable borrow occurs here
19 |     let second = it.next().unwrap();
   |                  ^^ second mutable borrow occurs here
20 |     println!("{} {}", first.len(), second.len());
   |                       ----- first borrow later used here
```

对照 `ok.rs`（先把 `first` 用完再 `next()`）`rustc --edition 2024` → exit 0。

**自验**：`mdbook test -c ch15-generics-limits.md` → **exit 0**（加图前基线同为 exit 0）。探针与栅格化产物 `.scratch/tmp-37/` 已删除。未用 `git`、未 `mdbook build`/`serve`。

**遗留**：HTML 真实渲染（主题变量取色）留给协调者收尾 build 确认；本票因 standalone SVG 无 CSS 变量，只能以「替换实色后的栅格化目视 + 结构自查」兜底。无新增术语。
