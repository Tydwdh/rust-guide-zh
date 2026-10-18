# 19 第 10 章 迭代器与闭包

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 10 章：闭包（捕获方式与 Fn/FnMut/FnOnce）、迭代器协议、适配器与消费器、惰性求值、collect 与零成本抽象初识。

- 心智模型：闭包捕获与所有权的关系；迭代器链为什么不产生中间集合。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：文件 `src/ch10-iterators-closures.md`（786 行；去代码块汉字 4501、全文汉字 4867）+ `snippets/ch10/log_stats.rs`（47 行：`parse` + 命令式版 + 链式版 + `assert_eq!` 钉住两版一致）。
  - 自验：`mdbook test -c ch10-iterators-closures.md` → exit 0（普通块 16 个通过、`compile_fail` 5 个断言必须失败、`rust,ignore` 3 个跳过；无 ERROR 输出）。`rustc --edition 2024 snippets/ch10/log_stats.rs -o …` 运行输出 `ERROR 2 条，时间：09:02:03、09:02:04`，与正文 `text` 块逐字一致。
  - 实测输出清单（全部本机 1.98 真跑后再粘）：闭包语法 `3 6 42`/`开始`；三种捕获 `借用捕获 len = 4，name 仍可用：rust` / `可变借用捕获 bump() = 2` / `外部看到 count = 2` / `move 捕获 = 3`；Fn 家族 `Fn：23`、`FnMut：1, 2`、`FnOnce：闭包搬走了我`；函数指针 `函数名当参数：42`/`fn 指针：42`；`make_counter` `1 2 3`；`positives` `[2, 4]`；for 三种形态 `按值遍历：10/20/30` + `借用遍历：[1, 2, 3]`；适配器/消费者 10 行（`chain [1, 2, 5, 6]`、`sum 21 / fold 720`…）；collect `[2, 4, 6, 8] [2, 4, 6, 8] rust Err(ParseIntError { kind: InvalidDigit })`；惰性 `next() = Some(2)` 分步输出；链交错输出 `map(1)/filter(2)/map(2)…结果 [4, 8]`；未消费块的两个警告原文（`unused_must_use` + `map_unit_fn`）。
  - 报错实测（`compile_fail` + 关键行节选，均 1.98 本机复现）：E0502（闭包共享借用 vs `v.push`）、E0499（两个闭包同时可变借用）、E0283（`collect` 目标类型不明；另测 `str::parse` 的元素类型变体）、E0382（`FnOnce` 闭包调用两次，note 含 "not `Copy`"）、E0373（闭包逃逸需 `move`，给的是 fn-only 最小复现）；各配已验证修法（`&mut F: FnMut`、`|| s.clone()` 使闭包变 `Copy`、E0502 两种修法实测通过）。附带：E0507（FnMut 里搬走捕获值，脚注一句）、E0369（`&&i32` 做算术，写进提示未整段贴）、E0599（`Vec::map` 不存在，只点名错误码）。
  - 遗留：① `snippets/ch10/log_stats.rs` 整文件 47 行，核心逻辑（`parse` + 两版实现）约 22 行；按「15–30 行」若只计核心逻辑符合、按整文件计超 —— 如必须压，可删 `assert_eq!` 与注释（会削弱「两版结果一致」的演示，需要协调者定）。② `HashMap` 收集只讲不打印：遍历顺序不稳定，输出不可复现。③ 篇幅：去代码块汉字 4501（压到刚好触上限），全文 4867 —— 若「字」按全文（含代码与输出）计，超上限约 367；按正文计达标。④ 零成本只讲原理 + 指向第 29 章，未在本章跑基准（票面要求如此）。
  - 新术语（GLOSSARY 未收，建议协调者统一）：适配器（adapter）、消费者（consumer）、惰性求值（lazy evaluation）、拉模型 / 推模型（pull / push）；「语言对照角」另用到视图（view，C++ ranges）、流（stream，Java）。表内已有的「迭代器 / 闭包」「零成本抽象」按表使用（零成本已确认由第 9 章首现，本票只回引）。
  - 邻章衔接：回指第 4 章（捕获 = 借用或移动）、第 6 章（`iter`/`into_iter`/`iter_mut`）、第 7 章（`Option`/`Result`/`?`）、第 9 章（泛型 + 单态化；已与 Ch09Generics 对过：单态化/零成本由第 9 章首现，两章无循环引用）；前指第 13 章（`dyn Fn`、`Box<dyn Iterator>`）、第 14 章（自定义迭代器、返回 `impl Iterator`、`'_`）、第 17 章（`RefCell`）、第 19/21 章（`move` 进线程 / async）、第 29 章（基准与剖析）。按协调者 IRC 要求新增可选节「语言对照角」（C++/Java/Python，两条轴：闭包捕获、迭代器/流惰性）。
  - 图示需求：① `Fn: FnMut: FnOnce` 包含关系图（嵌套圆/金字塔，附「边界越窄、接收面越小」标注）；② 迭代器链「套娃类型」示意（`Map<Filter<Iter>>` 嵌套盒）；③ 惰性管线对比图（元素逐个穿过 map→filter→collect，对照「每步建一个中间 Vec」的分层画法）。本批按约定用 `text` 块 / 表格呈现，未引入 mermaid。
