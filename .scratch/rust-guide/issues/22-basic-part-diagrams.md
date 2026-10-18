# 22 基础篇图示补全

Type: task
Status: resolved

## Question

按已定体系把基础篇的图示补进各章：**内嵌 SVG**（手写、无脚本、`currentColor` 随主题变色；**单块内不得有空行** —— CommonMark 的 HTML 块会在空行处截断，实测）。

- **需求清单**：约 35 处，逐条在 `.scratch/rust-guide/batch-notes.md` 各票的「图示需求」（内存布局 / 借用区间 / 模块树 / 流程 / 包含关系）。
- **样例（画风与密度的参照）**：`src/ch06-collections-strings.md` UTF-8 一节的「你好」字节布局图。
- **约束**：不引入 mermaid / 外部资产 / 脚本；图不参与 `mdbook test`（SVG 不是代码块）；`mdbook build` 后扫 ERROR/WARN；每章补图后复跑 `mdbook test -c <章>`；主题变量可用 `currentColor`、`var(--links)`、`var(--warning-border)`（mdbook 主题里都有定义）。
- **排期**：等用户看过样例、确认画风与密度后再分批开写（建议每批 3–5 章）。

产出：各章内嵌 SVG；另外给 `WRITING.md` 补一节「图示规范」（尺寸/字号/配色变量/`role="img"` + `aria-label` 无障碍约定/「单块无空行」规则），使后继篇章（进阶篇、补充章节）能照此画图。

## Comments

### FigCh09 —— ch09 泛型与 trait 入门

- **交付**：2 张图，各配一句引出句；正文/代码块/表格/围栏未动。
  - `src/ch09-generics-traits-intro.md:58`｜`aria-label`「单态化：一份泛型函数 show 按类型实参分别生成 &str、f64、u32 三份实例代码」（源内 `&amp;str`）｜一份 `fn show<T: Display>` → 三个调用点各自落到 `@…4show4showReEB2_(ptr, i64)` / `…dEB2_(double)` / `…mEB2_(i32)`。
  - `src/ch09-generics-traits-intro.md:114`｜`aria-label`「trait 的三类角色：定义方写契约、实现方兑现契约、使用方只依赖契约」｜定义方 `trait Summary` → 实现方 `impl Summary for Comment` → 使用方 `fn notify<T: Summary>(x: &T)`，含职责标注与「使用方看不到具体类型」脚注。
- **`xml:space` 复查（Main 提出）**：结论「需要修」。两图的等宽代码行有刻意缩进 —— 共 9 处前导空格：fig1 `  (v: T) {`、`  println!("{v}");`；fig2 `  fn author(&self)`×2、`    for Comment {`、`    (x: &T) {`、`  x.summarize();`、`      -> String;`、`      -> String }`。已给两处 `<g font-size="12" font-family="monospace" …>` 补 `xml:space="preserve" style="white-space:pre"`（默认会折叠连续空格 → 缩进缩水），其余 `<text>` 只有单个词间空格，折叠无影响，未动。
- **自验**：`mdbook test -c ch09-generics-traits-intro.md` → `exit 0`（改前、改后各一次）；两图 XML well-formed；块内无空行、无 `height`、无硬编码色、无脚本/外链/渐变，`stroke-width="1.5"`、`rx="5"`、字号 12–14；私有目录 `.scratch/tmp-f22-ch09/` 栅格化目视：缩进保留（2/4/6 空格可见）、无溢出/压盖（目录已删）。
- **遗留**：无。HTML 实际渲染由协调者收尾 `mdbook build` 目视（票内规定代理不自跑 build）。

