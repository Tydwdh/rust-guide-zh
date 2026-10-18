# 项目 1：命令行小工具

前面 11 章把概念一个个拆开讲。这一章反过来：从零做一个能用的待办清单工具 `todo-cli`，把那些概念合起来用一遍。代码放在仓库的 `examples/todo-cli/`，只用标准库 —— 一个外部 crate 都不加。

## 这一章解决什么问题

### 要做的东西

`todo-cli` 用命令行参数决定动作，把任务存进一个文本文件，下次运行还能读回来。四条命令：

| 命令 | 作用 |
|---|---|
| `add <文本>` | 追加一条待办 |
| `list [筛选]` | 列出任务，筛选默认 `--all`，另有 `--pending` / `--done` |
| `done <编号>` | 把某条任务标记为完成 |
| `rm <编号>` | 删除某条任务 |

### 验收标准

做完要能满足下面四条，缺一条都算没做完：

1. `cargo run -p todo-cli -- <参数>` 能跑起来（在 `examples/` 目录里；从仓库根出发则写 `cargo run --manifest-path examples/Cargo.toml -p todo-cli -- <参数>`）；
2. 参数写错、编号不存在、数据文件被写坏时，都给出人话错误并以退出码 2 结束，绝不 panic；
3. 任务写进文件后能被下一次运行读回来：同一编号、同一状态、同一文本；
4. `cargo test --manifest-path examples/Cargo.toml` 全绿，测试覆盖正常路径、边界与错误路径。

本次验证用稳定版 rustc 1.98.0、edition 2024。下面所有终端记录都真跑过；只省掉 cargo 自己的 `Compiling` / `Finished` / `Running` 行，以及错误路径上它追加的 `error: process didn't exit successfully … (exit code: 2)`。

最终效果先看一遍，心里有个整图：

```console
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- add 读完第 11 章
已添加 #1
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- add 给项目 1 写测试
已添加 #2
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- list
#1 待办 读完第 11 章
#2 待办 给项目 1 写测试
共 2 条：2 条待办，0 条已完成
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- done 1
已把 #1 标记为完成
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- done 1
#1 本来就是完成状态，没有改动
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- rm 2
已删除 #2：给项目 1 写测试
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- list
#1 已完成 读完第 11 章
共 1 条：0 条待办，1 条已完成
```

数据文件长这样（一行一条，`编号 TAB 标记 TAB 文本`，`o` = 待办、`x` = 已完成）：

```text
1	x	读完第 11 章
```

## 设计取舍

动手之前先定几件影响全局的事，每条都给「为什么不选另一个」。

**1. 任务怎么表示？** 用结构体加枚举：

```rust
pub struct Task { id: u32, status: Status, text: String }
pub enum Status { Pending, Done }
```

不选「在文本前面塞 `[x] ` 前缀」：那等于把状态编码进字符串，筛选时还得再解析一遍，而且拼错的字符串没人拦。不选 `done: bool`：bool 现在够用，但等你要加「延期」「取消」时就得改整个数据结构，枚举只要加一个变体。字段全部私有，状态只能通过 `Task::mark_done` 改变 —— 第 8 章的可见性在这里用来守不变量。

**2. 数据文件用什么格式？** 一行一条记录，字段用制表符分隔：`编号 TAB 标记 TAB 文本`。不选 JSON：只用标准库就得自己写解析、转义和数组逗号，那是另一个项目的工作量。不选空格分隔：任务文本里有空格就分不清边界。不选「编号就是行号」：删掉中间一条之后所有编号都会漂移，而用户手里记着的编号不该因为一次删除而改变，所以编号写进文件、只增不减（`next_id` 取「已有最大编号 + 1」）。

代价是文本里出现制表符或换行就会破坏结构：在入口处**拒绝**这类文本（`AppError::UnstorableText`），不做转义 —— 转义要同时处理 `\t`、`\n` 和反斜杠本身，规则多一倍、出错面也大一倍。空行在读取时忽略（方便手工编辑），但**不设注释语法**：少一条要维护的规则。

**3. 错误怎么处理？** 一个 `AppError` 枚举装下所有可恢复错误，配一个 `Result` 别名：

