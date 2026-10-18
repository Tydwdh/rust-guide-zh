# 第 27 章 宏：macro_rules!

## 这一章解决什么问题

带感叹号的调用在 Rust 里无处不在：`println!`、`vec!`、`assert_eq!`、`format!`，连 `#[derive(Debug)]` 也是宏。它们都属于**宏（macro）**——在编译期把一段 token 序列翻译成代码的工具。函数有三件事做不到：参数个数不固定、参数是「代码的形状」而不是值、能生成项（函数、结构体、trait 实现）。

本章只讲**声明宏（`macro_rules!`）**，官方叫法是「宏按例（macros by example）」。读完你应当能：

- 写出多臂的 `macro_rules!`，知道哪条臂会被选中、为什么；
- 在十几个片段说明符（fragment specifier）里挑对那一个，知道 follow set 限制从哪来；
- 说清 Edition 2024 对片段匹配做了什么——以及 `pat` 的变化其实发生在 2021；
- 预测展开里某个名字会被解析到定义处还是调用处（卫生性）；
- 用 `$crate` + `#[macro_export]` 把宏交给别的 crate，并知道 `use` 与可见性的边界；
- 写递归宏（TT muncher），知道深度上限在哪。

**边界先划清**：过程宏（derive、属性宏、函数式宏）在第 28 章；`cfg` 条件编译与构建脚本在第 30 章；`macro_metavar_expr` 这类 nightly 特性在第 39 章；模式规则在第 5 章。

## 为什么这样设计

Reference 的开场定义很干脆：

> `macro_rules` allows users to define syntax extension in a declarative way. We call such extensions “macros by example” or simply “macros”.
>
> （引自 Rust Reference）

关键词是 **declarative**：你写出「调用长什么样」和「它该变成什么」，而不是一个操作 token 的函数。这正是宏与函数的分工差别：

| | 函数 | 宏 |
|---|---|---|
| 参数个数 | 签名固定 | 任意 |
| 参数是什么 | 值 | 代码 |
| 何时生效 | 调用时 | 编译时展开 |
| 能做什么 | 算数据 | 生成项、实现 trait |

代价同样真实：宏不受签名检查，报错常指向调用点而不是宏内部；定义也比函数难读。**能用函数、泛型或 `impl Trait` 表达的，就别写宏。**

心智模型只有一句话：**声明宏 = 匹配器（matcher）+ 转写器（transcriber）**。

> Each macro by example has a name, and one or more *rules*. Each rule has two parts: a *matcher*, describing the syntax that it matches, and a *transcriber*, describing the syntax that will replace a successfully matched invocation.
>
> （引自 Rust Reference）

调用点的 token 序列按臂逐个匹配，选中**第一个**成功的臂，再转写：元变量换成匹配到的片段，得到新序列交给后续阶段（名字解析、类型检查、单态化）。**展开发生在名字解析之前**——所以宏能定义类型、实现 trait。

两个设计目标解释了几乎全部「怪癖」：

1. **展开必须终止、无歧义。** 于是有了：按顺序尝试、不回溯、不做 lookahead、follow set 白名单、递归上限 128——为的是让「读一遍调用就知道匹配哪条臂」永远成立，也给未来语法留余地。
2. **宏要能在不认识的代码里安全工作。** 卫生性让宏内部的局部名字不打扰调用处；`$crate` 让宏引用自家 crate 的辅助项时不受调用处作用域影响。

卫生性为什么是「混合」的？Reference 的原话：

> Macros by example have *mixed-site hygiene*. This means that loop labels, block labels, and local variables are looked up at the macro definition site while other symbols are looked up at the macro invocation site.
>
> （引自 Rust Reference）

局部变量和标签按**定义处**解析（宏里的 `let tmp = ...` 不污染调用处）；函数、类型、生命周期按**调用处**解析（`vec!` 展开用的是调用处的 `Vec`）。两边各有理由，合起来就是「混合」。

最后纠两个常见误解：

- **宏不是文本替换。** 它处理 token 树，替换后还带着卫生信息；把展开打印成文本是有损的：`Be aware that macro expansion to text is a lossy process.`（引自 cargo-expand README）
- **宏不是「更快的函数」。** 展开在类型检查之前，没有签名可对照，诊断通常更差。

## 机制与规则

### 27.1 匹配臂与展开

语法形态记住三件事：`macro_rules!` 后跟宏名（**不带**感叹号）；每条臂是「匹配器 `=>` 转写器」；两者各自被一对定界符包住：

```rust
macro_rules! double {
    ($e:expr) => {
        $e * 2
    };
}

fn main() {
    assert_eq!(double!(21), 42);
}
```

匹配器里，不属于 `$` 特殊用法的 token 都是**字面匹配**：`(zero)` 只匹配恰好写了 `zero` 的调用。元变量写作 `$名字:片段说明符`，在转写器里直接用 `$名字` 引用；一个元变量可以转写多次，也可以一次都不用（引自 Rust Reference）。

决定「哪条臂被选中」的几条：

- **臂按书写顺序尝试，第一处匹配胜出。** 兜底臂（`($($tt:tt)*)`）永远放最后。
- **不做回溯。** `It transcribes the first successful match; if this results in an error, then future matches are not tried.`（引自 Rust Reference）——匹配成功但转写结果有语法错误时，后面的臂不会有机会。
- **匹配不做 lookahead。** `When matching, no lookahead is performed; if the compiler cannot unambiguously determine how to parse the macro invocation one token at a time, then it is an error.`（引自 Rust Reference）——`($($i:ident)* $j:ident)` 这种「读到 ident 后还要看下一个 token 才知道归谁」的匹配器会直接报局部歧义（见「报错现场」）。
- **匹配器外层定界符可以是三对中任意一对。** `(())` 能匹配 `{()}`，匹配不了 `{{}}`；`$` 无法被字面匹配或字面转写。
- `ident` 片段能匹配**关键字**（`_` 除外），所以 `kind!(fn)` 合法。

下面这个例子把前几条摆在一起（`snippets/ch27/arms.rs`）：

