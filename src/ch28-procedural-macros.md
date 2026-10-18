# 第 28 章 过程宏

## 这一章解决什么问题

第 27 章的 `macro_rules!` 是**模式替换**：你写下一组 `(模式) => { 展开 }`，编译器在调用点做语法片段的配对与替换。它能消掉大量重复，但有两件事它做不到。一是**算**——宏体里不能调用函数、不能循环、不能读文件；二是**看见结构**——它匹配的是 token 树，没有办法「遍历一个结构体的字段」。`#[derive(Serialize)]`、`#[tokio::main]`、`sqlx::query!` 这类真正懂输入语义的代码生成器，都不是声明宏。

它们的实现方式叫**过程宏（procedural macro）**：一个**在编译期被编译器调用的普通 Rust 函数**，吃进去一段 token 流，吐出来一段 token 流。Reference 把它概括成一句话："Procedural macros allow creating syntax extensions as execution of a function."（引自 Rust Reference）

本章交付三件事：

- 三种形态（derive / 属性 / 函数式）的**形状与落点**：谁在什么时候被调用、输入是什么、输出放在哪；
- `proc_macro` 的 **API 面**：`TokenStream`、`TokenTree` 的四类 token、`Ident`/`Punct`/`Literal`/`Group`、`Span`；
- 一份**零依赖、手写解析**的实现：`examples/macro-demo` 里的 `#[derive(Describe)]`（外加一个属性宏 `#[repeat(N)]` 和一个函数式宏 `sum!`），由 `examples/macro-demo-use` 消费并跑真实测试；最后交代 syn/quote 的取舍。

读完你应当能：

- 判断一个需求该用声明宏还是过程宏，以及用三种形态中的哪一种；
- 读懂一个 derive 宏的实现，包括它怎么拿到输入、怎么拼输出；
- 自己写一个只处理具名字段结构体的最小 derive，并为它写测试；
- 让错误落在正确的位置：`compile_error!` 与 span 归属；
- 知道哪些能力（诊断 API、`quote!` 宏）在 1.98 还没稳定。

**边界**：`macro_rules!` 的片段说明符、卫生性、`$crate` 在第 27 章；宏在构建流水线里的位置（`build.rs`、依赖解析与构建顺序）在第 30 章；rustc 内部怎么把诊断渲染成波浪线，第 37 章一句话带过。

## 为什么这样设计

### 接口是 token 流，不是 AST

过程宏的输入输出都不是「语法树」，而是 **token 流（token stream）**。这是故意选的：语法树是编译器内部表示，随版本演进；而 token 流是语言的表层，几乎不变。Reference 把这条理由写得很直白："Procedural macros operate over token streams instead of AST nodes, which is a far more stable interface over time for both the compiler and for procedural macros to target."（引自 Rust Reference）

代价落在宏作者身上：**解析要自己做**。给过程宏的输入没有「字段列表」这种东西，只有一串 token；认出「这是结构体、这是字段名、这是字段类型」是你的工作——除非你引 `syn`（见 28.6）。

token 流大致等价于 `Vec<TokenTree>`，而 `TokenTree` 是词法层面的一个单元。Reference 的对照："A token stream is roughly equivalent to `Vec<TokenTree>` where a `TokenTree` can roughly be thought of as lexical token. For example `foo` is an `Ident` token, `.` is a `Punct` token, and `1.2` is a `Literal` token. The `TokenStream` type, unlike `Vec<TokenTree>`, is cheap to clone."（引自 Rust Reference）注意最后半句：`TokenStream` 不是 Vec，克隆它很便宜（文档专门强调了这一条），所以「先克隆一份、再逐段消费」是常规写法。

### 为什么必须是独立 crate，而且不能自己用

过程宏必须住在一个 `proc-macro` 类型的 crate 里，并且**定义它的 crate 不能使用它**。Reference："Procedural macros must be defined in the root of a crate with the crate type of `proc-macro`. The macros may not be used from the crate where they are defined, and can only be used when imported in another crate."（引自 Rust Reference）

原因是它的运行方式：过程宏是**在编译别的 crate 时被编译器加载执行的代码**（编译 `macro-demo-use` 时，rustc 先编译 `macro-demo` 成动态库，再把它加载进自己的进程，调你的函数）。既然它是插件，它的「公共接口」就只有那些宏。想顺便导出个 trait 或辅助函数？rustc 直接拒绝。这条限制以 Rust 1.98 的原话是：

