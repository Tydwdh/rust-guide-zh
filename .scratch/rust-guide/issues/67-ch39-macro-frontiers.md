# 67 第 39 章 宏系统极限

Type: task
Status: claimed

## Question

写第六篇第六章：**宏系统极限**（素材 = D6：reference `procedural-macros.html`、unstable-book）。

- **范围**：`macro_metavar_expr`（`${count}`/`${index}` 等，nightly 标注 + 只文档化）；声明宏与过程宏的组合形态（函数式宏生成属性宏调用、属性宏叠加顺序——stable 部分真跑）；`#[track_caller]`（stable，真跑 `Location::caller`）；诊断消息设计（`compile_error!` 的 span 与布局、`proc_macro::Diagnostic` nightly 标注）；卫生性/`$crate` 的极限情形（回指 ch27，不重讲）。
- **注意**：**不要碰 `examples/macro-demo` / `macro-demo-use`**（ch28 章内 include 依赖其逐行布局）；需要过程宏就在 `snippets/ch39/` 里自建独立小 crate（零依赖）。
- **边界**：宏基础 → ch27/28 回指；不提供 nightly 特性的伪造输出。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch39-macro-frontiers.md` + `snippets/ch39/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch39-macro-frontiers.md` exit 0（snippet crate 测试私有 target）；探针 `.scratch/tmp-67/`；收尾 `Status: claimed` + `## Comments`。

## Comments

**文件（本票独占，未碰 `examples/macro-demo` / `macro-demo-use`）**

- `src/ch39-macro-frontiers.md`：成稿，八节模板 + 1 张内嵌 SVG（属性宏叠加展开顺序），5497 汉字（在 4000–5500 内）。
- `snippets/ch39/`：`track_caller.rs`、`hygiene_limits.rs`（都在正文里用 `{{#rustdoc_include}}` 全量纳入，且都可编译可运行）、`Cargo.toml`（workspace；`Cargo.lock` 不入库，跑一次 `cargo test` 会自动生成）、`attr-demo/`（proc-macro crate，手写 token 处理、零外部依赖：`tag` 属性宏 / `Sniff` derive / `make_item!` 函数式宏）、`attr-demo-use/`（消费方 + 一个 `#[test]`）。

**自验**

| 命令 | 结果 |
|---|---|
| `mdbook test -c ch39-macro-frontiers.md` | exit 0（14 块：3 passed / 6 `ignore` / 5 `compile_fail`） |
| `cd snippets/ch39 && cargo test` | 1 passed（`tests::markers_exist`） |
| `cd snippets/ch39 && cargo run -q -p ch39-attr-demo-use` | 13 行输出，已粘进正文 |
| `rustc --edition 2024 snippets/ch39/{track_caller,hygiene_limits}.rs` | 均 exit 0、无警告，运行输出已粘进正文 |

snippet crate 全程用私有 target（`CARGO_TARGET_DIR=.scratch/tmp-67/...`），`snippets/ch39/target/` 不存在；探针目录 `.scratch/tmp-67/` 收尾已删除。

**实测过的输出（全部真跑真粘）**

1. `track_caller.rs`：`located()` 67 / `wrapper()` 13（两次一致）/ `outer()` 70 / `generic::<u8>()` 71 / `S.method()` 72 / `&dyn Declared`（声明已标注）75 / `&dyn Undeclared`（只标实现）57 / 函数指针 7 / `unwrap` panic 90。
2. `hygiene_limits.rs`：`call_helper!() = 10`、`bump!(x) = 11（调用处的 x 仍是 10）`。
3. `attr-demo*`：编译期 14 行 `[tag]`/`[Sniff]` 日志（顺序证据：外层 `item` 里仍含内层属性；`#[derive]` 写在前面则 derive 先跑且输入里带着后面的属性；宏生成的属性宏调用照常展开）+ 运行期 13 行 + `cargo test` 结果。
4. 报错现场（`main.rs` 行号与正文块逐行一致，已程序化比对）：E0658（`${count($x)}`；`$$` 是同一条错误）、E0428（item 名不卫生）、E0425（宏体里的函数按调用处解析，含 rustc 的 `help: consider importing`）、`compile_error!` 双诊断布局（主 span 落在宏定义、`in this macro invocation` 标调用点）。
5. 少写 `$` 的真实原文：`meta-variables within meta-variable expressions must be referenced using a dollar sign`。

**引文来源**（逐字核对，正文均标注出处）

- Rust Reference：procedural-macros（token 流理由、不卫生、属性宏第二个参数的措辞）、attributes/codegen 的 `track_caller`（allowed-positions、trait、extern、behavior、shim 退化）、macros-by-example（混合卫生、`$crate`）——用 1.98 本地 doc 快照核对。
- `std::panic::Location`：`caller()` 的「继续往上」原话。
- rustc 错误索引 E0737（`extern "C"` + `#[track_caller]`）。
- RFC 3086（元变量表达式语法表）、rust-lang/rust#83527（`$` 前缀要求的提案、方向之争、`$$`/`$$crate` 未决、生成字面量的卫生问题）、#124225 / #54140 / #54724 / #87417、unstable-book 对应页。

**版本口径**：nightly-only 内容（`${count}`、`$$`、`${concat}`、`proc_macro::Diagnostic`、闭包 `#[track_caller]`、`Span::def_site`）一律 `rust,ignore` + `> [!WARNING]`，只给文档化用法，**未粘贴任何伪造的 nightly 输出**。

**遗留不确定**