```rust
{{#rustdoc_include ../snippets/ch27/arms.rs}}
```

```console
1: 字面 token zero
2: 字面 token one
3: 表达式 1 + 2
4: 其余 token "a b c"
A: expr 臂赢
parens: 匹配了
parens: 匹配了
ident 匹配: fn
ident 匹配: struct
```

三处细节：`shape!(1 + 2)` 落到第 3 条臂，`shape!(a b c)` 落到兜底的第 4 条——第 3 条臂在 `a` 之后发现还有 token，匹配失败但不影响后面的臂继续尝试。`first!(1 + 1)` 打印 `A`：写在前面的 `expr` 臂先赢，尽管后面那条臂「看起来更精确」。`parens![()]` 与 `parens! {()}` 都能匹配同一条 `(())` 臂。

### 27.2 元变量与片段说明符

片段说明符决定「这个位置能吃什么语法」。清单如下（引自 Rust Reference，中文为译述）：

| 说明符 | 接受的东西 |
|---|---|
| `block` | 块表达式（不收内部属性） |
| `expr` | 一个表达式 |
| `expr_2021` | 表达式，但不含 `_` 与 `const` 块（见 27.4） |
| `ident` | 标识符或关键字（不含 `_`、raw、`$crate`） |
| `item` | 一个项 |
| `lifetime` | 生命周期 token（`'a`） |
| `literal` | 字面量，可带负号（`-7`） |
| `meta` | 属性，或属性的内容 |
| `pat` | 模式（2021 起含顶层或模式，见 27.4） |
| `pat_param` | 不含顶层或模式的模式 |
| `path` | 类型路径 |
| `stmt` | 语句，不含尾分号（需分号的 item 语句除外） |
| `tt` | 一棵 token 树 |
| `ty` | 一个类型 |
| `vis` | 可见性限定符（可为空） |

用起来大致是这样（`snippets/ch27/specifiers.rs`）：

```rust
{{#rustdoc_include ../snippets/ch27/specifiers.rs}}
```

```console
add(1, 2) = 3
字面 token 42
literal 片段: -7
path 片段: std::mem::size_of
meta 片段: derive(Debug, Clone)
vis 片段: "pub(crate)"
stmt 片段: let x = 1
tt 片段: a
tt 片段: [1, 2, 3]
lifetime 片段: 'static
pat 片段: Some(x) | None
map! 造出 2 条记录，ra = 90
```

选择原则按优先级排：

1. **能不解析就别解析：优先 `tt`。** 它接受面最大，也不限制后面的 token；要自己写解析（TT muncher，见 27.8）只能靠它。
2. **要让编译器替你把关，再用具体说明符。** 希望那里出现合法表达式就写 `expr`；`def_fn!` 里的 `$ty:ty`、`$body:block` 同理——写错时调用点就有明确报错。
3. **接受面小的别选大的。** 名字用 `ident` 而不是 `expr`（后者会连 `a + b` 一起吃下去）；属性用 `meta` 而不是 `tt`。
4. **转发片段时，第二个宏只能用同类型说明符（或 `tt`/`ident`/`lifetime`）。** 最容易踩：

> When forwarding a matched fragment to another macro-by-example, matchers in the second macro will see an opaque AST of the fragment type. The second macro can't use literal tokens to match the fragments in the matcher, only a fragment specifier of the same type. The `ident`, `lifetime`, and `tt` fragment types are an exception, and *can* be matched by literal tokens.
>
> （引自 Rust Reference）

实测报错会直接把原因写出来（以 Rust 1.98 为例）：

```rust,compile_fail
macro_rules! foo {
    ($l:expr) => { bar!($l); };
}
macro_rules! bar {
    (3) => {};
}
fn main() {
    foo!(3);
}
```

```text
error: no rules expected `expr` metavariable
 --> src/main.rs:2:25
  |
2 |     ($l:expr) => { bar!($l); };
  |                         ^^ no rules expected this token in macro call
3 | }
4 | macro_rules! bar {
  | ---------------- when calling this macro
...
8 |     foo!(3);
  |     ------- in this macro invocation
  |
note: while trying to match `3`
 --> src/main.rs:5:6
  |
5 |     (3) => {};
  |      ^
  = note: captured metavariables except for `:tt`, `:ident` and `:lifetime` cannot be compared to other tokens
  = note: see <https://doc.rust-lang.org/nightly/reference/macros-by-example.html#forwarding-a-matched-fragment> for more information
  = help: try using `:tt` instead in the macro definition
```

`$l` 里明明写的是 `3`，`bar!(3)` 却匹配不上：`$l:expr` 装的是**不透明的表达式 AST**，不是三个 token。把 `foo` 的说明符换成 `tt`，或让 `bar` 也用 `expr` 接住，就好了。

谁跟谁配对也有规则：片段后面能跟哪些 token 是一份**白名单**，为的是防止「当前或未来版本的语法出现歧义」（译自 Rust Reference）。名单如下（引自 Rust Reference）：

- `expr`、`stmt`：后随 `=>`、`,`、`;`。
- `pat_param`：后随 `=>`、`,`、`=`、`|`、`if`、`in`。
- `pat`：后随 `=>`、`,`、`=`、`if`、`in`（2021 起**不含** `|`，见 27.4）。
- `path`、`ty`：后随 `=>`、`,`、`=`、`|`、`;`、`:`、`>`、`>>`、`[`、`{`、`as`、`where`，或 `block` 元变量。
- `vis`：后随 `,`、非 `priv` 的标识符、任何能开始类型的 token，或 `ident`/`ty`/`path` 元变量。
- 其余说明符没有限制。

为什么要有白名单？Reference 的例子很好懂：`$i:expr [,]` 今天可以无歧义解析，但 **`[` 本身能开始一个尾表达式**，将来若允许 `[,]` 作为表达式的一部分，这个匹配器就会变歧义、把已工作的代码弄坏。