```rust,ignore
pub type Result<T, E = AppError> = std::result::Result<T, E>;
```

`main` 只做一件事：把错误变成标准错误上的一行话，加上退出码 2。不用 `panic!` / `unwrap`：用户敲错参数不是程序 bug，panic 打的是给开发者看的 backtrace。也不用 `Box<dyn Error>`（第 13、17 章展开）：这里的错误种类是固定的，而且之后还想按种类区分（「编号不存在」和「参数错」不是一回事），枚举更好用。解析单行这一层返回 `Result<Task, String>`：那里只需要一句给用户看的原因，「第几行」由 `Store::parse` 补上（`BadRecord { line, reason }`）。错误分几层、带多少上下文，是 API 设计的一部分（第 18 章系统讲）。

**4. 参数怎么解析？** 手写：只用标准库就没有 clap，`std::env::args` 拿到 `Vec<String>` 自己走一遍。语法刻意收小：全局选项只认「命令之前」的位置，命令之后的参数原样交给子命令。`add` 后面的参数用 `join(" ")` 拼成一段文本，所以 `todo-cli add 买 牛奶` 就够，不必加引号（Windows 与 Unix 的引号行为不一致，最容易在这里踩坑）。代价是 `todo-cli add --file x` 里的 `--file` 是任务文本而不是选项 —— 这个行为有测试固定住，不是意外。

**5. 保存怎么写？** 每次把全部任务重写进文件（`fs::write`），不做「追加一行」。因为 `rm` 和 `done` 都要改中间的行，追加做不到；几十条任务全量重写的时间可以忽略。没做「写临时文件再改名」的原子替换（第 40 章那种生产要求）：接受「写到一半失败会让文件停在半截」，换来少一套逻辑。

## 逐块实现

下面每块都是从 `examples/todo-cli/src/*.rs` 切出来的原文，块末注明用到哪几章的概念。顺序按依赖来：错误类型 → 数据模型 → 参数解析 → 存储 → 入口。

> [!NOTE]
> 这些块大多标了 `ignore`：它们是跨模块互相引用的片段，`mdbook test` 会把每个块当独立程序编译，单独的片段自然编译不过（例如 `Task` 用了同文件的 `Status`）。片段本身与文件逐字一致；整份代码由下面 `cargo test` 真跑验证。

### 1. 错误类型：`src/error.rs`

先把可能出错的地方列全：

```rust
# use std::io;
{{#include ../examples/todo-cli/src/error.rs:6:24}}
```

同一个文件里还给了它三个 trait 实现：`Display` 把错误渲染成中文（`NoSuchTask(7)` 打印成「没有编号为 7 的任务」），`Error::source` 把底层 `io::Error` 挂成原因链，`From<io::Error>` 让 `?` 能把 `io::Error` 自动转成 `AppError`。这是第 7 章的错误处理套路加第 9 章的 trait 实现（`source` 里的 `dyn Error` 第 13 章展开）。

**这块用到第 7 章的自定义错误与 `?` 转换、第 9 章的 trait 实现。**

### 2. 数据模型：`src/task.rs`

状态的两个出口分开：`marker()` 只管文件格式（短字符），`label()` 只管展示（中文），将来改文件格式不会动到界面文案：

```rust
# use std::str::FromStr;
{{#include ../examples/todo-cli/src/task.rs:6:41}}
```

`impl FromStr for Status` 让 `"o".parse::<Status>()` 可用 —— 解析时不用写一堆 `match`。标准库的 `parse` 不认你的类型，但它认 `FromStr`。

任务本身分两个 `impl` 块：一块管构造与访问，一块管文件格式：

```rust,ignore
{{#include ../examples/todo-cli/src/task.rs:43:80}}
```

`new` 只接受 `id` 和文本，状态固定从「待办」开始 —— 刚添加的任务不可能是已完成。`text(&self) -> &str` 借出文本不复制（第 4 章），`mark_done` 返回「是不是第一次标记」，调用方因此能区分「已标记」与「本来就这样」。

文件格式这一块，方向是「一行文本 ↔ 一条任务」：

```rust,ignore
{{#include ../examples/todo-cli/src/task.rs:82:104}}
```

