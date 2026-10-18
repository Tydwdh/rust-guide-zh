# 第 18 章 错误处理与 API 设计

## 这一章解决什么问题

第 7 章讲的是错误处理的**机制**：`Result`、`?`、`Display` 与 `Debug` 的分工、`Error::source()` 串起的错误链。那一章关心的是「怎么写才能编译过、怎么把失败交出去」。

这一章关心**取舍**。当你的 crate 被别人依赖时，错误类型和公开函数一样，是契约的一部分：每个 `pub fn f() -> Result<T, E>` 都把 `E` 摆在调用者面前，改它、加它、让它变大，都会改变别人的编译结果。于是「具体 enum 还是不透明的 `Box<dyn Error>`」「`?` 要不要配 `From`」「参数收 `&str` 还是 `String`」「能不能加一个变体」——每个问题都有代价，而代价落在不同的人身上。

读完这一章，你能：

- 按「库 / 应用」两种位置分别选错误类型的形状，并说清各自付出什么；
- 为一个错误枚举设计 `From` 与 `source()`，让错误链完整、不重复、不掉信息；
- 用 `#[non_exhaustive]` 把「将来要加变体」变成兼容性上安全的事，并且知道它的代价是什么；
- 用 builder 与 typestate 把误用挡在编译期；
- 拿 Cargo 的清单判断一次改动是 major 还是 minor；
- 用 `#[must_use]`、`#[deprecated]` 把你的意图写进类型系统。

## 为什么这样设计

先给一个判断标准：**错误类型是公开 API 的一部分，它有两个读者，服务方式不同。**

- 读代码的人要看得到**类别**：`match` 出「文件不存在」和「格式不对」，决定重试还是报给用户。这要求错误是具体类型，变体可匹配、可穷尽。
- 看终端和日志的人要读到**一句话**：`Display` 只写外层发生了什么，细节通过 `source()` 往下问。

这两个读者拉向相反的方向：给匹配的人方便，就得把变体集合**冻结**成公开承诺；给所有人方便（一个 `Box<dyn Error>` 走天下），就得放弃匹配能力。所以这不是「哪个更正确」，而是**将来变化的成本由谁承担**。本书的答案是：

1. 库把错误类型做成**具体 enum**，标上 `#[non_exhaustive]`，给自己留出加变体的空间；
2. 应用内部、尤其是 `main` 附近，允许退化成 `Box<dyn Error>` 一网打尽；
3. 两边靠 `Error::source()` 保持信息不丢。

用一张表看清谁在做什么：

| | 库（library crate） | 应用（application crate） |
|---|---|---|
| 错误类型 | 具体 `enum` + `impl Error` | 可以是 `Box<dyn Error>` |
| 匹配变体 | 调用者会依赖它，因此要能长大 | 通常不匹配，统一上抛即可 |
| 打印 / 日志 | **不做**：库不决定输出到哪、长什么样 | 做：终端提示、结构化日志、退出码 |
| 全局状态 | 不碰 | 自己管 |
| 失败的表达 | `Result`；`unwrap` 只用在「这不可能失败」的地方 | 可以用 `?` + `Box<dyn Error>` 一把抓 |

这条分工不是本书发明的：The Book 讲 panic 取舍时给的理由正是「让库的使用者自己决定怎么处理」——

> it's best to return an error if you can so that the user of the library can decide what they want to do in that case.

打印日志、决定退出码、改全局状态，都是从这个理由推出来的：它们都是**使用者**的决定。

> [!NOTE]
> **本章引用的 Rust API Guidelines 的适用范围**
>
> 后面几节会引用 Rust API Guidelines（`rust-lang.github.io/api-guidelines`）。它由 Rust 库团队基于标准库与生态的经验写成，但它自己在开头就声明这不是强制规范：
>
> > These are only guidelines, some more firm than others. […] These guidelines should not in any way be considered a mandate that crate authors must follow
>
> 另有两点限制：其一，它**不覆盖 2024 edition 之后的新语法风格**，也不讨论本书这一代的工具链细节；其二，它给出的建议面向「要长期维护的公开库」，内部 crate 与一次性程序不必照搬。本章引用它时只当**设计惯例**用，凡是有版本敏感的结论，都以 1.98 的实测为准。

## 机制与规则

### 18.1 库与应用：错误类型该长什么样

库这一侧，错误类型的底线要求是一句话能说清的：**任何出现在公开 `Result<T, E>` 里的 `E`，都必须实现 `std::error::Error`**。API Guidelines 把它写成条目 C-GOOD-ERR：

> An error type is any type `E` used in a `Result<T, E>` returned by any public function of your crate. Error types should always implement the [`std::error::Error`] trait

最小合规的形状——`Debug`（父 trait 要求）+ `Display` + `Error`：

```rust
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct MissingKey {
    pub key: &'static str,
}

impl fmt::Display for MissingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "缺少配置项 {}", self.key)
    }
}

impl Error for MissingKey {}

# fn main() {}
```

三条容易漏的细节：

1. **别拿 `()` 当错误类型**。API Guidelines 的措辞是「Never use `()` as an error type, even where there is no useful additional information for the error to carry」，理由有四条（不实现 `Error`、不实现 `Display`、`Debug` 没信息、没法给下游写 `From<()>`）。真要一个不带数据的错误，就用单元结构体。
2. **错误类型要 `Send + Sync + 'static`**。Guidelines 要求错误类型实现 `Send` 和 `Sync`；这不是洁癖——`std::io::Error::new` 的签名是 `E: Into<Box<dyn Error + Sync + Send>>`，1.98 的文档里就是这么写的。库的错误类型一旦不是 `Send`，调用者就没法把它塞进跨线程的路径。校验方式是在库里加一行编译期断言：

   ```rust
   # use std::error::Error;
   # #[derive(Debug)]
   # pub struct MyErr;
   # impl std::fmt::Display for MyErr {
   #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "x") }
   # }
   # impl Error for MyErr {}
   // 编译期检查：不满足就编译不过
   const _: fn() = || {
       fn assert<T: Send + Sync + 'static>() {}
       assert::<MyErr>();
   };

   # fn main() {}
   ```

