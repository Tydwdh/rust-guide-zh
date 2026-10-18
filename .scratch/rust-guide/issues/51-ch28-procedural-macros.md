# 51 第 28 章 过程宏

Type: task
Status: claimed

## Question

写第五篇第二章：**过程宏**。

- **范围**：三种形态（derive / attribute / function-like）；`proc_macro` 的 API 面（`TokenStream`/`Punct`/`Ident`/`Literal`/`Group`、`Span`、`TokenStream::from_str`）；在 `examples/macro-demo` 里实现一个**零依赖、手写解析**的最小 derive 宏（如 `#[derive(Describe)]` 生成 `fn describe(&self) -> String`，只支持命名结构体——不引 syn/quote）；在 `examples/macro-demo-use` 里消费并写测试；`syn`/`quote` 的取舍作为说明性对照（外部依赖，`ignore` 块）；错误报告（`compile_error!` 与 span 归属；`proc_macro::Diagnostic` 不稳定标注）。
- **素材**：`research/02-knowledge-structure.md` 的 I18 行（依赖 I17/I1；`reference/procedural-macros.html`、书 ch20.5、`std/proc_macro/`）；本机 1.98 实测（`cargo test -p macro-demo-use`）。
- **脚手架已就位**（协调者）：`examples/macro-demo`（`proc-macro = true` stub）与 `examples/macro-demo-use`（path 依赖）；workspace members 已配 —— **勿改 `examples/Cargo.toml`**；只写这两个 crate 的 `src/`/测试与它们自己的 `Cargo.toml`（如需 feature）。
- **章内代码**：过程宏实现代码一律 `rust,ignore`（`proc_macro` 只在 proc-macro crate 可用，doctest 编不了）；真实验证走 workspace 两 crate；消费端示例尽量用 `{{#include}}` 引 `macro-demo-use` 真实代码。
- **边界**：声明宏 → 第 27 章（回指）；宏在构建系统里的位置 → 第 30 章；诊断/span 的 rustc 内部 → 第 37 章（一句）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch28-procedural-macros.md` + `examples/macro-demo/**` + `examples/macro-demo-use/**` + 本票；禁 `git`/禁 build；自验：`CARGO_TARGET_DIR=.scratch/tmp-51/target cargo test --manifest-path examples/Cargo.toml -p macro-demo-use` 全绿 + `mdbook test -c ch28-procedural-macros.md` exit 0；探针 `.scratch/tmp-51/`；收尾 `claimed` + `## Comments`。

## Comments

（写作者收尾；未置 `resolved`，`## Answer` 留给协调者/用户。）

### 文件
- `src/ch28-procedural-macros.md`：新写成稿，695 行，八节模板齐（这一章解决什么问题 / 为什么这样设计 / 机制与规则 28.1–28.7 / 动手看看 / 报错现场 / 小结 / 练习 3 题带答案 / 延伸阅读）；正文汉字 5351（去代码块），带 1 张内嵌 SVG。
- `examples/macro-demo/src/lib.rs`（314 行，重写）：`#[proc_macro_derive(Describe)]`（具名字段结构体 → `fn describe(&self) -> String`）、`#[proc_macro_attribute] repeat`（复制项 + 改名，`Ident::new` + 沿用 span）、`#[proc_macro] sum`（编译期整数求和，`Literal::i64_unsuffixed`）；零依赖，全部手写解析 + 手拼 token。
- `examples/macro-demo-use/src/lib.rs`（84 行，重写）：`Describe` trait、`pub use macro_demo::{Describe, repeat, sum}`、3 个示例结构体、5 个测试（含泛型约束、空结构体、`Debug`-only 字段、repeat/sum）。
- 未改：`examples/Cargo.toml`、`examples/macro-demo/Cargo.toml`、`examples/macro-demo-use/Cargo.toml`（无需 feature）、`src/SUMMARY.md`。

### 自验命令与结果
1. `CARGO_TARGET_DIR=.scratch/tmp-51/target cargo test --manifest-path examples/Cargo.toml -p macro-demo-use` → **exit 0**，`test result: ok. 5 passed; 0 failed`（describe_lists_fields_in_declaration_order / describe_handles_empty_struct / describe_works_behind_a_generic_bound / repeat_copies_the_item_and_keeps_the_original / sum_is_evaluated_during_expansion）。
   - 注：编译 `macro-demo` 时 MSVC 链接器打印一条中文提示，触发 `#[warn(linker_messages)]`（本机环境噪声，与代码无关；正文未粘这条）。
2. `mdbook test -c ch28-procedural-macros.md` → **exit 0**。
   - 栅栏普查（脚本统计，含列表内缩进的块）：**35 块 = 22 rust（21 `ignore` + 1 真跑）+ 11 `text` + 1 `toml` + 1 `console`**。`mdbook test` 实际列出的 doctest 也是 22 个（rustdoc 分两批：`running 18` + `running 4`），其中 **21 ignored、1 真跑**（28.4「展开等价手写代码」，纯 `rust`，断言 `describe()` 输出）。
   - 门禁有效性实测：把该块断言改成 `"SENTINEL-WRONG"` → exit 101（rustdoc 报 assertion failed，两批分别 `0 passed; 1 failed; 17 ignored` 与 `4 ignored`）；恢复后 exit 0。故本章不是「零测试的假绿」。

### 实测过的输出清单（真跑真粘）
- derive 输入的 token 树 dump（探针 proc-macro + `println!`）：`Ident(struct) / Ident(Point) / Group(Brace) … Punct(':', Alone) … Punct(':', Joint) … Punct(''', Joint)`（1.98 本机，整段贴进 28.2）。
- 属性宏看到的 `attr`/`item`：`attr = 3` 与 `Ident(const) Ident(SCALE) Punct(':', Alone) Ident(i32) Punct('=', Alone) Literal(2) Punct(';', Alone)`（贴进 28.5）。
- proc-macro crate 导出非宏项：`error: \`proc-macro\` crate types currently cannot export any items other than functions tagged with …`（贴进「为什么这样设计」与练习答案）。
- 普通 lib crate 写 `#[proc_macro_derive]`：`the \`#[proc_macro_derive]\` attribute is only usable with crates of the \`proc-macro\` crate type` + `E0432: unresolved import \`proc_macro\``（含误导性 `cargo add proc_macro` help）→ 报错现场一。
- derive 用在 enum：`error: Describe: 只支持 struct，这里是 \`enum\``，波浪线落在 `enum` 上 → 报错现场二。
- 元组结构体 / 泛型结构体 / `sum!(1.5)`：三条自定义 `compile_error!`，span 分别落在字段组、`<`、字面量上（probe 已验证，正文按需引用）。
- trait 不在作用域：`error[E0404]: expected trait, found derive macro \`Describe\`` + `= note: this error originates in the derive macro` → 28.7。
- 忘记 `use`：`error: cannot find derive macro \`Describe\` in this scope` + `E0599` 的 `= note: candidate #1: \`macro_demo_use::Describe\`` → 报错现场三。
- 宏里 panic：`error: proc-macro derive panicked … = help: message: 这个 derive 只会 panic` → 28.7。
- `cargo test -p macro-demo-use` 的 `running 5 tests … test result: ok.` → 动手看看。

### 版本口径 / 稳定性核对（读 1.98 本机 rustdoc）
- 用到的都稳定：`TokenStream::{new, from_str, is_empty}`、`IntoIterator`/`FromIterator`/`Extend`、`Span::{call_site(1.29), mixed_site(1.45), source_text(1.66)}`、`Ident::{new(1.29), new_raw(1.47), span, set_span}`、`Punct::{new, spacing, span}`、`Literal::{string, i64_unsuffixed, span, set_span}`、`Group::{new, stream, delimiter, span, set_span}`、`Spacing`/`Delimiter`。
- nightly-only（章内已显式标注为 "nightly-only experimental API"）：`proc_macro::Diagnostic`、`Span::{error, warning, note, help}`、`Span::join`、`Span::def_site`、`proc_macro::quote!` 宏、`proc_macro::tracked`（1.98 实测无 nightly，全部只标注不实跑）。
- 一个读者易踩的实测细节：`Vec<u8>,` 里的 `>` 是 `Joint`（紧跟 `,` 无空白），所以正文只把「字段冒号是 `Alone`」当作解析依据，没有把 Joint/Alone 说成一条完备规则。

### 遗留不确定
- 未跑 `mdbook build`（协调者统一跑）：SVG 的 HTML 渲染与 linkcheck2 我这一侧无法验证。SVG 已按 §9 自查（单块无空行、`viewBox`+`width="100%"` 无 `height`、无脚本、只用主题变量、`role="img"`+`aria-label`、`[`/`]`/`-->` 用实体）；正文无裸方括号，`[` 只出现在 `[!NOTE]`/`[!TIP]`/`[!WARNING]` 与延伸阅读的链接语法里。
- 28.6 的 syn/quote 对照块是 `rust,ignore` 说明性代码，**未在本机编译**（环境无这两个依赖，也不该为示例引入）。其中容易被审校盯的语法点已单独验证：`format!("Empty" , )`（空重复的尾逗号）在本机 `rustc --edition 2024` 下编译并运行通过；其余为 syn 2 / quote 1 的常规用法。
- 示例宏的边界（已在正文写明）：`Describe` 不支持泛型/元组结构体/enum；`#[repeat]` 只支持 `const`/`static`/`fn`；`sum!` 只吃无后缀整数。
- 跑自验命令会在 `examples/` 下生成/更新 `Cargo.lock`（cargo 对 workspace 的常规行为，非手工改动；`.gitignore` 只覆盖 `target/`，未覆盖 `Cargo.lock`）——是否入库由协调者决定。探针目录 `.scratch/tmp-51/` 已整体删除。
- 正文要求「调用点 `use` 到 trait」（生成 `impl Describe for X` 靠调用点解析）。生产宏的替代方案（`crate = "…"` 参数 / 固定别名）只在 28.7 的 note 里提了一句，未实现。

### 新增术语（请协调者并表 `GLOSSARY.md`）
- 过程宏（procedural macro）、derive 宏（derive macro）、属性宏（attribute macro）、函数式宏（function-like macro）、token 流（token stream）、token 树（token tree）、span 归属、辅助属性（helper attribute，惰性 inert）、不卫生（unhygienic）。
- 与既有条目对齐：复用「宏（macro）」「derive（`#[derive]`）」「span」；首现写「中文（English）」；**没有**使用「派生」这一中译。`macro-demo` 里刻意让 derive 宏与 trait 同名 `Describe`（对应 serde 的 `Serialize`+derive 同名的做法），正文说明了命名空间不冲突。

### 衔接提示
- 回指：第 27 章（`macro_rules!` 的卫生性与片段说明符）、第 30 章（宏在构建流水线里的位置）、第 37 章（rustc 诊断渲染，仅一句）；与第 27 章分工明确：27 只讲声明宏，28 只讲过程宏。
- 第 27 章若也引用「书 ch20.5 宏入口」，两章就不冲突（28 只列 Book 的宏一章链接，不重复讲声明宏）。
- `SUMMARY.md` 第 50 行条目已由协调者预置，本章 H1 保持「# 第 28 章 过程宏」，与条目一致；未改 SUMMARY。

### 图示需求
- 已画 1 张（「为什么这样设计」末尾）：三种形态的**输入与输出落点对照**（derive 追加 / 属性宏替换 / 函数式宏替换），760×240。
- 候选（未画，供协调者取舍）：28.2 的 token 树嵌套图（`Group(Brace)` 层级）；本机 dump 的 `text` 块已给出等价信息，加图只是锦上添花。

### 审校修复（RvCh28 回执，第二轮；只动 `src/ch28-procedural-macros.md` + 本票）
- **F1（major，练习 2 答案）**：按实测整体重写。新建探针（`buggy` = 去掉 spacing 判断的 `macro-demo` 副本 + 消费端 `Config { value: std::string::String }`）实跑，真实输出是**两条** `E0609`（`std` 与 `string`）、类型为 `&Config`、波浪线落在**源码里 `std`/`string` 那颗 ident 上**（span 沿用输入 token），旧文「一条错误 + 指向 `#[derive(Describe)]`」两处都不准，已改为粘贴实测两份错误块 + 三点说明；并补了「想让错误落到派生宏处 → `Ident::new(&field.to_string(), Span::call_site())` / `set_span`」的改法，与 28.3 的 span 课呼应。探针原件：`error[E0609]: no field \`std\` on type \`&Config\`  --> src\main.rs:9:16  |  pub value: std::string::String, ^^^ unknown field  = note: available field is: \`value\``（第二条 `string` 同形）。
- **F2（minor）**：练习 3 答案中 `impl<T> Describe for Wrap<T>；` 的**全角分号**已删（现为 `impl<T> Describe for Wrap<T>` + 破折号衔接）。
- **F3（minor）**：开头「边界」的 ch30 回指收窄为「宏在构建流水线里的位置（`build.rs`、依赖解析与构建顺序）」；已核对 ch30 自述「宏不在本章展开」，原措辞「宏 crate 的编译顺序」落空，已改。
- **F6（minor）**：28.2 的 `Dump` 块补上 `walk` 的完整实现（用 `{:?}` 打印 `Delimiter`/`Spacing`，与本章后面粘贴的 dump 文本逐字一致，且不再出现未定义标识符）。
- **F5（minor，本票口径）**：块计数更正为「**35 栅栏 = 22 rust（21 `ignore` + 1 真跑）+ 11 `text` + 1 `toml` + 1 `console`**」，并注明 `mdbook test` 列出 22 个 doctest（rustdoc 分两批 `running 18` + `running 4`）。
- **跨章口径**：「动手看看」的 `cargo test` 前言由「Windows 10」改为「Windows 11」（本机内部版本 10.0.26200）。
- **F4（不归本票改，记录）**：GLOSSARY 条目的拼写「proc macro」与章内「procedural macro」不一致——按裁定章内保持 `procedural macro` 不动，由协调者并表时统一。
- 复验：`mdbook test -c ch28-procedural-macros.md` → exit 0（sentinel 复测仍 exit 101，见上）；`cargo test --manifest-path examples/Cargo.toml -p macro-demo-use` → exit 0，`5 passed`。本轮探针目录 `.scratch/tmp-51/` 已删除。
