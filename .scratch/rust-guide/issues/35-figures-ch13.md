# 35 进阶篇图示：第 13 章（dyn 兼容性与 trait 对象）

Type: task
Status: resolved

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

- 2026-10-19 三图全部落地；只动 `src/ch13-dyn-trait-objects.md`（三处：两处 ASCII 草图换成 SVG、一处插入图 + 引出句）与本票。行号以 716 行的成稿为准。
- **图 1（清单第 1 项，最高优先）胖指针 + vtable**
  - 位置：`### vtable 与三种持有形式`（小节标题现第 195 行），SVG 在第 229–272 行；图前引出句沿用小节原有那句「……这种指针是**胖指针**——两个机器字，一个指向数据，一个指向 vtable：」，未改一字。
  - aria-label：`胖指针与 vtable 布局：&dyn Shape 的两个机器字分别指向具体值和该类型的 vtable，Circle 与 Square 各有一张表，表头是元数据、后面每个可分发方法一格`（源码中 `&` 写作 `&amp;`）。
  - 内容：`&dyn Shape`（16 字节 = 两个 8 字节机器字）拆成 data / vtable 两格；data → `Circle { x, y }`，vtable → Circle 的表（表头「头部元数据：size / align / drop（1.98 实测 24 字节）」+ `area → Circle::area`、`name → Circle::name`）；第二行同一指针形状指向 `Square { side }` 与 Square 的另一张表，落实「每个具体类型一份 vtable」。
  - **删除论证**：原小节 ASCII 草图（`&dyn Shape` / `Circle{x, y}` / 头部元数据 24 字节 / `area` / `name`）被本图逐项覆盖（含逐字保留的「1.98 实测 24 字节」），且本图多出第二个具体类型的独立表 → 判完整覆盖，按票删除该 ```text 块；其后正文整段（「vtable 里除了……」）未动。
- **图 2（清单第 2 项）泛型 vs dyn 代码路径对照**
  - 位置：`### 两条多态路线的账`（标题第 30 行），SVG 在第 34–70 行；引出句用小节原有那句「第 9 章已经量过泛型那条路：……但它拿到的指针是「两只手」：」，未改。
  - aria-label：`泛型与 dyn 两条代码路径对照：单态化按类型生成 show 的三份实例、调用点直接跳转且可内联；dyn 只有一份 show_dyn，收到 data 与 vtable 两个指针，每次调用经 vtable 取槽后间接跳转，挡住内联`。
  - 内容：左 = `show::<u32>` / `show::<&str>` / `show::<f64>` 三份机器码（括号标注「3 份机器码 / 每个类型一份」）+「✓ 直接调用：跳转目标编译期定死，可内联」；右 = `show_dyn(&dyn Display)`（沿用本小节 `show_dyn` 命名，与第 9 章口径一致）+ 胖指针两行（`data 指针 → 实际值`、`vtable 指针 → 表`）+ `取槽 → 实际实现`（间接调用）+「✗ vtable 取址 + 间接跳转，挡住内联」；两栏各带一行代价注（代码体积/编译时间 vs 每次调用多一次取址）。
  - **删除论证**：原 5 行 ASCII 的全部信息（三种类型实参各一份代码、直接调用可内联、一份函数体、`data 指针 → 值`、`vtable 指针 → 方法槽 → 实际实现`、间接调用无法内联）逐条在图中有对应元素，`show_dyn(&dyn Display)` 亦原样保留 → 判完整覆盖，删除该 ```text 块；小节其余文字未动。
- **图 3（清单第 3 项）向上转型**
  - 位置：`### 向上转型（1.86 起）`（标题第 343 行），SVG 在第 351–394 行，插在 Reference 引文之后、版本段之前；引出句为本票唯一新增正文文字：「下图把这次转型摊开：两处 `data` 指向同一个值，换掉的只是 vtable 视图。」
  - aria-label：`向上转型的两步：dyn Pet 与 dyn Animal 共用同一个数据指针，只把 vtable 换成父 trait 的视图；横向转到无关 trait 不合法，报 E0605`。
  - 内容：上格 `&dyn Pet` → data（`var(--links)` 蓝线）→ `Dog`，vtable →「Pet 视图：name + owner」（`name → Dog::name`、`owner → Dog::owner`）；下格 `&dyn Animal` → data（同色）→ 另一个 `Dog`，vtable →「Animal 视图：只有 name」（附「owner 不在这个视图里」）；两处 data 同色并加注「两处 data 指向同一个值：指针逐位不变」，落实「数据指针不变、只换 vtable 视图」；底部两条「✓ 只沿父 trait 方向：dyn Pet → dyn Animal（1.86 起）」与「✗ 横向不合法：&dyn Shape as &dyn Named → E0605」（与本节第三条补充一致）。