`parse_record` 把「字段太多」「字段不够」「编号不是数字」「状态标记不认识」四种情况分别报出来，全靠 `?` 短路；调用方只需知道「哪一行、为什么不行」。`split('\t')` 给的是切片迭代器（第 6 章）。`Task` 还实现了 `Display`，列表里直接 `writeln!(out, "{task}")`。

**这块用到第 5 章的结构体与枚举、第 4 章的借用、第 6 章的 `String` 与字符串操作、第 9 章的 `FromStr`/`Display`、第 7 章的 `Result` 与 `?`。**

### 3. 命令行解析：`src/cli.rs`

解析的目标是把 `argv` 变成两个东西：数据文件路径，以及一个描述「用户想干嘛」的值：

```rust,ignore
{{#include ../examples/todo-cli/src/cli.rs:11:49}}
```

`Options` 是解析结果，`Command` 是动作。`Help` 也是一个命令（而不是「解析失败」），所以 `--help` 输出用法后退出码是 0。`parse` 里那个 `while let Some((first, tail)) = rest.split_first()` 是本块唯一需要想一下的地方：它在切片上「取头、留尾」，遇到不是全局选项的参数就 `break`，剩下的整段留给子命令 —— 第 4 章的切片（`split_first` 是切片上的方法，本书在这里第一次用到）和第 5 章的 `while let` 模式合成了一件事。

子命令的派发是纯模式匹配：

```rust,ignore
{{#include ../examples/todo-cli/src/cli.rs:51:81}}
```

`let Some((name, rest)) = args.split_first() else { ... }` 是第 5 章的 `let ... else`：拿到「没有命令」的情况就早退，后面主流程不必再处理 `None`。`add` 之后的所有参数拼成一段文本，空文本在解析阶段就拦掉。

帮助文本是一个 `&'static str`，直接来自源码，不另建文件（少一个要同步的地方）：

```rust
{{#include ../examples/todo-cli/src/cli.rs:110:136}}
```

**这块用到第 3 章的变量与函数、第 4 章的切片（`split_first` 是切片方法，这里首次用到）、第 5 章的 `match`/`let ... else`/切片模式、第 8 章的模块可见性。**

### 4. 存储：`src/store.rs`

`Store` 拥有任务集合，也记住数据文件在哪：

```rust,ignore
{{#include ../examples/todo-cli/src/store.rs:10:24}}
```

`Filter` 是给 `list` 用的三值枚举，`Store` 的三个字段全私有 —— 外面只能通过方法读。文件进出单独一块：

```rust,ignore
{{#include ../examples/todo-cli/src/store.rs:26:77}}
```

`open` 有个值得说的决定：**文件不存在不是错误**（`io::ErrorKind::NotFound` 单独一个分支，得到一个空 `Store`），否则第一次运行就得先让用户手工建文件。`parse` 是一条迭代器链：`lines().enumerate().filter(跳过空行).map(解析).collect()`，收集的是 `Result<Vec<Task>>` —— 任何一个 `Err` 都会让整条链短路返回，这是第 10 章「`collect` 也能收集 `Result`」的实战用法。`save` 用 `map(Task::to_record).collect::<Vec<_>>().join("\n")` 拼全文，方法引用当闭包用。

算 `next_id` 用的是 `checked_add`：编号只增不减，数据文件里已出现 `u32::MAX` 时就把它钉死在 `u32::MAX`，添加返回 `AppError::IdsExhausted`，只读命令照常可用（写成 `max + 1` 的话，Debug 构建会溢出 panic）。

剩下的操作分成两类，一类只读、一类改动：

```rust,ignore
{{#include ../examples/todo-cli/src/store.rs:79:97}}
```

```rust,ignore
{{#include ../examples/todo-cli/src/store.rs:99:132}}
```

`add` 在写入前检查文本里没有制表符/换行 —— 这是「第 2 个取舍」在代码里的落点，也是那份文件格式唯一的守卫；编号用满时同样返回错误而不是溢出。`mark_done` 用 `iter_mut().find(...)` 拿到可变借用，`remove` 用 `position` 找到下标再 `Vec::remove`；两个都靠 `.ok_or(AppError::NoSuchTask(id))?` 把 `Option` 转成错误，length 检查、下标越界这些事一样都不用做。

