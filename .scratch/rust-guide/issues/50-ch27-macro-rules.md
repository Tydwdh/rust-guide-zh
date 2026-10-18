# 50 第 27 章 宏：macro_rules!

Type: task
Status: resolved

## Question

写第五篇第一章：**宏：`macro_rules!`**。

- **范围**：声明宏的基本形态（匹配臂与展开）；片段说明符（`expr`/`ty`/`ident`/`tt`/`pat`/`stmt`/`item`/`path`/`literal`/`meta`）与选择原则；**Edition 2024 的片段匹配变化**（`expr`/`pat`/`stmt` 片段在新 edition 的匹配差异——以 Edition Guide 逐条核对）；卫生性（值/类型/生命周期卫生的边界）；`$crate` 与跨 crate 展开；`#[macro_export]` 与作用域（`#[macro_use]` vs 路径调用）；递归宏；调试（`cargo expand` 作为外部工具说明性引用）。
- **素材**：`research/02-knowledge-structure.md` 的 I17 行（依赖 B2/I3；`reference/macros-by-example.html`、书 ch20.5、`rust-by-example`）与 Edition Guide；本机 1.98 实测。
- **边界**：过程宏 → 第 28 章；宏与 Cargo/build → 第 30 章；`cfg` 条件编译 → 第 30 章（回指）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；nightly-only 只标注。
- **DoD**：§8 协议 —— 独占 `src/ch27-macro-rules.md` + `snippets/ch27/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch27-macro-rules.md` 绿；探针 `.scratch/tmp-50/`；收尾 `Status: claimed` + `## Comments`（§8 清单）。

## Comments

### 0. 状态与交付物

- `Status: claimed`（2026-10-19）。
- **只改动了票内文件**：`src/ch27-macro-rules.md`（950 行）、`snippets/ch27/`（7 个新文件）、本票。未动 `SUMMARY.md`/`book.toml`/`WRITING.md`/`GLOSSARY.md`/地图/其它票；未跑 `git`、未跑 `mdbook build`。
- 章模板八节齐全：这一章解决什么问题 / 为什么这样设计 / 机制与规则（27.1–27.9）/ 动手看看 / 报错现场 / 小结 / 练习（3 题带答案）/ 延伸阅读。H1 保持 `# 第 27 章 宏：macro_rules!`，与 SUMMARY 一致。

### 1. 自验命令与结果

| 命令 | 结果 |
|---|---|
| `mdbook test -c ch27-macro-rules.md` | `EXIT=0`（绿；最后一次运行在正文精简与补 include 之后） |
| 负对照 1：正文末尾临时塞入 `let _: i32 = "x";` | `exit 101` + `error[E0308]` → 已删除并复跑绿（证明本章确实被测） |
| 负对照 2：`snippets/ch27/edition2024.rs` 临时追加同样错误 | `exit 101`（2 处 `E0308`）→ 还原后复跑 `EXIT=0`（证明 include 的片段确实被编译） |
| 7 个 snippet 各自 `rustc --edition 2024 -o x.exe` + 运行 | 全部编译通过，输出与正文 console 块逐字一致 |
| 跨 crate 探针 `rustc --crate-type=lib` / `--extern` + 运行 | 见 §5，输出与正文逐字一致 |

**代码块构成**（脚本清点，栅栏全部闭合，共 52 块）：受测 `rust` 14 个（含 7 个 `{{#rustdoc_include}}` + 7 个内联/整块例子）、`rust,compile_fail` 10 个、`rust,ignore` 1 个（`user.rs`：引用外部 crate `helper`，单文件编不过）、`text`/`console` 27 个（非 Rust 片段，均已标注语言）。

### 2. §1 编译真实性

- 正例：7 个 snippet 全部以 `{{#rustdoc_include ../snippets/ch27/*.rs}}` 写在栅栏内接入，`mdbook test` 真编译真运行：
  `arms.rs`（匹配臂/展开/定界符/关键字 ident）、`specifiers.rs`（各说明符 + `map!`）、`edition2024.rs`（`expr` vs `expr_2021`、`pat` vs `pat_param`）、`hygiene.rs`（混合卫生四组对照）、`dollar_crate.rs`（`$crate`）、`scope.rs`（`#[macro_export]`/重导出/`#[macro_use] mod`/文本遮蔽路径）、`recursion.rs`（`count!`/`sum!`/`transpose!`）。
- 反例：10 处 `rust,compile_fail`，错误输出均来自 `.scratch/tmp-50/` 真跑后节选，行/列号与正文代码块逐行对齐（沿用 ch26 的 `--> src/main.rs:N:M` 写法）：follow set 违规、局部歧义（无 lookahead）、转写后解析失败不回退、跨调用不共享局部变量（E0425 + hygiene help）、`break 'outer`（E0426）、无限递归（recursion limit）、转发片段的不透明性、`($($p:pat)|+)`、缺片段说明符。
- 跨 crate 场景无法用单文件 doctest 表达，按 §1 用 `rust,ignore` 展示源码 + 真实 `rustc --extern` 运行输出（不冒充受测块）。