```text
error: `proc-macro` crate types currently cannot export any items other than functions tagged with `#[proc_macro]`, `#[proc_macro_derive]`, or `#[proc_macro_attribute]`
```

这条限制有一个所有人都见过的后果：**serde 被拆成两个 crate**——`serde` 放 trait，`serde_derive` 放宏，再用 `pub use` 把宏从前者重新导出，让用户只写一个依赖。我们的例子照抄这个结构：宏和解析代码在 `macro-demo`，`Describe` trait 在 `macro-demo-use`。

### 一次调用就是一个函数：返回语法、panic、死循环

过程宏函数的三种结局，Reference 列得很清楚："As functions, they must either return syntax, panic, or loop endlessly. Returned syntax either replaces or adds the syntax depending on the kind of procedural macro. Panics are caught by the compiler and are turned into a compiler error. Endless loops are not caught by the compiler which hangs the compiler."（引自 Rust Reference）

两点值得记住：

- **panic 不是崩溃，是一条编译错误**（`error: proc-macro derive panicked`，后面跟着你的 panic 消息）。能编译、但不好看，所以正式做法是用 `compile_error!`（见 28.7）；
- **死循环不会被打断**，编译器就是卡住。写宏时别写可能不收敛的循环。

### derive 是「追加」，属性宏和函数式宏是「替换」

三种形态的差别不在「能不能算」（都能，都是函数），而在**输入是什么、输出放到哪**：

| 形态 | 写在哪调用 | 输入 | 输出落点 |
|---|---|---|---|
| derive 宏 `#[derive(X)]` | 结构体/枚举/联合体定义上 | 被标注的**整个 item** 的 token 流 | **追加**在该 item 之后 |
| 属性宏 `#[x(…)]` | item、`extern` 块里的项、`impl`、`trait` 定义上 | `(属性括号里的 token, 整个 item)` | **替换**整个 item |
| 函数式宏 `x!(…)` | 宏调用能出现的任何位置（表达式、语句、模式、类型、item…） | 括号里的 token | **替换**整个宏调用 |

derive 的「追加」是最反直觉的一条。"These items are appended following the input item within the same module or block."（引自 Rust Reference）也就是说，`#[derive(Describe)] struct Point { … }` 展开后，模块里同时存在**原来的 `Point`** 和宏生成的新 items——所以你**不能**在输出里再定义一个同名结构体，那是重复定义。属性宏与函数式宏则是彻底替换：你返回什么，那里就只剩什么；想把原 item 留下，得把它自己再吐回去（我们的 `#[repeat]` 就这么做）。

> [!NOTE]
> derive 宏还能声明**辅助属性（helper attribute）**：`#[proc_macro_derive(X, attributes(helper))]` 之后，`#[helper]` 可以写在字段或变体上，供宏读取。Reference 强调这些属性是**惰性（inert）**的——它们不产生任何效果，只是「标签」，实际语义由宏自己解释。

### 过程宏不卫生

`macro_rules!` 有卫生性（第 27 章）；过程宏**没有**。"Procedural macros are unhygienic. This means they behave as if the output token stream was simply written inline to the code it's next to."（引自 Rust Reference）展开结果就像你在调用点手抄了一遍。两条实践规则因此成立：

- 生成的代码里引用路径要写**绝对路径**（`::std::string::String`，而不是 `String`），否则调用点恰好有个同名项就会解析错；
- 生成的辅助名字要**不容易撞**（如 `__internal_foo` 而不是 `foo`），因为它们在调用点的作用域里可见。

下面这张图把三种形态的输入与输出落点并排摊开：

<svg viewBox="0 0 760 240" width="100%" role="img" aria-label="三种过程宏的输入与输出落点对照：derive 的输出追加在原 item 之后，属性宏和函数式宏的输出替换原来的 item 或调用">
  <g font-family="monospace" font-size="13" fill="currentColor">
    <text x="12" y="20">derive &#91;输入：整个 item&#93;</text>
    <rect x="12" y="30" width="300" height="40" rx="5" fill="none" stroke="currentColor"/>
    <text x="24" y="55">#&#91;derive(Describe)&#93; struct Point {..}</text>
    <text x="326" y="55" fill="var(--links)">追加 --&gt;</text>
    <rect x="430" y="30" width="310" height="40" rx="5" fill="none" stroke="var(--links)" stroke-dasharray="5 3"/>
    <text x="442" y="55">impl Describe for Point {..}</text>
    <text x="12" y="102">属性宏 &#91;输入：(属性, item)&#93;</text>
    <rect x="12" y="112" width="300" height="40" rx="5" fill="none" stroke="currentColor"/>
    <text x="24" y="137">#&#91;repeat(3)&#93; const SCALE: i32 = 2;</text>
    <text x="326" y="137" fill="var(--warning-border)">替换 --&gt;</text>
    <rect x="430" y="112" width="310" height="40" rx="5" fill="none" stroke="var(--warning-border)" stroke-dasharray="5 3"/>
    <text x="442" y="137">SCALE_0 … SCALE_2，再加原件</text>
    <text x="12" y="184">函数式宏 &#91;输入：括号里的 token&#93;</text>
    <rect x="12" y="194" width="300" height="40" rx="5" fill="none" stroke="currentColor"/>
    <text x="24" y="219">sum!(10, 20, 30)</text>
    <text x="326" y="219" fill="var(--warning-border)">替换 --&gt;</text>
    <rect x="430" y="194" width="310" height="40" rx="5" fill="none" stroke="var(--warning-border)" stroke-dasharray="5 3"/>
    <text x="442" y="219">Literal(60)</text>
  </g>
</svg>

## 机制与规则

### 28.1 三种形态与注册方式

形态由**属性**决定，函数签名也随之不同。三种注册方式都要写在 crate 根、函数都得是 `pub` 且用 Rust ABI，不能加 `async`/`unsafe` 这类限定符（`derive` 那个还允许 `const`）：

```rust,ignore
// 形态一：derive（crate 根）
#[proc_macro_derive(Describe)]
pub fn derive_describe(input: TokenStream) -> TokenStream { /* … */ }

// 形态二：属性宏
#[proc_macro_attribute]
pub fn repeat(attr: TokenStream, item: TokenStream) -> TokenStream { /* … */ }

// 形态三：函数式宏
#[proc_macro]
pub fn sum(input: TokenStream) -> TokenStream { /* … */ }
```