这里没用 `HashMap`：任务只有几十条，顺序遍历就够；任务本来就按加入顺序列出，`Vec` 天然给了这个顺序，`HashMap` 反而要额外排序。

**这块用到第 6 章的 `Vec`/`String`、第 7 章的 `Result`/`?`/`From`、第 8 章的模块与 `pub`、第 9 章的 `impl Into<PathBuf>`（参数位置的泛型简写）、第 10 章的迭代器与闭包。**

### 5. 入口与输出：`src/main.rs`

四个 `mod` 声明把文件变成模块，`main` 只负责「跑，然后决定退出码」：

```rust,ignore
{{#include ../examples/todo-cli/src/main.rs:16:26}}
```

`main` 返回 `ExitCode`：成功是 0，任何错误都打印到标准错误并返回 2。`run` 里所有的 `?` 把错误原样往上抛，`main` 只有一个地方处理它们 —— 这就是「错误在哪一层被翻译成给用户看的话」的设计。

列出任务时，输出目标是个泛型参数：

```rust,ignore
{{#include ../examples/todo-cli/src/main.rs:61:82}}
```

`render_list<W: Write>` 不直接 `println!`，而是接受「往哪儿写」。`main` 传的是 `io::stdout().lock()`，测试里传 `Vec<u8>` 然后断言内容 —— 这就是第 9 章的 trait 约束带来的可测试性，第 11 章的单元测试就靠它。汇总行只统计本次列出的集合（所以 `list --pending` 会显示「0 条已完成」）。

**这块用到第 3 章的函数与控制流、第 7 章的 `Result` 与退出码、第 8 章的模块声明、第 9 章的 trait 约束与泛型、第 10 章的 `filter`/`count`。**

### 概念落点对照

| 章 | 概念 | 在本项目里的落点 |
|---|---|---|
| 1 | 工具链、`cargo run`、edition 2024 | 上面的跑法；`examples/todo-cli/Cargo.toml` |
| 2 | 读懂编译器报错 | 「报错现场」两段真实报错 |
| 3 | 变量、类型、函数、控制流 | `main`、`parse_id`、`render_list` |
| 4 | 所有权、借用、切片 | `Task::text() -> &str`、`rest: &[String]`、`Store::tasks() -> &[Task]` |
| 5 | 结构体、枚举、模式匹配 | `Task`/`Status`/`Command`/`Filter`、`match name.as_str()`、`[id]` 切片模式、`let ... else` |
| 6 | 集合与字符串 | `Vec<Task>`、`split('\t')`、`join(" ")`、`contains(['\t', '\n', '\r'])` |
| 7 | 错误处理 | `AppError`、`Result` 别名、`?`、`From<io::Error>`、退出码 |
| 8 | 模块、包与依赖 | `mod cli/error/store/task`、`todo-cli` 是 `examples` 工作区的成员 |
| 9 | 泛型与 trait | `impl Into<PathBuf>`、`render_list<W: Write>`、`FromStr`/`Display`/`Error`/`From` |
| 10 | 迭代器与闭包 | `parse` 的链、`find`/`position`/`max`、`collect::<Result<…>>()`、`filter` |
| 11 | 测试与调试 | 各模块 `#[cfg(test)]` + `tests/cli.rs` 端到端 |

表里「第 6 章」一行的 `split` / `join` / `contains` 按名字查 std 的 `str` 页；第 6 章只演示了其中一部分。

## 报错现场

### 报错一：`add` 返回引用（E0502）

一个自然的写法是让 `add` 返回刚加进去的那条任务，好在 `main` 里打印。真这么写会撞上下面的错误：