3. **消息写给谁看**。Guidelines 对英文的准则是「lowercase without trailing punctuation, and typically concise」；中文照同一精神办——短句、不带句末标点（写法经验见第 7 章 7.6 节）。要排查的细节交给 `source()`，别塞进这一句话。

应用这一侧则相反：能在 `main` 附近把所有底层错误统一成 `Box<dyn Error>`，就不必为每个 `?` 配一次转换。

```rust,no_run
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct MyErr;

impl fmt::Display for MyErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "我的错误")
    }
}

impl Error for MyErr {}

fn inner() -> Result<(), MyErr> {
    Err(MyErr)
}

fn app() -> Result<(), Box<dyn Error>> {
    inner()?;                          // MyErr -> Box<dyn Error>
    std::fs::read_to_string("nope")?;  // io::Error -> Box<dyn Error>
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    app()
}
```

`?` 之所以在这里通吃，是因为 `Box<dyn Error>` 有一个覆盖所有 `E: Error + 'static` 的 `From` 实现。实测输出（Rust 1.98，Windows）：

```text
$ rustc --edition 2024 src/main.rs
$ .\main.exe
Error: MyErr
```

注意打印的是 `Debug` 而不是 `Display`，所以看到的是 `Error: MyErr`（结构体名），不是那句中文；进程退出码是 1（实测）。这就是「应用层决定怎么呈现」的默认版本。

不透明度的代价也要说清楚：`Box<dyn Error>` 把变体信息抹掉了。想按类别处理，只能 `downcast_ref` 试回去：

```rust
use std::error::Error;

#[derive(Debug)]
struct MyErr;

impl std::fmt::Display for MyErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "我的错误")
    }
}

impl Error for MyErr {}

fn classify(err: &Box<dyn Error>) -> &'static str {
    if err.downcast_ref::<MyErr>().is_some() {
        "我认识的错误"
    } else {
        "别人的错误"
    }
}

fn main() {
    let e: Box<dyn Error> = Box::new(MyErr);
    println!("{}", classify(&e));
}
```

实测输出：

```text
我认识的错误
```

`downcast_ref` 定义在 `dyn Error` 上（`impl dyn Error`，另在 `dyn Error + Send`、`dyn Error + Sync + Send` 上各有一份转发），签名要求 `T: Error + 'static`。也就是说：**这条路能用，但每次都要先猜类型**。库把错误类型做成具体 enum，就是为了让调用者不必猜。

### 18.2 `From` 归谁、错误链怎么搭

`From` 的归属不自由，孤儿规则（第 9 章）已经定死：你只能写「外部类型 → 自己的类型」。所以策略问题只剩一个——**该不该写**。

`std::convert::From` 的文档给了三条准入条件，翻译过来是：转换不能失败、不能丢信息、语义要对得上。原文对第一条最硬：

> Note: This trait must not fail. The `From` trait is intended for perfect conversions. […] The conversion is infallible: if the conversion can fail, use `TryFrom` instead; don’t provide a `From` impl that panics.

用它判两个常见场景：`io::Error → MyError` 语义唯一、无信息损失，**写 `From`**，让 `?` 自动转换；`ParseIntError → MyError` 则说不清是哪个字段失败、原文是什么，**不要写 `From`**，改用显式构造函数——`From` 是隐式的，一旦存在，调用者再遇到 `?` 就分不清被转成了哪个变体。

一个同时体现「该写」和「不该写」的错误枚举：

```rust
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum StoreError {
    NotFound(&'static str),
    Io(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound(what) => write!(f, "找不到 {what}"),
            StoreError::Io(_) => write!(f, "读写失败"),
        }
    }
}

impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            StoreError::NotFound(_) => None,
            StoreError::Io(e) => Some(e),
        }
    }
}

// 唯一无歧义的来源：写 From，`?` 才能自动转换
impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

# fn main() {}
```

有歧义的那个，改成显式构造：

```rust
# use std::num::ParseIntError;
# #[derive(Debug)]
# enum PortError {
#     BadNumber { raw: String, cause: ParseIntError },
# }
impl PortError {
    // 不用 From：光一个 ParseIntError 说不清是哪个字段、原文是什么
    pub fn bad_number(raw: impl Into<String>, cause: ParseIntError) -> Self {
        PortError::BadNumber { raw: raw.into(), cause }
    }
}
# fn main() {}
```

搭链的规则第 7 章已经引过，这里再点一次，因为它决定了错误类型**内部怎么存**：包装型错误的底层错误，要么由 `source()` 交出去，要么由 `Display` 渲染，**不能两处都来**（std 原文 "but not both"）。所以 `StoreError::Io(_)` 的 `Display` 只写「读写失败」，具体原因由 `source()` 提供。

链条可以逐层问下去。写一个复用的小工具：