配 `examples/macro-demo/Cargo.toml` 里的 crate 类型：

```toml
[lib]
proc-macro = true
```

几个位置限制是硬规则，写错了直接编译失败：

- **`#[proc_macro_derive]` 只能用在结构体、枚举、联合体上**，且输入是那个 item 的 token 流（含它自己的属性与可见性前缀）；
- **属性宏只能用在 item 上**：普通 item、`extern` 块里的项、`impl` 块、`trait` 定义。它拿不到表达式、语句；
- **函数式宏位置最自由**：语句、表达式、模式、类型、item 位置都能调用（这也是 `sqlx::query!` 能把一句 SQL 变成类型安全查询的原因）。

### 28.2 `TokenStream` 与 `TokenTree`：宏作者眼里的源码

`proc_macro` 的文档自我定位是一句话："A support library for macro authors when defining new macros."（引自 `proc_macro` 模块文档）它提供的类型就这么多：`TokenStream`、`TokenTree`（四类）、`Group`/`Ident`/`Punct`/`Literal`、`Span`、两个枚举 `Delimiter`/`Spacing`、错误类型 `LexError`。API 契约也很小：

- **建**：`TokenStream::new()`、`"一段源码".parse::<TokenStream>()`（`FromStr`，解析失败返回 `LexError`）、`FromIterator<TokenTree>`、`Extend<TokenTree>`；
- **读**：`impl IntoIterator<Item = TokenTree>`、`Display`（`to_string()` 会重新打印成源码）、`is_empty()`、便宜的 `clone()`；
- **拆**：`Group::stream()` 取出括号内的流、`Group::delimiter()` 认出 `()`/`{}`/`[]`。

「源码在宏眼里长什么样」值得亲手看一次。写一个只打印输入的 derive：

```rust,ignore
#[proc_macro_derive(Dump)]
pub fn dump(input: TokenStream) -> TokenStream {
    walk(input, 0); // 按树形缩进打印每一颗 token
    TokenStream::new()
}

/// 缩进打印一棵 token 树（看清结构即可，不做美化）。
fn walk(stream: TokenStream, depth: usize) {
    let pad = "  ".repeat(depth);
    for token in stream {
        match token {
            TokenTree::Group(group) => {
                println!("{pad}Group({:?})", group.delimiter());
                walk(group.stream(), depth + 1);
            }
            TokenTree::Ident(id) => println!("{pad}Ident({id})"),
            TokenTree::Punct(p) => println!("{pad}Punct('{}', {:?})", p.as_char(), p.spacing()),
            TokenTree::Literal(lit) => println!("{pad}Literal({lit})"),
        }
    }
}
```

拿它标注

```rust,ignore
#[derive(Dump)]
struct Point {
    x: i32,
    name: std::string::String,
    tag: &'static str,
}
```

再 `cargo build`，宏里 `println!` 的输出会出现在构建日志里（以 Rust 1.98、本机为例，节选）：

```text
Ident(struct)
Ident(Point)
Group(Brace)
  Ident(x)
  Punct(':', Alone)
  Ident(i32)
  Punct(',', Alone)
  Ident(name)
  Punct(':', Alone)
  Ident(std)
  Punct(':', Joint)
  Punct(':', Alone)
  Ident(string)
  Punct(':', Joint)
  Punct(':', Alone)
  Ident(String)
  Punct(',', Alone)
  Ident(tag)
  Punct(':', Alone)
  Punct('&', Alone)
  Punct(''', Joint)
  Ident(static)
  Ident(str)
  Punct(',', Alone)
```

几条从这份 dump 里直接读出来的事实，后面写解析时全都用得上：

- **关键字也是 `Ident`**：`struct`、`static`、`str` 都只是标识符，宏不替你做语法校验——你是不是真在写一个 struct，得你自己判断；
- **`{}` 里的内容是一个 `Group`**，不是「平铺的 token」。字段信息在 `Group::stream()` 里；
- **字段名后面那颗 `:` 的 spacing 是 `Alone`**，而 `std::string::String` 里两颗 `:` 的第一颗是 `Joint`。`Spacing` 的定义是 "Indicates whether a `Punct` token can join with the following token to form a multi-character operator."（引自 `proc_macro::Spacing` 文档）——`::` 的第一颗后面紧跟着另一颗标点，所以是 `Joint`；字段冒号后面跟的是类型，所以是 `Alone`。**这就是区分「字段名」与「路径」的抓手**；
- **`'static` 里的 `'` 是 `Joint`**（文档说明：单引号紧跟标识符时标 `Joint`），生命周期不是单独一种 token，而是 `'` + `Ident`；
- 大白话总结：**token 流是「词法层」的，不是「语法层」的**。想要 `Vec<&'static str>` 里的 `&`、`str` 变成一个「类型节点」，得自己搭。

> [!TIP]
> 这类探针是调试宏的最快路径：把输入 dump 出来，比你对着文档猜「我拿到的是啥」快得多。它也能救命——写解析时先看 dump，再写循环。

### 28.3 `Span`：报错定位的货币

