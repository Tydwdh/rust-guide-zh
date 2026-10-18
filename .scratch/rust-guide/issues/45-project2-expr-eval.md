# 45 项目 2：表达式求值器（小型库 crate）

Type: task
Status: claimed

## Question

写第二篇收尾项目：**表达式求值器**（`examples/expr-eval/` 已建 stub，crate 名 `expr-eval`；写作者替换 stub 为真实现）。

- **目标**：零依赖库 crate：词法（tokenizer）→ 语法分析（递归下降或 Pratt，自选并论证）→ AST → 求值；覆盖第二篇概念并让读者看见它们「为什么这么设计」：trait 与关联类型（操作符/节点设计）、泛型与 impl Trait 的取舍、生命周期（借用词法切片 vs 拥有 String）、智能指针（`Box<Expr>`，必要时讨论 Rc）、错误设计（自定义错误 enum + `Display`/`Error`/`From` 实例）、API 设计（公开面最小化、doc 注释、`#[must_use]`/`#[non_exhaustive]` 的取舍）。
- **章结构**：对齐项目 1 成稿（逐块实现 + 设计取舍 + 报错现场 + 小结 + 练习 2–3 题带答案）；正文用 `{{#include}}`/`{{#rustdoc_include}}` 引 crate 真实代码；每段引用的代码与测试都必须真实且全绿。
- **素材**：已定稿的 ch12–18（尤其 12/14/15/17/18）；算法自包含，无需外部资料。
- **边界**：std-only（零依赖）；不用 `unsafe`；含一个薄 CLI `src/bin/eval.rs`（能被 e2e 测试，参照 todo-cli 的写法）；异步/并发不涉。
- **篇幅**：4000–5500 汉字（不含代码块）。
- **DoD**：§8 —— 独占 `src/project2-expr-eval.md` + `examples/expr-eval/**`（含其 `Cargo.toml`）+ 本票；禁 `git`/禁 `mdbook build`；自验：`CARGO_TARGET_DIR=.scratch/tmp-45/target cargo test --manifest-path examples/Cargo.toml -p expr-eval` 全绿 + `mdbook test -c project2-expr-eval.md` 绿；探针 `.scratch/tmp-45/`（收尾删）；**勿动** `examples/Cargo.toml`（workspace 根，members 已配好）、`examples/README.md`、其他 crate、其他章/票；收尾 `Status: claimed` + `## Comments`（同 §8 清单 + crate 的测试计数）。

## Comments

**做了什么**

- `examples/expr-eval/`（替换 stub；零依赖、`#![forbid(unsafe_code)]`、全文无 `unsafe`、无 panic 路径）：`src/lib.rs`（crate 文档 + 私有模块 + 公开面 `pub use`）、`src/error.rs`（`Error` 八变体 + `Result<T, E = Error>` + `offset()` + `Display`/`std::error::Error`）、`src/lexer.rs`（`TokenKind<'a>`/`Token`/`Lexer: Iterator`/`tokenize`，标识符借用源串）、`src/ast.rs`（`BinOp` + `binding_power`、`Expr`、完全括号化 `Display`）、`src/parser.rs`（递归下降 + Pratt，`parse`）、`src/eval.rs`（`Env` trait + `Bindings` + `call_builtin` + `Expr::eval` + `eval_str`）、`src/bin/eval.rs`（薄 CLI：`--ast` / `--var` / `-` 读 stdin / `--help`；`CliError` 带 `From<io::Error>`、`From<EvalError>`；退出码 2 + 脱字符）、`tests/cli.rs`（端到端 9 条）。crate 自己的 `Cargo.toml` 未改（`[dependencies]` 保持空）。
- `src/project2-expr-eval.md`（替换占位）：结构对齐项目 1 成稿（这一章解决什么问题 / 设计取舍 / 逐块实现 / 概念落点对照 / 报错现场×3 / 测试 / 收尾 / 小结 / 练习 3 题带答案 / 延伸阅读）；31 处 `{{#include}}` 全引 crate 原文；1 张手写内嵌 SVG。篇幅 **5482 汉字**（不含代码块，落在 4000–5500）。

**自验命令与结果**

- 说明：为不碰共享的 `examples/target/`，所有 cargo 命令都带 `CARGO_TARGET_DIR=.scratch/tmp-45/target`；正文的 console 块按读者口径写（不带该变量），输出与之一致。
- `CARGO_TARGET_DIR=.scratch/tmp-45/target cargo test --manifest-path examples/Cargo.toml -p expr-eval` → 全绿：**单元测试 35 passed**、**端到端 9 passed**、**文档测试 6 passed**（合计 50）；无 warning。
- `mdbook test -c project2-expr-eval.md` → **exit 0**（绿）。负向对照：临时插入一个坏块 → exit 101，证明门禁确实在编译本章。本章 45 个代码块中**实编 5 个**（3 个 `rust` + 2 个 `rust,compile_fail`，即 5 passed）、**30 个标 `rust,ignore`**（跨模块片段，沿用项目 1 的做法）、其余为 `console`/`text`。
- `cargo clippy --manifest-path examples/Cargo.toml -p expr-eval --all-targets` → 0 warning；`cargo fmt --manifest-path examples/Cargo.toml -p expr-eval -- --check` → 干净（先跑过一次 `cargo fmt -p expr-eval`）。