```rust
# use std::error::Error;
# #[derive(Debug)]
# struct E1;
# impl std::fmt::Display for E1 {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "e1") }
# }
# impl Error for E1 {}
fn print_chain(err: &dyn Error) {
    let mut depth = 0;
    let mut cur: Option<&dyn Error> = Some(err);
    while let Some(e) = cur {
        println!("{}{e}", "  ".repeat(depth));
        cur = e.source();
        depth += 1;
    }
}

# fn main() { print_chain(&E1); }
```

`source()` 的签名是 `fn source(&self) -> Option<&(dyn Error + 'static)>`：返回的引用寿命跟着 `&self`，所以上面这个循环没有生命周期问题。

> [!WARNING]
> 标准库有一个现成的链迭代器 `dyn Error::sources()`，但它在 1.98 上仍是 nightly-only 的实验 API（门 `error_iter`，issue #58520，文档标注 “This is a nightly-only experimental API”）。**stable 上必须自己写上面这个循环**，或者把 `source()` 的结果交给日志/序列化库去遍历。

还有一条容易反过来的：**错误类型要小**。`Result<T, E>` 是把 `E` 内联存进去的，`E` 越大，每个成功返回值都要多搬这些字节。Clippy 默认就盯着这件事，实测（Rust 1.98 / clippy 0.1.98）：

```text
warning: the `Err`-variant returned from this function is very large
 --> src\lib.rs:6:19
  |
6 | pub fn cheap() -> Result<u8, BigError> {
  |                   ^^^^^^^^^^^^^^^^^^^^ the `Err`-variant is at least 256 bytes
  |
  = help: try reducing the size of `BigError`, for example by boxing large elements or replacing it with `Box<BigError>`
  = note: `#[warn(clippy::result_large_err)]` on by default
```

所以错误类型里别放整块缓冲、别放长字符串数组；大 payload 装箱，或者只在变体里存「出错位置 + 一句 `&'static str`」。

### 18.3 让错误枚举能长大：`#[non_exhaustive]`

公开枚举的变体集合是承诺：调用者会写出穷尽匹配，你一加变体，他的代码就编译不过。要摆脱这个约束，办法是第一天就声明「这个类型以后还会有新变体」。Reference 的定义很直接：

> The `non_exhaustive` attribute indicates that a type or variant may have more fields or variants added in the future.

它还说明了一条：**在定义它的 crate 内部，`non_exhaustive` 没有任何效果**（原文 “Within the defining crate, `non_exhaustive` has no effect.”）。所以别指望同一 crate 内的测试能验证它的行为——它的全部作用发生在 crate 边界上。下面是三种位置的效果，全部是把库和用户代码分别真编了一次得到的结果：

| 标在哪 | 外部 crate 的后果 | 实测诊断 |
|---|---|---|
| `enum` 上 | 匹配**必须**加通配分支；已存在的变体仍可构造 | `E0004` + note “`PortError` is marked as non-exhaustive, so a wildcard `_` is necessary to match exhaustively” |
| 单元 / 元组变体上 | 该变体在外部**不可构造** | `E0603`：`unit variant A is private` / `tuple variant B is private` |
| 结构体变体、`struct` 上 | 不可用结构体字面量构造（含函数式更新语法） | `E0639`：`cannot create non-exhaustive variant using struct expression` |

在枚举整体上标 `#[non_exhaustive]` 之后，外部匹配的样子（库与用户代码各一个文件，真跑）：

```text
$ cat portlib/src/lib.rs
#[non_exhaustive]
#[derive(Debug)]
pub enum PortError { Missing, BadNumber }

$ cat userapp/src/main.rs
fn main() {
    let e = portlib::PortError::Missing;   // 既有变体：可以构造
    match e {                              // 匹配：必须留通配
        portlib::PortError::Missing => println!("缺端口"),
        portlib::PortError::BadNumber => println!("不是数字"),
    }
}

$ rustc --edition 2024 --crate-type lib --crate-name portlib portlib/src/lib.rs -o libportlib.rlib
$ rustc --edition 2024 --extern portlib=libportlib.rlib userapp/src/main.rs -o app.exe
error[E0004]: non-exhaustive patterns: `_` not covered
 --> userapp/src/main.rs:5:11
  |
5 |     match e {
  |           ^ pattern `_` not covered
  |
note: `PortError` defined here
 --> portlib/src/lib.rs:4:1
  |
4 | pub enum PortError {
  | ^^^^^^^^^^^^^^^^^^
  = note: `PortError` is marked as non-exhaustive, so a wildcard `_` is necessary to match exhaustively
```

这条 note 点明了机制：枚举的变体集合不是全部真相，所以必须留 `_` 兜底。

什么时候该加？看 Cargo 手册 SemVer 章节的两条判定：给公开枚举加变体（枚举没标 `#[non_exhaustive]`）是 **major**——原文 “It is a breaking change to add a new enum variant if the enum does not use the `#[non_exhaustive]` attribute.”；事后把已有枚举补成 `#[non_exhaustive]` 同样是 **major**（它会让外部已有的穷尽匹配报错）。两条合起来只有一个结论，手册把它写成了缓解策略：

> Mark structs, enums, and enum variants as `#[non_exhaustive]` when first introducing them, rather than adding `#[non_exhaustive]` later on.

**所以：公开错误枚举默认标 `#[non_exhaustive]`，从第一个版本开始。**

代价要说清楚：调用者失去穷尽匹配的**编译器保护**——将来你加变体，他们的代码不会报错，只会在运行时走 `_` 兜底分支。这是明确的权衡：**便利留给你，漏处理的风险转给调用者**。另外 `#[non_exhaustive]` 只保护「加」不保护「删」：删变体、改变体字段仍然是 major。反过来说，库内部错误类型、以及要长期冻结变体集合的枚举（例如已被下游用 `match` 做完整状态机的协议类型），不必加它。