1. `macro_metavar_expr` 的 depth 语义：已按 1.98 源码（`metavar_expr.rs` 的 `0 is the innermost`）与 UI 测试断言（`[13, 4, 2]`）写明「0 = 最内层、1.76 起与 `index`/`len` 同向」；本机只有 stable，所以这些结论来自源码与测试，**没有**本机 nightly 的实跑输出。若要再加一道保险，可由协调者在 nightly 上跑一遍 `tests/ui/macros/metavar-expressions/macro-expansion.rs` 复核。
2. SVG 只用 `read :img` 自检过渲染（单块无空行、700×250、主题变量、`&amp;str` 转义）；**HTML 版式仍请协调者在收尾 build 时目视确认**。
3. `serde` 的 `#[serde(crate = "…")]` 属转述生态惯例（第 28 章已有同样表述），未在本机验证。

**新增术语（供 GLOSSARY 并表）**：元变量表达式（macro metavariable expression）、组合形态（macro composition）、调用点（call site）。

**衔接提示**：前置 = 第 27 章（`macro_rules!` 匹配/卫生/`$crate`）+ 第 28 章（proc macro、`Span`、`compile_error!` 的两条路）；章内一律写「第 27/28 章」回指，未重讲基础。`snippets/ch39` 是独立 workspace，不在 `examples/Cargo.toml` 成员里，`cargo test --manifest-path examples/Cargo.toml` 不受影响。

## 修复清单（审校回执 RvCh39：1 blocker + 2 major + 7 minor，全部采纳）

| 条目 | 处理与依据 |
|---|---|
| **F1 · blocker**（count 与 index/len「方向相反」在 1.98 不成立） | 采纳，39.1 表格与第三条说明重写为「1.76 起统一：`0` = 最内层，往外递增；`${count($x)}` 与 `${count($x, 0)}` 等价」，RFC 的旧口径（默认统计所有深度、`${count(x, 1)}` 数最外层）并列作对照。依据（均为 1.98 一手材料）：`compiler/rustc_expand/src/mbe/metavar_expr.rs` 的 `Index`/`Len` 变体 doc 写着 "0 is the innermost repetition"、`parse_count` 缺省 depth 为 0；UI 测试 `tests/ui/macros/metavar-expressions/macro-expansion.rs` 断言 `[${count($x,0)}, ${count($x,1)}, ${count($x,2)}] == [13, 4, 2]`（RFC 是 `(2, 4, 13)`）；PR #117050 的 milestone 经 GitHub API 实查为 **1.76.0**（closed 2023-12-13）。另外该节开头的「等于 `$x` 这段重复的总次数」也改成「单层重复里就是一共几个元素」以免在嵌套场景里失真。 |
| **F2 · major**（`${length(depth)}` 非法） | 采纳：表格改为 `${index(depth)}` / `${len(depth)}`，并新增一条「实现里的名字是 `len`，RFC 文档写 `length`」。依据：`metavar_expr.rs` 的 `match ident.name` 只认 `concat`/`count`/`ignore`/`index`/`len`，`MveUnrecognizedExpr` 列的合法名就是这五个；`tests/ui/macros/metavar-expressions/count-and-length-are-distinct.rs` 用的正是 `${len()}`。 |
| **F3 · minor**（`$` 前缀不是「对齐 RFC」） | 采纳：改为「有意偏离 RFC」，并直引提案原话 "For unblocking purposes, the requirement of `$` is being suggested."（RFC 原本不带 `$`）。 |
| **F4 · minor**（`Span::error/…` 无「更早」之说） | 采纳：删「更早的」，改为「同一 gate 下」，并补上 tracking issue #54140。 |
| **F5 · minor**（`def_site` 挂在 #54724 的具体说法无据） | 采纳：#54724 原文引 "likely blocked on larger hygiene reform and more thorny Macros 2.0 issues"，另述 lang team 2022 年把该 issue 标为 needs-summary、要求先把 intended effect 写成可评审文档；不再声称「与 `mixed_site` 的差别、路径解析未定」有出处。 |
| **F6 · minor**（include 少了收尾 `}`） | 采纳：`attr-demo/src/lib.rs:9:28` → **`:8:29`**（从 `use proc_macro::…` 到 `tag` 的 `}`）。已复核该区间末尾确为 `}`，`mdbook test` 仍 exit 0。 |
| **F7 · minor**（`$$` 的报错描述） | 采纳：改为「写 `$$` 得到的是同一类 E0658——每处 `$$` 各报一条（note 都指向 issue #83527），并且在解析中断的地方还会跟着一条普通语法错误」。 |
| **F8 · minor**（术语首现缺英文括注） | 采纳：首现处补 `元变量表达式（macro metavariable expression）`、`组合形态（macro composition）`、`调用点（call site）`；删掉 39.1 正文里重复的英文括注。 |
| **F9 · minor**（票内数字与条目） | 采纳：自验表改为 3 passed / 6 ignored / 5 compile_fail = 14 块；删 `Cargo.lock` 交付物行并删除该文件（跑一次 `cargo test` 会自动重新生成，本目录不下沉 lockfile）；删「`${concat(…)}` 示例取自 unstable-book」；删「诊断级别」词条；「遗留不确定」第 1 条按 F1 的新口径重写。 |
| **F10 · minor**（SVG 第二个框文字贴底） | 采纳：第二个框的两行文本 `y=96/112` → `y=88/106`（底边距 8px，与第一个框一致），已重新用 `read :img` 目视确认。 |

复验：`mdbook test -c ch39-macro-frontiers.md` → **exit 0**（3 passed / 6 `ignore` / 5 `compile_fail`）；探针 `.scratch/tmp-67/` 再次清理。