每个 token 都带一个 `Span`，它就是「这段源码在哪个位置」的凭证，编译器的所有波浪线都要靠它。Reference 的两句话定义了它的边界："All tokens have an associated `Span`. A `Span` is an opaque value that cannot be modified but can be manufactured."（引自 Rust Reference）——**你改不了它的内容，但可以造一个新的、也可以把它从一个 token 搬到另一个 token 上**。

常用面（都以 1.98 stable 为准）：

| 想要的东西 | 写法 |
|---|---|
| 「调用点」的 span（错误指向 `#[derive(X)]` 那一行） | `Span::call_site()` |
| `macro_rules!` 式卫生：局部变量、标签、`$crate` 按定义点解析，其余按调用点 | `Span::mixed_site()` |
| 造一个标识符并指定归属 | `Ident::new("name", span)`；要原始标识符 `r#type` 时用 `Ident::new_raw` |
| 造字面量 | `Literal::string("…")`、`Literal::i64_unsuffixed(6)` |
| 改某个 token 的归属 | `token.set_span(span)`（`Ident`/`Punct`/`Literal`/`Group` 都有） |
| 读源码片段（调试用） | `span.source_text()` |

实践含义只有一句：**你生成的 token 落在谁的 span 上，用户看到的错误就指向哪里**。我们的 derive 用两种归属：结构的骨架（`impl`、方法名、`format!` 调用）用 `call_site`，出错时指向 `#[derive(Describe)]`；而 `compile_error!` 用的 span 是**用户输入里那颗出错 token 的 span**，于是错误波浪线精确地画在 `enum` 或字段上。两种做法都对，区别是「读者看到错误时该去找宏还是找自己的代码」。

> [!WARNING]
> 更细的 span 能力还在 nightly：`Span::join`（把两个 span 拼成一个）、`Span::{error, warning, note, help}` 与 `proc_macro::Diagnostic` 在 1.98 都标着 "nightly-only experimental API"。stable 上想让宏产生**警告**，只能绕道生成带 `#[deprecated]` 之类的属性，或者干脆用 `compile_error!` 报错。别在 stable 代码里依赖它们。

### 28.4 手写解析：一个最小 derive

目标：`#[derive(Describe)]` 给**具名字段结构体**生成一个 `impl`，让

```rust,ignore
#[derive(Describe)]
pub struct Point { pub x: i32, pub y: i32 }
```

拥有 `point.describe() == "Point { x = 1, y = 2 }"`。范围限定为具名字段结构体、不支持泛型——这是示例的边界，不是过程宏的边界。

trait 放在消费端 crate（`macro-demo-use`），宏只生成 `impl`：

```rust,ignore
{{#include ../examples/macro-demo-use/src/lib.rs:7:13}}
```

整个实现只用一个文件、零依赖。下面按四步走。

#### 第一步：把输入读成 token 表，再做校验

先把 token 流收进 `Vec`（后面要按下标前后看），然后跳过前缀的属性与可见性、确认关键字是 `struct`、读出结构体名、拒绝泛型、取出字段表所在的 `Group`：

```rust,ignore
{{#include ../examples/macro-demo/src/lib.rs:98:140}}
```

三处设计值得说明：

1. **`let Some(...) = ... else { return ... }`**：输入不合法时**返回**一段 `compile_error!`，而不是 panic。返回值是 `TokenStream`，所以错误和正常输出走同一条通路；
2. **错误都带 span**：`keyword.span()`、`p.span()`、`body.span()`——这样错误指向用户的源码，而不是宏本身；
3. **收窄范围要显式报错**：泛型分支直接说「本示例不支持泛型结构体」，比让后面的解析悄悄吐出一堆怪东西好得多。

用来生成错误的辅助函数就是一小段拼 token 的代码：

```rust,ignore
{{#include ../examples/macro-demo/src/lib.rs:56:65}}
```

它手写构造了 `compile_error!("…")` 的全部 token：`Ident("compile_error")` + `Punct('!')` + 一个 `{}` 分组，并把**整段调用的 span**（以及里面字面量的 span）设成用户那颗出错 token 的 span。这就是 28.7 里「错误落在哪里」的实现基础。

#### 第二步：挑出字段名

字段表里什么都有：属性、可见性、类型、逗号。但我们只要字段名。利用 28.2 的观察——**字段名后面紧跟一颗 `Alone` 的 `:`**——一段 `windows(2)` 滑窗就够了：

```rust,ignore
{{#include ../examples/macro-demo/src/lib.rs:191:203}}
```

`std::string::String` 里的 `::` 会被自动跳过：两颗 `:` 中只有第二颗是 `Alone`，而它前面是 `Punct` 不是 `Ident`。这类「靠 token 形状而不是靠语法树」的技巧，是没有 `syn` 时的家常菜——也是它脆的地方（练习 2 就在这一点上埋了个坑）。

#### 第三步：拼输出

输出是一棵手工搭起来的 token 树：`impl Describe for Point { fn describe(&self) -> ::std::string::String { ::std::format!("…", &self.x, &self.y) } }`。

```rust,ignore
{{#include ../examples/macro-demo/src/lib.rs:142:189}}
```

注意四件事：