### 18.4 builder 与 typestate：把误用挪到编译期

当「构造一个值」需要很多可选项、还要区分必填与可选时，构造函数参数列表会失控。API Guidelines 的 C-BUILDER 给出了五条骨架，头两条最关键：

> 1. Introduce a separate data type `TBuilder` for incrementally configuring a `T` value. […]
> 2. The builder constructor should take as parameters only the data *required* to make a `T`.

也就是：**必填项交给构造函数，可选配置交给链式方法**。Guidelines 区分两种风格，「终端方法只需引用时，用非消费式（借用式）更好」；下面是标准库 `Command` 那种写法：

```rust
{{#rustdoc_include ../snippets/ch18/builder.rs}}
```

实测输出（Rust 1.98，Windows）：

```text
example.com:9000 (重试 3)
example.com:443 (重试 0)
```

注意 `port`/`retries` 返回 `&mut Self`（于是能一行串下去），而终端方法 `connect` 只要 `&self`——构造不需要拿走配置。必填项在构造时给是由**签名**保证的；可选配置多了以后还有更强的机制：把「配置到哪一步了」编码进类型参数，术语叫 typestate——用类型表示值的状态，让「顺序错」变成类型错，「忘了设必填项」直接从运行时错误变成编译错误：

```rust
{{#rustdoc_include ../snippets/ch18/typestate.rs}}
```

实测输出（Rust 1.98，Windows）：

```text
example.com:8080 (重试 3)
example.com:9000 (重试 0)
```

`build` 只挂在 `ConnBuilder<Present>` 上，所以 `ConnBuilder::new()` 之后必须先 `host(...)`。漏掉这一步的诊断：

```rust,compile_fail
use std::marker::PhantomData;

pub struct Missing;
pub struct Present;

pub struct ConnBuilder<State> {
    host: Option<String>,
    _state: PhantomData<State>,
}

impl ConnBuilder<Missing> {
    pub fn new() -> Self {
        ConnBuilder { host: None, _state: PhantomData }
    }
}

impl ConnBuilder<Present> {
    pub fn build(&self) -> String {
        self.host.clone().unwrap_or_default()
    }
}

fn main() {
    let out = ConnBuilder::new().build();
    println!("{out}");
}
```

报错关键行（以 Rust 1.98 为例）：

```text
error[E0599]: no method named `build` found for struct `ConnBuilder<Missing>` in the current scope
  --> src/main.rs:24:34
   |
 6 | pub struct ConnBuilder<State> {
   | ----------------------------- method `build` not found for this struct
...
24 |     let out = ConnBuilder::new().build();
   |                                  ^^^^^ method not found in `ConnBuilder<Missing>`
   |
   = note: the method was found for `ConnBuilder<Present>`
```

最后那句 note 是对读者的交代：**方法的可用范围就是状态的类型参数**。`PhantomData<State>` 只当标记，零大小、零开销（第 15 章）。

别把这两件工具当默认配置：builder 只在「构造复杂」时才值得（Guidelines 列的四种信号：输入很多、复合数据、可选配置、几种风格）；typestate 只在「顺序错误确实会发生」时才值得，代价是每个状态成为独立类型、类型名会出现在错误信息里。另外 builder 产出的配置结构体本身也常标 `#[non_exhaustive]`。

### 18.5 参数怎么收：`&str`、`String` 与泛化边界

参数的形状也影响兼容性。把 `fn f(s: &str)` 泛化成 `fn f(s: impl AsRef<str>)`，对应手册的 “**Minor: generalizing a function to use generics (supporting original type)**”——只要新签名能实例化回原类型就是安全的（手册同时提醒，这种泛化**可能引入类型推断失败**，“usually rare, and may be acceptable”）；反过来把已经泛化的签名收紧回具体类型，对应 “**Major: generalizing a function to use generics with type mismatch**”。既然放宽影响兼容性，就不能随便放宽——先看准则怎么说。API Guidelines 的 C-CALLER-CONTROL 给的是两条对称的规则：

> If a function requires ownership of an argument, it should take ownership of the argument rather than borrowing and cloning the argument.
>
> If a function *does not* require ownership of an argument, it should take a shared or exclusive borrow of the argument rather than taking ownership and dropping the argument.

翻译成人话：**要所有权就直接要，别 `&T` 进来再 `clone`；不要所有权就别拿走**。剩下「要不要泛化」交给 C-GENERIC：

> The fewer assumptions a function makes about its inputs, the more widely usable it becomes. […] Prefer `fn foo<I: IntoIterator<Item = i64>>(iter: I)` over `fn foo(c: &[i64])`.

标准库的现成先例是 `File::open`，它收 `impl AsRef<Path>`，于是字符串字面量、`Path`、`OsString`、`PathBuf` 都能传。下表把常见写法的收益与代价摊开（「能传什么」一列全部实测过）：

| 参数写法 | 调用者能传 | 代价 / 注意 |
|---|---|---|
| `&str` | `&str`、`&String`（解引用强转） | 拿到 `String` 的调用者要写 `&s`；函数内要留下就得再分配 |
| `&String` | 只有 `&String` | 收窄：手里只有 `&str` 的调用者得先 `to_string()`；clippy 的 `ptr_arg` 会提醒 |
| `String` | `String` | `&str` 传入要 `.to_string()`；强制调用者交出所有权 |
| `impl Into<String>` | `&str`、`String` | 单态化一份新代码；参数是匿名的，**不能显式指定类型** |
| `impl AsRef<Path>` | `&str`、`String`、`&String`、`&Path`、`PathBuf` | 同上；泛化后类型错误的信息更长 |
| `&[T]` | `&Vec<T>`、`&[T; N]`、`&[T]` | 不能按值传集合 |
| `impl IntoIterator<Item = T>` | `Vec<T>`、`&[T]`、`0..n` | 同上；函数体只能迭代一次 |