标准库也在这条边界上：`matches!` 用单个 `$pattern:pat` 接住；`assert_matches!` 要拆开 `|` 连接的分支，用的是 `$( $pattern:pat_param )|+`（源码在 `library/core/src/macros/mod.rs`）。

### 27.3 重复

`vec!`、`map!` 这类宏的通用性来自重复（repetition）：把要重复的部分包进 `$( ... )`，后面跟一个重复操作符，可选中一个分隔符。规则（引自 Rust Reference）：

- `*` 表示任意次（可以为 0），`+` 表示至少一次，`?` 表示 0 或 1 次；
- 分隔符是除定界符与重复操作符以外的任意 token，最常见的是 `,` 和 `;`；
- `?` 最多匹配一次，**不能带分隔符**；「可选尾逗号」写成 `$(,)?`；重复可以嵌套。

转写一侧的约束更硬，Reference 给了两条（引自 Rust Reference）：

1. 元变量在转写器里出现的**次数、种类与嵌套层级**必须和匹配器一致：`$( $i:ident ),*` 对应 `=> { $( $i );* }` 合法，`=> { $i }`、`=> { $( $( $i )* )* }`、`=> { $( $i )+ }` 都不合法。
2. 每个重复至少要含一个元变量（否则不知道展开几次）；同一重复里若有多个元变量，它们绑定的片段数量必须相同，嵌套的每一层都如此。

第二条的实际含义：`( $( $i:ident ),* ; $( $j:ident ),* ) => ( $( ($i, $j) ),* )` 要求两边数量一致，`(a, b, c; d, e, f)` 合法、`(a, b, c; d, e)` 报错。

### 27.4 Edition 2024 的片段匹配变化

本节结论都对着 Edition Guide 逐条核对过。Edition 按 crate 生效，片段匹配用的是**定义宏那个 crate 的 edition**，调用方的 edition 不影响（引自 The Rust Edition Guide）。

**2024 版：`expr` 现在也匹配 `const` 块与 `_` 表达式。** Edition Guide 的措辞是：`The expr fragment specifier now also supports const and _ expressions.`；之所以等到新 edition：`const` 块（1.79 稳定）与 `_` 表达式（1.59 稳定）若当时就加进 `expr`，可能改变已有宏的选臂结果——那是破坏性变更；旧行为由 `expr_2021` 保留。

同一份宏在三个 edition 下编译运行（实测，Rust 1.98），差别只在 2024：

| 调用 | 2018 | 2021 | 2024 |
|---|---|---|---|
| `which!(const { 1 + 1 })` | const 臂 | const 臂 | **expr 臂** |
| `under!(_)` | 下划线臂 | 下划线臂 | **expr 臂** |
| `which2021!(const { 1 + 1 })` | const 臂 | const 臂 | const 臂 |

（`which!` 的臂是 `($e:expr)` 在前、`(const $e:expr)` 在后；`under!` 是 `($e:expr)` 在前、`(_)` 在后；`which2021!` 的第一条臂用 `$e:expr_2021`。）

只有 2024 那两行变了。Reference 的注记补了一句：`Before the 2024 edition, expr fragment specifiers do not match UnderscoreExpression or ConstBlockExpression at the top level. They are allowed within subexpressions.`——注意「顶层」：子表达式一直能被 `expr` 吞下，变的只是它能不能整体作为一个 `expr` 片段。

完整代码在 `snippets/ch27/edition2024.rs`，2024 版下的输出（实测）：

```rust
{{#rustdoc_include ../snippets/ch27/edition2024.rs}}
```

```console
which!(const { 1 + 1 })    -> expr 臂
under!(_)                    -> expr 臂
which2021!(const { 1 + 1 }) -> const 臂
or_pat!(Some(x) | None)      -> pat 片段接住: Some(x) | None
any_of!(3, 1 | 2 | 3)        -> true
any_of!(9, 1 | 2 | 3)        -> false
```

迁移：`cargo fix --edition` 会跑 `edition_2024_expr_fragment_specifier` lint，把 `expr` 一律改成 `expr_2021`，但 Edition Guide 提醒：`In most cases, you will likely want to keep the expr specifier instead, in order to support the new expressions.`——lint 是保守的，你得自己确认换成新行为会不会让别的臂失去机会。

**`pat` 的变化发生在 2021，不是 2024。** Edition Guide 的 2021 条目很清楚：`$_:pat` 从 2021 起匹配顶层或模式（`A | B`），新加的 `$_:pat_param` 与旧的 `pat` 行为一致，所有 edition 都可用。Reference 补了限制：`pat` 在 2021 及以后**不能被显式 `|` 紧跟**，所以「用 `|` 连接多个 `pat` 分支」必须换成 `pat_param`。实测（Rust 1.98）：

```text
error: no rules expected `|`
 --> src/main.rs:7:36
  |
1 | macro_rules! or_pat {
  | ------------------- when calling this macro
...
7 |     println!("{}", or_pat!(Some(x) | None));
  |                                    ^ no rules expected this token in macro call
  |
note: while trying to match meta-variable `$p:pat`
 --> src/main.rs:2:6
  |
2 |     ($p:pat) => {
  |      ^^^^^^

error: aborting due to 1 previous error
```

2018 下这条臂只吃到 `Some(x)`，随后撞上不认识的 `|`；同一份源码在 2021/2024 下运行打印 `Some(x) | None`（实测）。

而 `($($p:pat)|+)` 这种老写法在 2021/2024 里会直接报错，编译器还顺手给出改法（以 Rust 1.98 为例）：

```rust,compile_fail
macro_rules! alts {
    ($($p:pat)|+) => { stringify!($($p)|+) };
}
fn main() {
    println!("{}", alts!(1 | 2 | 3));
}
```

```text
error: `$p:pat` is followed by `|`, which is not allowed for `pat` fragments
 --> src/main.rs:2:15
  |
2 |     ($($p:pat)|+) => { stringify!($($p)|+) };
  |        ------ ^ not allowed after `pat` fragments
  |        |
  |        help: try a `pat_param` fragment specifier instead: `$p:pat_param`
  |
  = note: allowed there are: `=>`, `,`, `=`, `if`, `if let` or `in`

error: aborting due to 1 previous error
```

