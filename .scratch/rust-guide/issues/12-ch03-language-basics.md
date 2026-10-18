# 12 第 3 章 语言基础：变量、类型、函数与控制流

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 3 章：let/mut、标量与复合类型、函数与表达式/语句、if/loop/while/for、注释与文档注释。

- 起点：会一门主流语言、不假定泛型/编译期概念 —— 用 C++/Java/Python 的最小对照。
- 篇幅纪律：只详讲「能讲清为什么」的部分；纯语法细节走查表，不许罗列式堆砌。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：交付 `src/ch03-language-basics.md`（644 行；正文 3624 汉字，含代码 3927 汉字）、`snippets/ch03/tour.rs`（动手程序，70 行）、`snippets/ch03/overflow.rs`（溢出程序，6 行）。
  - 自验：`mdbook test -c ch03-language-basics.md` → **exit 0**（日志仅 `INFO Testing chapter ...`）。为拿到逐块证据，把两个 `{{#rustdoc_include}}` 展开后跑 `rustdoc --test --edition 2024` → `14 passed / 13 passed`，共 27 个代码块（14 个运行块 + 12 个 `compile_fail` + 1 个 `should_panic`）全绿。
  - 实测输出清单（全部真跑真粘，Windows x86_64 + rustc/cargo 1.98.0）：
    1. 溢出：`cargo run`（debug）`i8` 127+1 → `thread 'main' (…id) panicked at src\main.rs:4:5: attempt to add with overflow`，退出码 101；`cargo run --release` → `加 1 之后：-128`，退出码 0（同一份源码）。
    2. 编译期溢出：`i32::MAX + 1` → `error: this arithmetic operation will overflow` + `#[deny(arithmetic_overflow)]`；同一程序 `cargo check` 退出 0（1.98 实测差异，已写进正文并标注口径）。
    3. `tour.rs` 12 行输出：`遮蔽后 x = 6` / `42 默认是 i32，3.5 默认是 f64` / `char 占 4 字节，'中' 的标量值 = 20013` / 元组解构 / 数组长度 / `7 是奇数` / `double(21) = 42` / `loop 返回 30` / `while 结束 i = 3` / `奇数之和 = 4` / `300 as u8 = 44` / `200_i64.try_into::<u8>() = 200`。
    4. 转换：`300 as u8 = 44`、`3.9_f64 as i32 = 3`、`(-1_i32) as u8 = 255`、`200_i64.try_into::<u8>() = 200`；`300_000_i64.try_into::<u8>().unwrap()` → `panicked at src\main.rs:3:36: called Result::unwrap() on an Err value: TryFromIntError(PosOverflow)`。
    5. 块值：`{ 1 + 1 }` vs `{ 1 + 1; }` → `a = 2, b = ()`，后者附警告 `unused arithmetic operation that must be used`。
    6. 显式溢出 API：`n: i8 = 127` 的 `wrapping_add/saturating_add/checked_add` → `-128` / `127` / `None`。
    7. 类型实测：`usize`/`isize` 各 8 字节（64 位平台）、`char` 4 字节、`char::from_u32(0xD800) = None`、`"42".parse::<u8>()` 与 `let n: i32 = "42".parse().unwrap()` 均通过。
    8. 报错原文（`cargo check`/`cargo run` 终端输出，按放入 `src/main.rs` 的位置粘贴）：`expected `;`, found keyword `let``（含 `help: add `;` here`）、E0308 尾表达式多分号（含 `help: remove this semicolon`）、E0308 `if`/`else` 类型不一致、E0308 `expected i64, found i32`（含 `help: you can convert an i32 to an i64`）、E0308 `expected bool, found integer`、E0384、E0381。另实测但未配块：E0571（`while` 里 `break` 带值）、E0317（`if` 缺 `else` 赋给 `i32`）、运行期 `index out of bounds: the len is 3 but the index is 5`、常量下标越界被 `#[deny(unconditional_panic)]` 编译期拦下。
  - 遗留：
    - E0571、E0317 只在正文写了一句（各已实测），未配 `compile_fail` 块——按篇幅纪律取舍。
    - 数组运行期越界（`index out of bounds`）只用一句正文带过，未给完整示例。
    - 「`cargo check` 看不到 `arithmetic_overflow`」是 1.98 实测观察，未在官方文档中找到明述（正文已标「1.98 实测」）。
    - 遮蔽后「旧绑定仍在作用域内、何时 drop」的精确语义留给第 4 章；正文只说到「没法再按名字访问」。
    - `research/02 §4.1` 与本票直接相关的项（尾表达式临时值作用域、`if let` 临时值作用域、let chains、match ergonomics）都属第 4/5/7 章范围，本票未展开。
  - 新术语（建议并入 `GLOSSARY.md`，正文首现处已写「中文（English）」）：绑定（binding）、遮蔽（shadowing）、单元类型（unit type）、尾表达式（tail expression）、整数溢出（integer overflow）、文档注释（doc comment）、解构（destructuring）、类型标注 / 类型推断（annotation / inference）、可变绑定与不可变绑定（mutable / immutable binding）。已在表中的 `所有权（ownership）`、`宏（macro）`、`迭代器（iterator）`、`模式匹配（pattern matching）`、`泛型（generics）`、`trait`（首现加一句「行为契约」）均按表处理。
  - 邻章衔接：第 4 章（所有权，开篇即预告）、第 5 章（`Option`/`match`/解构/模式匹配）、第 6 章（`Vec`、UTF-8 与字符串、索引）、第 7 章（`Result` 与 `.unwrap()`）、第 8 章（模块与 `pub`）、第 9 章（泛型与 trait、重载/默认参数的替代）、第 10 章（迭代器与 `for` 的语法糖身份）、第 18 章（构造式 API）、第 27 章（宏）、第 32 章（文档工程）。全部只写一句「（第 N 章展开）」，未越界。
  - 图示需求：①「表达式导向 vs 语句导向」——想画一张对比图（语句序列逐步执行 vs 块的值自下而上汇出）；②整数溢出的两条路——`debug → panic / release → 环绕` 的决策树（现用文字 + 两块真实输出表达）。都用 `text` 块可画，等 08 票定图示体系后再统一。
  - **与 `WRITING.md` 冲突（实测，未改 WRITING.md）**：§1 写「走 `{{#include}}`（不参与 `mdbook test`）」——mdbook 0.5.4 实测**会参与**：我 include 的 `snippets/ch03/overflow.rs` 按设计会 panic，`mdbook test -c` 因此 FAILED（exit 101），给该块标上 `rust,should_panic` 后通过。⇒ 用 `{{#include}}`/`{{#rustdoc_include}}` 的票（02/04/05/08…）请一并自查：include 进来的代码会被真正编译并执行；需要「故意 panic/越界」的示例就标 `rust,should_panic`，`rust,compile_fail` 与 `should_panic` 都是 mdbook test 可用的正向断言。
