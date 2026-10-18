# 21 项目 1：命令行小工具

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇的实战项目：一个命令行小工具（候选：待办清单 CLI / 文本统计工具），把第 1–11 章的概念合成一个完整可跑的项目。

- 结构建议：需求 → 设计取舍 → 逐块实现（每块指回对应章）→ 测试 → 收尾（错误处理与用法说明）。
- 要求：完整可编译可运行；代码入库（目录位置按 06 定的示例规范）。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿（票 21，写作者）：
  - **文件**：章 `src/project1-cli-tool.md`（H1「项目 1：命令行小工具」；正文汉字 4450，不含围栏代码块）；代码 `examples/Cargo.toml`（workspace 根：`[workspace]` + `resolver = "3"` + `members = ["todo-cli"]`）+ `examples/todo-cli/`（`edition = "2024"`，无 `[dependencies]`，只用 std：`src/{main,cli,error,task,store}.rs` + `tests/cli.rs`）。
  - **选型：待办清单 CLI（`todo-cli`）**。理由：一张表里同时要有「状态」（`Status`）、「动作」（`Command`）、「筛选」（`Filter`）、「错误」（`AppError`）四个枚举，第 5 章的模式匹配与第 7 章的错误枚举才有真实落点；文件格式又是「自己做取舍 + 在入口守不变量」的天然载体（制表符分隔 → `add` 处拒绝制表符/换行）。文本统计工具的重心在纯函数 + 迭代器（第 6、10 章），枚举/模式匹配分量不足，故未选。
  - **章内结构**：需求与验收标准 → 设计取舍（数据表示 / 文件格式 / 错误策略 / 参数解析 / 写文件，每条含「为什么不选另一个」）→ 逐块实现（5 块，每块末一句「这块用到第 N 章的 X」，末尾附「概念落点对照」表）→ 报错现场（2 个）→ 测试 → 收尾（用法 + 错误与退出码 + 怎么继续扩展）→ 小结 → 练习（3 题 + `<details>` 答案）→ 延伸阅读（8 条官方一手链接，全部按 1.98 快照核对存在）。

  ### 自验（全绿）
  1. `cargo test --manifest-path examples/Cargo.toml` → exit 0：`17 passed; 0 failed`（单元，`src/main.rs`）+ `2 passed; 0 failed`（集成 `tests/cli.rs`），共 **19** 个测试。
  2. `cargo build --manifest-path examples/Cargo.toml` → exit 0，**0 warning**。
  3. `mdbook test -c project1-cli-tool.md` → exit 0。
  4. `cargo fmt --manifest-path examples/Cargo.toml --check` → exit 0（先跑过 `cargo fmt`）。
  5. include 行号切片核对：脚本解析章内 13 条 `{{#include}}`，逐条打印头/尾行，全部落在 doc 注释/项首与闭合 `}` 上（清单见下）。
  6. 章内终端记录全部来自真跑（见「实测输出清单」）。

  ### 实测输出清单
  - 开场全流程（cwd=仓库根，数据文件为默认 `todo.txt`）：`add 读完第 11 章`→`已添加 #1`；`add 给项目 1 写测试`→`已添加 #2`；`list`→两行 `#n 待办 …` + `共 2 条：2 条待办，0 条已完成`；`done 1`→`已把 #1 标记为完成`；再 `done 1`→`#1 本来就是完成状态，没有改动`；`rm 2`→`已删除 #2：给项目 1 写测试`；`list`→`#1 已完成 读完第 11 章`。数据文件内容实测 `1\tx\t读完第 11 章`。
  - 取完记录后 `D:\Rust\todo.txt` 已删除（临时探针文件，收尾清理）。
  - `--help`（cwd=仓库根）：完整用法文本已粘进收尾节。
  - cwd=examples 的写法：`cargo run -p todo-cli -- --file ../todo.txt list --done` → `#1 已完成 读完第 11 章` / `共 1 条：0 条待办，1 条已完成`。
  - 错误路径（全部 `cargo run` 退出码 2）：`done 7`→`错误：没有编号为 7 的任务`；`list --bogus`→``错误：参数错误：`list` 不认识 `--bogus` ``；`add`（无文本）→``错误：参数错误：`add` 后面要跟任务文本``；`frobnicate`→``错误：参数错误：不认识的命令 `frobnicate`；跑 `todo-cli help` 看用法``；`--file`（缺值）→``错误：参数错误：`--file` 后面要跟一个路径``；坏数据文件 → ``错误：数据文件第 2 行无法解析：未知的状态标记 `?` ``。
  - `cargo test` 两段真实输出（unit 17 / integration 2 的用例名与 `test result` 行）已节选粘进「测试」节。
  - 报错现场两个最小复现各自用 `rustc --edition 2024 main.rs` 跑过（文件在 `.scratch/tmp-21/{borrow,fromstr}/main.rs`，已删）：E0502 `--> main.rs:17:5`（mutable borrow 在 16 行、immutable 在 17 行、later used 在 18 行）；E0405 `--> main.rs:6:6` + `help: consider importing this trait / use std::str::FromStr;`。章内 `compile_fail` 块与紧跟的 `text` 输出即这两次运行的结果。

  ### 第 1–11 章概念 → 代码落点（映射表）
  | 章 | 概念 | 落点 |
  |---|---|---|
  | 1 | 工具链 / `cargo run` / edition 2024 | `examples/todo-cli/Cargo.toml`、章内两种跑法 |
  | 2 | 读懂编译器报错 | `src/task.rs:4` 的 `use std::str::FromStr;`（E0405 复现）、`add` 的返回值设计（E0502 复现） |
  | 3 | 变量/类型/函数/控制流 | `main.rs`（`main`/`run`）、`cli.rs`（`parse_id`/`usage`） |
  | 4 | 所有权/借用/切片 | `task.rs` `text() -> &str`；`cli.rs` `rest: &[String]` + `split_first`；`store.rs` `tasks() -> &[Task]` |
  | 5 | 结构体/枚举/模式匹配 | `Task`/`Status`/`Filter`/`Command`/`AppError`；`match name.as_str()`；`[id]`/`[]`/`_` 切片模式；`let ... else` |
  | 6 | 集合与字符串 | `Vec<Task>`；`lines()`/`split('\t')`/`join(" ")`/`contains(['\t','\n','\r'])`/`to_owned` |
  | 7 | 错误处理 | `error.rs` 全篇；`Store::open`（`NotFound` 视为空集合）/`save` 的 `?`；`main` → 退出码 2 |
  | 8 | 模块/包/依赖 | `main.rs` 的 4 个 `mod`；`pub` 与私有字段；`examples` workspace + `-p todo-cli` |
  | 9 | 泛型与 trait | `impl FromStr for Status`、`Display`/`Error`/`From<io::Error>`、`open(path: impl Into<PathBuf>)`、`render_list<W: Write>` |
  | 10 | 迭代器与闭包 | `Store::parse` 的链 + `collect::<Result<Vec<_>, _>>()`；`find`/`position`/`max`；`select` 的 `filter` 闭包；`render_list` 的 `filter`/`count` |
  | 11 | 测试与调试 | 各模块 `#[cfg(test)]`（15 个用例）+ `tests/cli.rs` 端到端 2 个（`env!("CARGO_BIN_EXE_todo-cli")`） |

  ### include 切片清单（相对 `examples/todo-cli/`）
  - `src/error.rs:6:22`（17 行，`rust` ✅ 过 mdbook test）
  - `src/task.rs:6:41`（36 行，`rust` ✅）、`src/task.rs:43:80`（38 行，`ignore`）、`src/task.rs:82:104`（23 行，`ignore`）
  - `src/cli.rs:11:49`（39 行，`ignore`）、`src/cli.rs:51:81`（31 行，`ignore`）、`src/cli.rs:110:136`（27 行，`rust` ✅）
  - `src/store.rs:10:24`（15 行，`ignore`）、`src/store.rs:26:72`（47 行，`ignore`）、`src/store.rs:94:126`（33 行，`ignore`）
  - `src/main.rs:16:26`（11 行，`ignore`）、`src/main.rs:61:82`（22 行，`ignore`）
  - `tests/cli.rs:22:46`（25 行，`ignore`）

  ### 遗留 / 需要协调者定夺
  - **`WRITING.md` §1 的括注是错的（实测）**：`{{#include}}` 的内容**会**参与 `mdbook test`（票里的说法对）。证据：临时把探针块指向 `cli.rs:1:6`（含 `use crate::error::AppError;`）→ `mdbook test -c project1-cli-tool.md` 报 `error[E0432]: unresolved import `crate::error``、exit 101。建议改 WRITING.md §1 那句「（不参与 mdbook test）」。附带证据：故意加一个坏块后，`mdbook test` 报 `running 18 tests` —— 12 ignore + 3 rust + 2 compile_fail + 1 坏块，坏块被判 FAILED、exit 101。
  - 由此产生取舍：**跨模块的项目片段无法单独编译**，章内 13 个 include 块里只有 3 个是 `rust`（真过 `mdbook test`），其余 10 个按票内约定标 `rust,ignore` 并在正文 NOTE 里说明原因；另有 2 个 `compile_fail` 块。**`ignore` 块的切片错了不会报错**，所以 13 条切片我另外用脚本逐条核对边界（结果见上）。若协调者要求「所有块都进 mdbook test」，唯一可行方案是把项目切成 `snippets/` 下的自包含片段（与「项目代码入库 examples/」冲突），请定。
  - 正文篇幅：章内正文汉字 4450（不含围栏代码块；含代码块内注释为 4774）。若验收改用「汉字 + 标点 + 西文词」口径会超 4500，需再削约 300 字。
  - `examples/Cargo.lock` 由 cargo 生成（二进制 workspace 惯例提交），未删；`examples/target/` 已被 `.gitignore` 的 `target/` 覆盖。
  - `examples/README.md`（06 留下的占位，写着「定稿后本目录的用法会写在这份 README」）**未改**：不在本票文件所有权内，留给协调者统一写。
  - `.scratch/tmp-21/` 已删除。

  ### 新术语（GLOSSARY 候选，未自行加表）
  - `exit code | 退出码`
  - `integration test | 集成测试`（`tests/` 下那类）
  - `end-to-end test | 端到端测试`
  - `workspace`（建议保留英文，不译「工作区」）
  - `bin target | 二进制目标`（本项目仅以 `env!("CARGO_BIN_EXE_todo-cli")` 出现）

  ### 邻章衔接
  - 前置全部落在第 1–11 章；出章处点到：`dyn Error`/`Box<dyn Error>` → 第 13、17 章；错误分层与 API 设计 → 第 18 章；临时文件 + rename 的原子替换 → 第 40 章；引入外部 crate → 第 8 章；并发/异步 → 第 19、21 章。
  - 与第 11 章（测试与调试）的重叠只保留「单元 vs 集成的位置差别 + 本项目用例」，不重讲断言/调试基础 —— 建议第 11 章保持基础面，本项目只引用。
  - 本章明确写了「这里没用 `HashMap`，为什么」：若第 6 章强调 `HashMap`，可作为反向对照。
  - 跑法依赖 `examples/Cargo.toml` 的最小 workspace（`resolver = "3"`）：第 30 章（Cargo 工程化）可回指本章当最小例子。
  - 其余项目票（22–25）只需往 `examples/Cargo.toml` 的 `members` 追加自己的 crate；`resolver = "3"` 已就位。

  ### 图示需求（本批统一用 text/表格，未引 mermaid/HTML-SVG）
  - `## 逐块实现` 开头：想要「模块依赖图」（`main → cli → store → task/error`）——ASCII 盒子图或表格。
  - 开场「要做的东西」：想要「命令 → 数据文件 → 输出」的流程小图。
  - `## 设计取舍` 第 2 条：想要「一行记录 = 编号 | 标记 | 文本（TAB 分隔）」的字段示意图。
  - `## 报错现场` 报错一：想要「借用区间时间轴」（`add` 的可变借用跨到 `println!`）——借用图的统一制图方案待 08 票定。