- **`String` 写成了 `::std::string::String`**：不卫生的展开必须用绝对路径；
- **`describe` 用 `call_site`**：同名冲突时报错会指向 `#[derive(Describe)]` 那一行；
- **`token.name`（结构体名）沿用输入的 span**：万一这个 identifier 有问题，错误会指向用户写的 `struct Point`；
- **`format!` 的参数是 `&self.x` 而不是 `self.x`**：借用一下，字段不需要 `Copy`。

拼 token 是真的啰嗦：为了让上面这段能跑，文件里还躺着一批小工具（`ident`/`punct`/`joint`/`group`/`path`）。这正是 `quote` 要解决的问题（28.6）。

#### 第四步：`#[derive(Describe)]` 展开后等价于什么

宏的输出不好直接看（查看展开要 nightly 工具链，如 `rustc -Zunpretty=expanded` 或社区的 `cargo expand`），但它等价的手写代码是普通 Rust——下面这段可以直接编译、运行，`mdbook test` 会真的跑它：

```rust
pub trait Describe {
    fn describe(&self) -> String;
}

struct Point {
    x: i32,
    y: i32,
}

// `#[derive(Describe)]` 追加的 items 就是这一段
impl Describe for Point {
    fn describe(&self) -> ::std::string::String {
        ::std::format!("Point {{ x = {:?}, y = {:?} }}", &self.x, &self.y)
    }
}

fn main() {
    assert_eq!(Point { x: 1, y: 2 }.describe(), "Point { x = 1, y = 2 }");
}
```

对照着看，你能确认三件事：输出里没有重新定义 `Point`（derive 是追加）；用的是 `{:?}`，所以字段类型只需要 `Debug`；`impl` 里的 trait 名 `Describe` 在调用点解析——**所以调用点必须 `use` 到那个 trait**。

### 28.5 另外两种形态的实现要点

**属性宏 `#[repeat(N)]`**：把被标注的项复制 N 份、另起名字，原件保留。

```rust,ignore
{{#include ../examples/macro-demo/src/lib.rs:213:226}}
```

参数 `attr` 就是括号里那一段——`#[repeat(3)]` 里 `attr` 是 `Literal(3)`。可以拿探针实测（同一个 dump 宏加个属性版）：

```text
attr  = 3
item:
Ident(const)
Ident(SCALE)
Punct(':', Alone)
Ident(i32)
Punct('=', Alone)
Literal(2)
Punct(';', Alone)
```

`item` 是整条 `const SCALE: i32 = 2;`（注意分号也在里面）。复制与改名这一步是 `Ident::new` 的典型用法：

```rust,ignore
{{#include ../examples/macro-demo/src/lib.rs:251:265}}
```

**新标识符沿用原名 ident 的 span**（`id.span()`），所以 `SCALE_1` 上的任何错误都会指回源码里的 `SCALE`。最后 `output.extend(item)` 把原件吐回去——不做这一步，`SCALE` 本身就没有了。

**函数式宏 `sum!`**：编译期把一串整数字面量加起来。

```rust,ignore
{{#include ../examples/macro-demo/src/lib.rs:275:314}}
```

它展示了过程宏相对 `macro_rules!` 最大的不同：**宏体里可以真的跑代码**——这里是在编译期遍历 token、做整数加法、再把结果当字面量吐出来。`Literal::i64_unsuffixed(total)` 生成的是**无后缀**字面量，因此类型由上下文推断：`const TOTAL: i64 = sum!(10, 20, 30);` 与 `let x: i32 = sum!(1, 2);` 都成立。

### 28.6 syn 与 quote：什么时候值得引

上面 80 行解析 + 拼装，处理的是**一个被刻意缩小的语言子集**。真实世界的宏要面对：泛型参数与 `where` 子句、`enum` 的变体、联合体、元组结构体、原始标识符 `r#type`、属性、以及「调用点里套着别的宏调用」这种不可能靠 `windows(2)` 处理的输入。手写覆盖这些语法，等于自己写一个 Rust 解析器。

社区的标准答案是两个 crate：

| crate | 解决什么 | 关键点 |
|---|---|---|
| `syn` | **解析**：把 token 流变成结构化 AST（`DeriveInput`、`Data`、`Fields`…） | 覆盖稳定语法；feature 要按需开（`derive`、`full`、`parsing`、`extra-traits`）；编译时间会涨 |
| `quote` | **生成**：用 `#var` 插值、`#(…)*` 重复，代替手拼 token | 依赖 `proc-macro2`，因此**能在普通 crate 里写单元测试** |
| `proc-macro2` | 让 `syn`/`quote` 在 proc-macro crate 之外也能用 | 测试宏的解析逻辑不必启动编译器 |

同一个 derive，用 syn/quote 写出来是这样（`ignore`，需要这两个外部依赖）：