### 3. 版本口径

全部示例在 stable **1.98**（`rustc 1.98.0 (88d9e12ae 2026-08-18)`）、edition 2024 下实测；无 nightly 实跑。
nightly-only 内容只标注不实跑：
- `-Zunpretty=expanded`（正文写出 stable 被拒的原文并说明 cargo-expand 靠 `RUSTC_BOOTSTRAP=1`，见其 `src/main.rs`）；
- `macro_metavar_expr`（只在新术语/边界句里提一句并回指第 39 章；本机实测报 `error[E0658]: meta-variable expressions are unstable`）。

### 4. Edition 2024 逐条核对（Edition Guide 三页）

| Edition Guide 页面 | 结论 | 本机实测 |
|---|---|---|
| `rust-2024/macro-fragment-specifiers` | `expr` 现在也匹配 `const` 块与 `_` 表达式；新增 `expr_2021` 保留旧行为；迁移 lint `edition_2024_expr_fragment_specifier` | 同一份 `which!/under!/which2021!` 分别用 2018/2021/2024 编译：仅 2024 走 `expr 臂`；`expr_2021` 三个 edition 都是 `const 臂` |
| `rust-2024/missing-macro-fragment-specifiers` | `missing_fragment_specifier` 由 lint 变硬错误；NOTE：1.89 起对**所有** edition 生效 | 1.98 在 2021 与 2024 下均报 `error: missing fragment specifier`（硬错误） |
| `rust-2021/or-patterns-macro-rules` | **`pat` 的匹配变化发生在 2021**：`$_:pat` 起接受顶层或模式，`$_:pat_param` 保留旧行为；相关性别的判定按**定义宏的那个 crate** | 2018 报 `no rules expected '|'`；2021/2024 同一源码通过并打印 `Some(x) | None` |

**票面对 `pat`/`stmt` 的预设经核对不成立，正文按事实写并纠偏**：
- `pat` 的变化属于 **2021**，2024 版 Edition Guide 未改 `pat`；2024 只改了 `expr`。
- `stmt` **没有** edition 差异（三本 Edition Guide 没有对应条目；2018/2021/2024 实测输出一字不差）。
- 2024 与声明宏直接相关的 Edition Guide 页面只有上述两条（另有 2021 的 `or-patterns-macro-rules`）。

### 5. 本机实测输出清单（正文里的 console/text 块）

- `arms.rs` 9 行输出；`specifiers.rs` 12 行（含 `map! 造出 2 条记录`）；`edition2024.rs` 6 行；`hygiene.rs` 8 行（含 `passthrough 走了一遍调用处的 'a: 7`）；`dollar_crate.rs` 3 行；`scope.rs` 6 行；`recursion.rs` 3 行（`transpose!(a, b; c, d) = ((1, 3), (2, 4))`）。
- 三 edition 对照表（2018/2021/2024 × 三个调用）。
- 跨 crate 实录（`ch/helper.rs` + `ch/user.rs`）：
  `rustc --edition 2024 --crate-type=lib --crate-name helper helper.rs -o libhelper.rlib` →
  `rustc --edition 2024 --extern helper=libhelper.rlib -L . -o user.exe user.rs` →
  `.\user.exe` 输出 `调用处的 helper` / `helper 宏` / `helper 1.0`。
  对照失败版：`cannot find macro 'helper' in this scope`（含 `helper is in scope, but it is a crate, not a macro`）与 `error[E0603]: function 'priv_fn' is private`。
- `#[macro_use] extern crate helper;` 版本实测通过。
- `RUSTC_BOOTSTRAP=1 rustc -Zunpretty=expanded` 对 `first_x!` 例子的展开文本（`fn f() -> i32 { let x = 1; ... let x = 2; x + x }`）。
- **未实测、也不冒充**：`cargo expand` 本机未安装，正文只用其 README 原话（包装 `cargo rustc --profile=check -- -Zunpretty=expanded`、免责声明「lossy」）与本机对 `-Zunpretty=expanded` 的真实反馈，不贴伪造的 `cargo expand` 输出。

### 6. 素材与引文来源