**`stmt` 没有 edition 变化。** 2024 的 Edition Guide 里与声明宏直接相关的页面只有两个（`macro-fragment-specifiers`、`missing-macro-fragment-specifiers`），2021 版只有一个 `or-patterns-macro-rules`。把下面这个文件（宏定义 + 调用）分别用 2018/2021/2024 编译运行，输出一字不差（实测）：

```rust
macro_rules! show {
    ($s:stmt) => { println!("stmt: [{}]", stringify!($s)) };
}

fn main() {
    show!(let x = 1);
    show!(if true { 1 } else { 2 });
    show!(struct S;);
}
```

```text
stmt: [let x = 1]
stmt: [if true { 1 } else { 2 }]
stmt: [struct S;]
```

`stmt` 的「怪」不在 edition，而在定义：它匹配一条**不含尾分号**的语句（需分号的 item 语句除外），如 `show!(let x = 1)` 接住 `let x = 1`、`show!(struct S;)` 连分号一起接住。

**顺带一条**：`missing_fragment_specifier` 从 lint 变成了硬错误——原本只对 2024 edition 报错，但 `In Rust 1.89, released after Rust 2024, the lint was made into a hard error in all editions.` 本机 1.98 在 2021 与 2024 下都是硬错误：

```rust,compile_fail
macro_rules! foo {
    () => {};
    ($name) => {};
}
fn main() {
    foo!();
}
```

```text
error: missing fragment specifier
 --> src/main.rs:3:6
  |
3 |     ($name) => {};
  |      ^^^^^
  |
  = note: fragment specifiers must be provided
  = help: valid fragment specifiers are `ident`, `block`, `stmt`, `expr`, `pat`, `ty`, `lifetime`, `literal`, `path`, `meta`, `tt`, `item` and `vis`, along with `expr_2021` and `pat_param` for edition compatibility
help: try adding a specifier here
  |
3 |     ($name:spec) => {};
  |           +++++

error: aborting due to 1 previous error
```

`$name` 后面漏了 `:片段说明符`，这条臂就落不到任何输入上；修法是把没用的臂删掉，或补上说明符。

### 27.5 卫生性

这一节讲名字解析：局部变量、循环标签、块标签按**定义处**，函数、类型、生命周期、常量按**调用处**。Reference 还补了一条：`Labels and local variables defined in macro expansion are not shared between invocations`——同一宏的两次展开，局部变量互不可见。

完整文件 `snippets/ch27/hygiene.rs`：

```rust
{{#rustdoc_include ../snippets/ch27/hygiene.rs}}
```

```console
check!() = 调用处的 func
调用块里的 x = 2
宏里的 x = 100
调用处的 x = 1
标签卫生：展开内部的 'inner 对外不可见
first_x!() = 1，调用处的 x = 42
make!() 造出调用处的类型: hygiene::main::MyType
passthrough 走了一遍调用处的 'a: 7
```

四组对照，逐个看：

**局部变量取定义处。** `assert_eq!(x, 1)` 拿到定义处的 `x`（值为 1），尽管调用块里遮蔽出了 `x = 2`；同一段展开里的 `func()` 却调到了**调用处**的 `func`（定义处那个会 `unreachable!()`，没被调用）。`first_x!()` 同理：宏定义后又 `let x = 42;`，宏取的仍是 `x = 1`。

**宏里的绑定不外泄，也不打架。** `decl_x!()` 写了 `let x = 100;`，调用处的 `x` 还是 `1`；反过来，宏**引入**的名字在展开之外看不见：

```rust,compile_fail
macro_rules! m {
    (define) => { let x = 1; };
    (refer)  => { dbg!(x); };
}
fn main() {
    m!(define);
    m!(refer);
}
```

```text
error[E0425]: cannot find value `x` in this scope
 --> src/main.rs:3:24
  |
3 |     (refer)  => { dbg!(x); };
  |                        ^ not found in this scope
  |
help: an identifier with the same name is defined here, but is not accessible due to macro hygiene
 --> src/main.rs:2:23
  |
2 |     (define) => { let x = 1; };
  |                       ^
```

help 说得很清楚：同名标识符**存在**，只是被卫生性挡住。真按文本替换，上面这段会打印 `1`。

**标签同样按定义处解析。** 展开里的 `'inner` 循环自洽，外部看不见它；反方向直接报错：

```rust,compile_fail
macro_rules! leave {
    () => {
        break 'outer;
    };
}
fn main() {
    'outer: loop {
        leave!();
    }
}
```

```text
error[E0426]: use of undeclared label `'outer`
 --> src/main.rs:3:15
  |
3 |         break 'outer;
  |               ^^^^^^ undeclared label `'outer`
...
8 |         leave!();
  |         -------- in this macro invocation
```

调用处写了 `'outer: loop`，但宏里的 `break 'outer` 解析不到它——标签是定义处的。

**类型与生命周期按调用处解析。** `make!()` 的展开式里写 `MyType`（只在调用处定义），编译通过并打印 `hygiene::main::MyType`；展开式里也能直接写出调用处签名上的 `'a`。卫生性**不覆盖类型名和生命周期名**——它们按展开后的位置解析，这也是宏能生成项、实现 trait 的前提。

把这几条合起来记：

| 名字种类 | 解析位置 | 实用含义 |
|---|---|---|
| 局部变量、标签 | 定义处 | 宏内临时变量安全；取定义时可见的绑定 |
| 函数、类型、常量、模块路径 | 调用处 | 宏能引用调用方类型；宏里的名字必须在调用处存在（`$crate` 解决） |
| 生命周期 | 调用处 | 展开式可以写调用处的生命周期参数 |

### 27.6 `$crate` 与跨 crate 展开

卫生性保护了宏内部的名字，但**宏引用自家 crate 的辅助项**时有反向问题：展开结果在调用处解析，而调用处一般不 `use` 你的辅助宏。