「不能显式指定类型」是 hard 限制，实测（Rust 1.98）：

```rust,compile_fail
fn takes_into(s: impl Into<String>) -> String {
    s.into()
}

fn main() {
    // `impl Trait` 参数是匿名的，不能像泛型参数那样用 turbofish 指定
    let _ = takes_into::<&str>("a");
}
```

```text
error[E0107]: function takes 0 generic arguments but 1 generic argument was supplied
 --> src/main.rs:7:13
  |
7 |     let _ = takes_into::<&str>("a");
  |             ^^^^^^^^^^-------- help: remove the unnecessary generics
  |             |
  |             expected 0 generic arguments
  |
note: function defined here, with 0 generic parameters
 --> src/main.rs:1:4
  |
1 | fn takes_into(s: impl Into<String>) -> String {
  |    ^^^^^^^^^^
  = note: `impl Trait` cannot be explicitly specified as a generic argument
```

选择顺序：**先写最具体的类型**（读文本 `&str`、读字节 `&[u8]`），签名短、类型错误清楚；**只在「调用点多 + 调用者类型杂」时才泛化**，标准库有先例的（`AsRef<Path>`）直接照抄；泛化时**用最小的 trait 约束**，只需要迭代就用 `IntoIterator`。

最后别把两个建议弄混：**实现**转换时写 `From` / `TryFrom`，不要写 `Into` / `TryInto`（它们有基于前两者的 blanket 实现，Guidelines 原话是 “The following conversion traits should never be implemented: `Into`, `TryInto`”）；而**约束参数**时写 `impl Into<String>` 完全正常——它依赖的是别人写好的 `From`。

### 18.6 SemVer：哪些改动算破坏

有公开 API 的 crate 就要守 SemVer。Cargo 手册把改动分成三档：**Major change**（必须提升主版本号）、**Minor change**（只需提升次版本号）、**Possibly-breaking change**（**有些**项目认为它破坏了兼容性、另一些不认为——“The impact of these changes should be considered carefully.”）。`0.y.z` 阶段另算：`y` 位当 major、`z` 位当 minor，`0.0.z` 一律按 major 处理（Cargo 只把最左边非零的那一位看作不兼容边界）。

下面挑出与错误类型及 API 形状最相关的条目（等级与英文条目名照抄手册）：

| 改动 | 等级 |
|---|---|
| `renaming/moving/removing any public items`（含删错误变体） | Major |
| `adding new public items`（新错误类型、新构造函数、新 `impl`） | Minor |
| `adding new enum variants (without non_exhaustive)` | Major |
| `adding new fields to an enum variant` | Major |
| `adding non_exhaustive to an existing enum, variant, or struct with no private fields` | Major |
| `adding a non-defaulted trait item`（给公开 trait 加必须实现的方法） | Major |
| `adding a defaulted trait item`（加有默认实现的方法） | Possibly-breaking |
| `adding a trait item that makes the trait non-object safe` | Major |
| `tightening generic bounds`（例如给 `struct Foo<A>` 补上 `A: Eq`） | Major |
| `loosening generic bounds` | Minor |
| `generalizing a function to use generics (supporting original type)` | Minor |
| `generalizing a function to use generics with type mismatch` | Major |
| `adding/removing function parameters` | Major |
| `introducing new lints`（加 `#[deprecated]`、`#[must_use]`） | Minor |
| `changing the minimum version of Rust required` | Possibly-breaking |
| `adding a new Cargo feature` / `removing a Cargo feature` | Minor / Major |
| `making an unsafe function safe` | Minor |

**那么「错误类型变大算不算破坏」？**它是两个问题，分开答：

1. **变体集合长大了**（枚举多了一个变体）——**是** major，除非你从第一天就标了 `#[non_exhaustive]`：标了之后外部匹配本来就必须留通配分支，加变体就只是 minor。把新失败类别做成**独立的错误类型**同样安全（「新增公开项」是 minor）。
2. **类型占的字节变大了**（例如某个变体换了个更宽的字段）——**不是** SemVer 破坏。手册讲布局时写得很清楚：默认表示（不带 `repr`）的类型“do not have a well-defined alignment, layout, or size”，“code should not make any assumptions about it”，所以即使有外部代码因为假设了大小而出错，“This is not considered a SemVer breaking change since those assumptions should not be made.” 但它仍值得避免——调用方每次都在 `Result` 里内联搬这些字节，clippy 的 `result_large_err` 会直接报出来（18.2）。

配套的两条实践建议：

- **先让类型能长大**：公开枚举第一天就标 `#[non_exhaustive]`（18.3），公开结构体字段私有 + 提供构造函数或 builder（18.4）。
- **删除走三步**：先 `#[deprecated]` 并保留至少一个 minor 版本（加 deprecated 属 minor，见上表），再在下一个 major 里删除。用 sealed trait 保护「只想自己实现」的 trait，这样给 trait 加方法就不再是 major（第 12 章）。

### 18.7 把意图写进属性：`#[must_use]` 与 `#[deprecated]`

两个成本极低、效果立竿见影的属性。

`#[must_use]` 标记「这个返回值被丢掉多半是 bug」。Reference 说明了它在类型上的行为：当表达式语句的值是那个类型时，触发 `unused_must_use` lint：