```rust,ignore
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Describe)]
pub fn derive_describe(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // 解析由 syn 完成：不合法的输入自带 span 与错误信息
    let Data::Struct(data) = &ast.data else {
        return syn::Error::new_spanned(name, "Describe 只支持 struct")
            .to_compile_error()
            .into();
    };
    let Fields::Named(fields) = &data.fields else {
        return syn::Error::new_spanned(name, "Describe 只支持具名字段结构体")
            .to_compile_error()
            .into();
    };

    let names: Vec<_> = fields.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
    let fmt = if names.is_empty() {
        name.to_string()
    } else {
        let labels = names
            .iter()
            .map(|n| format!("{n} = {{:?}}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{name} {{ {labels} }}")
    };

    // 生成由 quote 完成：`#name` 插值，`#( … ),*` 展开重复
    quote! {
        impl Describe for #name {
            fn describe(&self) -> ::std::string::String {
                ::std::format!(#fmt, #(&self.#names),*)
            }
        }
    }
    .into()
}
```

对比一下能力边界：

- **泛型结构体现在自动就能过**（`ast.generics` 里都是现成的），不需要我们再拒绝或特殊处理；
- **错误报告更整齐**：`syn::Error::new_spanned(节点, "…")` 直接把消息挂在某个语法节点上，`to_compile_error()` 出 token；
- **代价是依赖**：过程宏 crate 的依赖会被编译进宏的动态库，编译时间与供应链都变长。

选择标准其实很朴素：**输入语法在你能用 30 行手写覆盖的范围内**（比如只是读几个字面量、只处理一种结构体写法），手写解析更小、更快、依赖为零；**输入是「任意 Rust」**（derive 一个通用 trait、写属性宏改写任意函数体），用 syn——自己解析迟早会在边角语法上翻车。本章的示例刻意选了前者，为的是让你看见解析这一层本身；真写生产宏时，先想清楚你打算支持多少语法。

### 28.7 错误往哪里报

过程宏的结局只有三种——返回语法、panic、死循环（见前面）。但在「返回语法」这条路上，出错也有两条走法："Procedural macros have two ways of reporting errors. The first is to panic. The second is to emit a `compile_error` macro invocation."（引自 Rust Reference）

**panic 的实拍**（一个只会 panic 的 derive，被用在结构体上）：

```text
error: proc-macro derive panicked
 --> src\bin\panic_case.rs:3:10
  |
3 | #[derive(Boom)]
  |          ^^^^
  = help: message: 这个 derive 只会 panic
```

能用，但你只能给一句话，位置固定在 `#[derive(...)]` 上。**推荐路径是生成 `compile_error!`**：位置由 span 决定，消息可以带上下文，还能在同一个宏里报多处错误（每条 `compile_error!` 都会独立显示）。

span 归属带来的差别同样体现在「trait 不在作用域」这种错误上。展开结果里的 `Describe` 落在调用点，所以如果调用点只 `use` 到了宏、没 `use` 到 trait：

```text
error[E0404]: expected trait, found derive macro `Describe`
 --> src\bin\no_trait.rs:3:10
  |
3 | #[derive(Describe)]
  |          ^^^^^^^^ not a trait
  = note: this error originates in the derive macro `Describe` (in Nightly builds, run with -Z macro-backtrace for more info)
```

rustc 坦白了错误来自宏展开（`= note`），并且指出它在类型位置上找到的其实是**宏**。要修，就在调用点把 trait 也引进来——`use macro_demo_use::Describe;` 一条语句会同时引入宏（宏命名空间）和 trait（类型命名空间），这也正是 serde 系生态的用法。

> [!NOTE]
> 想让宏在**用户代码不 `use` trait** 的情况下也能编译，通用做法是让用户显式传给宏一个路径（`#[serde(crate = "…")]` 那种），或者约定一个固定别名（如 `use my_crate as _internal;`）。我们的示例选择了最直接的方案：要求调用点 `use` 到 trait，并且在文档里说清楚。

## 动手看看

两个 crate 都在 `examples/` 下，是同一个 workspace 的成员：

```text
examples/
├── macro-demo/          # proc-macro = true：三个宏与手写解析
│   └── src/lib.rs
└── macro-demo-use/      # 消费端：trait、示例结构体、测试
    └── src/lib.rs
```

消费端全貌（宏与 trait 一起 `use` 进来，然后只写 `#[derive(Describe)]`）：

```rust,ignore
{{#include ../examples/macro-demo-use/src/lib.rs:15:31}}
```

三个派生结构体分别覆盖：普通字段、空字段表、以及字段类型不是简单标量的情况。测试与调用（`#[repeat]`、`sum!` 都在这里）：

```rust,ignore
{{#include ../examples/macro-demo-use/src/lib.rs:33:84}}
```

真跑一遍（Rust 1.98、Windows 11；为省版面只留关键行）：

```console
$ cargo test --manifest-path examples/Cargo.toml -p macro-demo-use
   Compiling macro-demo v0.1.0 (D:\Rust\examples\macro-demo)
   Compiling macro-demo-use v0.1.0 (D:\Rust\examples\macro-demo-use)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.44s
     Running unittests src\lib.rs (target\debug\deps\macro_demo_use-….exe)

running 5 tests
test tests::describe_handles_empty_struct ... ok
test tests::describe_lists_fields_in_declaration_order ... ok
test tests::describe_works_behind_a_generic_bound ... ok
test tests::repeat_copies_the_item_and_keeps_the_original ... ok
test tests::sum_is_evaluated_during_expansion ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

这五个测试是本章所有断言的真凭实据：字段顺序、空结构体、`Debug` 只需字段实现、`repeat` 的「复制 + 保留原件」、`sum!` 的无后缀字面量推断。想自己加东西的话，改 `macro-demo/src/lib.rs` 再重跑同一条命令即可——宏的源码与消费端在同一棵树上，改完立刻生效。

## 报错现场

三个现场都用**临时探针 crate**复现（错误场景不能待在 `examples/` 里，那里要一直全绿）。探针只做一件事：引用 `macro-demo-use` 并故意写错。因此输出里的文件路径是探针 crate 自己的路径。

### 现场一：忘了 `proc-macro = true`

最经典的一步错过的配置。把 derive 写在普通 lib crate 里：

```rust,ignore
use proc_macro::TokenStream;