修法是 `$crate`（`snippets/ch27/dollar_crate.rs`）：

```rust
{{#rustdoc_include ../snippets/ch27/dollar_crate.rs}}
```

```console
直接调用 foo()     -> 调用处的 foo
call_foo!()        -> crate 里的 foo
crate::call_foo!() -> crate 里的 foo
```

调用处也有 `foo`，但宏里的 `$crate::inner::foo()` 钉在**定义宏的 crate** 上，取到的一直是 `crate 里的 foo`。Reference 的定义是：

> A special case is the `$crate` metavariable. It refers to the crate defining the macro, and can be used at the start of the path to look up items or macros which are not in scope at the invocation site.
>
> （引自 Rust Reference）

两条配套规则必须记住：

- **提到非宏项时，`$crate` 后面要写全模块路径**：`Note that, because $crate refers to the current crate, it must be used with a fully qualified module path when referring to non-macro items`（引自 Rust Reference）。
- **`$crate` 不绕过可见性**（引自 Rust Reference）：`its use has no effect on visibility. An item or macro referred to must still be visible from the invocation site.`——私有辅助函数被外部 crate 的宏调用，一样报「私有」。

第二条有实测：两个 crate 走一遍（`helper` 是库，`user` 是使用者）：

```console
$ rustc --edition 2024 --crate-type=lib --crate-name helper helper.rs -o libhelper.rlib
$ rustc --edition 2024 --extern helper=libhelper.rlib -L . -o user.exe user.rs
$ .\user.exe
helper 宏
```

（编译 `helper.rs` 会多一条 `priv_fn` 未使用的 dead_code 警告，与本节无关，已略去。）

其中 `helper.rs` 里的关键部分是：

```rust
#[macro_export]
macro_rules! helper {
    () => { "helper 宏" };
}

// 用 $crate 把内部辅助宏钉在定义 crate 上
#[macro_export]
macro_rules! helped {
    () => { $crate::helper!() };
}

// 不用 $crate：展开式里的 helper! 需要在调用处可见
#[macro_export]
macro_rules! helped_nocrate {
    () => { helper!() };
}

// $crate 不绕过可见性
#[macro_export]
macro_rules! call_priv {
    () => { $crate::priv_fn() };
}

fn priv_fn() -> &'static str { "私有函数" }
```

使用者只 `use helper::helped;`，没导入 `helper!`，调用照样成功；换成 `helped_nocrate!` 就崩了（以下两处是两 crate 探针的关键行节选，已略去 `-->` 定位行）：

```text
error: cannot find macro `helper` in this scope
  |
4 |     println!("{}", helped_nocrate!());
  |                    ^^^^^^^^^^^^^^^^^
  |
  = note: `helper` is in scope, but it is a crate, not a macro
  = note: this error originates in the macro `helped_nocrate`
```

最后那句 note 很典型：调用处确实有 `helper` 这个名字——它是 crate 本身。用 `call_priv!` 调私有函数则报可见性错误：

```text
error[E0603]: function `priv_fn` is private
  |
4 |     println!("{}", call_priv!());
  |                    ^^^^^^^^^^^^ private function
```

还有一个时代产物要认识但**不要用**：`#[macro_export(local_inner_macros)]` 会给宏定义里所有单段宏调用自动加 `$crate::` 前缀（Reference 原话：`Adding local_inner_macros to the macro_export attribute causes all single-segment macro invocations in the macro definition to have an implicit $crate:: prefix.`），而 Reference 对它的定位是：`Its use is discouraged in new code.`——它是 `$crate`（Rust 1.30）之前的老代码迁移工具，新代码一律手写 `$crate::`。

### 27.7 `#[macro_export]` 与作用域

宏的作用域和普通项不一样，这是历史原因：

> For historical reasons, the scoping of macros by example does not work entirely like items. Macros have two forms of scope: textual scope, and path-based scope.
>
> （引自 Rust Reference）

**文本作用域**按源码顺序生效：`macro_rules!` 定义之后（到所在模块结束）才能按名字调用，可以递归（名字在调用点查找）。它能进入子模块、跨文件，但**不能**从模块内向外扩散，除非给模块加 `#[macro_use]`；宏也能定义在函数体里，作用范围是该函数体剩余部分。定义之前调用会报：

```text
error: cannot find macro `m` in this scope
 --> src/main.rs:2:5
  |
2 |     m!();
  |     ^ consider moving the definition of `m` before this call
  |
note: a macro with the same name exists, but it appears later
 --> src/main.rs:5:14
  |
5 | macro_rules! m {
  |              ^
```

**路径作用域**和普通项一样按路径解析，来源有二：`use` 重导出，以及 `#[macro_export]`。后者的效果是：

> The `macro_export` attribute exports the macro from the crate and makes it available in the root of the crate for path-based resolution.
>
> （引自 Rust Reference）

宏定义在哪个模块不重要：`#[macro_export]` 之后它的**路径位置**是 crate 根（所以 `crate::宏名!()` 生效，`crate::定义它的模块::宏名!()` 不生效）。没导出的宏没有路径作用域，`self::m!()`、`crate::m!()` 都会报 `cannot find m in self/crate`。

查找顺序（引自 Rust Reference）：**未限定**的调用先查文本作用域，查不到再查路径作用域；**带路径**的调用只查路径作用域。而且 `Textual scope name bindings for macros shadow path-based scope bindings to macros.`——文本绑定优先。

可见性（引自 Rust Reference）：`Macros have an implicit visibility of pub(crate). #[macro_export] changes the implicit visibility to pub.`——没导出的宏默认只在本 crate 可见（`pub(crate) use` 够用），给外部用就得 `#[macro_export]` 或公开重导出。

完整例子（`snippets/ch27/scope.rs`）：

```rust
{{#rustdoc_include ../snippets/ch27/scope.rs}}
```

```console
crate::shout!("hi") -> "hi"!
yell!("alias")     -> "alias"!
user::via_use()    -> mac::inner
user::via_path()   -> mac::inner
lifted!()          -> 被 macro_use 提升的宏
shadowed!()        -> 文本作用域的 shadowed
```