> When the attribute is applied to a struct, enumeration, or union, if the expression of an expression statement has that type, the use triggers the `unused_must_use` lint.

写法与实测诊断（Rust 1.98）：

```rust
// 返回本次写入的字节数；只看不记就白做了
#[must_use = "写入的字节数不用就没意义"]
fn write_all(data: &str) -> usize {
    data.len()
}

fn main() {
    let n = write_all("abc");
    println!("写入 {n} 字节");
}
```

如果把返回值丢掉，诊断长这样（实测）：

```text
warning: unused return value of `write_all` that must be used
 --> src/main.rs:8:5
  |
8 |     write_all("abc");
  |     ^^^^^^^^^^^^^^^^
  |
  = note: 写入的字节数不用就没意义
  = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
  |
8 |     let _ = write_all("abc");
  |     +++++++
```

三点补充。**带消息**（`#[must_use = "..."]`）比光秃秃的 `#[must_use]` 有用得多：消息会进诊断，直接说明为什么。**位置不止类型**：Reference 列的允许位置是「Struct、Enumeration、Union、Function、Trait」，1.98 上**方法**同样有效（实测诊断是 `unused return value of \`Token::issue\` that must be used`），给 builder 的终端方法、「拿到就要用」的方法加它收益最大；而 `Result` **本身已经**带了这个属性（std 源码写的是 `#[must_use = "this \`Result\` may be an \`Err\` variant, which should be handled"]`），你自己加它的价值在于覆盖**不是** `Result` 的返回值。最后**有一个例外**：`Result<(), E>` 在 `E` 是无值类型（uninhabited）时不触发 lint，`ControlFlow<B, ()>` 同理，因为那种值根本构造不出来。Reference 还补了一条边界：「A `#[non_exhaustive]` type from an external crate is not considered uninhabited for this purpose, because it may gain constructors in the future.」下面这段带着 `#![deny(unused_must_use)]` 也能编译，说明确实没有警告：

  ```rust
  #![deny(unused_must_use)]

  enum Never {} // 没有任何值的类型

  fn noop() -> Result<(), Never> {
      Ok(())
  }

  fn main() {
      // 不会触发 unused_must_use：Err 分支永远构造不出来
      noop();
      println!("通过");
  }
  ```

`#[deprecated]` 标记「别再用这个」。Reference 列出的形式有 `deprecated`、`deprecated = "message"`，以及带 `since` / `note` 两个可选字段的元列表语法；`rustc` 不解析 `since` 里的版本号，但 Clippy 等工具会看。它还能写在模块、`impl` 块这类容器上——「all child items inherit the deprecation attribute」。

```rust
#[deprecated(since = "0.3.0", note = "改用 `parse_config`：它会保留错误链")]
pub fn load(path: &str) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

fn main() {
    let _ = load("x");
}
```

实测诊断（Rust 1.98）：

```text
warning: use of deprecated function `load`: 改用 `parse_config`：它会保留错误链
 --> src/main.rs:7:13
  |
7 |     let _ = load("x");
  |             ^^^^
  |
  = note: `#[warn(deprecated)]` on by default
```

两个属性都对应 18.6 表里的「`introducing new lints`」——**Minor change**。但这不等于可以随便加：Cargo 手册提醒，如果用户把警告 `deny` 掉了，新 lint 会直接编译失败，所以「library authors should be cautious about introducing new warnings」。手册还给了一条进阶做法：把弃用藏在 feature 后面（`#[cfg_attr(feature = "deprecated", deprecated = "use bar instead")]`），让使用者自己决定什么时候面对它。

## 动手看看

下面这个完整程序把本章的主线串起来：库一侧的公开错误枚举（`#[non_exhaustive]`、`From` 策略、`source()`）、应用一侧的错误链遍历与 `Box<dyn Error>` 边界。源码在 `snippets/ch18/port_config.rs`，用 `rustc --edition 2024 port_config.rs` 编译后运行即可。

```rust
{{#rustdoc_include ../snippets/ch18/port_config.rs}}
```

实测输出（Rust 1.98，Windows）：

```text
"" -> Err
配置里缺少端口
"port = 80a0" -> Err
端口 "80a0" 不是数字
  └─ invalid digit found in string
"port = 8080" -> Ok(port = 8080)
上层拿到 Box<dyn Error>：读取配置文件失败
底层原因还在：source().is_some() = true
```

对照着看四件事：

- `PortError` 上标了 `#[non_exhaustive]`，所以外部调用者匹配它时必须留 `_`（18.3）；
- `From<io::Error>` 写了（语义唯一），`ParseIntError` 那条**没写**，改用 `PortError::bad_number(raw, cause)` 把原文一起带上（18.2）；
- `Display` 每个变体只写一句话，`BadNumber` 的原文只在 `Display` 里出现、底层原因只在 `source()` 里出现，两边不重复；
- 链条遍历是调用者的事，`print_chain` 顺着 `source()` 一路问到 `invalid digit found in string` 为止。

## 报错现场

### 现场一：`?` 说「转换不了」

写好了错误枚举、也写了 `impl Error`，但忘了 `From`：

```rust,compile_fail
use std::fmt;

#[derive(Debug)]
enum AppError {
    Missing,
    Io(std::io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Missing => write!(f, "缺少配置"),
            AppError::Io(_) => write!(f, "读取失败"),
        }
    }
}

impl std::error::Error for AppError {}
// 这里故意不写 impl From<std::io::Error> for AppError

fn load(path: &str) -> Result<String, AppError> {
    let text = std::fs::read_to_string(path)?;
    Ok(text)
}

fn main() {
    let _ = load("config.toml");
}
```