- 2026-10-18 审校回改（reviewer RwCh03：1 major + 4 minor）：
  - **major** 练习 2 答案 C 张冠李戴已修：原文把 `cannot add i32 to u8` 当成 E0308 的文案；实测该片段报**两条**错——`E0308: mismatched types`（`expected u8, found i32`）与 `E0277: cannot add i32 to u8`（含 `help: the trait Add<i32> is not implemented for u8`）。答案改为“报两条错”并分别给出错误码与文案。
  - **minor** 「没有未初始化变量」与「报错现场 ⑤」的 `let x: i32;`（声明合法、只是不可读）自相矛盾 → 开篇改为「**没有「未初始化」这种可读取的状态**。变量在被读取之前必须确实被赋过值（`let x: i32;` 声明合法，读到才错，见 ⑤）」；「为什么这样设计」同句同步。
  - **minor** 「没有隐式类型转换」绝对化 → 全文限定为「**数值类型之间**没有隐式转换」（开篇、「为什么这样设计」小标题、报错现场 ④ 标题、小结一条，共 4 处）；`&mut T`→`&T` 之类 coercion 留给第 4 章，未展开。
  - **minor** `'😀'` 作 `char` 数据示例：协调者裁定保持（数据非装饰）→ 未改。
  - **minor** 「构造式 API（第 18 章）」：已对版图核对，第 18 章含 builder/typestate → 保持不动。
  - 复跑：`mdbook test -c ch03-language-basics.md` → **exit 0**；include 展开后 `rustdoc --test --edition 2024` → `14 passed / 13 passed`（27 块全绿，数量未变）。篇幅变为正文 3683 汉字 / 含代码 3986 汉字，仍在 3000–4000。仍不写 `## Answer`、不置 `resolved`。