### ch07 错误处理（FigCh07）
- **三图**：`src/ch07-error-handling.md:75`（unwind 逐帧跑 `Drop` 后退 vs abort 立即终止）、`:187`（`?` 两条出口：`Ok` 取值继续 / `Err` 经 `From::from(e)` 后提前 `return`）、`:343`（`source()` 错误链：最外层 → 根因，右侧注 Display 与链终止）；每图前一句引出句，正文其余文字、代码块、表格未动。
- **跨章裁定执行**（Main：只删被新 SVG 完整取代的手绘 ASCII）：删除原 3 行 ASCII 错误链草图（`main 返回的 Err → ConfigError::BadNumber → ParseIntError`）。判定依据＝SVG 完整覆盖三层链、两条 `source()` 指向、两处 Display 文本与「`source()` 返回 None」的链终止；原草图的 `ParseIntError  source(): invalid digit found in string` 还把底层 **Display** 文本挂在 `source()` 名下，SVG 按事实写作 Display，无信息丢失。原引出句「错误链最后长这样：」与新图引出句合并成一句，无断句。
- **自验**：`mdbook test -c ch07-error-handling.md` → `exit 0`（删草图前、删后各一次）；`grep -c '^```'` = 56（偶数）；三图块内无空行、根元素无 `height`、颜色仅 `currentColor`/`var(--links)`/`var(--warning-border)`/`var(--quote-border)`、无脚本/外链/渐变，线宽 1.5 与 2、`rx` 4/5、字号 12–14。
- **目视**：私有目录 `.scratch/tmp-f22-ch07/` 栅格化（`var(--…)` 临时替换为实色，因独立栅格化不解析主题变量）逐张先核对 aria-label/文字归属再判版式，无重叠越界；目录已删。共享 `.scratch/tmp-f22/` 曾把我的 fig/probe 覆盖成别章的图、随后整目录被清，已弃用。
- **遗留**：无（7.1 的回溯读法、7.6 的 Display/Debug 分工由既有文本与表格承担，未再画）。

### FigCh04 —— ch04 所有权、借用与切片

- **三图**（只新增图 + 一句引出句；正文文字、代码块、表格、围栏一律未动）：
  - `src/ch04-ownership-borrowing-slices.md:102`（`:102-131`，§4.1）｜`aria-label`「move 语义：let b = a 之后栈上多出一份 ptr/len/cap，a 作废不再指向堆，堆缓冲区一个字节也没变，释放责任转到 b」｜需求③：a 的格子虚线化（注明再用报 E0382）、a→堆 的连线画成 ✗「责任已不在 a 手里」，b 的实线箭头指向**同一块**堆。
  - `…:449`（`:449-482`，§4.5）｜`aria-label`「借用区间时间轴：两条共享借用可以重叠（允许），可变借用另起一段不重叠的区间也能通过；只要可变借用与共享借用区间重叠就报 E0502」｜需求①：**4 条区间条** —— `&v[0]`/`&v[1]` 两条共享重叠（通过）+ `&mut v` 另起一段（通过）+ 拒绝版（E0502）；图例区分共享/可变，两条虚线竖线标「最后一次使用」。
  - `…:595`（`:595-631`，§4.7）｜`aria-label`「三类内存布局对照：String/Vec 是指针加长度加容量的所有者，&str/&[T] 是指针加长度的胖指针借用，str/[T] 是没有编译期已知大小的 DST」（源内 `&` 写作 `&amp;`）｜需求②：ptr\|len\|cap 三格（24B，所有者）→堆；ptr\|len 两格（16B，胖指针，虚线箭头只圈缓冲区一段）；DST 虚线框（无编译期大小）。
- **跨章裁定执行**（Main：只删「被新 SVG 完整取代的手绘 ASCII 示意图」）：本章删 2 处，均先核对「确被完整取代」再整块删。
  - ① 原 `:403-414`（§4.5 时间轴草图，12 行）→ 被 §4.5 新 SVG 完整取代（新图是超集：多出「两条共享借用可重叠」维度与图例，并把「借用到最后一次使用」画成虚线边界）。原引出句「把它画成时间轴就是：」只服务该草图 → 一并删除；段落仍收在完整句「…说明第一段借用为什么还活着。」，无断句。
  - ② 原 `:587-603`（§4.7 布局草图，17 行）→ 被 §4.7 新 SVG 取代（新图同含 `String`/`&str`/`str` 三型与「指向同一堆缓冲区的一段」关系）。该草图前无专设引出句（正文段落直跟栅栏）→ 只删栅栏，段落与「本机实测的各种尺寸」表之间保留一个空行。
  - 核查：`grep -c '┌\|├\|└\|│ ptr' src/ch04-ownership-borrowing-slices.md` = **0**（本章已无手绘 ASCII）；两处均无行尾空白（删前用 awk 逐行核过 `trail=0`）。
