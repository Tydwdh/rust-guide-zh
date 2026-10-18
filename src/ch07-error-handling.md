# 第 7 章 错误处理：Result、panic 与自定义错误

## 这一章解决什么问题

到第 6 章为止，我们写的函数基本都假定一切顺利：解析一定成功、文件一定存在、用户输入一定合法。真实程序不是这样——失败是常态，不是例外。

大多数语言把「函数可能失败」这件事留给文档和约定：调用者要么记得查文档，要么在运行时被异常或错误码打个措手不及。Rust 走了另一条路：**把失败写进类型**。一旦某个函数返回 `Result<T, E>`，类型系统就会逼着调用者处理那个 `Err`，想忘也忘不掉——代码根本编译不过。

读完这一章，你能做到：

- 判断一次失败该走可恢复的 `Result` 还是不可恢复的 `panic`；
- 用 `?` 运算符把错误往上传，并看懂它引发的类型错误；
- 手写一个自定义错误类型（`enum` + `Display` + `Error`），既能打印给人看，也能让上层拿到错误链；
- 知道 `unwrap`/`expect` 什么时候可以用，`expect` 的消息该怎么写。

错误类型怎么设计成 API、对外承诺什么，留到第 18 章；本章只把基础篇要用的部分讲透。

## 为什么这样设计

先看三条路线：

- **异常路线（Java、Python、C++）**：`try { ... } catch (IOException e) { ... }`。写起来省事，但函数签名里看不出它会失败。调用者必须读过文档或踩过坑，才知道要 catch 什么。
- **错误码路线（C）**：`int ret = read_file(...); if (ret < 0) { ... }`。签名里同样没有失败信息，而且返回值要同时承担「结果」和「状态」两个职责，函数一复杂就得额外传指针出去。
- **Rust 路线**：失败就是一个普通的值，类型是 `Result<T, E>`：

```rust
enum Result<T, E> {
    Ok(T),   // 成功：带着结果
    Err(E),  // 失败：带着失败原因
}

# fn main() {}
```

`Result` 不是什么编译器魔法，就是标准库里一个普通的枚举（第 5 章讲过枚举怎么建模状态）。它的全部威力集中在一点：**类型写在签名里**。谁调用 `parse_port`，谁就拿到一个 `Result`；要么显式处理 `Err`，要么就用下文的 `?` 声明「我把失败交给调用者」。两条路都在类型系统里，绕不过去。

由此得到本书反复用的选择标准：

- **失败在预期之内、调用者可能想恢复** → 返回 `Result`，把决定权交出去。
- **出现了不该出现的情况、程序状态已经不可信** → `panic`，立刻停下，而不是带着坏数据继续跑。

还差一块拼图。`Option<T>` 回答的是「有没有」，不回答「为什么没有」；要向上层解释原因，就得用 `Result`。

## 机制与规则

### 7.1 两类失败：`Result` 与 `panic`

`Result` 有两个变体，靠模式匹配取用（第 5 章的 `match` 直接可用）：

```rust
fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    s.trim().parse()
}

fn main() {
    match parse_port("8080") {
        Ok(port) => println!("端口 = {port}"),
        Err(e) => println!("解析失败：{e}"),
    }
}
```

`panic` 是另一个方向。调用 `panic!`、对 `None` 调 `unwrap`、数组越界，都会触发它。默认行为叫**展开（unwind）**：Rust 沿着调用栈往回走，逐个清理遇到的值（跑它们的 `Drop`），最后退出进程。你也可以在 `Cargo.toml` 里选另一条路——直接终止（abort），不做清理，把内存交还给操作系统：

```toml
[profile.release]
panic = "abort"
```

展开更安全（保证清理能跑），abort 的二进制更小、崩溃更快；选择理由属于工程策略（第 40 章展开）。本书全部示例用默认的展开。

panic 的终端输出长这样（下例是对 `None` 调 `unwrap` 的实测输出）：