```rust,compile_fail
struct Store {
    tasks: Vec<String>,
}

impl Store {
    fn add(&mut self, text: &str) -> &String {
        self.tasks.push(text.to_owned());
        self.tasks.last().unwrap()
    }

    fn save(&self) {}
}

fn main() {
    let mut store = Store { tasks: Vec::new() };
    let task = store.add("写测试");
    store.save();
    println!("{task}");
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0502]: cannot borrow `store` as immutable because it is also borrowed as mutable
  --> main.rs:17:5
   |
16 |     let task = store.add("写测试");
   |                ----- mutable borrow occurs here
17 |     store.save();
   |     ^^^^^ immutable borrow occurs here
18 |     println!("{task}");
   |                ---- mutable borrow later used here
```

定位路径：`add` 返回的引用带着对 `store` 的可变借用（第 4 章：引用还活着，借用就没结束），`store.save()` 又要一次共享借用，于是第 17 行冲突 —— 真正的原因是第 18 行还在用 `task`。

修法有两条：让 `add` 只返回编号（本项目采用）—— `pub fn add(&mut self, text: &str) -> Result<u32>`，`main` 拿编号打印，引用根本不产生；或者保留返回引用，但把借用缩到打印之前（先 `save`、再取 `add` 的结果，或换个语句块）。选第一条是因为 `u32` 不携带借用，调用顺序就自由了。

### 报错二：忘 `use` 就实现 `FromStr`（E0405）

上面 `src/task.rs` 里的 `impl FromStr for Status`，新手第一遍常写成这样（缺了文件顶部那行 `use`）：

```rust,compile_fail
enum Status {
    Pending,
    Done,
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "o" => Ok(Status::Pending),
            "x" => Ok(Status::Done),
            other => Err(format!("未知的状态标记 `{other}`")),
        }
    }
}

fn main() {}
```

以 Rust 1.98 为例：

```text
error[E0405]: cannot find trait `FromStr` in this scope
 --> main.rs:6:6
  |
6 | impl FromStr for Status {
  |      ^^^^^^^ not found in this scope
  |
help: consider importing this trait
  |
1 + use std::str::FromStr;
  |
```

报错信息把修法都写出来了：实现 trait 之前它得先进入作用域，在文件顶部加 `use std::str::FromStr;` 即可（`src/task.rs` 第 4 行就是这么写的）。顺带记住这条：`impl Trait for Type` 的 `Trait` 必须在作用域里；而调用 `.parse()` 这类方法不写 `use` 也能用 —— 它是 `str` 的固有方法，trait 只出现在它的约束里。

## 测试

测试分两层：单元测试在各模块的 `#[cfg(test)] mod tests`，端到端测试在 `examples/todo-cli/tests/cli.rs`（单独的集成测试 crate，第 11 章讲过位置差别）。它把编译好的二进制当黑盒，只看退出码和两路输出：

```rust,ignore
{{#include ../examples/todo-cli/tests/cli.rs:22:46}}
```

`env!("CARGO_BIN_EXE_todo-cli")` 是 cargo 在集成测试里预置的环境变量，指向刚编好的二进制；`.output()` 一次拿到退出码、标准输出和标准错误。这样「添加 → 列表 → 标记完成 → 删除」整条链路都走了一遍，而不是只测函数。

跑一遍的样子：

```console
$ cargo test --manifest-path examples/Cargo.toml

running 19 tests
test cli::tests::add_joins_the_remaining_arguments_into_one_text ... ok
test cli::tests::missing_file_path_is_an_error ... ok
test error::tests::io_error_keeps_its_source_for_the_cause_chain ... ok
test cli::tests::file_option_comes_before_the_command ... ok
test cli::tests::non_numeric_id_is_an_error ... ok
…
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 2 tests
test unknown_id_exits_with_code_2_and_explains_why ... ok
test full_round_trip_through_the_binary ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

（取自一次真跑；libtest 并行跑用例，顺序每次可能不同，`…` 处都是 `ok`。合计：单元测试 19 个、端到端 2 个。）三类路径都有覆盖：

- **正常路径**：`record_round_trip_keeps_id_status_and_text`、`select_filters_by_status`、`list_shows_the_marker_and_a_summary`、`full_round_trip_through_the_binary`；
- **边界**：空行被忽略、行号仍按物理行算（`bad_record_reports_the_physical_line_number` 断言错误里出现「第 3 行」）、编号从 `max + 1` 继续而不复用、用满 `u32::MAX` 时 `add` 返回 `IdsExhausted`（`exhausted_id_space_reports_an_error_instead_of_overflowing`）、空选择集的提示行；
- **错误路径**：编号不存在、编号不是数字、缺 `--file` 的参数、坏记录、带制表符的文本 —— 分别断言错误文案、错误种类或退出码 2。

调试时两个顺手的手段：`cargo test -- --nocapture` 看测试里的打印；把出错的数据文件留下，直接 `cargo run ... -- list` 复现一遍。

## 收尾：用法、错误与下一步

### 用法

```console
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- --help
todo-cli —— 只用标准库的待办清单