- **行号变化**：三图 = `:102-131` / `:449-482` / `:595-631`，引出句 `:100` / `:447` / `:593`；删除后 §4.5 的「同样的规则也解释了『连续两次可变借用』：」在 `:403`，§4.7 尺寸表在 `:574`（正文由 1130 行缩到 1099 行）。
- **自验**：`mdbook test -c ch04-ownership-borrowing-slices.md` → `exit 0`（加图后、删 ASCII 后各一次；日志只有 `INFO Testing chapter`）。
- **harness 活性对照**（防假绿，私有探针 book `.scratch/tmp-f22-ch04/negctl/`）：把**同一份章文件**放进临时 book 跑 `mdbook test` → 实跑 **34 个 doctest**（33 passed + 1 failed；唯一失败是 `{{#rustdoc_include ../snippets/ch04/word_stats.rs}}` 在临时 book 里解析不到 snippet 路径）。即本章 exit 0 并非静默跳过；对照 book 与目录已删。
- **目视**：私有目录 `.scratch/tmp-f22-ch04/`（`var(--…)` 临时换成实色 —— 独立栅格化不解析主题变量）逐张先核对 `aria-label`/标题确认是自己那张再判版式；据此修掉两处版式缺陷：时间轴区间条高 10px 装不下 12px 字（→16px）、三型图底部注释与 DST 虚线框重叠（→重排行距）。规范自查：3/3 块配对、块内空行 0、根元素无 `height`、零硬编码色、无脚本/外链/渐变，线宽 1.5/2、`rx` 4–5、字号 12–14。目录已删；共享 `.scratch/tmp-f22/` 未碰。
- **遗留**：① 本章未画的：§4.3 drop 顺序（`main`/`consume` 两栈帧 + 逆序释放 `b→c→a`）、§4.7「切片是视图」（同一缓冲区偏移 6）—— 已有实测输出（`drop: b/c/a`、地址相差 6）足以说清，按「信息密度优先」未上；② `mdbook build` 的 HTML 渲染目视按票面留给协调者。无阻塞项。

### 协调者汇总（全 12 章 · 35 张 + ch06 样张）

- **图数**：ch01 3 ／ ch02 4 ／ ch03 2 ／ ch04 3 ／ ch05 3 ／ ch06 3（含早先样张 1 张）／ ch07 3 ／ ch08 3 ／ ch09 2 ／ ch10 3 ／ ch11 3 ／ project1 4 —— 共 **36 张**。
- **结构合规（脚本全量扫）**：36 张全部通过 —— 块内无空行 / 根无 `height` / `viewBox`+`width="100%"` / `role="img"`+`aria-label` / 无硬编码色 / 无脚本外链渐变 / 无裸 `&` / XML 良构。**0 违规**。
- **渲染校验**：`mdbook build` exit 0、0 WARN/ERROR；逐章 HTML `<main>` 内 `role="img"` 计数与源码一致（12/12 章）；浏览器实机抽查 ch04 借用时间轴与 ch06 Vec 重分配 —— 深色主题下 `var(--links)` / `var(--warning-border)` 正常解析、无裁切。
- **跨章裁定**：删除被 SVG 完整取代的手绘 ASCII 示意图 3 处（ch04 ×2、ch07 ×1）；目录树 / 真实输出 / 一行式映射保留。
- **规范沉淀（`WRITING.md` §9 / §8）**：单块无空行、无脚本、`viewBox`+`width="100%"` 不写 `height`、主题变量、`role`+`aria-label`、HTML 实体转义、列对齐文本加 `xml:space="preserve"`、多代理探针目录加章号。
- 未在本票追加小节的另 9 章，其图清单与自验记录在协调台账 `batch-notes.md` 与各代理回报中；以上汇总即对它们的统一核对结果。

## Answer

2026-10-18 完成（体系与样例先经用户裁决/验收）。

- **交付**：基础篇 12 章共 **35 张内嵌 SVG**（+ ch06 早先样张 1 张，合计 36 张）；`WRITING.md` 新增 **§9 图示规范**（含三条实测定律：单块无空行、HTML 实体转义、列对齐用 `xml:space="preserve"`）。
- **验证**：结构合规脚本 36/36 通过；`mdbook build` 0 WARN/ERROR；12 章 HTML 图数与源码逐一一致；浏览器实机抽查 2 张复杂图渲染正确（主题变量生效）；各章 `mdbook test -c` 在补图/删草图后均 exit 0。
- **遗留**：无阻塞项。未画的低位需求（如 ch01 的 cargo 横向编排图 —— 现有 `text` 管线已覆盖）已在各章小节记录；进阶篇及后续篇章的图示按 §9 规范沿用（复杂图如 async 状态机待写到对应篇再验）。