#[proc_macro_derive(Describe)]
pub fn derive_describe(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
```

以 Rust 1.98 为例：

```text
error: the `#[proc_macro_derive]` attribute is only usable with crates of the `proc-macro` crate type
 --> src\lib.rs:3:1
  |
3 | #[proc_macro_derive(Describe)]
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0432]: unresolved import `proc_macro`
 --> src\lib.rs:1:5
  |
1 | use proc_macro::TokenStream;
  |     ^^^^^^^^^^ use of unresolved module or unlinked crate `proc_macro`
  |
  = help: if you wanted to use a crate named `proc_macro`, use `cargo add proc_macro` to add it to your `Cargo.toml`
```

**定位路径**：第一条错误说的是 crate 类型，不是代码——去 `Cargo.toml` 加 `[lib] proc-macro = true`。第二条错误顺带暴露了 `proc_macro` 的特殊身份：它不是普通依赖，只有 proc-macro crate 才能 `use`；那个 `cargo add proc_macro` 的 help 是**误导**，照做没有用（crates.io 上没有这个包）。

### 现场二：把 derive 用在 `enum` 上

输入检查没做好的宏会在这里吐出一堆莫名其妙的解析错误；我们的实现选择自己报错。探针是 `src/bin/enum_case.rs`：

```rust,ignore
use macro_demo_use::Describe;

#[derive(Describe)]
enum Shape {
    Circle { radius: f64 },
}

fn main() {
    let _ = Shape::Circle { radius: 1.0 };
}
```

以 Rust 1.98 为例：

```text
error: Describe: 只支持 struct，这里是 `enum`
 --> src\bin\enum_case.rs:4:1
  |