用法：
    todo-cli [--file <路径>] <命令> [参数]

命令：
    add <文本>      添加一条待办（文本可以带空格，不必加引号）
    list [筛选]     列出任务，默认 --all
                      --all / -a       全部
                      --pending / -p   只看待办
                      --done / -d      只看已完成
    done <编号>     把某条任务标记为完成
    rm <编号>       删除某条任务
    help            打印这份说明

选项：
    --file <路径>, -f <路径>   数据文件，默认 todo.txt
    --help, -h                 打印这份说明

退出码：
    0  成功
    2  参数或数据有问题（错误信息写到标准错误）
```

在 `examples/` 目录里可以省掉 `--manifest-path`（`--file` 的相对路径按当前目录解析，这里指的是仓库根那份）：

```console
$ cargo run -p todo-cli -- --file ../todo.txt list --done
#1 已完成 读完第 11 章
共 1 条：0 条待办，1 条已完成
```

### 错误处理与退出码

所有可恢复错误都走同一条路：`run` 里 `?` 冒泡 → `main` 打印 → 退出码 2。真跑出来的几种：

```console
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- done 7
错误：没有编号为 7 的任务
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- list --bogus
错误：参数错误：`list` 不认识 `--bogus`
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- add
错误：参数错误：`add` 后面要跟任务文本
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- frobnicate
错误：参数错误：不认识的命令 `frobnicate`；跑 `todo-cli help` 看用法
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- --file
错误：参数错误：`--file` 后面要跟一个路径
```

参数错误的几条有两层前缀：`main` 加「错误：」，`AppError::BadArgs` 加「参数错误：」；`NoSuchTask` / `BadRecord` 就只带 `main` 那一层。数据文件写坏时同样报错，而且带上行号：

```console
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- list
错误：数据文件第 2 行无法解析：未知的状态标记 `?`
```

这几条的退出码都是 2（`cargo run` 会转发子进程的退出码，自己再补一行提示）。为什么不是 1：`main` 返回 `Result` 时任何错误都以 1 退出；这里把「输入有问题」单独定成 2，脚本能把两类失败分开。

### 怎么继续扩展

想接着练的话，几个由易到难的方向：

1. **加一个全局选项**：例如 `--quiet`，让 `add`/`done`/`rm` 成功时不打印。落点是 `cli::Options` 加字段、`cli::parse` 多认一个开关、`main::run` 用它决定要不要打印 —— 纯第 3、5、8 章的活儿。
2. **加子命令**：`edit <编号> <新文本>`，或者 `list --sort`。落点是 `Command` 加变体 + `parse_command` 加分支 + `Store` 添一个方法；模式匹配会把要改的地方全指出来（枚举相对 bool 的好处）。
3. **把状态改成三态**：加 `Deferred`（延期）—— 只改 `Status`、文件的标记字符、`Filter`，编译器会把剩下的调用点一个个报给你。
4. **换掉存储**：把每行格式换成 JSON 或 SQLite，需要引入外部 crate（第 8 章讲依赖），`Store` 的接口不用动 —— 因为外面只通过方法访问它。
5. **让它快/让它并发**：任务上万条时整文件重写会成为瓶颈，可以只写变化的部分；再往上就是多线程或异步（第 19、21 章）。

完整代码在 `examples/todo-cli/`，跑法只有一句：`cargo test --manifest-path examples/Cargo.toml`。

## 小结

- 骨架是「数据模型 → 输入解析 → 存储 → 入口编排」四块，`error.rs` 被所有人共用。
- 数据表示决定后面能写多爽：枚举 + 私有字段把「只有两种状态」变成类型事实，状态变更也收进一个方法。
- 文件格式是自己定的，代价也要自己认：选了制表符分隔，就必须在入口拒绝制表符和换行。
- 错误全部走 `Result`：`main` 是唯一把错误翻译成人话的地方，退出码是给脚本看的接口。
- 迭代器链能干掉手写循环，`collect::<Result<Vec<_>, _>>()` 还能做「遇到第一个错误就停」。
- 输出目标写成泛型参数（`W: Write`）几乎不花成本，却让列表输出变得可测试。

## 练习

1. `Store::open` 把「数据文件不存在」当成正常情况（返回空集合），而不是返回错误。为什么这是个好选择？如果反过来（文件不存在就报错），用户第一次使用会遇到什么？
2. 把 `Task::to_record` / `parse_record` 的分隔符从制表符改成逗号，需要改哪些地方？会新引入什么风险？如果非要支持「文本里有逗号」，你会怎么改？
3. `render_list` 为什么要写成 `fn render_list<W: Write>(tasks: &[&Task], out: &mut W)`，而不是直接在函数里 `println!`？这样做给测试带来了什么？

<details><summary>参考答案</summary>

**1.** 首次运行、或用户刚把数据文件删掉重来时，文件本来就不存在，这是**正常状态**而非异常：`open` 用 `io::ErrorKind::NotFound` 单独一个分支返回空 `Store`，用户直接 `add`，`save` 负责建文件。反过来（不存在就报错）就得先手工建空文件才能开始用 —— 一个把自己用不下去的工具。仍要区分「不存在」与「存在但读不了」（权限、路径是目录）：后两种照旧报 `Io` 错误，不能一起吞掉。

**2.** 至少要改四处：`to_record` 里的 `format!` 分隔符、`parse_record` 里的 `split('\t')`、`add` 里拒绝的字符集合（`contains(['\t', '\n', '\r'])` 要换成拒绝逗号），以及测试里那些写死 `"3\to\t买牛奶"` 的字面量。原来的做法靠「制表符在正常文本里几乎不会出现」这个经验，改成逗号以后这个假设就不成立了：`买牛奶, 顺便买鸡蛋` 这样的文本会被解析成两条记录 —— 数据静默损坏，比直接报错更糟。要支持逗号，必须引入转义（比如 `\,` 表示字面逗号、`\\` 表示反斜杠），并在解析时按字符扫描而不是简单 `split`；转义的读取和写入要成对实现，还得为「写到一半的转义序列」写测试。

**3.** 让「往哪儿写」成为参数，函数就不再依赖全局的标准输出：`main` 传 `io::stdout().lock()`，测试传一个 `Vec<u8>`，跑完直接断言字节内容（`assert_eq!(String::from_utf8(buf).unwrap(), "（没有符合条件的任务）\n")`）。如果写死 `println!` 的话，验证输出只能起子进程、抓标准输出，单元测试退化成端到端测试，慢且脆。`W: Write` 是第 9 章讲的 trait 约束最简单的实用形态：不关心具体类型，只要求「能写」。

</details>

## 延伸阅读

- The Book 第 12 章《一个 I/O 项目》：<https://doc.rust-lang.org/book/ch12-00-an-io-project.html>
- `std::env::args`（读命令行参数）：<https://doc.rust-lang.org/std/env/fn.args.html>
- `std::process::ExitCode`（退出码）：<https://doc.rust-lang.org/std/process/struct.ExitCode.html>
- `std::io::Write`（`render_list` 的 trait 约束）：<https://doc.rust-lang.org/std/io/trait.Write.html>
- `std::str::FromStr`：<https://doc.rust-lang.org/std/str/trait.FromStr.html>
- `std::error::Error`（含 `source` 原因链）：<https://doc.rust-lang.org/std/error/trait.Error.html>
- Cargo Book《Workspaces》（`examples/Cargo.toml` 这种多 crate 布局）：<https://doc.rust-lang.org/cargo/reference/workspaces.html>
- cargo 子命令 `cargo test` 与二进制目标环境变量：<https://doc.rust-lang.org/cargo/commands/cargo-test.html>