逐条对应：

- `shout!` 带 `#[macro_export]`，所以在任何模块里都能按 `crate::shout!(...)` 调用（例子里就是在 `main` 中调）；
- `pub(crate) use shout as yell;` 是重导出，给宏一个新的路径名；
- `mac::inner!` 没有路径作用域；模块里一句 `pub(crate) use inner;` 给了它一条路径，之后 `use crate::mac::inner; inner!()` 与 `crate::mac::inner!()` 都行；
- `#[macro_use] mod` 把模块内宏的文本作用域延伸到模块之外，外部可直接 `lifted!()`；
- 最后一行是遮蔽：先有文本绑定的 `shadowed!`，再有 `use other as shadowed;` 的路径绑定。**文本绑定赢**，所以打印「文本作用域的 shadowed」。

给别的 crate 用时还有个老写法：`#[macro_use] extern crate helper;` 把对方导出的宏放进 `macro_use` prelude，不用 `use` 就能调用（本章实测通过）。2018 起更推荐 `use helper::helped;` 按路径引入——这也正是 `$crate` 与重导出存在的意义。

### 27.8 递归宏与 TT muncher

宏里没有循环，处理「数量不定」的输入只有两招：重复（27.3）与递归。递归宏就是**让某条臂调用自己**，通常是最后一条或带 `@` 前缀的内部规则。

`@` 不是语法，只是一个不易被用户写出的 token，用来把内部规则和对外接口分开（匹配器里除了 `$` 和定界符，任何 token 都能字面匹配）。「吃掉一个 token 再递归」的形态（`$head:tt $($rest:tt)*`，可以按需收窄成 `ident`）就是 **TT muncher**。

完整文件 `snippets/ch27/recursion.rs`：

```rust
{{#rustdoc_include ../snippets/ch27/recursion.rs}}
```

```console
count! 得到 5
sum!(1, 2, 3, 4) = 10
transpose!(a, b; c, d) = ((1, 3), (2, 4))
```

三段各有侧重：

- `count!` 是标准 TT muncher：`@acc` 臂带累加器 `$n` 一次吃掉一个 `ident`，直到 `$($rest)*` 空掉、命中基例 `(@acc $n:expr;)`，此时 `$n` 已经累加成一串 `0 + 1 + ...`，所以能用在常量上下文里。
- `sum!` 把「递归展开成嵌套表达式」当作结果，基例是空参数。
- `transpose!` 展示嵌套重复的转置：两段重复数量一致才合法（27.3 的第二条约束）。

递归必有尽头：`#![recursion_limit]` 控制「可能无限递归的编译期操作」的深度（引自 Rust Reference），默认 128。忘写基例就是活例子：

```rust,compile_fail
macro_rules! munch {
    ($($tt:tt)*) => { munch!($($tt)*) };
}
fn main() {
    munch!(1);
}
```

```text
error: recursion limit reached while expanding `munch!`
 --> src/main.rs:2:23
  |
2 |     ($($tt:tt)*) => { munch!($($tt)*) };
  |                       ^^^^^^^^^^^^^^^
...
5 |     munch!(1);
  |     --------- in this macro invocation
  |
  = help: consider increasing the recursion limit by adding a `#![recursion_limit = "256"]` attribute to your crate (`main`)
```

「输入不变、永远递归」会在 128 层被拦下。见到这条错误，先想**基例为什么不成立**，而不是急着调大上限。

### 27.9 调试

宏出问题时最缺的是「它到底展开成了什么」。按投入从低到高：

**一、把片段打印出来。** `stringify!` 在编译期把代码变成字符串，用来确认元变量吃到了什么；本章的 snippet 里大量用了它。

**二、主动报错。** `compile_error!("...")` 在展开到某一支时终止编译，给不支持的输入一个明确提示：

```rust
macro_rules! only_ints {
    ($($i:literal),* $(,)?) => { vec![$($i as i64),*] };
    ($($t:tt)*) => { compile_error!("只接受字面量整数列表") };
}

fn main() {
    assert_eq!(only_ints!(1, 2, 3), vec![1i64, 2, 3]);
}
```

**三、看展开结果。** `cargo-expand` 是常用工具（`cargo install cargo-expand`），README 说它 `prints out the result of macro expansion and #[derive] expansion applied to the current crate`，实现上是这条命令的包装：`cargo rustc --profile=check -- -Zunpretty=expanded`（引自 cargo-expand README）。`-Zunpretty=expanded` 是 nightly 专用开关，stable 上直接跑会被拒（实测）：

```console
$ rustc --edition 2024 -Zunpretty=expanded src/main.rs
error: the option `Z` is only accepted on the nightly compiler

help: consider switching to a nightly toolchain: `rustup default nightly`
```

`cargo expand` 能在 stable 上工作，是因为它内部给 rustc 设了 `RUSTC_BOOTSTRAP=1`（见其 `src/main.rs`）。把练习 3(b) 的 `fn f()` 展开（与 cargo-expand README 示例同形；实测）：

```text
fn f() -> i32 {
    let x = 1;
    macro_rules! first_x { () => { x }; }
    let x = 2;
    x + x
}
```

最后一行是 `x + x`：两个 `x` 文本上一模一样，但**不是同一个绑定**（右边那个来自宏内部，绑定到 `x = 1`）。这就是那句免责声明的意思：`Be aware that macro expansion to text is a lossy process. This is a debugging aid only.`——展开文本能确认「臂选对了、token 拼对了」，但不是真相；要验证行为还得跑起来。

`RUSTC_BOOTSTRAP=1` 是给工具用的逃生口，不是给项目用的——CI 上别依赖它。

## 动手看看

把上面的零件拼成一次实战：写一个宏导出给另一个 crate，确认它在「调用处有同名符号」时正确。

第一步，写出库与它的辅助宏（真实工程里是 `helper/src/lib.rs`，这里直接放 `helper.rs`）：