报错关键行（以 Rust 1.98 为例）：

```text
error[E0277]: `?` couldn't convert the error to `AppError`
  --> src/main.rs:22:45
   |
21 | fn load(path: &str) -> Result<String, AppError> {
   |                        ------------------------ expected `AppError` because of this
22 |     let text = std::fs::read_to_string(path)?;
   |                -----------------------------^ the trait `From<std::io::Error>` is not implemented for `AppError`
   |                |
   |                this can't be annotated with `?` because it has type `Result<_, std::io::Error>`
   |
note: `AppError` needs to implement `From<std::io::Error>`
  --> src/main.rs:4:1
   |
 4 | enum AppError {
   | ^^^^^^^^^^^^^
   = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
```

**定位路径**：第一句就说明白了（`?` 转不过去），中间的插入符告诉你「哪个表达式的错误类型不匹配」，最后一行 note 点出机制——`?` 会隐式调用 `From` 做转换。**修法**两条，选哪条取决于语义：转换无歧义就补 `impl From<io::Error> for AppError { … }`（18.2 的第一种情形）；有歧义或要额外上下文，就用 `.map_err(...)` 显式构造变体。

### 现场二：库加了变体，`match` 就报错

库里给公开错误枚举加了一个变体，用户代码里那段穷尽匹配立刻失效：

```rust,compile_fail
#[derive(Debug)]
enum ConfigError {
    Empty,
    BadNumber,
    BadPort, // 库新加的变体
}

fn describe(e: ConfigError) -> &'static str {
    match e {
        ConfigError::Empty => "为空",
        ConfigError::BadNumber => "不是数字",
    }
}

fn main() {
    println!("{}", describe(ConfigError::BadPort));
}
```

报错关键行（以 Rust 1.98 为例）：

```text
error[E0004]: non-exhaustive patterns: `ConfigError::BadPort` not covered
  --> src/main.rs:9:11
   |
 9 |     match e {
   |           ^ pattern `ConfigError::BadPort` not covered
   |
note: `ConfigError` defined here
  --> src/main.rs:2:6
   |
 2 | enum ConfigError {
   |      ^^^^^^^^^^^
...
 5 |     BadPort, // 库新加的变体
   |     ------- not covered
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
   |
11 ~         ConfigError::BadNumber => "不是数字",
12 ~         ConfigError::BadPort => todo!(),
   |
```

**定位路径**：`E0004` 是穷尽性检查失败，箭头指出**是哪个模式没覆盖**，`help` 直接把缺失的分支补全给你看。**修法**：用户侧加分支或加 `_`；库侧的正解是**从一开始**就在枚举上标 `#[non_exhaustive]`（18.3）——事后补它就不是修法而是 major 变更了（Cargo 手册的 “Major: adding non_exhaustive to an existing enum, variant, or struct with no private fields”）。

## 小结

- 错误类型是公开 API：`pub fn -> Result<T, E>` 里的 `E` 要实现 `Error`，应当是 `Send + Sync + 'static`，不能用 `()`。库把失败**返回**出去；打印、日志、退出码、全局状态是应用的事。
- 应用侧可以在 `main` 附近统一成 `Box<dyn Error>`，让 `?` 一把抓；代价是变体信息被抹掉，要按类别处理得 `downcast_ref` 猜回去。
- `From` 只在「不能失败、不丢信息、语义唯一」时写；有歧义的改用显式构造函数。包装型错误的底层原因要么由 `source()` 给出、要么由 `Display` 渲染，不能两处都来。错误类型要小，大 payload 装箱（`clippy::result_large_err`）。
- 公开错误枚举第一天就标 `#[non_exhaustive]`：外部匹配必须留通配分支，以后加变体就是 minor 而不是 major；代价是调用者失去穷尽匹配的保护，而它只保护「加」不保护「删」。
- builder 用于「构造复杂」：必填项进构造函数，可选项走链式方法，终端方法只需引用时优先非消费式。typestate 把状态编码进类型参数，让顺序错误变成编译错误，代价是类型数量与文档复杂度。
- 参数先写最具体的类型，只在「调用点多 + 调用者类型杂」时泛化；实现转换写 `From`/`TryFrom`，不写 `Into`/`TryInto`。
- 判兼容性查 Cargo 手册三档清单：加变体/加字段/收紧泛型约束/删公开项是 major，新增公开项、放宽约束、把具体类型泛化（原调用仍能用）是 minor，提高 MSRV 属 possibly-breaking；类型占的字节变大本身不算破坏。
- 用属性把意图写进编译期：`#[must_use = "为什么"]` 挡住「拿了不用」，`#[deprecated(since, note)]` 给出迁移路径；两者都是 minor，但用户若 `deny` 掉警告就会编译失败，所以要克制、要带说明。

## 练习

1. 某库的公开错误类型是 `pub enum CacheError { Miss, Io(std::io::Error) }`，且**没有** `#[non_exhaustive]`。现在作者想加一个 `Corrupt` 变体。请分别回答：(a) 直接加会发生什么、属于哪一档变更；(b) 有没有办法在不发大版本的前提下加；(c) 如果现在就补 `#[non_exhaustive]` 呢？

2. 下面这个函数有两处可改进的 API 设计。指出它们，并说明各自属于本章哪条规则。

   ```rust
   fn save(path: &String, body: String) -> bool {
       std::fs::write(path, body).is_ok()
   }

   # fn main() {}
   ```

