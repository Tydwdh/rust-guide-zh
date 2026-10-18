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

- 2026-10-18 初稿：文件 `src/ch05-structs-enums-patterns.md`（H1 = `# 第 5 章 结构体、枚举与模式匹配`）+ 完整程序片段 `snippets/ch05/task_state.rs`。篇幅：全文汉字 3969、正文（不含代码块）汉字 3605；受测块 24 个（19 普通 rust + 5 `compile_fail`），非 Rust 块全部标 `text`/`console`。
- 自验：`mdbook test -c ch05-structs-enums-patterns.md` → exit 0（只输出 INFO 行）。为排除「有块被静默跳过」，做了两次对照实验：往一个 inline 块和一个 `{{#rustdoc_include}}` 片段里各塞一个语法错误，两次都 exit 101 且精确点名该章该块（`机制与规则::__5_Option (line 238)`、`动手看看 (line 424)`）→ 含 include 在内所有块都真在编译；实验已还原并复核为 exit 0。
- 实测输出（rustc 1.98.0 / edition 2024 / Windows；均真跑）：5.1 `7 第 5 章草稿`、`1`；5.2 `2`/`2`；5.3 `Point { x: 1, y: 2 } == Point { x: 1, y: 2 } -> true`、`Point { x: 1, y: 2 }`、`Job { title: "第 5 章" }`；5.4 `3.14`/`6.00`、`下载中`/`完成`；5.5 `Some("ch05.md")`/`ch05.md`/`None`/`7`；5.6 四行（`0 是零`/`7 是一位数 7`/`42 是别的`/`-1 是负数`）；5.7 `任务 7：第 5 章草稿`、`1 2 3 4`/`10`/`1 2`/`1`；5.8 `timeout = 30`/`Some(30)`、`1`/`2`/`3`、`rust`/`rust`、`完成？true`；练习 3 `true`；`task_state.rs` 六行输出（正文）。报错现场三条都真跑：E0004（`non-exhaustive patterns: &Status::Done not covered` + `todo!()` 建议）、E0382 partial move（`value has type String, which does not implement the Copy trait` + `ref` 建议）、E0609（`unknown field` + 相似名建议）；练习 1 的 E0204 也实测（初稿误写成 `trait bound ... is not satisfied`，已按实测改成 E0204 文案）。`ref`/`mut` 限制实测：同一段代码 2021 只警告、2024 报 `cannot explicitly borrow within an implicitly-borrowing pattern`。
- 遗留不确定：(1) edition 2024 的 `if let` 临时值作用域变化与 `let else` 的 drop 顺序未写（怕与第 4 章重复 / 超范围）；(2) enum 的判别值、`repr`、内存布局完全未碰（第 25 章）；(3) `while let` 用 `split_first` 而非 `Vec::pop`（避开第 6 章内容），可读性略绕；(4) 报错现场输出的 `-->` 路径行按 §2 剔掉，行号保留（对应正文里的最小复现文件）。
- 新术语（GLOSSARY 缺，建议并表）：结构体 | struct；枚举 | enum；变体 | variant；元组结构体 | tuple struct；单元结构体 | unit struct；字段 | field；关联函数 | associated function；方法 | method；解构 | destructuring；穷尽性 | exhaustiveness；匹配守卫 | match guard；或模式 | or-pattern；部分移动 | partial move；匹配人体工程学 | match ergonomics；代数数据类型 | ADT；哨兵值 | sentinel value；`#[derive]` | 保留英文不译。
- 邻章衔接：5.2/5.3 回指第 4 章（所有权 / 借用 / Copy 边界）；`Result` 与 `?` 指向第 7 章；trait 与 `#[derive]` 原理指向第 9 章；`Vec`/`String`/`HashMap` 与 `clone` 的开销指向第 6 章；加了一条 NOTE 指向第 13 章的 trait 对象（枚举适合封闭变体集合，开放集合用 `dyn`）。本章未用迭代器/闭包。
- 图示需求：(1) 5.4「bool/字符串拼状态 vs enum 建模」对照图（现为 `text` 块，08 定了图示体系可换）；(2) 5.2「`self` / `&self` / `&mut self` ↔ 所有权 / 借用」关系图（现为文字 + 代码）；(3) 5.6 match 自上而下 + 穷尽性的流程示意；(4) 5.7 模式速查已用表格实现，是否图化待 08 决定。
- 主动取舍：未逐段对照官方 ch5/ch6；未写 struct 更新语法 `..rest`、`Display`、`Default`、`#[non_exhaustive]`（判为超范围）；报错现场只保留票要求的三个，2024 的 `ref` 限制放 NOTE 而未升为第四个现场；「语言对照角」未单列（C/Java/Python/C++/C# 的对比已并入「为什么这样设计」）。