```rust
// helper.rs（= helper/src/lib.rs）
#[macro_export]
macro_rules! helper {
    () => { "helper 宏" };
}

#[macro_export]
macro_rules! helped {
    () => { $crate::helper!() };
}

pub fn version() -> &'static str {
    "helper 1.0"
}
```

第二步，使用方只引入 `helped!`，并故意定义一个同名函数 `helper`（真实工程里是 `user/src/main.rs`，这里放在 `user.rs`）：

```rust,ignore
// user.rs（= user/src/main.rs）
use helper::helped;

// 故意制造同名符号：宏展开不该取它
fn helper() -> &'static str {
    "调用处的 helper"
}

fn main() {
    println!("{}", helper());
    println!("{}", helped!());   // 走 $crate，取的是 helper 宏
    println!("{}", helper::version());
}
```

第三步，跑起来。本机用 `rustc` 直接编这两个文件（`cargo` 的等价流程见第 30 章）：

```console
$ rustc --edition 2024 --crate-type=lib --crate-name helper helper.rs -o libhelper.rlib
$ rustc --edition 2024 --extern helper=libhelper.rlib -L . -o user.exe user.rs
$ .\user.exe
调用处的 helper
helper 宏
helper 1.0
```

三行分别对应三次调用：直接调同名函数、`helped!()` 经 `$crate` 取到 `helper` 宏、通过外部 crate 路径调 `version()`。注意第二行——宏**没有**取到调用处的 `helper()`。

把 `helped!` 换成 27.6 里不带 `$crate` 的版本，你会拿到 `` cannot find macro `helper` in this scope ``——值得亲手试一遍。

想再进一步，把 `helped!` 改成接受参数列表、逐个处理再交回 `helper!` 的 TT muncher 形态，会同时用到重复、递归与 `$crate`。

## 报错现场

**一、follow set 违规。** 最小复现：

```rust,compile_fail
macro_rules! bad {
    ($e:expr + $r:expr) => { $e + $r };
}
fn main() {
    let _ = bad!(1 + 2);
}
```

以 Rust 1.98 为例，关键几行：

```text
error: `$e:expr` is followed by `+`, which is not allowed for `expr` fragments
 --> src/main.rs:2:14
  |
2 |     ($e:expr + $r:expr) => { $e + $r };
  |              ^ not allowed after `expr` fragments
  |
  = note: allowed there are: `=>`, `,` or `;`

error: aborting due to 1 previous error
```

定位路径：报错指向**匹配器**里 `expr` 后面那个 token，不是你写的表达式。`+` 不在白名单里，这条臂无法定义。修法：把 `+` 挪进片段（`($e:expr)`，让调用方写 `1 + 2`），或用 `tt` 自己解析（`($l:tt + $r:tt)`）。**别把运算符写在片段外面**。

**二、局部歧义（无 lookahead）。** 最小复现：

```rust,compile_fail
macro_rules! ambiguity {
    ($($i:ident)* $j:ident) => {};
}
fn main() {
    ambiguity!(error);
}
```

以 Rust 1.98 为例：

```text
error: local ambiguity when calling macro `ambiguity`: multiple parsing options: built-in NTs ident ('i') or ident ('j').
 --> src/main.rs:5:16
  |
5 |     ambiguity!(error);
  |                ^^^^^

error: aborting due to 1 previous error
```

定位路径：`error` 既可归给重复里的 `$i`，也可归给收尾的 `$j`；宏系统一次只看一个 token，所以判不了。这类报错**永远指向调用点**，病根却在匹配器。修法：改成无歧义形状，比如先出现标记 token（`($($i:ident)* => $j:ident)`）。

**三、卫生性导致「找不到名字」。** 最小复现：

```rust,compile_fail
macro_rules! m {
    (define) => { let x = 1; };
    (refer)  => { dbg!(x); };
}
fn main() {
    m!(define);
    m!(refer);
}
```

以 Rust 1.98 为例：

```text
error[E0425]: cannot find value `x` in this scope
 --> src/main.rs:3:24
  |
3 |     (refer)  => { dbg!(x); };
  |                        ^ not found in this scope
...
7 |     m!(refer);
  |     --------- in this macro invocation
  |
help: an identifier with the same name is defined here, but is not accessible due to macro hygiene
 --> src/main.rs:2:23
  |
2 |     (define) => { let x = 1; };
  |                       ^
...
6 |     m!(define);
  |     ---------- in this macro invocation
```

定位路径：看到 `not accessible due to macro hygiene` 就知道不是拼写问题，而是两次展开的绑定属于不同世界。修法是**让数据走参数**：把产生的结果用 `$name` 传给下一条臂，而不是指望两次展开共享一个 `let`。宏的重用单位是「一次展开」，不是「一段作用域」。

## 小结

- **声明宏 = 匹配器 + 转写器**：编译期把 token 序列换成新序列；臂按顺序尝试，第一处匹配胜出，**不回溯、不做 lookahead**。
- **片段说明符决定接受面**：默认 `tt`，要语法检查再升级到 `expr`/`ty`/`pat`；转发片段时第二个宏只能用同类型说明符（`tt`/`ident`/`lifetime` 例外）；后随 token 有白名单，`expr`/`stmt` 只允许 `=>`、`,`、`;`。
- **2024 只改了 `expr`**（也匹配 `const` 块与 `_`，旧行为由 `expr_2021` 保留，迁移 lint 为 `edition_2024_expr_fragment_specifier`）；**`pat` 的变化在 2021**（需紧跟 `|` 时用 `pat_param`）；**`stmt` 没变化**；缺片段说明符从 1.89 起是硬错误。
- **卫生性是混合的**：局部变量与标签按定义处解析，函数/类型/生命周期按调用处解析。
- **跨 crate 用 `$crate`**：指向定义宏的 crate，非宏项要写全路径，且不绕过可见性；`local_inner_macros` 别用。
- **作用域有两套**：文本作用域（按顺序，`#[macro_use]` 可扩展）与路径作用域（`use` 重导出、`#[macro_export]` 落到 crate 根）；未限定调用先文本后路径，文本遮蔽路径；隐式可见性 `pub(crate)`。
- **递归宏靠「吃 token 再调自己」**（常用 `@` 前缀分内部规则）；深度上限默认 128。
- **调试顺序**：`stringify!` → `compile_error!` → `cargo expand`（文本展开有损）。