4 | enum Shape {
  | ^^^^
```

**注意波浪线画在 `enum` 上**，不是 `#[derive(Describe)]` 那一行。这就是 28.4 里 `keyword.span()` 的效果：报错坐标由宏生成时的 span 决定。定位路径很直接——消息里已经写清支持什么；修法是把 derive 从枚举上摘掉，或者扩展宏让它支持枚举。

### 现场三：忘了把宏引进来

```rust,ignore
#[derive(Describe)] // 没有 use
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let point = Point { x: 1, y: 2 };
    println!("{}", point.describe());
}
```

以 Rust 1.98 为例（节选）：

```text
error: cannot find derive macro `Describe` in this scope
 --> src\bin\no_import.rs:1:10
  |
1 | #[derive(Describe)]
  |          ^^^^^^^^

error[E0599]: no method named `describe` found for struct `Point` in the current scope
 --> src\bin\no_import.rs:9:26
  |
2 | struct Point {
  | ------------ method `describe` not found for this struct
...
9 |     println!("{}", point.describe());
  |                          ^^^^^^^^ method not found in `Point`
  |
  = help: items from traits can only be used if the trait is implemented and in scope
  = note: the following trait defines an item `describe`, perhaps you need to implement it:
          candidate #1: `macro_demo_use::Describe`
```

**两条错误是同一个原因**：宏没 `use`，也就没生成 `impl`，于是方法也不存在。第二条里的 `candidate #1` 是最有用的线索——它直接告诉你要 `use` 的是哪个 trait。修法：`use macro_demo_use::Describe;`（这一条同时把宏和 trait 带进作用域）。

## 小结

- **过程宏是编译期执行的函数**：输入一段 token 流，输出一段 token 流；由 crate 的 `[lib] proc-macro = true` 声明，必须放在独立 crate 里，且定义它的 crate 不能使用它（所以 serde 拆成两个 crate）。
- **接口是 token 流而不是 AST**，为了长期稳定；代价是解析自己做。`TokenTree` 只有 `Ident`/`Punct`/`Literal`/`Group` 四类，关键字与生命周期也不是特殊形态。
- **三种形态的差别在输入与落点**：derive 读整个 item、输出**追加**在其后；属性宏读 `(属性, item)`、输出**替换**；函数式宏读括号内 token、输出**替换**整个调用。
- **`Span` 决定错误指向哪里**：骨架用 `Span::call_site()`，指向用户的错误则借用用户 token 的 span。`set_span` 可以把归属搬到任何 token 上。
- **错误用 `compile_error!` 报，不要 panic**；panic 会变成 `proc-macro derive panicked`，只能给一句话。
- **不卫生**：生成的代码要用绝对路径（`::std::string::String`），辅助名字要不容易撞。
- **syn/quote 是解析与生成的成熟方案**：语法面窄、能手写覆盖时零依赖更省；输入是「任意 Rust」时引它们——代价是依赖与编译时间。
- 1.98 stable 上**没有**：`proc_macro::Diagnostic`、`Span::{error,warning,note,help}`、`Span::join`、`proc_macro::quote!` 宏（都还是 nightly-only）。

## 练习

1. `macro-demo` 里有 `pub trait Describe`，你会得到一条错误。这条错误说的是什么？为什么 `Describe` trait 必须住在 `macro-demo-use` 里，而不是和宏放一起？
2. 下面的 `field_names` 版本少了 spacing 判断。指出什么输入会出错、错在哪、以及会报出什么样的错误；然后写出修法。

   ```rust,ignore
   fn field_names(body: TokenStream) -> Vec<Ident> {
       let tokens: Vec<TokenTree> = body.into_iter().collect();
       let mut names = Vec::new();
       for pair in tokens.windows(2) {
           if let [TokenTree::Ident(name), TokenTree::Punct(colon)] = pair {
               if colon.as_char() == ':' {
                   names.push(name.clone());
               }
           }
       }
       names
   }
   ```

3. 如果要把 `#[derive(Describe)]` 扩展成支持泛型结构体（`struct Wrap<T> { value: T, count: usize }`），最少要改哪些地方？各自会遇到什么新问题？

<details><summary>参考答案</summary>

**1.** 错误是：

```text
error: `proc-macro` crate types currently cannot export any items other than functions tagged with `#[proc_macro]`, `#[proc_macro_derive]`, or `#[proc_macro_attribute]`
```

原因在过程宏的运行方式：它是被编译器**加载进自己进程**执行的插件（编译 `macro-demo-use` 时先编译 `macro-demo` 成动态库）。插件能对外暴露的只有宏。所以「宏 + trait」拆成两个 crate 不是风格选择，而是硬约束——serde/serde_derive 就是这么来的。

**2.** 出错的输入是**任何字段类型里带 `::` 路径的写法**，例如：

```rust,ignore
#[derive(Describe)]
struct Config {
    value: std::string::String,
}
```

少了 spacing 判断后，`(Ident(std), Punct(':'))` 和 `(Ident(string), Punct(':'))` 这两对都会被当成「字段名」，于是 `field_names` 返回 `["value", "std", "string"]`；宏生成的 `format!` 参数里出现 `&self.std`、`&self.string`。实测（Rust 1.98）报**两条**错：

```text
error[E0609]: no field `std` on type `&Config`
 --> src\main.rs:9:16
9 |     pub value: std::string::String,
  |                ^^^ unknown field
  = note: available field is: `value`

error[E0609]: no field `string` on type `&Config`
 --> src\main.rs:9:21
9 |     pub value: std::string::String,
  |                     ^^^^^^ unknown field
  = note: available field is: `value`
```

三点值得留意：类型打出来是 `&Config`（生成的参数是 `&self.std`）；两条错误对应两个被误认的「字段名」；波浪线落在**源码里的 `std`、`string` 上**，不是 `#[derive(Describe)]` 那一行——因为 `field_names` 把输入的 `Ident` 原样交给了生成的 `&self.<字段>`，span 沿用输入 token（28.3 讲的正是这件事）。那条 note 里的 `available field is` 是最有用的线索。想让这类错误统一落到派生宏上，拼参数时换成调用点归属即可：`Ident::new(&field.to_string(), Span::call_site())`（或对克隆出来的 token 调 `set_span`）。

修法就是补上判断，只认「孤立」的冒号：

```rust,ignore
if colon.as_char() == ':' && colon.spacing() == Spacing::Alone {
    names.push(name.clone());
}
```

道理在 28.2：`::` 的第一颗 `:` 是 `Joint`，字段冒号是 `Alone`。

**3.** 至少四处：

- **捕获泛型参数**：现在遇到 `<` 直接报错，要改成把 `<T>`（含 `where` 子句）原样收集起来，既用于 `impl` 也用于类型名；
- **生成带参数的 impl 头**：`impl<T> Describe for Wrap<T>` —— 泛型参数与约束要在 `impl` 的 `<…>` 里原样重放，结构体名后面接同名参数列表（`Wrap<T>` 而不是 `Wrap`）；
- **字段类型的 `Debug` 约束**：生成代码用的是 `{:?}`，所以每个泛型参数都得能 `Debug`；手写版要自己给 `T: ::std::fmt::Debug`，syn 版可以从字段类型反推（或者干脆要求用户标注）；
- **解析难度上升**：`<…>` 里的逗号不再是「字段分隔符」，需要配对计数（`>>` 是两个 `>`、`where` 子句里有 `=` 和逗号）——这正是 28.6 说的「语法面变宽就该换 syn」的信号。

</details>

## 延伸阅读

- Rust Reference：[Procedural Macros](https://doc.rust-lang.org/reference/procedural-macros.html)——三种形态的语法、位置限制、helper attribute 的权威定义；
- 标准库：[`std::proc_macro`](https://doc.rust-lang.org/std/proc_macro/index.html)——`TokenStream`/`TokenTree`/`Span` 的 API 与稳定性标注；
- The Book：[Macros](https://doc.rust-lang.org/book/ch20-05-macros.html)——声明宏与过程宏的入门对照；
- `syn` 文档：[docs.rs/syn](https://docs.rs/syn)——`DeriveInput`/`Data`/`Parse` 的用法；
- `quote` 文档：[docs.rs/quote](https://docs.rs/quote)——`#var` 插值与 `#(…)*` 重复的规则；
- 练习仓库：[dtolnay/proc-macro-workshop](https://github.com/dtolnay/proc-macro-workshop)——从 `#[derive(Builder)]` 到 `#[sorted]` 的分级练习，写真实宏前值得过一遍。