```text
thread 'main' (5400) panicked at src/main.rs:3:21:
called `Option::unwrap()` on a `None` value
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

三行各有用途：第一行是**位置**（哪个线程、哪个文件、哪一列），第二行是**原因**，第三行是**下一步线索**。这类 panic 默认以状态码 101 退出（实测；正常返回是 0）。

按提示打开回溯（backtrace），把上面那份程序存成 `src/main.rs` 后用 `RUST_BACKTRACE=1` 重跑，得到（只保留前几帧，函数名里的 `config_tool` 是 crate 名）：

```text
thread 'main' (11952) panicked at src/main.rs:3:21:
called `Option::unwrap()` on a `None` value
stack backtrace:
   0: std::panicking::panic_handler
   1: core::panicking::panic_fmt
   2: core::panicking::panic
   3: core::option::unwrap_failed
             at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2251
   4: config_tool::main
   5: config_tool::main
   6: std::rt::lang_start::<()>
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

读法和别的语言一样：**从上往下读，第一处出现你自己写的文件或函数的地方就是问题所在**（这里是 `config_tool::main`，对应 `src/main.rs:3`）。它上面的帧是你的代码调用的东西，下面的帧是调用了你的代码的东西。想要完整（也更吵）的输出用 `RUST_BACKTRACE=full`；回溯里出现文件名行号的前提是编译时带调试符号——`cargo build` 默认带，`--release` 默认不带。

> [!WARNING]
> 从 Rust 1.97 起，符号修饰（symbol mangling）默认换成 v0 格式，回溯文本与旧教程里的示例不再逐字一致；个别旧版调试器与剖析器也需要更新才能正常显示函数名。

### 7.2 `unwrap` 与 `expect`：可以用，但要有纪律

`unwrap` 是「我确定它是 `Ok`，不是就 panic」的简写；`expect` 再多给一条自定义消息：

```rust
fn main() {
    let raw = "127.0.0.1";
    let addr: std::net::IpAddr = raw.parse().expect("硬编码的 IP 必须是合法的");
    println!("{addr}");
}
```

这里 `.expect(...)` 的文本不是「错误说明」，而是**你此刻掌握的额外信息的记录**：你知道这行字面量是合法 IP，编译器不知道，所以你把「为什么这里不可能失败」写下来。官方文档的说法是：`expect` 的参数应该描述「你认为不会有 `Err` 变体的理由」。好的写法是断言不变式：

```rust
fn port_from(config: &std::collections::HashMap<&str, u16>) -> u16 {
    *config
        .get("port")
        .expect("配置解析后必有 port（由 parse_config 保证）")
}
```

坏写法是 `.expect("error")`、`.expect("failed")`——它把 panic 消息降级成噪音：真出问题时既不知道期望什么，也不知道是谁的锅。

纪律可以概括成一句：**`unwrap`/`expect` 是「把可恢复失败就地升级成 panic」的声明，只在两种场合合理**——

1. 示例、原型、测试：`unwrap` 是最短的占位符，等逻辑稳定了再换成真正的处理；
2. 你掌握编译器不知道的不变式：这时 `expect` 的消息就是给未来的自己留的文档。

库的公共 API 里尽量别用 `unwrap`：调用者按签名（`Result`）准备好的处理路径会被你一句 `unwrap` 直接掐断。真要 panic（比如入参违反契约），得按「panic 是契约的一部分」来处理，并在文档里写明；怎么标、怎么写进 API 文档，第 18 章展开。

### 7.3 `?`：把错误往上抛

每层都写 `match` 会迅速淹没代码。`?` 运算符（`?` operator）就是为此存在的：

```rust
fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    let port: u16 = s.trim().parse()?;
    Ok(port)
}

# fn main() {}
```

语义只有两条，写成伪代码最清楚：

```text
foo()?  等价于
match foo() {
    Ok(v)  => v,                           // 继续往下走
    Err(e) => return Err(From::from(e)),   // 提前返回，顺便把错误类型转成当前函数的 E
}
```

注意 `?` 只负责「`Ok` 就取值、`Err` 就提前返回」，**不负责清理**——无论 `?` 还是 `panic` 离开作用域，值都会正常 `Drop`。

