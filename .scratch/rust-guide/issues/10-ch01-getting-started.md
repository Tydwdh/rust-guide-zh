# 10 第 1 章 起步：工具链与第一个程序

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 1 章：rustup/cargo 的安装与验证、第一个程序、编译与运行、`cargo` 基本命令、本书约定（edition 2024、示例怎么跑、报错去哪看）。

- 心智模型先行：从「一次编译、一次运行」解释 cargo 扮演的角色，而不是命令罗列。
- 诊断训练：第一次读报错的解剖（拼写错误里的 error/note/help）。
- 素材：02 §4.1（2024 edition 要点）。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：文件 `src/ch01-getting-started.md`（H1 = `# 第 1 章 起步：工具链与第一个程序`；正文 2789 个汉字；八节齐全：这一章解决什么问题 / 为什么这样设计 / 机制与规则 / 动手看看 / 报错现场 / 小结 / 练习+答案 / 延伸阅读）。本章无「读者看 3 行、实际是完整程序」的需求，**未建** `snippets/ch01/`；两段可编译示例（`hello` 程序、加一行的程序）直接内联。
  - 自验：`mdbook test -c ch01-getting-started.md` → ` INFO Testing chapter '第 1 章 起步：工具链与第一个程序': "ch01-getting-started.md"`，exit 0。
  - 1.98 本机实测并粘贴的真实输出（Windows x86_64-pc-windows-msvc；rustc 1.98.0 `88d9e12ae 2026-08-18`；cargo 1.98.0 `797e8a9bc 2026-08-05`；rustup 1.29.1 `d95a37b6a 2026-08-13`）：
    1. `rustup --version`（含两条 `info:`）、`rustc --version`、`cargo --version`；
    2. `cargo new hello`（`Creating binary (application) `hello` package` + note 行）、生成后的目录树、`.gitignore`（`/target`）、`Cargo.toml`、`src/main.rs` 原文；
    3. `cargo check`（fresh 0.87s；缓存 0.18s）、`cargo run`（fresh 1.18s；改一行后 0.16s）、`cargo build`（fresh 1.36s）、`cargo build --release`（2.29s）、`cargo run --release`（`Running `target\release\hello.exe``）；
    4. `cargo check` 后 `target/debug/` 清单里**没有** `hello.exe`（练习 2 答案的实证）；`target/debug/` 完整目录项；
    5. 拼错宏 `printlin!` 的 `cargo run` 报错全文（含 `help:` 与 `similarly named macro` 指针；exit 101）；
    6. 清空 `src/main.rs` 后 `cargo run` 的 `error[E0601]` + `= note: consider adding a main function to src\main.rs`（`note:` 角色的真实素材）；
    7. `rustc --edition 2024 solo.rs` 静默成功、产出 `solo.exe`、运行打印 `Hello, world!`（支撑「成功不打印」与「可执行文件无后缀差异」两句）；
    8. `cargo fmt --version`（rustfmt 1.9.0-stable）、`cargo clippy --version`（clippy 0.1.98）。
  - 一手来源核对（1.98 本机离线快照）：Cargo Book `commands/cargo-build|run|check|fmt|clippy.html`（三条命令的官方定义、`--` 传参、fmt/clippy 属「随工具链分发的可选组件」）、`commands/cargo-new.html`（`--edition`：`The edition to use. Default is 2024.`）、`reference/config.html`（`[cargo-new] vcs`，支撑 `--vcs none`）；Book `ch01-01-installation.html`（安装入口、Linux/macOS 脚本、`rustup update` / `self uninstall` / `doc`、版本串格式）、`appendix-04`（`Rust installations include rustfmt by default`）、`guide/creating-a-new-project.html` 与 `faq.html`（`Cargo.lock` 与版本控制）；edition 2024 事实取 `research/02` §4.1（Edition Guide：2024 稳定于 1.85）。
  - 遗留不确定：① 正文里的耗时数字（0.87s / 1.18s / 2.29s…）是本机并发写盘下的实测值（同批其他票也在跑 cargo，出现过 `Blocking waiting for file lock on package cache` 行，已剔除），只作量级参考，正文没有据此下任何比较性结论；② 编译行里的项目路径统一压成 `/path/to/hello`、报错里的工具链路径压成 `...\library\std\src\macros.rs:141:1`，正文已声明前一处；③ `rustc hello.rs → .\hello.exe` 的探针实际跑的是同内容的 `solo.rs`（文件名不同、结论一致）。
  - 新术语：无待入表新词。正文按 GLOSSARY 首现标注了「包（package）」「crate（编译单元）」「宏（macro）」；`profile（构建配置）` **不在** GLOSSARY 内，本章按「profile（构建配置）」标注，建议协调者决定是否入表；`edition` 沿用社区原词未译。
  - 邻章衔接：第 2 章（报错方法论）、第 8 章（依赖与模块）、第 27 章（宏）、第 30 章（cargo 全命令面：features/workspace/发布）、第 33 章（edition 迁移）均以「（第 N 章展开）」标出；第 4 章（所有权）未触碰，本章只提到「rustc 做完整检查（类型、借用、……）」。全书跨章指引写法也写进了「本书约定」第 6 条。
  - 图示需求：①「cargo run 的编排」现在是 text 块里的 ASCII 竖排流程（命令 → cargo 编排 → rustc → target/ → 终端），08 票统一图示体系后建议升级为横向管线图；②「三层分工 rustup / cargo / rustc」现在是表格，可升级为同心/分层图；③ `target/debug/` 目录树想配「哪些是可删缓存、哪些该提交（`Cargo.lock` 反之）」的标注图；④ `cargo check` vs `build` vs `run` 的产出差异（有无 `.exe`）值得一张对照图。
