# 49 第三批图示：第 22 章（Pin 与自引用）

Type: task
Status: resolved

## Question

给 `src/ch22-pin-self-reference.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 33 的「图示需求」。

- **图清单**：
  1. **`Pin<&mut T>` 的三层结构 + 移动前后对照**（必做）：左侧值/指针/`Pin` 包装三层，`ptr` 指向自身字段成立；右侧值被搬到新地址后 `ptr` 仍指旧地址 → 正好承载 22.1 实测的 `true → false`。
  2. **`Unpin` 的字段传播**（候选，做）：一个 `PhantomPinned` 字段如何关掉整个类型的 `Unpin`（字段级向内传播的示意）。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若某 `text` 草图被新图**完整覆盖**可删（逐条论证并记票）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动（本章刚经审校定稿）。
- **DoD**：§8 协议 —— 独占 `src/ch22-pin-self-reference.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch22-pin-self-reference.md` 绿；SVG 自查同票 47 口径；栅格化目视 `.scratch/tmp-49/`（收尾删）；收尾 `Status: claimed` + `## Comments`。

## Comments

2026-10-19 图票完成。只动 `src/ch22-pin-self-reference.md`（两张图 + 两句引出句）与本票；正文文字/代码块/表格逐字未动。

**图 1 — `Pin<&mut T>` 三层结构 + 移动前后对照**（必做，已做）

- 位置：22.2 小节内，`src/ch22-pin-self-reference.md:155-197`（引出句在 153）。插在「`Pin` 包住的是指针 / pointee」那段之后、「访问被 pin 的值，走的是 `Deref`」之前。
- 引出句（153）：`下图把这三层摊开，并用 22.1 那次移动对照说明 pin 契约要挡住什么（右格即实测的 `true → false`）：`
- aria-label：`Pin<&mut T> 的三层结构：Pin 包装层包住 &mut T 指针层，指针指向 T 的值层；值内部的 ptr 指向自己的 data 字段。移动前值在地址 A，ptr 指向自身、有效（true）；值被搬到新地址 B 后，Pin 与 &mut T 都跟着指向 B，ptr 却仍指向 A，于是失效（false）`
- 内容：左格 = 原地初始化（`Pin<&mut T>` → `&mut T` → `T 的值（地址 A）`，值内 `data`/`ptr` 两字段，`ptr` 箭头回指 `data`，标 `ptr 指向自身字段 ✓`）；右格 = 值被搬到新地址 B，`Pin`、`&mut T` 随新地址走，`ptr` 的箭头绕出值框指向虚线灰框「旧地址 A / 已被复用」，标 `ptr 仍指旧地址 ✗`。列头分别是 `移动前：原地初始化 → true` / `移动后：值被搬走 → false`。
- 编辑决定：右格是「pin 契约要挡住的那种移动」的反事实展示（22.1 的 `SelfRef` 没有 `Pin`），靠引出句点明，不在图内加编号或长注。
- viewBox `0 0 660 300`、`width="100%"`、无 height。

**图 2 — `Unpin` 的字段传播**（候选，已做）

- 位置：22.4 小节内，`src/ch22-pin-self-reference.md:435-454`（引出句在 433）。插在第二条 E0277 输出块之后、「第一条的错误标题就很有信息量…」那段之前 —— 选这个落点是为了**不拆散任何既有段落**（该处本来就是段落边界），正文一字未改。
- 引出句（433）：`下图把字段到类型的这条传播路径摊开：`
- aria-label：`Unpin 的字段传播：HasPin 的两个字段中，data: String 是 Unpin（✓），_pin: PhantomPinned 不是 Unpin（✗）；只要有一个字段不是 Unpin，整个类型 HasPin 就不是 Unpin`
- 内容：上格「类型」= `HasPin` / `不是 Unpin ✗`；下格虚线框「字段」= `data: String`（`Unpin ✓`，`var(--links)`）+ `_pin: PhantomPinned`（`不是 Unpin ✗`，`var(--warning-border)`）；两条竖箭头分别由字段框顶指向类型框底，`_pin` 那条用警告色。字段用正是 22.4 那段 `compile_fail` 里的 `HasPin`，与报错里的 `within HasPin` 对得上。
- viewBox `0 0 660 250`、`width="100%"`、无 height。

**`text` 草图盘点（逐条论证，结论：全部保留）**

本章 11 个 ` ```text ` 块没有一个是 ASCII/字符草图，全是**真跑/真编译输出**，不属于「被图完整覆盖可删」的对象：53（三次打印地址）、133（`true`/`false`）、213（`drop first`/`现在 second`/`地址不变 true`）、289（`job-1 -> true` ×2）、330（`这些都是 Unpin`）、356 / 499 / 544 / 577（E0277 / E0596 / E0716 报错栏位）、411（`v`）、468（`pin! stack -> true`）。两处报错栏位（356、499）本身带 `^^^`/`|` 列对齐，属复刻工具输出的证据，删掉就丢了证据；两条新图都不含工具输出的字符栅格，覆盖不了它们。故一张未删，也未新增 `xml:space="preserve"`（两图无列对齐文本）。

**自验**

- 门禁：`mdbook test -c ch22-pin-self-reference.md` → **exit 0**（mdbook v0.5.4）。并做过**阳性对照**：临时在章末追加一个必定编译失败的 `rust` 块后同命令 → **exit 101**（stderr `ERROR One or more tests failed`），还原后再跑 → exit 0，证明该门禁确实在编译本章而不是静默跳过。
- SVG 结构自查（脚本过两图，全过）：块内无空行；`viewBox` + `width="100%"`，`<svg>` 上无 `height`；无 `<script>`/`<image>`/`<foreignObject>`/外链；颜色集合 = `currentColor` / `var(--links)` / `var(--warning-border)` / `var(--quote-border)` + `fill="none"`，无写死色；`role="img"` + `aria-label` 齐；整块无裸 `&`（正则 `&(?!amp;|lt;|gt;)` 无命中），`<`/`>` 在文本与 aria-label 中均为 `&lt;`/`&gt;`；文本节点闭合配对正常。
- 栅格化目视：`.scratch/tmp-49/fig1.svg`、`fig2.svg`（把主题变量临时替成具体色后栅格化）逐张看过 —— 两图文字未溢框、箭头指向正确、图 1 的 `ptr` 绕行线（右出、下走、左回、上指旧地址框）不压框不压字、底部 `ptr 仍指旧地址 ✗` 与脚注不重叠；结论：布局可接受。目录已删。

**遗留 / 提示**

- 两图用到的 `✓`/`✗` 与既有 ch04 图同款；主题变量在真实 mdBook 渲染下的对比度由协调者收尾 build 统一确认（§9 口径）。
- 未改 `src/SUMMARY.md`、`book.toml` 或其他章；未新增术语。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch22-pin-self-reference.md` 补 **2 张** —— `Pin<&mut T>` 三层结构 + 移动前后对照（:155–197，承载 22.1 的 `true → false`）、`Unpin` 字段传播（:435–454）。
- `text` 块 11 个全是真实输出，**0 删**。
- 自验：`mdbook test -c ch22-pin-self-reference.md` exit 0 + 注入反证（101 → 绿）；结构自查全过；栅格化目视（含主题变量实色对照）。
- 遗留：无。