- 2026-10-18 交叉审校回修（reviewer `RwProject1` 的 14 条；#3 归协调者，未动）：
  - **#1** `:345` 正常路径改为真实用例名（`record_round_trip_keeps_id_status_and_text`、`select_filters_by_status`、`list_shows_the_marker_and_a_summary`、`full_round_trip_through_the_binary`）；边界一行补上新增用例 `exhausted_id_space_reports_an_error_instead_of_overflowing`。
  - **#2** 延伸阅读 8 条链接去掉 `/1.98.0/` 版本段（`replace_all` → `https://doc.rust-lang.org/…`）；版本敏感事实只留在正文。
  - **#4** 「trait 边界」→「trait 约束」（4 处，对齐 GLOSSARY 的 `_Avoid_`）。
  - **#5** 概念落点表第 8 行改为「`todo-cli` 是 `examples` 工作区的成员」。
  - **#6** 编号耗尽不再 panic（先复现、再修）：`error.rs` 新增 `AppError::IdsExhausted`（Display + 单测）；`store.rs::add` 用 `checked_add` 返回 `IdsExhausted`；`store.rs::open` 用 `checked_add`，把用满的 `next_id` 钉在 `u32::MAX`，**只读命令照常可用**；新增测试 `exhausted_id_space_reports_an_error_instead_of_overflowing`（断言失败时集合不被改动）。实测 reviewer 的复现路径：数据文件 `4294967295\to\t最后一条` → `list` / `done` 正常（exit 0），`add` → `错误：任务编号已经用满，无法再添加`（exit 2，文件未被写入）。另补跑一轮敌意输入：id 大于 u32、超长数字、UTF-8 BOM、CRLF、空文件、只含空行、`--file` 指向目录 —— 全部给出清晰错误，无 panic。
  - **#7** 改为「参数错误的几条有两层前缀」，并点明 `NoSuchTask` / `BadRecord` 只有 `main` 那一层。
  - **#8** 改为「调用 `.parse()` 这类方法不写 `use` 也能用 —— 它是 `str` 的固有方法，trait 只出现在它的约束里」。
  - **#9** 设计取舍里 `struct Task` / `enum Status` 那个块从 `rust,ignore` 改回 `rust`（它确实能独立编译）；`ignore` 只留给引用 `AppError` 的别名块。
  - **#10** `cargo test` 节选改为某次真跑的前 5 行原序，并注「libtest 并行执行，顺序每次可能不同」；数字同步为 `19`（单元）+ `2`（端到端）。
  - **#11** `split_first` 不再记在第 4 章名下：改为「第 4 章的切片（`split_first` 是切片上的方法，本书在这里第一次用到）」，块末覆盖注释同样处理。
  - **#12** 表下补注：`split` / `join` / `contains` 按名字查 std 的 `str` 页即可，第 6 章只演示了其中一部分。
  - **#13** 开场「省略了哪些行」补上错误路径的 `error: process didn't exit successfully: … (exit code: 2)`。
  - **#14** 退出码 2 的理由改为「`main` 返回 `Result` 时任何错误都以 1 退出；这里把『输入有问题』单独定成 2」，不再声称 1 是「真出意外」专用。
  - **切片行号重算**（代码改动使 `error.rs` / `store.rs` 位移）：`error.rs:6:22`→`6:24`、`store.rs:26:72`→`26:77`、`store.rs:94:126`→`99:132`，并新增只读 impl 块 `store.rs:79:97`（让「分成两类」的说法与展示一致）。脚本复核 14 条切片边界全部落在 doc 注释/项首与闭合 `}`（`error.rs:6:24` 以 `Result` 别名行结尾，是有意的）。
  - **复验**（`CARGO_TARGET_DIR=.scratch/tmp-21/target`）：`cargo test --manifest-path examples/Cargo.toml` → exit 0，`19 passed; 0 failed` + `2 passed; 0 failed`；`cargo build` → exit 0、0 warning；`cargo fmt --check` → exit 0；`mdbook test -c project1-cli-tool.md` → exit 0。
  - **篇幅回修**：正文汉字 **4495**（不含围栏代码块）；`.scratch/tmp-21/` 已删除。
