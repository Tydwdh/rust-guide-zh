# 60 第四批图示：第 26 章（FFI 与 ABI）

Type: task
Status: resolved

## Question

给 `src/ch26-ffi-abi.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 44 的「图示需求」。

- **图清单**（1 张，必做）：
  1. **FFI 边界与 panic 三条防线**（横向示意）：`Rust 调用方 →（extern "C" 边界）→ 回调方`，标注三种情形——① 直接越界 → abort 点；② 边界内侧 `catch_unwind` → 就地转成返回值；③ 全链 `extern "C-unwind"` → 外层 `catch_unwind` 接住。与 26.6 的三条事实一一对应。
- **放置**：插在 26.6 小节内；图前一句话引出；不写图注编号。`text` 块仅在「被新图完整覆盖」时可删（论证并记票）。
- **边界**：只加图与引出句；正文一字不动（本章刚定稿、修回后）。
- **DoD**：§8 —— 独占 `src/ch26-ffi-abi.md` + 本票；禁 `git`/禁 build；`mdbook test -c ch26-ffi-abi.md` 绿；SVG 自查同票 58 口径；栅格化目视 `.scratch/tmp-60/`（收尾删）；收尾 `Status: claimed` + `## Comments`。

## Comments

**文件**：`src/ch26-ffi-abi.md`（只加 26.6 一节内的 1 张内嵌 SVG + 1 句引出；正文其余一字未动；插入点之后所有行号 +37）。

**图 1：FFI 边界与 panic 三条防线（票内唯一必做图，已完成）**

- 位置：26.6 小节内。引出句 = 行 227；SVG 块 = 行 229–262（34 行，块内无空行）；图前 228、图后 263 为空行；无图注编号。
- 引出句原文：「三条事实对应调用链上的三个位置（中间的 C 帧简化成一道 `extern "C"` 缝），三行依次对应下面三条：」
- `aria-label` 原文：「FFI 边界与 panic 的三条防线：直接越界导致进程 abort；在回调方函数体内用 catch_unwind 就地接住并转成返回值；全链 extern C-unwind 时 panic 穿透边界，被外层 catch_unwind 接住」
- 与 26.6 三条事实的对应：第 1 行（`panic!()` 越过 `extern "C"` 边界 → 边界处 ✗ → 「越界 → abort：进程立即中止」）= 事实一；第 2 行（右栏是 `extern "C" fn` 函数体，内含 `catch_unwind(|| { panic!() })`，箭头以普通返回值向左回到调用方，「就地转成返回值（Err）」）= 事实二；第 3 行（两侧同一强调色，`panic!()` 箭头带越界标记穿过边界，进入左栏「外层 Rust 帧 / catch_unwind」框，「panic 穿透 → 被外层 catch_unwind 接住」）= 事实三。左栏标 `Rust 调用方（外层帧）`、右栏标 `回调方（Rust 的 extern "C" fn）`、中线标 `extern "C" ABI 边界`，与票 44 的示意图（`Rust 调用方 →（extern "C" 边界）→ 回调方`）一致。
- SVG 自查（票 58 口径，逐项跑过命令）：
  - 块内无空行：`awk 'NR>=229 && NR<=262'` 输出 34 行、无 `<EMPTY>` ✓
  - 无脚本 / 无外链：`grep -n 'height=\|<script\|url(\|xlink'` 只命中两个 `<rect … height="50" …>`（矩形属性，非 svg 根）✓；svg 根 = `viewBox="0 0 760 260" width="100%"`，**无 `height`** ✓
  - 颜色只用主题变量：`grep -o 'fill="[^"]*"\|stroke="[^"]*"' | sort | uniq -c` → `fill="currentColor"`×15、`fill="none"`×7、`fill="var(--links)"`×5、`fill="var(--warning-border)"`×1、`stroke="currentColor"`×7、`stroke="var(--links)"`×3、`stroke="var(--warning-border)"`×1；无字面色值 ✓
  - 无障碍：`role="img"` + `aria-label` ✓
  - 实体转义：块内 `grep '&' | grep -v '&amp;\|&lt;\|&gt;\|&quot;'` 0 命中（图上没有 `&`/`<`/`>` 字样，标识符里的引号直接用 `"`）✓
  - `xml:space="preserve"`：未复刻列对齐文本（无 `^^^`/`|`/`---` 栏目）→ 不适用 ✓
  - 字号：标注 12–13、栏目标题 13（在 §9 的 12–14 内）；线宽 1.5–2；圆角 `rx="5"` ✓
- 自验命令与结果：`mdbook test -c ch26-ffi-abi.md` → `INFO Testing chapter '第 26 章 FFI 与 ABI': "ch26-ffi-abi.md"`，`EXIT=0`（绿）。
- 目视（栅格化）：从在库文件抽块生成 `.scratch/tmp-60/fig-render-check.svg`（命令：`sed 's/var(--links)/#20609f/g; s/var(--warning-border)/#ff8e00/g'`，两值取自本仓 `book/html/css/variables-8daf115d.css` 明主题；几何与在库版本逐字一致，仅颜色换字面量）→ `read …:img`。目视结论：三行互不重叠、各处文字从左右两端都未溢出 viewBox、边界虚线不穿字、abort ✗ 落在虚线上、两处方框与三条箭头位置正确。
  - 探针留痕（已随目录删除）：`.scratch/tmp-60/probe-path.svg` 实测本机栅格化器（resvg 路径）**不解析 `stroke="var(...)"`**（同文件里 `stroke="var(--warning-border)"` 的线与 ✗ 不渲染，`fill="var(...)"` 的文字正常），所以目视必须用字面量替换版；mdBook 输出在浏览器里按主题变量正常渲染（本仓既有图同样用 `stroke="var(--warning-border)"`，例：`src/ch02-reading-the-compiler.md:498`、`src/ch01-getting-started.md:271`）。这不是交付物缺陷。
- `text` 草图处置（票要求的论证）：26.6 内**没有** ```` ```text ```` 草图，故无可删项。全章 6 个 `text` 块都在「动手看看」之后（行 333/356/460/537/564/585，均为程序输出/目录类内容），全部保留。
- 遗留不确定：
  1. 图按票 44 的示意把「两侧之间隔着的 C 帧」抽象成一道 `extern "C"` ABI 缝（引出句已注明），没有画显式 C 帧；如需显式绘制，本图宽度已到 §9 建议上限 760，需要重排或改票。
  2. 本机 resvg 对 `var()` 在 `stroke` 上的解析缺陷（见上）只能靠浏览器确认最终配色；收尾 build 时可在 HTML 里核对强调色（`--links`）与警示色（`--warning-border`）是否按预期出现。
- 新增术语：无（图内用词沿用正文：只用 `catch_unwind`、`extern "C"`、`extern "C-unwind"`、abort）。
- 衔接提示：后续章节如需引用本图，可说「第 26 章 26.6 的三行示意图」；图内「外层 Rust 帧」与 26.6 事实二的「边界之内」、事实三的「那边（外层）」措辞并不冲突。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch26-ffi-abi.md` 内嵌 1 张（FFI 边界与 panic 三防线，与 26.6 三条事实一一对应），插在 26.6。
- 自验：`mdbook test -c` exit 0；§9 自查全过；栅格化目视；协调者收尾浏览器复核（HTML 计数 ch26=1）。
- `text` 块审计：26.6 内无 text 草图，0 删。
- 全量链（批次级）全绿。