**正文里真跑真粘的输出**

- 成功路径：`1 + 2 * 3` → `7`；`--ast 1 + 2 * 3` → `(1 + (2 * 3))`；`--var x=3 "x^2 + 1"` → `10`；`-2^2` → `-4`；`sqrt(abs(-16))` → `4`；
- 错误路径（退出码均 2）：`1 / 0`、`2 + * 3`（含脱字符行）、`x + 1`、`pow(2)`、`1 @ 2`、`--var x`、无参数；
- 其他真跑：`--help` 全文、`echo "2 ^ 10" | eval -` → `1024`；
- 报错现场：E0499（`peek` 后 `next`）与 E0597（切片借用的局部源串）各写一个自包含最小复现，用 `.scratch/tmp-45/probe/*.rs` + `rustc --edition 2024 --emit=metadata` 实跑粘贴；两条警告（`#[must_use]` 触发的 `unused_must_use`、漏写生命周期的 `mismatched_lifetime_syntaxes`）分别用探针 crate 与 rustc 实跑粘贴。

**实测到的 1.98 行为（与旧印象不同，正文已按实况写并标注）**

- edition 2024 下 `fn lex(src: &str) -> Vec<Token>` **不报 E0106**（唯一输入生命周期由省略规则接上），只给 `mismatched_lifetime_syntaxes` 警告。**返修订正**：这条与 edition 无关（2015/2018/2021/2024 输出完全一致），该 lint 自 1.89 起默认打开；E0106 只在省略规则判不出来时才出现（两个输入生命周期）。正文已按此改写。
- `mdbook test` 全绿时**不打印任何测试计数**（只在失败时回显 rustdoc 输出）；已用坏块负向对照确认门禁有效。

**遗留不确定 / 需协调者接手**

1. `examples/README.md` 的成员表目前只有 `todo-cli` 一行，需补一行 `expr-eval`（按票要求未动该文件）。
2. debug 里的 `src[..offset]` 字符对齐依赖「非法多字节字符会在词法阶段先报错」这一事实；正文原有一小段解释，为压篇幅删掉了，若要保留可回填。
3. `examples/Cargo.toml`、其他 crate、其他章/票均未改。

**新增术语**：无。用到的「词法分析 / 语法分析 / 绑定力 / 内置函数」均为普通叙述用词，未新增 GLOSSARY 条目；已有条目（trait、关联类型、借用、切片、智能指针、退出码…）照用。

**衔接提示**：本章引用第 4/5/6/7/9/10/12/13/14/15/16/17/18 章；「概念落点对照」表逐章给出落点，其中第 15 章一行明确写「AST 刻意不泛型化：泛型会传染每个签名」，供 ch15 审校核对口径。

**图示需求**：本章自带 1 张手写内嵌 SVG（三步流水线：借用 → 拥有 → 求值，附错误回流说明行），已按 WRITING §9 自查（单块无空行、无脚本/外链、`viewBox` + `width="100%"` 且无 `height`、仅用主题变量、`role="img"` + `aria-label`、`&`/`<`/`>` 已转义、无需要 `xml:space` 的列对齐文本）。若后续补图票统一调整风格，可一并处理。

---

**复审返修（reviewer RwProj2：7 major + 11 minor，逐条已处理）**

*事实 / 版本*

1. 【major】`Error` 变体数：本轮加深了 `TooDeep`，实数 **8**（`UnexpectedChar`/`BadNumber`/`UnexpectedToken`/`TooDeep`/`UnknownName`/`UnknownFunction`/`WrongArity`/`NotFinite`），正文「八个变体」与分组句（词法 2、语法 2、求值 4；带字节偏移的只有前三个）现在都成立。
2. 【major】删掉「edition 2024 下不再报 E0106」的框架：**实跑四个 edition（2015/2018/2021/2024），输出完全一致**（同一条 `mismatched_lifetime_syntaxes`、都不报错），该 lint 自 1.89 起默认打开、与 edition 无关；正文改成「不是 edition 的问题 + E0106 只在省略规则判不出来时才出现（`fn lex(a: &str, b: &str) -> Vec<Token>`）」。
3. 【minor】警告块与片段对齐：片段补 `fn main()`（`fn lex` 落在第 6 行），输出改 `--> main.rs:6:13`；`dead_code` 噪声按 §2 剔除。
4. 【major】现场一行号：按所示片段实跑得 gutter **23/24/25**（span `24:21`），正文照贴；尾段「第 26/25 行」改「第 25/24 行」。
5. 【major】现场二行号：实跑 `--> main.rs:13:13`、gutter **11/12/13/14**；「结果活到第 16 行」改「第 15 行」。
6. 【major】`--var` 演示：实测 `--var x 1` 报「`` `x` 不是 `名字=值` 的形式 ``」，裸 `--var` 才报「`` `--var` 后面要跟 `名字=值` ``」；console 块改用裸 `--var`。
7. 【minor】「第 4 章『生命周期是数据寿命的记账』」在 ch04 不存在（原话是「不需要运行时记账，也不需要人工检查」）→ 改成「这正是第 4 章借用规则背后的那个问题」，不加引号。
8. 【major】练习 2 重做（题干改「依次改成 8、3、1」，答案按实跑）：**8** → `-2 ^ 2` 变 `((-2) ^ 2)` = 4（`-2 ^ 2 ^ 3` → `((-2) ^ (2 ^ 3))` = 256）；**3** → `-2 * 3` 树变 `-(2 * 3)`（值仍 -6），`-1 + 1` **不变**（`+` 左绑定力 1 < 3）；**1** → `-1 + 1` 变 `-(1 + 1)` = -2、`-1 - -1` 变 `-(1 - (-1))` = -2。
9. 【minor】练习 1：删「每个数字也复制一次」（数字在词法阶段已变 `f64`），改为「每个标识符在词法阶段复制一次」。

