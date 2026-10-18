# 16 第 7 章 错误处理：Result、panic 与自定义错误

Type: task
Status: resolved
Blocked by: 06

## Question

写基础篇第 7 章：Option/Result、`?`、panic 与可恢复错误的分工、自定义错误类型、main 的返回约定。

- 心智模型：错误在类型里 —— 为什么 Rust 逼你把失败写进签名。
- 诊断训练：读懂 `?` 引入的类型错误与 `From` 转换报错。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：`src/ch07-error-handling.md`（八节模板齐全，3368 个 CJK 字，计法为正文全部汉字、含代码注释）；片段 `snippets/ch07/config_error.rs`（完整可运行程序，被 `{{#rustdoc_include}}` 引入 —— 实测 mdbook test 会编译**并实际运行**它）。
- 自验：`mdbook test -c ch07-error-handling.md` → exit 0。共 18 个 doctest：14 个编译/运行块 + 4 个 `compile_fail`；其中 2 个故意以非零退出/panic 收尾的块标 `rust,no_run`（7.3 的 `main -> Result` 失败路径、现场三的 `None.unwrap()`），只编译不运行。门禁复验：把现场三临时改回 `rust` 后立即 FAILED（exit 101），确认该命令确实在跑本章块。
- 实测输出（全部在 `.scratch/tmp-16/` 真跑，rustc 1.98.0 stable / Windows / edition 2024；探针已删）：
  1. `None.unwrap()` panic：`thread 'main' (5400) panicked at src/main.rs:3:21:` + `called \`Option::unwrap()\` on a \`None\` value` + `note: run with \`RUST_BACKTRACE=1\`…`；退出码 101。
  2. `RUST_BACKTRACE=1` 回溯（`--crate-name config_tool`）：帧 0–6，含 `core::option::unwrap_failed` @ `option.rs:2251`、`config_tool::main`，末尾 `note: Some details are omitted…`。
  3. `main() -> Result<(), Box<dyn std::error::Error>>` 返回 Err：stderr `Error: ParseIntError { kind: InvalidDigit }`，退出码 1（`main` 打印的是 Debug）。
  4. `snippets/ch07/config_error.rs` 运行输出（3 组输入；逐条有 Display / Debug / source）。
  5. 7.6 的 `{:#?}` 多行 + `{}` 单行输出。
  6. 报错 1：`?` 用在返回 `()` 的函数里 → E0277，含 `help: consider adding return type` 与 `Result<(), Box<dyn std::error::Error>>` 完整建议。
  7. 报错 2：`Option` 的 `?` 用在返回 `Result` 的函数里 → E0277 + `use \`.ok_or(...)?\` to provide an error compatible with …`。
  8. 报错 3：自定义错误缺 `impl Display` → E0277 + `note: required by a bound in \`std::error::Error\`` / `pub trait Error: Debug + Display`。
  9. 练习 3 的 `{:#?}` 输出 `Err(\n    "上层：boom",\n)`。
- 遗留：① 现场四输出里的 `note` 源码路径按 §2「剔路径噪声」保留为 `library/core/src/error.rs:59:26`（原始输出带 `C:\Users\…\rustlib\src\rust\` 前缀）；② panic 首行括号里的线程号（`5400` / `11952`）是 1.98 的真实输出成分，每次运行都会变；③ `no_run` 块在渲染版仍可点 play，但 `mdbook test` 不运行它（有意为之）。其余无。
- 新术语（GLOSSARY 目前没有，请协调者统一并表；我未改 GLOSSARY）：① 展开（unwind）——表里只有 panic，建议按「panic 的展开（unwind）：沿调用栈清理并退出」补一条（与「宏展开」区分）；② 错误链（error chain）——`source()` 串起来的因果链；③ 错误传播（error propagation）——可挂在「问号运算符（`?` operator）」名下。另外 `Result`/`Option`/`unwrap`/`expect`/`ok_or`/`source` 是代码原名，我按惯例保留原文、未立条目，请定。
- 邻章衔接：第 5 章（枚举与模式匹配，是本章前提）、第 9 章（「panic 是契约的一部分」+ 写进 API 文档，文中一句引子）、第 12 章（coherence → `From` 为什么不能乱写）、第 13 章（`dyn` 与 `Box<dyn Error>`）、第 18 章（API 级错误设计、`source()` 链工程化，本章两处明写「第 18 章展开」）、第 40 章（`panic = "abort"` 策略）。
- 图示需求（本批按约定用 text/表格，未用 mermaid）：① 7.3 想要一张 `?` 语义流程图（现为 text 伪代码 + `match` 展开）；② 7.5 想要一张错误链 / `source()` 指向图（现为 text 树）；③ 7.1 想要一张「unwind 沿栈清理 vs abort 直接终止」对比图。
- 2026-10-18 复核修订（reviewer RwCh07 findings：1 blocker + 4 minor，全部采纳并修完）：
  1. **blocker**（首段 / 为什么这样设计末句 / 小结三条）：「忽略 `Result` 就编译不过」不成立 → 改为：忽略时编译器默认给出 `unused_must_use` 警告，`let _ =` 可显式丢弃；**要拿到 `Ok` 里的值就必须先处理 `Err`**。三处说法同步改准（不再说「绕不过去 / 想忽略都难」）。
  2. 「异常路线」限定为 Python、C++ 与 Java 的**非**受检异常；另起一条「Java 的受检异常（checked exception）」承认 `throws` 把失败写进了签名，但常被上层 `catch (Exception)` 架空。
  3. `supertrait` 首现改为「父 trait（supertrait）」，其后（现场四、延伸阅读）用「父 trait」。
  4. 回溯节选口径统一：0–6 帧一律只留「帧号: 符号」，删掉原输出中标准库帧的 `at /rustc/<hash>/…` 行，并在正文注明（举例 `core::option::unwrap_failed` → `library/core/src/option.rs:2251`，此处统一省去）；7.1 panic 首行补注「括号里的线程号每次运行都不相同」。
  5. 练习 2 与答案统一：题干改 `MyError::NotFound(u32)`，答案改 `ok_or_else(|| MyError::NotFound(id))`，并点明它与 7.4「需要构造就用 `ok_or_else`」规则的对应。
  - 复跑自验：`mdbook test -c ch07-error-handling.md` → **exit 0**（doctest 数与构成同前）。
  - 另注（非 findings）：本轮期间同一文件被并行写手改过——`## 延伸阅读` 6 个链接的 `/1.98.0/` 版本锚被去掉（改为不带版本的 stable 链接），其余内容与本稿一致。我未回退（不覆盖他人改动），请协调者确认全批链接是否统一去版本锚。

## Answer

2026-10-18 批次收尾（用户验收通过）。

- 交付：`src/ch07-error-handling.md`（3582 汉字）+ `snippets/ch07/config_error.rs`。
- 自验：`mdbook test -c ch07-error-handling.md` exit 0（18 doctest：14 编译/运行 + 4 `compile_fail`，含 2 个 `no_run`）。
- 交叉审校 5 条全修（**blocker：「忽略 `Result` 就编译不过」实为 `unused_must_use` 警告**；Java 受检异常表述；父 trait 体例；回溯节选口径与线程号；练习 2 与答案统一）。全量链：`mdbook build` 0 WARN/ERROR（含 linkcheck2）｜`mdbook test` exit 0（13 章）｜`cargo test --manifest-path examples/Cargo.toml` 19 + 2 passed。
- 遗留不确定：note 源码路径按规范剔噪（正文注明）。
