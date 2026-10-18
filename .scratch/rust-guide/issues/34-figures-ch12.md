# 34 进阶篇图示：第 12 章（trait 设计）

Type: task
Status: resolved

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

- 2026-10-19 交付（FigCh12）：`src/ch12-trait-design.md` 加 3 张内嵌 SVG（含可选图 3），每图前各一句引出句；除引出句外正文文字/代码块/表格一字未动；ASCII 树删除一处（论证见下）。自验 `mdbook test -c ch12-trait-design.md` → **exit 0**（与改前同：`INFO Testing chapter '第 12 章 trait 设计'`，无 fail）。栅格化目视用 `.scratch/tmp-34/fig{1,2,3}.svg`（`:img` 渲染），已删该目录。
- **图 1（一致性 2×2）**｜`src/ch12-trait-design.md:248-269`（引出句 `:246`「把「trait 的位置」与「类型的位置」当成两个轴，四种组合的去留就摊成一张 2×2：」）｜`aria-label`「孤儿规则 2×2：trait 本地时两格都允许；trait 外部而类型本地也允许；trait 与类型都外部则写不出来——具体外部类型报 E0117，裸露类型参数报 E0210；本地类型由 newtype 提供」｜位置：紧跟小节内那张四行表之后、「把第二行与第三行连起来看…」之前。**与票面「允许/允许/E0117/E0210 四格」的偏差（按语言规则落地，需协调者知悉）**：若按票面顺序把 (trait 外部 × 类型本地) 判成 E0117，会与本章表第 2 行「类型本地 → 允许（第 9 章的 newtype）」以及「扩展 trait」整节直接矛盾（`impl fmt::Display for WordList` 合法）。故四格实为 **3×「✓ 允许」+ 1 个错误格**：错误格集中在 (trait 外部 × 类型外部)，格内按两种形态分列 `具体外部类型 → E0117` 与 `裸露类型参数 → E0210`；票面要求的四个标签与角落注（「本地类型由 newtype 提供」，落在图底部注文）全部齐备｜自查：块内无空行 ✓／`viewBox`+`width="100%"` 且无 `height` ✓／无 `<script>`/`href`/mermaid ✓／`fill`/`stroke` 仅 `currentColor`+`none`+`var(--links|--warning-border)` ✓／`role="img"`+`aria-label` ✓／无裸 `&`（`impl&lt;T&gt;` 已转义）✓／无多空格文本节点、故 `xml:space` 不需要（未复刻 `^^^`/`|` 类列对齐）✓／标签配平 ✓｜目视：四格 2×2 整齐、行列表头与格内示例不越界、错误格三行与左格判词同基线（已把 `✗ 不能写` 由 y=176 调至 y=186 对齐）｜遗留：无。
- **图 2（密封 trait 三层）**｜`src/ch12-trait-design.md:135-154`（引出句 `:133`「同一句话换成模块可见性的分层图，那道墙的位置就清楚了：」）｜`aria-label`「密封 trait 的分层：外部 crate 能命名 pub trait Greet、能把它当边界，却不能命名 private 模块里的 trait Sealed，所以写不出 impl Greet，报 E0277；虚线那道墙隔开可实现区与不可实现区」｜位置：紧跟「机制只有一句话……」段落，**原地替换**原来的 `text` 模块树。**ASCII 删除论证（逐条）**：原树 4 行信息 = ①根 `crate greet` → 图内大框左上角 `crate greet`；②`mod private （不是 pub）` → 内层下方框首行 `mod private（不是 pub）`；③`└ trait Sealed ← 外部不可命名 → 无法实现` → 同框 `└ trait Sealed` + `← 外部不可命名 → impl Sealed 写不出来` 一行；④`└ trait Greet: Sealed（pub → 外部可命名、可当边界）` → 内层上方框 `pub trait Greet: private::Sealed` + `外部可命名、可当边界`。四条无一遗漏，且额外给出「墙 + 外部 crate 的 ✓/✗」这层新信息，故判为**完整覆盖**并删除（否则同一信息在同一段重复两遍）｜自查：同上 8 项全过；`fn f&lt;G: Greet&gt;(&amp;g)` 的 `&` 已转义 ✓｜目视：左「外部 crate（下游）」/中虚线墙/右 crate 内二层框层次清楚；墙标签「墙：外部不可实现」置于 y=30 位于两框上沿之上，不压边框｜遗留：无。
- **图 3（关联类型 vs 泛型参数，票内可选图，本轮**做了**）**｜`src/ch12-trait-design.md:70-88`（引出句 `:68`「把这两种写法落到「表」上，差别就是同一个类型能在同一张表里填几次：」）｜`aria-label`「关联类型与泛型参数对照：关联类型每个类型只有一格，再填就被判重叠报 E0119；泛型参数让同一类型并存多份实现，如 Add&lt;Meters&gt; 与 Add&lt;f64&gt;」｜位置：紧跟小节内「你想要的｜该用｜标准库例子」三行表之后、「还有三条实践差异值得记住」之前。左格 `type Item = u32; ← 只有一格` + 虚线格「再填一格 → 重叠，E0119」；右格 `impl Add<Meters> for Meters` 与 `impl Add<f64> for Meters` 两格并存｜自查：同上 8 项全过（`Add&lt;Rhs&gt;` 等 `<`/`>` 已转义）｜目视：左右两栏、两格/一格对比一眼可辨，文字不越框｜遗留：无。
- 术语：未新增（图内用词均沿用 GLOSSARY / 本章既有译法：一致性、孤儿规则、密封 trait、关联类型、泛型参数、newtype、impl）。
- 范围与遗留：只动 `src/ch12-trait-design.md` + 本票；未跑 `git`/`mdbook build`/`mdbook serve`；`.scratch/tmp-34/` 已删。HTML 实际渲染（主题变量换肤下的观感）按票留给协调者收尾 build 统一确认；若协调者认为「一致性 2×2」必须严格按票面四格文案（即把 (外部 trait × 本地类型) 也画成报错），请回票——那是与本章正文冲突的口径，需先改正文表格。

## Answer

2026-10-19 批次收尾（用户验收通过；图先经用户验收）。

- 交付：`src/ch12-trait-design.md` 补 **3 张**内嵌 SVG —— 一致性 2×2（`svg` 248–269）、密封 trait 三层（135–154，替换原 `text` 树）、关联类型 vs 泛型对照（70–88，票面可选图，已做）。
- 口径裁定（协调者采纳）：票面「允许/允许/E0117/E0210」按本章正文事实落为 **3 允许 + 1 错误格**（格内分列 E0117/E0210），角落注保留 —— 票面字面与正文表格冲突，以正文为准。
- 自验：`mdbook test -c ch12-trait-design.md` exit 0；SVG 结构自查 3/3 全过；栅格化目视无压盖；HTML 计数 3/3 与浏览器实机渲染由协调者收尾确认。
- 遗留：无。