## 练习

**1. 预测编译结果并解释**

```rust,compile_fail
macro_rules! pick {
    ($e:expr) => { let = $e; };
    ($($tt:tt)*) => { 0 };
}

fn main() {
    let _ = pick!(1);
}
```

会失败吗？是 `no rules expected` 还是别的？为什么兜底臂没救场？

**2. 预测运行结果并解释**

下面这段能编译，但有一个臂永远轮不到。打印什么？为什么？

```rust
macro_rules! log_one {
    (0) => { println!("zero 臂"); };
    ($e:expr) => { println!("expr 臂: {}", $e); };
}

macro_rules! log_all {
    ($($e:expr),*) => { $( log_one!($e); )* };
}

fn main() {
    log_one!(0);      // 直接调用
    log_all!(0, 1);   // 经 expr 片段转发
}
```

**3. 说明为什么**

（a）为什么 `$e:expr` 后面不能跟 `+`，而 `$e:tt` 后面可以？
（b）下面的 `f()` 返回什么？为什么？

```rust
fn f() -> i32 {
    let x = 1;
    macro_rules! first_x {
        () => { x };
    }
    let x = 2;
    x + first_x!()
}
```

<details><summary>参考答案</summary>

**1.** 会失败，但**不是** `no rules expected`，而是转写后解析失败：

```text
error: expected expression, found `let` statement
 --> src/main.rs:2:20
  |
2 |     ($e:expr) => { let = $e; };
  |                    ^^^
...
7 |     let _ = pick!(1);
  |             -------- in this macro invocation
  = note: only supported directly in conditions of `if` and `while` expressions

error: expected pattern, found `=`
 --> src/main.rs:2:24
  |
2 |     ($e:expr) => { let = $e; };
  |                        ^ expected pattern
...
7 |     let _ = pick!(1);
  |             -------- in this macro invocation

error: aborting due to 2 previous errors
```

原因：第一条臂只要求 `($e:expr)`，`1` 能匹配，所以**臂被选中了**；问题出在转写结果 `let = 1;` 不是合法语法。Reference 写得很清楚：`It transcribes the first successful match; if this results in an error, then future matches are not tried.`——兜底臂没机会尝试，因为失败发生在「匹配之后」。

**2.** 打印三行：

```text
zero 臂
expr 臂: 0
expr 臂: 1
```

直接调 `log_one!(0)` 时 `(0)` 字面臂命中；但经 `log_all!` 转发时，`$e` 是**不透明的 `expr` 片段**，`log_one` 的 `(0)` 臂没法拿字面 token 比（27.2 的转发规则），于是落到 `($e:expr)` 臂——第二行因此是 `expr 臂: 0`。

修法：让 `log_all!` 用 `tt` 转发（`($($e:tt),*)`），字面 token 就能原样到达 `log_one`（实测：改 `tt` 后三行变成 `zero 臂` / `zero 臂` / `expr 臂: 1`）。**跨宏转发时片段是「整体」，字面 token 匹配会静默失效。**

**3.（a）** `tt` 是**不解析**的片段：一棵 token 树，取了什么就是什么。`expr` 要解析，解析器必须能确定表达式在哪结束；`+` 可能成为表达式的一部分，所以不能进白名单。白名单是为了让今天的匹配在语法演进后依然无歧义——代价是少数写法被永久禁止。

**3.（b）** 返回 `3`：宏在定义处可见的绑定是 `x = 1`（那时 `let x = 2;` 还没出现），卫生性让 `first_x!()` 取的就是它；`cargo expand` 打出 `x + x` 也说不清这一点。

</details>

## 延伸阅读

- Rust Reference, Macros By Example：[https://doc.rust-lang.org/reference/macros-by-example.html](https://doc.rust-lang.org/reference/macros-by-example.html)
- Rust Reference, Macro ambiguity（follow set 的形式化规则）：[https://doc.rust-lang.org/reference/macro-ambiguity.html](https://doc.rust-lang.org/reference/macro-ambiguity.html)
- Rust Reference, Limits（`recursion_limit`）：[https://doc.rust-lang.org/reference/attributes/limits.html](https://doc.rust-lang.org/reference/attributes/limits.html)
- The Rust Edition Guide, Macro fragment specifiers（2024）：[https://doc.rust-lang.org/edition-guide/rust-2024/macro-fragment-specifiers.html](https://doc.rust-lang.org/edition-guide/rust-2024/macro-fragment-specifiers.html)
- The Rust Edition Guide, Missing macro fragment specifiers（2024）：[https://doc.rust-lang.org/edition-guide/rust-2024/missing-macro-fragment-specifiers.html](https://doc.rust-lang.org/edition-guide/rust-2024/missing-macro-fragment-specifiers.html)
- The Rust Edition Guide, Or patterns in macro-rules（2021）：[https://doc.rust-lang.org/edition-guide/rust-2021/or-patterns-macro-rules.html](https://doc.rust-lang.org/edition-guide/rust-2021/or-patterns-macro-rules.html)
- The Book, Macros：[https://doc.rust-lang.org/book/ch20-05-macros.html](https://doc.rust-lang.org/book/ch20-05-macros.html)
- Rust by Example, Macros：[https://doc.rust-lang.org/rust-by-example/macros.html](https://doc.rust-lang.org/rust-by-example/macros.html)
- `std::matches`：[https://doc.rust-lang.org/std/macro.matches.html](https://doc.rust-lang.org/std/macro.matches.html)
- `std::vec`：[https://doc.rust-lang.org/std/macro.vec.html](https://doc.rust-lang.org/std/macro.vec.html)
- cargo-expand（外部工具仓库）：[https://github.com/dtolnay/cargo-expand](https://github.com/dtolnay/cargo-expand)