- Rust Reference `macros-by-example.html`：定义句、匹配/转写、无 lookahead、外层定界符与 `$`、转发片段的不透明性、片段说明符全表、follow set 名单、重复与两条转写约束、文本/路径作用域与查找顺序、遮蔽、隐式 `pub(crate)`、`macro_use`、`macro_export`、`local_inner_macros`、混合卫生、`$crate`（全路径要求 + 不绕过可见性）。
- Rust Reference `attributes/limits.html`：`recursion_limit` 语义与「The default in rustc is 128」。
- Edition Guide：上述三页（引文逐字）。
- The Book `ch20-05-macros.html`（`#[macro_export]` 的作用、宏 vs 函数）。
- Rust by Example `macros.html`（延伸阅读）。
- std 源码 `library/core/src/macros/mod.rs`：`matches!` 用 `$pattern:pat`；`assert_matches!` 用 `$( $pattern:pat_param )|+`（用来佐证「需要紧跟 `|` 时用 `pat_param`」）。
- cargo-expand README 与其 `src/main.rs`（`RUSTC_BOOTSTRAP` 处理）。
- 英文引文逐字复制并标「引自/译自 xxx」；中文转述处写「译自」。

### 7. 遗留不确定

- `literal` 片段吃下 `-7`（带负号）为本机 1.98 实测；Reference 的措辞是 `-`?LiteralExpression，负号与元变量绑定的细节未再深究。
- `cargo expand` 在 stable 上的可用性只能从其源码（设置 `RUSTC_BOOTSTRAP=1`）与 README 推断，本机未安装实跑。
- 篇幅：正文 **5448 汉字（不含代码块）**，落在 4000–5500 内但贴近上限；首次成稿曾达 ~6900，已按票面要求做了多轮压缩（删冗余叙述、把多 edition 对照改成表格、压缩小结/练习答案），未删任何范围要点。审校修复新增了少量必要文字（`show!` 定义说明、误差输出补全、注记），已同步再压一轮。
- 报错输出的 `--> src/main.rs:N:M` 是仓库既有写法（ch26）下的路径归一化；行号与本章代码块严格对应，但读者若把代码放进别的文件名，行号会随布局变化。

### 8. 新增术语（请协调者统一并表到 GLOSSARY.md）

- **卫生性（hygiene）**：宏展开中名字解析的边界；「混合卫生（mixed-site hygiene）」是 Reference 用语。
- **片段说明符（fragment specifier）**：`$name:spec` 里 `spec` 那一半（`expr`/`ty`/`tt`…）。
- **元变量（metavariable）**：匹配器里 `$name` 绑定的片段。
- **匹配器（matcher）/ 转写器（transcriber）**：一条规则的左右两半。
- **文本作用域（textual scope）/ 路径作用域（path-based scope）**：宏的两套作用域。
- **跟随集合（follow set）**：片段后允许出现的 token 白名单。
- **TT muncher**：靠 `tt` 递归吃 token 的递归宏形态（保留英文）。
- **宏按例（macros by example）**：`macro_rules!` 的官方别称，本章首现与「声明宏」并列。
- 与现有条目一致：`宏（macro）` 按 GLOSSARY 指 `macro_rules!` 声明宏，过程宏单列。

### 9. 衔接提示（给相邻章与协调者）

- **第 28 章（过程宏）**：本章在 `$crate`、`#[macro_export]` 作用域、卫生性三处都止步于「声明宏怎么做」，并把「过程宏要自己处理名字与 `Span`」留成钩子；本章不出现 `syn`/`quote`/`proc_macro`，避免与 28 章重复。
- **第 30 章（Cargo/build）**：`cfg` 与宏的组合、构建脚本生成代码只回指，未展开。
- **第 33 章（Edition 迁移）**：`cargo fix --edition` 的 `edition_2024_expr_fragment_specifier` 与 `missing_fragment_specifier` 迁移（1.89 起全 edition 硬错误）可直接复用。
- **第 39 章（宏系统极限）**：`macro_metavar_expr` 等 nightly 特性已在此回指。
- 第 5 章（模式）：`pat`/`pat_param` 的模式语法细节回指第 5 章。

### 10. 图示需求

本章**未**插入内嵌 SVG：说明符、follow set、作用域遮蔽、卫生性对照都适合表格/列表，属 §9 里「一张表或三行文字能说清的」那一类。
若后续要补图（可选），建议一张单向流程图：`调用点 token 流 → 逐臂匹配（第一处胜出、不回溯） → 选中臂 → 转写（元变量替换） → 新 token 序列交给名字解析`；不建议画「卫生性解析位置」图（表格更清楚）。

### 11. 审校回执（RvCh27）处理记录 —— 4 major + 8 minor 全部采纳

复跑证据：`mdbook test -c ch27-macro-rules.md` → `EXIT=0`；代码块 52 块（`rust` 14 / `rust,compile_fail` 10 / `rust,ignore` 1 / `text` 17 / `console` 10），栅栏全闭合；正文 5448 汉字（不含代码块）。本轮回执新增/改动过的证据都在 `.scratch/tmp-50/` 真跑后删除。