3. 预测编译结果：下面这段能编译吗？如果不能，改哪里？如果能，输出什么？

   ```rust,compile_fail
   use std::marker::PhantomData;

   pub struct Plain;
   pub struct Locked;

   pub struct Door<State> {
       _state: PhantomData<State>,
   }

   impl Door<Plain> {
       pub fn new() -> Self {
           Door { _state: PhantomData }
       }
       pub fn lock(self) -> Door<Locked> {
           Door { _state: PhantomData }
       }
   }

   impl Door<Locked> {
       pub fn open(&self) -> &'static str {
           "门开了"
       }
   }

   fn main() {
       let d = Door::new();
       println!("{}", d.open());
   }
   ```

<details><summary>参考答案</summary>

1. (a) 直接加会**编译破坏**依赖方的穷尽匹配（现场二），Cargo 手册把它归为 “Major: adding new enum variants (without `non_exhaustive`)” → **major**，必须发大版本。
   (b) 可以不破坏，关键是**别动这个枚举的变体集合**：把新失败类别做成**独立的公开错误类型**（`CorruptError` 之类），「新增公开项」属于 minor，不影响调用者的 `match`。注意给 `Corrupt` 这个**变体**单独标 `#[non_exhaustive]` 没用——那只挡「构造」，挡不住「匹配」：调用者的穷尽匹配照样会漏掉新变体而报错（18.3 的表里，只有标在**枚举整体**上才逼出通配分支）。
   (c) 补 `#[non_exhaustive]` 本身是 “Major: adding non_exhaustive to an existing enum, variant, or struct with no private fields” → 也是 **major**（它会让外部已有的穷尽匹配全部报错）。所以这件事只能从第一天做；已经发出去的枚举，要么等到下一个大版本，要么用新增类型绕开。

2. 两处：
   - `path: &String` 把参数收得比需要的窄。按 C-CALLER-CONTROL「不要所有权就借，且借最宽能用的」，读路径应该收 `impl AsRef<Path>`（标准库 `File::open` 的先例，`&str`/`Path`/`PathBuf` 全能传）或至少 `&Path`；`&String` 还正好是 clippy `ptr_arg` 的对象。
   - 返回 `bool` 把失败原因丢光了。应该返回 `Result<(), SaveError>`（可 `#[non_exhaustive]`），让调用者能 `?`、能打印、能按类别处理；`bool` 满足不了「错误类型实现 `Error`」这条底线要求。
   顺带一提 `body: String` 倒是**对**的：函数把 body 交给了 `write`（需要所有权），按 C-CALLER-CONTROL 要所有权就拿走，不该写成 `&str` 再在内部 `to_owned()`。

3. **不能编译。** `Door::new()` 返回 `Door<Plain>`，而 `open` 只挂在 `Door<Locked>` 上，所以 `d.open()` 报 `E0599`，note 会写 “the method was found for `Door<Locked>`”。修法是先 `lock()`（它消费 `self` 并返回 `Door<Locked>`）：

   ```rust
   # use std::marker::PhantomData;
   # pub struct Plain;
   # pub struct Locked;
   # pub struct Door<State> { _state: PhantomData<State> }
   # impl Door<Plain> {
   #     pub fn new() -> Self { Door { _state: PhantomData } }
   #     pub fn lock(self) -> Door<Locked> { Door { _state: PhantomData } }
   # }
   # impl Door<Locked> {
   #     pub fn open(&self) -> &'static str { "门开了" }
   # }
   fn main() {
       let d = Door::new().lock();
       println!("{}", d.open());
   }
   ```

   输出 `门开了`。`lock` 是唯一能把状态从 `Plain` 推到 `Locked` 的操作，于是 `open` 的存在本身就是「门已锁」的编译期证明。

</details>

## 延伸阅读

- `std::error::Error`（`source()`、`Send`/`Sync` 要求、消息写法准则、`downcast_*`、nightly 的 `sources()`）：<https://doc.rust-lang.org/std/error/trait.Error.html>
- `std::fmt::Display` 与 `Debug` 的分工（小结见第 7 章 7.6 节）：<https://doc.rust-lang.org/std/fmt/index.html>
- `std::convert::From`（“must not fail” 与三条准入条件）：<https://doc.rust-lang.org/std/convert/trait.From.html>
- `std::convert::TryFrom`（可失败的转换，关联类型 `Error`）：<https://doc.rust-lang.org/std/convert/trait.TryFrom.html>
- Cargo 手册《SemVer Compatibility》（三档改动与服务端清单，本章 18.6 的来源）：<https://doc.rust-lang.org/cargo/reference/semver.html>
- Rust API Guidelines（C-GOOD-ERR、C-CONV-TRAITS、C-BUILDER、C-CALLER-CONTROL、C-GENERIC、C-SEALED 等条目）：<https://rust-lang.github.io/api-guidelines/>
- The Rust Reference，类型系统属性 `non_exhaustive`：<https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute>
- The Rust Reference，诊断属性 `must_use` / `deprecated`：<https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-must_use-attribute>
- The Book 第 9 章《Error Handling》（panic 与 `Result` 的取舍、库代码的准则）：<https://doc.rust-lang.org/book/ch09-00-error-handling.html>
- The Book《Object-Oriented Design Pattern Implementations》中的 “Encoding States and Behavior as Types”（另一个 typestate 例子）：<https://doc.rust-lang.org/book/ch18-03-oo-design-patterns.html>
- `std::marker::PhantomData`（typestate 里的状态标记为什么零开销）：<https://doc.rust-lang.org/std/marker/struct.PhantomData.html>
- Clippy 的 `result_large_err`：<https://rust-lang.github.io/rust-clippy/master/index.html#result_large_err>