正因为它会 `return`，`?` 对所在函数有要求：返回类型必须能承接这个失败。`()` 不行，`u16` 不行；`Result`、`Option` 可以。官方 Reference 说，`?` 依据 `Try` trait 决定「产出值还是返回给调用者」，而 `Try` 目前不稳定、用户类型不能实现它，日常可用的就是标准库那几种（`Result`、`Option`、`ControlFlow`）。

第二个关键点是伪代码里的 `From::from(e)`：`?` 在提前返回时，会把错误类型**自动转换**成当前函数声明的错误类型。所以只要给自定义错误实现 `impl From<底层错误>`，就能一路 `?` 上去。这是错误处理里最常用的组合拳，7.5 节用完整例子演示；转换对不上时的报错是「报错现场」的常客，而 `From` 为什么不能乱写要等第 12 章讲一致性（coherence）。

`main` 也能用 `?`，只要改签名：

```rust,no_run
fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    s.trim().parse()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = parse_port("80a0")?;
    println!("port = {port}");
    Ok(())
}
```

真跑起来，失败时程序打印 `Error: ` 加上错误值的 Debug，并以非 0 退出：

```text
Error: ParseIntError { kind: InvalidDigit }
```

（上一行与退出码 1 都是实测。）`Box<dyn std::error::Error>` 读作「任意一种错误」，`dyn` 的细节第 13 章展开。注意 `main` 打印的是 **Debug** 而不是 `Display`，所以自定义错误类型几乎总要先 `#[derive(Debug)]`。

### 7.4 `Option` 与 `Result` 的互转

`Option` 和 `Result` 之间不会自动转换，得显式说清楚「没有」代表什么错误：

```rust
fn require_port(s: &str) -> Result<u16, String> {
    let port = s
        .trim()
        .parse::<u16>()
        .ok()                                        // Result<u16, _> -> Option<u16>
        .ok_or_else(|| format!("{s:?} 不是端口"))?;  // Option -> Result，None 变成 Err
    Ok(port)
}

# fn main() {}
```

- `Option::ok_or` / `ok_or_else`：把 `None` 变成 `Err`。消息需要构造时用 `ok_or_else`（闭包按需执行，避免白构造），类型能直接对上时用 `ok_or`。
- `Result::ok`：把 `Err` 压掉，只留 `Option`。适合「失败原因不重要」的场景。
- `?` 可以用在 `Option` 上，前提是所在函数返回 `Option`：

```rust
fn first_word_len(text: &str) -> Option<usize> {
    let word = text.split_whitespace().next()?; // 没有词 -> 整个函数返回 None
    Some(word.len())
}

# fn main() {}
```

在返回 `Result` 的函数里对 `Option` 用 `?` 会报 E0277，编译器直接告诉你改用 `.ok_or(...)?`（见「报错现场」）；反过来，在返回 `Option` 的函数里对 `Result` 用 `?` 同样不行。

### 7.5 自定义错误类型

库或稍大的程序里，用 `String` 当错误类型太糊：调用者既没法按类别匹配，也拿不到底层原因。标准做法是定义自己的 `enum`，每个变体对应一类失败，嵌套变体保存底层错误：

```rust
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum ConfigError {
    Empty,                       // 端口为空
    BadNumber(ParseIntError),    // 不是数字，附带底层原因
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Empty => write!(f, "端口不能为空"),
            // 细节留给 source()，Display 只说外层发生了什么
            ConfigError::BadNumber(_) => write!(f, "端口不是合法数字"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Empty => None,
            ConfigError::BadNumber(e) => Some(e),
        }
    }
}

# fn main() {}
```

三个要点的顺序不能乱：

1. `#[derive(Debug)]`——`Error` 的 supertrait 要求 `Debug + Display`（std 里的声明是 `pub trait Error: Debug + Display`），少任何一个，`impl Error` 都过不了编译；
2. `impl Display`——面向人的一句话；
3. `impl Error`——让类型能进 `Box<dyn Error>`、能被 `?` 转换、能参与错误链。