- **F1（major）采纳**：以正文那份 `leave!` 代码块原样复跑，实际是 `--> src/main.rs:3:15` 与 `8 |         leave!();`（原先 4/10 来自带注释行的旧探针）。已改。
- **F2（major）采纳**：练习 1 实跑给**两**个错误 + `error: aborting due to 2 previous errors`。已补第二段 `error: expected pattern, found '='`（`--> src/main.rs:2:24`，含 `let _ = pick!(1);` 的 invocation 行）并把计数改成 2。
- **F3（major）采纳**：`scope.rs` 里确实没有 `deep` 模块（那是我早期草稿探针的残留）。已改为「所以在任何模块里都能按 `crate::shout!(...)` 调用（例子里就是在 `main` 中调）」。
- **F4（major）采纳**：`show!` 在原稿中没有定义。已在 27.4 的 `stmt` 段落补一个受测 `rust` 块（6 行：`macro_rules! show` + 三次调用），紧跟其真实输出；审校自建定义后的三 edition 输出与此一致。
- **F5（minor）采纳**：`cannot find macro 'helper'` 与 `E0603` 两块的定位行来自两 crate 探针、文件名与本章不同，已在引导句统一加注「（以下两处是两 crate 探针的关键行节选，已略去 `-->` 定位行）」；`cannot find macro 'm'` 块则按真实输出补全两条 `-->` 行（`src/main.rs:2:5` 与 note 的 `src/main.rs:5:14`）。
- **F6（minor）采纳**：把文件命名成 `main.rs` 复跑，help 行逐字为 `` consider increasing the recursion limit by adding a `#![recursion_limit = "256"]` attribute to your crate (`main`) ``，已补 `` (`main`) ``。
- **F7（minor）分两半**：① 采纳——我先前的 `-Zunpretty=expanded` 探针源码里多写了空行，正文因此多了两处空行；用**本章代码块原样格式**（无空行）复跑，展开输出确无空行，已删。② 采纳——归属改为「练习 3(b) 的 `fn f()`（与 cargo-expand README 示例同形）」，不再说成 27.5 的例子。
- **F8（minor）采纳**：把练习 2 的 `log_all!` 换成 `tt` 转发后复跑，三行为 `zero 臂` / `zero 臂` / `expr 臂: 1`（先前列成两行是因为我的探针只调了一次 `log_all_tt!`）。正文已改精确。
- **F9（minor）采纳**：27.6 的两 crate console 块后加注「（编译 `helper.rs` 会多一条 `priv_fn` 未使用的 dead_code 警告，与本节无关，已略去。）」。
- **F10（minor）采纳**：`block` 行改为「块表达式（不收内部属性）」，对齐 Reference 的 `BlockExpressionNoInnerAttributes`。
- **F11（minor）采纳（选推荐方案）**：两处 `helper.rs` 由 `rust,ignore` 改为 `rust`，现在同样被 `mdbook test` 编译；负对照（在其中塞 `let _: i32 = "x"`）→ `exit 101` + `E0308`，证明它们真在受测集合里。`user.rs` 块保持 `rust,ignore`（引用外部 crate `helper`，单文件编不过），与 §1 硬规则一致。
- **F12（minor）采纳**：TT muncher 定义补「（`$head:tt $($rest:tt)*`，可以按需收窄成 `ident`）」，与 `count!` 用 `ident` 一致；基例描述改为「直到 `$($rest)*` 空掉、命中基例 `(@acc $n:expr;)`，此时 `$n` 已经累加成一串 `0 + 1 + ...`」，不再说「只剩 `0`」。 

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch27-macro-rules.md`（八节模板；14 真编译 + 10 `compile_fail` + 1 `ignore` 经审校后）+ `snippets/ch27/`（7 个受测文件）。
- 关键事实（实测 + 逐条核对 Edition Guide）：2024 只改 `expr` 片段（`expr_2021` 保留旧行为）、`pat` 变化在 2021、`stmt` 无 edition 差异、`missing_fragment_specifier` 1.89 起全 edition 硬错误；`$crate` 跨 crate 用 `rustc --extern` 实跑。
- 自验：`mdbook test -c ch27-macro-rules.md` exit 0；反假绿双负对照（正文块与 snippet 各注入类型错误 → 101）。
- 交叉审校 RvCh27（4 major + 8 minor）全修：行号对齐、练习 1 补第二错误、`deep` 草稿残留、`show!` 定义补全、两块 `ignore` 升格受测等。
- 术语并表（协调者）：宏系统 11 条入 GLOSSARY（卫生性/片段说明符/元变量/跟随集合/TT muncher 等）。
- 全量链（批次级）：build 0 ERROR/0 警告；`mdbook test` 33 章 exit 0；examples 全 crate 绿。
- 遗留：`cargo expand` 未实跑（本机无该二进制，只引 README/源码）。