*代码可复现*

10. 【major】多字节路径补齐并修 bug：`lexer` 加全角空白测试（`"　1　+　2"` → 偏移 `[3, 7, 11]`，字节语义）；`bin/eval.rs` 新增 `columns_of`（ASCII 一列、宽字符两列、制表符推进到 8 的倍数，附 3 个单测）并**修掉脱字符列宽**（`"　1 @ 2"` 的 `^` 从第 3 列移到正确的第 4 列）；e2e 加 `a_wide_space_does_not_shift_the_caret_column`。
11. 【minor】「断言错误种类」改为「断言数值、错误消息与 `offset()`」（此前并未断言变体种类）。
12. 【minor】「`run` 里 `?` 冒泡」改为「`run` 把它们收进 `CliError` → 打印 → 退出码 2（`?` 只在参数解析与读标准输入两处自动转换）」。
13. 【minor】演示命令里的表达式一律加引号（`-- "1 + 2 * 3"`）；收尾对「拆成多个参数」的说明补了 shell 元字符提示；加引号的命令已重跑，输出一致。
14. 【minor→已修】「绝不 panic」的洞补上：`parser` 加深度上限 `MAX_DEPTH = 128`（新变体 `Error::TooDeep`）。1000/400 层括号从「栈溢出 exit 253」变成「错误：表达式嵌套太深（最多 128 层）」、退出码 2；新增 3 个 parser 单测（64 层可解析、超限报 `TooDeep`、2000 项左结合链仍走循环）+ 1 个 e2e（1000 层经 stdin → exit 2）；验收标准第 2 条同步写明。

*规范 / 跨章*

15. 【minor】「寿命」清零（标题 + 正文 4 处改成「借用 / 生命周期」措辞）。
16. 【minor】公开面表补 `call_builtin`（`lib.rs:30` 确有导出）。
17. 【major】「闭集 vs 开集」改指**第 13 章**（ch13:444「能被枚举穷举的，优先枚举」）；§4 的 callout、概念表 row 12/13 一并改。
18. 【minor】概念表 row 15 改为「泛型的边界（const 泛型、GAT）→ 本项目用不上」；「类型参数会传染签名」归入 row 13。

**返修后的自验（第二轮）**

- `CARGO_TARGET_DIR=.scratch/tmp-45/target cargo test --manifest-path examples/Cargo.toml -p expr-eval` → 全绿：**库单元 39**（+4）、**CLI 单元 3**（新增）、**端到端 11**（+2）、**文档测试 6**，合计 **59**；无 warning。
- `mdbook test -c project2-expr-eval.md` → **exit 0**；代码改动后 32 处 `{{#include}}` 的行号**逐条脚本复核**（每个范围的首行/末行都落在预期项上，如 `parser.rs 106:137` = 深度守卫 + 主循环、`bin/eval.rs 174:207` = `fail` + `columns_of`）。
- `cargo clippy -p expr-eval --all-targets` → 0 warning；`cargo fmt -p expr-eval -- --check` → 干净。
- 篇幅：**5495 汉字**（不含代码块，仍在 4000–5500；为容纳本轮事实修订，把「怎么继续扩展」压成一段、若干措辞收紧）。
- 本轮新实测输出：`"　1 @ 2"` → 「偏移 5 处有不认识的字符 `@`」+ 4 空格脱字符；400/1000 层括号 → 「表达式嵌套太深（最多 128 层）」；裸 `--var` 与 `--var x 1` 两种消息；`PREFIX_BP` = 5/3/1/0 的 AST 与值；四个 edition 的生命周期警告一致。

**仍未做（等协调者定）**

- `examples/README.md` 成员表仍只有 `todo-cli`（按票要求未动）。
- 正文未演示 CLI 上的深度上限（只在 §4 说明 + 单测/e2e 覆盖），若希望 console 里也出现一条 200 层括号的示例，可以补。