`source()` 是**错误链（error chain）**的接口：返回「造成我这个错误的下层错误」。它有一条容易踩的规范——底层错误要么通过 `source()` 暴露，要么直接在 `Display` 里渲染，**不要两处都来**（std 文档的原话是 "but not both"），否则打印整条链时同一句话会出现两遍。上面的 `Display` 就是按这条写的。

配上 `From`，`?` 就能自动包装底层错误：

```rust
# use std::num::ParseIntError;
# #[derive(Debug)]
# enum ConfigError { Empty, BadNumber(ParseIntError) }
impl From<ParseIntError> for ConfigError {
    fn from(e: ParseIntError) -> Self {
        ConfigError::BadNumber(e)
    }
}

fn parse_port(s: &str) -> Result<u16, ConfigError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ConfigError::Empty);
    }
    Ok(s.parse()?) // Err(ParseIntError) 在这里被 From 变成 Err(ConfigError)
}

# fn main() {}
```

错误链最后长这样：

```text
main 返回的 Err
  └─ ConfigError::BadNumber        Display: 端口不是合法数字
       └─ ParseIntError            source(): invalid digit found in string
```

完整可运行的程序在下一节。`source()` 链怎么被上层利用（批量格式化、日志、第三方错误库的做法）属于 API 设计，第 18 章展开。

### 7.6 给用户看 Display，给自己看 Debug

两个格式化 trait 分工不同，别混用：

| 写法 | 用哪个 trait | 读者 | 该写成什么样 |
|---|---|---|---|
| `{}` | `Display` | 用户 / 终端提示 | 一句人话：`端口不是合法数字` |
| `{:?}` | `Debug` | 开发者 / 日志 | 结构化，可由 derive 生成：`BadNumber(ParseIntError { kind: InvalidDigit })` |
| `{:#?}` | `Debug`（多行美化） | 开发者 / 调试 | 逐字段换行，适合写进日志 |

```rust
use std::fmt;

#[derive(Debug)]
struct ConfigError {
    field: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "配置项 {} 缺失", self.field)
    }
}

# fn main() {
#     let e = ConfigError { field: "port".into() };
#     println!("{e:#?}");
#     println!("{e}");
# }
```

实测输出（先 `{:#?}` 后 `{}`）：

```text
ConfigError {
    field: "port",
}
配置项 port 缺失
```

消息本身的写法，std 的 `Error` 文档给了一条准则：**简洁的小写句子，结尾不加标点**（原文 "Error messages are typically concise lowercase sentences without trailing punctuation"）。再补三条经验：说清楚「什么操作 + 什么值 + 什么要求」，不要只写 `error`/`failed`；不要把内部路径、`unwrap` 之类的实现细节写给用户；需要排查的细节让它走 `source()` 或日志，而不是塞进给用户的一句话里。

## 动手看看

下面这个完整程序把全章要素串起来：enum 错误、`Display`、`Error::source`、`From` + `?`、错误链遍历。源码在 `snippets/ch07/config_error.rs`，用 `rustc --edition 2024 config_error.rs` 编译后运行即可。

```rust
{{#rustdoc_include ../snippets/ch07/config_error.rs}}
```

实测输出（Rust 1.98，Windows）：

```text
" 8080 " -> Ok(8080)
"" -> Err
  Display: 端口不能为空
  Debug:   Empty
"80a0" -> Err
  Display: 端口不是合法数字
  Debug:   BadNumber(ParseIntError { kind: InvalidDigit })
  source:  invalid digit found in string
```

对照着看：`Display` 每个变体都只有一句话；`Debug` 保留了结构（含嵌套的 `ParseIntError`）；只有 `BadNumber` 沿 `source()` 问出了底层原因，`Empty` 的链在那里结束。

## 报错现场

### 现场一：在返回 `()` 的函数里用 `?`

最小复现：

```rust,compile_fail
use std::fs;

fn read_config() {
    let text = fs::read_to_string("config.toml")?;
    println!("{text}");
}

fn main() {
    read_config();
}
```

报错关键行（以 Rust 1.98 为例，下同）：