- 自查（三图同口径，脚本 + 人工核）：块内无空行（整图为单块）✓；无 `<script>` / 无外链 / 无 `url()` ✓；`viewBox` 定坐标 + `width="100%"`，`<svg>` 上无 `height` ✓（700×262、760×224、700×258，均在建议区间）；配色只取 `currentColor` / `var(--links)` / `var(--warning-border)` / `var(--quote-border)`，与全仓已发布 SVG 的取值集合比对无新增值、无写死颜色 ✓；`role="img"` + `aria-label` 齐备 ✓；`&`→`&amp;`、`<`/`>`→`&lt;`/`&gt;`（含 aria-label）✓；三块均能过 `xml.etree.ElementTree` 解析（标签闭合、无游离 `<`）✓；字号只用 12/13/14 ✓；无复刻工具输出的列对齐文本，故不需 `xml:space="preserve"`；图前后各留一个空行 ✓。
- 目视（栅格化）：本机 ImageMagick 的 `convert` 不能解主题变量、也未装 rsvg/cairosvg，改用无头 Chromium 渲染 `.scratch/tmp-35/preview.html`（同一份 SVG 分别放进 mdBook light 与 coal 两套真实主题变量上下文，正文栏宽 700px），三图 × light/coal 共 6 张截图逐张核对：文字无重叠、无越框，箭头落点正确，暗色下 `var(--links)` 蓝与 `var(--warning-border)` 橙均可辨认。结论：三图可读，与既有章节图表风格一致。**探针目录 `.scratch/tmp-35/` 已删**。
- 门禁：`mdbook test -c ch13-dyn-trait-objects.md` → `INFO Testing chapter '第 13 章 dyn 兼容性与 trait 对象': "ch13-dyn-trait-objects.md"`，`EXIT=0`（加图前后各跑一次均绿）；本章 ```text 块剩 6 个，仅两处 ASCII 草图被 SVG 取代，其余代码块未动。
- 遗留/提示：
  1. HTML 渲染（除 light/coal 外的其余主题、窄屏缩放）按 §8 留给协调者收尾 build 统一确认；本票只做了两套变量的栅格化目视。
  2. 图 2 中缝分隔线沿用第 10/11 章已发布图的口径 `stroke="var(--quote-border)"`：light 主题下该色极浅、近乎不可见（既有图表同况），未自创色值。
  3. 图 1 表头沿用「1.98 实测 24 字节」，与本章 `dispatch.ll` 摘录里的 `[24 x i8]`、`i64 24` 一致，系既有正文口径，未新增实测。
  4. 新增术语：无。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch13-dyn-trait-objects.md` 补 **3 张** —— 胖指针+vtable 布局（最高优先，:229–272）、泛型 vs dyn 代码路径（:34–70）、向上转型（:351–394，含「横向不合法 → E0605」）。
- 覆盖裁定：两处 ASCII 草图（vtable 段、两条多态路线）被新图逐项完整覆盖后删除（逐条论证记票）。
- 自验：`mdbook test -c ch13-dyn-trait-objects.md` exit 0；结构自查全过；**双主题（light/coal 真实变量）6 张截图**目视无重叠。
- 遗留：无（其余主题/窄屏渲染由协调者收尾 build 统一确认）。
