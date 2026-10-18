# 14 第 5 章 结构体、枚举与模式匹配

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 5 章：struct/enum 的定义、impl 与方法、常用 derive、match/if let/while let/let else、模式的完整语法（绑定、解构、守卫、@）。

- 心智模型：为什么「枚举 + 穷尽性检查」是 Rust 表达状态机的方式。
- 纪律：官方书对应章只作口径参照，不许逐段对照（地图 Out of scope）。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：文件 `src/ch05-structs-enums-patterns.md`（H1 = `# 第 5 章 结构体、枚举与模式匹配`）+ 完整程序片段 `snippets/ch05/task_state.rs`。篇幅（全部修订后）：全文汉字 3996、正文（不含代码块）汉字 3632；受测块 24 个（19 普通 rust + 5 `compile_fail`），非 Rust 块全部标 `text`/`console`。（修订前为 3969 / 3605；因补英文对照与两条 NOTE 扩容，做了等量精简，见下条。）
- 自验：`mdbook test -c ch05-structs-enums-patterns.md` → exit 0（只输出 INFO 行）。为排除「有块被静默跳过」，做了两次对照实验：往一个 inline 块和一个 `{{#rustdoc_include}}` 片段里各塞一个语法错误，两次都 exit 101 且精确点名该章该块（`机制与规则::__5_Option (line 238)`、`动手看看 (line 424)`）→ 含 include 在内所有块都真在编译；实验已还原并复核为 exit 0。
- 实测输出（rustc 1.98.0 / edition 2024 / Windows；均真跑）：5.1 `7 第 5 章草稿`、`1`；5.2 `2`/`2`；5.3 `Point { x: 1, y: 2 } == Point { x: 1, y: 2 } -> true`、`Point { x: 1, y: 2 }`、`Job { title: "第 5 章" }`；5.4 `3.14`/`6.00`、`下载中`/`完成`；5.5 `Some("ch05.md")`/`ch05.md`/`None`/`7`；5.6 四行（`0 是零`/`7 是一位数 7`/`42 是别的`/`-1 是负数`）；5.7 `任务 7：第 5 章草稿`、`1 2 3 4`/`10`/`1 2`/`1`；5.8 `timeout = 30`/`Some(30)`、`1`/`2`/`3`、`rust`/`rust`、`完成？true`；练习 3 `true`；`task_state.rs` 六行输出（正文）。报错现场三条都真跑：E0004（`non-exhaustive patterns: &Status::Done not covered` + `todo!()` 建议）、E0382 partial move（`value has type String, which does not implement the Copy trait` + `ref` 建议）、E0609（`unknown field` + 相似名建议）；练习 1 的 E0204 也实测（初稿误写成 `trait bound ... is not satisfied`，已按实测改成 E0204 文案）。`ref`/`mut` 限制实测：同一段代码 edition 2021 默认不报（只有无关的 dead_code 警告）、2024 报 `cannot explicitly borrow within an implicitly-borrowing pattern`；加 `-W rust-2024-incompatible-pat` 后 2021 也给同款警告。
- 遗留不确定：(1) edition 2024 的 `if let` 临时值作用域变化与 `let else` 的 drop 顺序未写（怕与第 4 章重复 / 超范围）；(2) enum 的判别值、`repr`、内存布局完全未碰（第 25 章）；(3) `while let` 用 `split_first` 而非 `Vec::pop`（避开第 6 章内容），可读性略绕；(4) 报错现场输出的 `-->` 路径行按 §2 剔掉，行号保留（对应正文里的最小复现文件）。
- 新术语（GLOSSARY 缺，建议并表）：结构体 | struct；枚举 | enum；变体 | variant；元组结构体 | tuple struct；单元结构体 | unit struct；字段 | field；关联函数 | associated function；方法 | method；解构 | destructuring；穷尽性 | exhaustiveness；匹配守卫 | match guard；或模式 | or-pattern；部分移动 | partial move；匹配人体工程学 | match ergonomics；代数数据类型 | ADT；哨兵值 | sentinel value；`#[derive]` | 保留英文不译。
- 邻章衔接：5.2/5.3 回指第 4 章（所有权 / 借用 / Copy 边界）；`Result` 与 `?` 指向第 7 章；trait 与 `#[derive]` 原理指向第 9 章；`Vec`/`String`/`HashMap` 与 `clone` 的开销指向第 6 章；加了一条 NOTE 指向第 13 章的 trait 对象（枚举适合封闭变体集合，开放集合用 `dyn`）。本章未用迭代器/闭包。
- 图示需求：(1) 5.4「bool/字符串拼状态 vs enum 建模」对照图（现为 `text` 块，08 定了图示体系可换）；(2) 5.2「`self` / `&self` / `&mut self` ↔ 所有权 / 借用」关系图（现为文字 + 代码）；(3) 5.6 match 自上而下 + 穷尽性的流程示意；(4) 5.7 模式速查已用表格实现，是否图化待 08 决定。
- 2026-10-18 审校修订（RwCh05：1 blocker + 5 minor；RwCh09 追加 Windows 产物名 1 条）：(1) `:358` NOTE 改为「edition 2024 起是硬错误；2021 及更早允许这种写法，可用默认 `allow` 的 `rust-2024-incompatible-pat` lint 提前发现」——lint 名与默认级别经 `rustc -W help` 核实（`rust-2024-incompatible-pat  allow  detects patterns whose meaning will change in Rust 2024`），2021 行为本机复跑：默认只报 dead_code，加 `-W` 后报 `cannot explicitly borrow within an implicitly-borrowing pattern in Rust 2024`；edition-guide 页面亦确认「In Rust 2021 and earlier editions, we allow this oddity」。(2) `:27` 跨语言断言改为「C++ 的 `std::variant`、F# 的 discriminated union 是近亲（C# 只有提案）」。(3) 首现补英文对照：元组结构体（tuple struct）、单元结构体（unit struct）、或模式（or-pattern）、哨兵值（sentinel value）；自查另补 解构（destructuring）。(4) `:266` 穷尽性范围改为「（外部 crate 里标了 `#[non_exhaustive]` 的枚举除外）」。(5) `:147` `HashMap` 键改为「但键还要求 `Hash`」。(6) 动手看看的运行命令改为 `-o task_state.exe` + `.\task_state.exe`，并注明其他平台写法；实测 windows-msvc 下 `rustc -o hi` 产物就是 `hi`（不补 `.exe`），故必须显式带后缀。复跑 `mdbook test -c ch05-structs-enums-patterns.md` → exit 0；`snippets/ch05/task_state.rs` 按正文命令原样编译运行，六行输出与正文一致。(7) `:437`（RwPreface 跨章观察）console 块提示符 `>` → `$`，符合 WRITING §5 口径「console 块的提示符只用 `$`」；运行行按该节补明后的口径写 `$ .\task_state.exe`（`.\` 前缀 cmd 与 PowerShell 都认）：实测 PowerShell 下裸名 `task_state.exe` 报 `not recognized`、`.\task_state.exe` 正常输出六行，cmd 两种写法均可；编译行 `-o task_state.exe` 保持（`rustc -o` 不补后缀）。本章再无 `>` 提示符与非正斜杠路径。(8) 按 WRITING §5 新增的「引用官方原文/报错行必须逐字核对」「延伸阅读链接不钉版本」两条复查全章：E0594 引文改为双反引号精确原文 ``cannot assign to `job.title`, which is behind a `&` reference``；其余引用的英文报错行（E0004 / E0382 / E0609 / E0204 / `cannot explicitly borrow within an implicitly-borrowing pattern`）逐条与 1.98 实测输出对照，均逐字一致；延伸阅读 8 条链接现均为不钉版本的稳定入口（`https://doc.rust-lang.org/book/…`、`/reference/…`、`/std/…`、`/edition-guide/…`），版本敏感事实留在正文（1.98 基线与「以 Rust 1.98 为例」）。注：链接去版本化非本次修订所为（原稿是 `1.98.0/…`），属外部改动，已按新口径复核后保留。配套精简（保持 ≤4000 字）：删「本次改动的」「顺手」等冗词、NOTE 与跨语言句收紧、5 处「细节见…」合并，最终 3996 / 3632。
- 主动取舍：未逐段对照官方 ch5/ch6；未写 struct 更新语法 `..rest`、`Display`、`Default`、`#[non_exhaustive]`（判为超范围）；报错现场只保留票要求的三个，2024 的 `ref` 限制放 NOTE 而未升为第四个现场；「语言对照角」未单列（C/Java/Python/C++/C# 的对比已并入「为什么这样设计」）。