```text
error[E0277]: the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`)
 --> src/main.rs:4:49
  |
3 | fn read_config() {
  | ---------------- this function should return `Result` or `Option` to accept `?`
4 |     let text = fs::read_to_string("config.toml")?;
  |                                                 ^ cannot use the `?` operator in a function that returns `()`
  |
help: consider adding return type
  |
3 ~ fn read_config() -> Result<(), Box<dyn std::error::Error>> {
4 |     let text = fs::read_to_string("config.toml")?;
5 |     println!("{text}");
6 +     Ok(())
  |
```

**定位路径**：消息第一句就是规则本身，注释行指出「这个函数应该返回 `Result` 或 `Option`」，插入符点在 `?` 上。**修法**两条：改签名让函数能承接失败（`help` 里连完整改法都写好了，见 7.3 的 `main -> Result`），或者就地处理掉错误（`match`、`unwrap_or_else` 等）。选哪条，取决于你想不想把失败交给调用者。

### 现场二：`Option` 的 `?` 用在返回 `Result` 的函数里

```rust,compile_fail
fn port_or_default(port: Option<u16>) -> Result<u16, String> {
    let port = port?;
    Ok(port)
}

fn main() {
    println!("{:?}", port_or_default(Some(8080)));
}
```

```text
error[E0277]: the `?` operator can only be used on `Result`s, not `Option`s, in a function that returns `Result`
 --> src/main.rs:2:20
  |
1 | fn port_or_default(port: Option<u16>) -> Result<u16, String> {
  | ------------------------------------------------------------ this function returns a `Result`
2 |     let port = port?;
  |                    ^ use `.ok_or(...)?` to provide an error compatible with `Result<u16, String>`
```

**定位路径**：报错点明两种类型没对齐，还直接给了改法。**修法**：`port.ok_or_else(|| "缺少端口".to_string())?`——把「没有」翻译成业务上说得通的错误（7.4 节）。

### 现场三：`unwrap()` 在 `None` 上 panic

```rust,no_run
fn main() {
    let port: Option<u16> = None;
    let port = port.unwrap();
    println!("{port}");
}
```

```text
thread 'main' (5400) panicked at src/main.rs:3:21:
called `Option::unwrap()` on a `None` value
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

**定位路径**：这不是编译错误，是运行时 panic。消息给出精确到列的位置 `src/main.rs:3:21`——`unwrap` 的调用点；加上 `RUST_BACKTRACE=1` 就能看到完整调用链（7.1 节）。**修法**：如果 `None` 真的可能发生，换成 `match`/`ok_or_else`/`unwrap_or`；如果属于「不可能发生」，换成 `expect` 并写下理由（7.2 节）。

### 现场四：自定义错误忘了 `impl Display`

```rust,compile_fail
#[derive(Debug)]
struct MyError;

impl std::error::Error for MyError {}

fn main() {}
```

```text
error[E0277]: `MyError` doesn't implement `std::fmt::Display`
 --> src/main.rs:4:28
   |
 4 | impl std::error::Error for MyError {}
   |                            ^^^^^^^ unsatisfied trait bound
   |
help: the trait `std::fmt::Display` is not implemented for `MyError`
  --> src/main.rs:2:1
   |
 2 | struct MyError;
   | ^^^^^^^^^^^^^^
note: required by a bound in `std::error::Error`
  --> library/core/src/error.rs:59:26
   |
59 | pub trait Error: Debug + Display {
   |                          ^^^^^^^ required by this bound in `Error`
```

**定位路径**：`note` 一行把根因写死——`Error` 的 supertrait 是 `Debug + Display`。上面这版已经有 `#[derive(Debug)]`，所以只缺 `Display`；如果连 `Debug` 也没写，同一段代码会一次报两个 E0277。**修法**：补 `impl Display`，让 `fmt` 输出一句面向用户的话（7.5 节）；`source()` 是可选方法，有底层错误时才实现。

## 小结

- Rust 把失败放进类型：预期内的失败返回 `Result<T, E>`，坏状态用 `panic` 停下，调用者想忽略都难。
- `unwrap`/`expect` 是把可恢复失败升级为 panic 的声明；示例、原型、测试里合理，生产与库代码里要慎重。`expect` 的消息写「为什么这里不会失败」。
- `?` = `Ok` 取值、`Err` 提前 `return`，并经 `From` 自动转换错误类型；因此所在函数必须返回能承接失败的类型（`Result`/`Option`）。`main` 也可以返回 `Result<(), Box<dyn Error>>`，失败时打印 Debug 并非 0 退出。
- `Option` 到 `Result` 用 `ok_or`/`ok_or_else`；`?` 在 `Option` 上也可用，但两端类型要对得上。
- 自定义错误 = `enum` + `#[derive(Debug)]` + `impl Display` + `impl Error`；`source()` 串起错误链，且底层错误不要在 `Display` 里重复渲染。
- `{}` 给用户（`Display`，简洁一句话），`{:?}`/`{:#?}` 给开发者（`Debug`）。

## 练习

1. 下面这个函数为什么不编译？给出两种修法，并说明各自把失败交给了谁。

```rust,compile_fail
fn load(path: &str) -> String {
    let text = std::fs::read_to_string(path)?;
    text
}

fn main() {
    println!("{}", load("config.toml"));
}
```

2. 已有 `fn find_user(id: u32) -> Option<User>`。现在要写 `fn load_user(id: u32) -> Result<User, MyError>`，把「找不到用户」表示为自定义错误 `MyError::NotFound`。不改 `find_user` 的前提下，用最少代码怎么写？

3. 预测输出：把 `Result::<i32, String>::Err("boom".into())` 用 `.map_err(|e| format!("上层：{e}"))` 处理之后，再用 `{:#?}` 打印，会是什么样？`{:#?}` 和 `{}` 分别在什么场合用？

<details><summary>参考答案</summary>

1. `?` 会把 `std::io::Error` 提前 `return` 出去，而签名是 `-> String`，接不住，触发 E0277（现场一）。修法一：改成 `fn load(path: &str) -> Result<String, std::io::Error>`，把失败交给调用者；修法二：在函数内部处理掉，例如

   ```rust
   fn load(path: &str) -> String {
       std::fs::read_to_string(path).unwrap_or_default()
   }

   # fn main() {}
   ```

   失败被就地吞掉，调用者永远拿不到原因——所以只在「读不到就用默认值」这种确实可接受时这么做。

2. 用 `ok_or`：`find_user(id).ok_or(MyError::NotFound)`。如果错误需要带上 `id` 之类的信息（构造要花点力气），用 `ok_or_else(|| MyError::NotFound(id))`。

3. 输出是

   ```text
   Err(
       "上层：boom",
   )
   ```

   `{:#?}` 适合调试日志与测试失败信息；`{}` 需要 `Display`，适合给终端用户看的提示。注意 `Err` 里的 `String` 用 `{:#?}` 打印会带引号，因为那是 `String` 的 Debug 表示。

</details>

## 延伸阅读

- The Book 第 9 章《Error Handling》：<https://doc.rust-lang.org/1.98.0/book/ch09-00-error-handling.html>（9.1 讲展开/abort 与回溯读法，9.3 讲 panic 取舍指南）
- The Rust Reference，try 传播表达式（`?` 的语义与 `Try` 现状）：<https://doc.rust-lang.org/1.98.0/reference/expressions/operator-expr.html#the-try-propagation-expression>
- `std::result::Result` API：<https://doc.rust-lang.org/1.98.0/std/result/enum.Result.html>
- `std::option::Option`（`ok_or` / `ok_or_else`）：<https://doc.rust-lang.org/1.98.0/std/option/enum.Option.html>
- `std::error::Error`（supertrait、`source()`、消息写法准则）：<https://doc.rust-lang.org/1.98.0/std/error/trait.Error.html>
- Cargo 手册，`[profile]` 的 `panic` 设置：<https://doc.rust-lang.org/1.98.0/cargo/reference/profiles.html#panic>
