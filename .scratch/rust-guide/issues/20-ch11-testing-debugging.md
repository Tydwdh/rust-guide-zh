# 20 第 11 章 测试与调试

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 11 章：cargo test、单元/集成测试、断言与 `dbg!`、从测试失败回推问题的路径（诊断训练的收束章）。

- 心智模型：测试是「可执行的规格」；失败输出怎么读。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：
  - 文件：`src/ch11-testing-debugging.md`（H1 = `# 第 11 章 测试与调试`）；新增 `snippets/ch11/`（`Cargo.toml`、`src/lib.rs`、`tests/integration.rs`，即演示库 `ch11-demo` 的可跑完整版，lib.rs 为已修复版本）。篇幅：正文 3119 个汉字 + 代码块（全文 3189 汉字，18.9 KB）。
  - 自验：`mdbook test -c ch11-testing-debugging.md` → `INFO Testing chapter …` / EXIT=0（通过；mdbook 成功时只打 INFO 行、不回显 rustdoc 输出）。已做反向确认：故意把**非 `#[test]` 体**里的代码改成类型错误（`let _bad: i32 = "no";`），`mdbook test` 立刻 ERROR + EXIT=101，说明本章代码块确实被编译检查。
  - 实测输出（全部真跑真粘，`rustc/cargo 1.98.0`，Windows x86_64-msvc）：
    1. 失败态 `cargo test`（`ch11demo`：3 单元测试，`ignores_extra_spaces` 8≠2、`empty_text` 1≠0）→ 粘进「动手看看」；
    2. 修复后 `cargo test` 三段输出（lib 3 passed / integration 1 passed / Doc-tests 0）→ 粘进「动手看看」；
    3. `cargo test counts_words`（子串过滤，同时命中单元与集成目标）→ 粘进「动手看看」；
    4. `#[should_panic]` 未 panic 的失败输出（`test did not panic as expected`，`panics` crate）→ 粘进「报错现场」；
    5. `#[should_panic]` + 返回 `Result` 的编译错误：`error: functions using #[should_panic] must return ()`（`rustc --edition 2024 --test`）→ 正文引用该关键行；
    6. `dbg!` 在 `-- --no-capture` 与 `-- --nocapture` 下的输出（`[src\lib.rs:31:9] text = "a b"` / `n = 2`）→ 用于说明两拼写都能跑；`cargo test -- --help` 实测只列 `--no-capture`，`--nocapture` 已不在 help 中（正文按此措辞）。
  - 遗留 / 不确定：
    1. **重要发现（给批次）**：mdbook/rustdoc 的 `#[test]` 代码块里，**测试函数体既不被类型检查也不被执行**。最小复现（独立 mini book）：块内 `#[test] fn t() { let x: i32 = "no"; }` → `mdbook test` 判为 ok、EXIT=0；同一书里把该 fn 换成普通 `fn f() { let x: i32 = "no"; }` → 立刻 E0308 + EXIT=101。（另测：`#[test] fn t(){ assert_eq!(1+1,3); }` 也判 ok。）结论：`mdbook test` 对测试章只提供「非测试项」的编译检查，测试语义必须靠真 cargo 探针验证——本章即照此做（`ch11demo`/`verify`/`panics` 三个临时 crate 真跑，收尾已删）。建议在 WRITING §7 补一条。
    2. 「`--nocapture` 1.88 起弃用」：本机 1.98 实测**不报警告、仍可用**，只是不在 `--help` 列出。正文已按实测措辞（「仍被接受，但已不在 --help 里」），未用「弃用」这一较强说法。
    3. `expected` 为子串匹配，已实测（panic 信息须包含该子串）；原文示例第一版用 `expected = "越界"` 配 `v[99]`，实测**不通过**（真实信息是 `index out of bounds…`），已改成自定义 `panic!("文本不能为空")` + `checked_len`，并真跑通过。
    4. 断点调试只按任务要求给「VS Code + CodeLLDB + 官方仓库链接」两三句，未实测（本机未装该扩展）。
  - 新术语（English | 建议中文）：unit test | 单元测试；integration test | 集成测试；assertion | 断言；test runner | 测试运行器；test harness | 测试框架/测试工具；documentation test (doctest) | 文档测试；`#[ignore]` 保留英文即可。（本章首现「单元测试（unit test）」「集成测试（integration test）」，已按首现格式；其余建议交协调者统一并表。）
  - 邻章衔接：第 7 章（`Result`/`panic`/`RUST_BACKTRACE`）多处回指；第 2 章（`cfg`/读编译器）回指；第 5 章（`#[derive(Debug)]`）回指；第 8 章（模块）回指；doctest 与 compile_fail / proptest / nextest / CI 一样归第 31 章（测试进阶），本章只预告。项目 1（CLI）可复用本章「从失败回推」循环。
  - 图示需求：本章现用 1 个 `text` 目录树 + 1 个宏对照表。若 08 票定图示体系，适合加图处：(a)「单元测试 vs 集成测试」与 cargo test 三段目标（lib / tests / doctests）的关系图；(b)「失败→定位→修复」循环图（复现→最小化→假设→验证）；(c) `#[cfg(test)]` 在 `cargo build` 与 `cargo test` 两条路径下「模块是否编入产物」的对比图。

- 2026-10-18 交叉审校修订（reviewer RwCh11 转来 6 × minor，全部处理，只动 `src/ch11-testing-debugging.md` + 本票）：
  1. `:116` / `:399` 逐字引用的编译错误改用双反引号行内码保留原样（原文漏了内层反引号）：``functions using `#[should_panic]` must return `()` ``。
  2. doctest 回指由第 32 章改为**第 31 章**（对齐 `outline-v0.md` 第 31 章「集成/doctest、compile_fail、proptest…」）；本 Comments 上一版「第 32 章（doctest）预告」同处已一并订正。
  3. 两段真跑输出重跑为从 `snippets/ch11/` 执行的版本（`Compiling ch11-demo v0.1.0 (D:\Rust\snippets\ch11)`），失败态同步为本次 rustc 1.98 真跑的测试顺序与行号（`src\lib.rs:17:9` / `:22:9`；`@ 报错现场` 引用的 thread id 一并改为本次的 13212）。为让 fail 态行号与正文错版一致，`snippets/ch11/src/lib.rs` 去掉了原首行 `//!` 模块文档（现第 1 行 `/// 统计…`、第 3 行 `split_whitespace()`），其余不变；`cargo test`（cwd = `snippets/ch11`，失败态/修复态/子串过滤）均真跑真粘。
  4. `--nocapture` 措辞对齐一手来源：`library/test/src/cli.rs:204` 为 `optflag("", "nocapture", "Deprecated, use --no-capture")`；正文改为「自 1.88 起已弃用，但仍被接受，且不再出现在 `--help` 里」。
  5. 失败起点补精确复现指令：「打开 `snippets/ch11/src/lib.rs`，把第 3 行 `split_whitespace()` 换回 `split(' ')`，在 `snippets/ch11/` 跑 `cargo test`」。
  - 复跑：`mdbook test -c ch11-testing-debugging.md` → `INFO Testing chapter …` / **EXIT=0**。清理：删除 `snippets/ch11/target` 与 `Cargo.lock`，目录仅剩 `Cargo.toml`、`src/lib.rs`、`tests/integration.rs`。现值：20492 字节，正文 3163 汉字（仍在 3000–4000）。
