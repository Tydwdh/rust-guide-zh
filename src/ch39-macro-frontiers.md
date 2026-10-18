# 第 39 章 宏系统极限

## 这一章解决什么问题

第 27 章的 `macro_rules!` 和第 28 章的三类过程宏讲的都是「怎么把宏写出来」。这一章问的是另一类问题：**这套机制能走多远，走到哪里会撞墙**。有的需求它天生满足不了（宏想数一数自己展开了几遍），有的能力被锁在 nightly 后面，还有一些行为「能跑但反直觉」——不写下来，你会在调试真实宏时才发现。

本章交付四件事：

- **元变量表达式（macro metavariable expression）**（`${count($x)}`、`${index()}`、`${ignore($x)}`）：声明宏第一次能读到「重复了几次、现在第几轮」这类元数据。它从 2021 年的 RFC 起就挂在 nightly，本章只讲语法与现状，不伪造输出。
- **组合形态（macro composition）**：函数式宏可以生成属性宏调用，声明宏也可以；同一个 item 上叠多个宏属性时，展开顺序是确定的——这份顺序决定了「谁看到什么」。
- **`#[track_caller]`**：把**调用点（call site）**在哪儿变成可以在函数之间传递的数据。`unwrap`、`assert_eq!` 这些你每天用的东西全靠它，而它有明确的失效场景。
- **诊断消息设计**：宏出错的难点不是「报错」，而是「报在哪儿、报几条、说什么」。这里给出 `compile_error!` 的真实布局，以及 stable 与 nightly 各能做什么。

读完你应当能：判断一个需求落在宏系统的哪一格；写属性宏时预判展开顺序；让宏的错误指向用户的代码而不是你的宏体；知道哪些「宏系统的下一步」还只能读文档。

**边界**：`macro_rules!` 的匹配、卫生与 `$crate` 本身在第 27 章；`TokenStream`/`Span`、proc-macro crate 的建立与 syn/quote 的取舍在第 28 章。本章只补极限情形。

## 为什么这样设计

### 天花板是故意画低的

Rust 的宏系统选择「能生成代码」，并把接口收窄到一个刻意很小的面上：输入输出都是 token 流。Reference 给的理由是稳定性——token 流是语言的表层，语法树是编译器内部表示，随版本演进：

> Procedural macros operate over token streams instead of AST nodes, which is a far more stable interface over time for both the compiler and for procedural macros to target.
>
> （引自 Rust Reference）

代价在本章到处可见。宏看不见结构，于是「数一数」「拿到当前下标」这种编译器早就知道的信息，宏作者只能自己用 TT muncher 拼一个计数器：慢、难读、边界多。RFC 3086 想解决的就是这个落差——把编译器已知的重复元数据以 `${…}` 的形式直接交给宏。它 2021 年提出，到 Rust 1.98 仍挂在 `#![feature(macro_metavar_expr)]` 后面：tracking issue 上还留着未解决的语义争论与卫生性问题（见 39.1）。这就是「宏系统极限」：不是能力不存在，而是要么没稳定，要么在别处漏风。

### 卫生性是分层的

第 27 章说过声明宏是**混合卫生**（mixed-site hygiene）：局部变量、标签、`$crate` 按定义处解析，其余符号按调用处解析。第 28 章说过过程宏**完全不卫生**：

> Procedural macros are unhygienic. This means they behave as if the output token stream was simply written inline to the code it's next to.
>
> （引自 Rust Reference）

为什么过程宏拿不到卫生？因为卫生要求回答「这个名字属于哪一次展开」，而过程宏的入口只有两段 token 流和它们的 span——span 只标位置，不携带「谁声明的」。真要给宏作者一个「定义点 span」，`proc_macro` 里有 `Span::def_site`，但它至今没稳定：tracking issue（#54724）里的说法是它 "likely blocked on larger hygiene reform and more thorny Macros 2.0 issues"；lang team 2022 年还专门记过一笔，大意是这个 issue 需要的不只是补测试，而是先把「def-site 到底该是什么效果」写成可评审的文档——连它该定义成什么都还没结论。所以过程宏的卫生实践只能靠约定：路径写全（`::std::option::Option`）、内部名字加不可能撞的前缀（Reference 举的例子是 `__internal_foo` 而不是 `foo`）、把必需的路径作为参数让用户传进来（`#[serde(crate = "…")]`）。这些约定在 39.5 会变成具体的失败案例。

### 诊断是宏的门面

宏展开之后，用户看到的是**完全陌生的代码**：他不知道 `#[tag]` 里发生了什么，也不知道报错指向的那一行是否由宏生成。于是宏的质量几乎等于诊断质量，而诊断只有三件事要设计：**位置**（span 挂在谁身上）、**数量**（一次报几条）、**措辞**（能不能给出定位线索）。三件事都不难，但都没有默认值——rustc 不会替你猜。39.4 会把它们逐一落到真实输出上。

## 机制与规则

### 39.1 元变量表达式：让声明宏自己数数（nightly）

`vec!` 的教科书实现是逐条 `push`；想改成 `Vec::with_capacity` 就得知道「有几个元素」，而声明宏没有任何计数手段。RFC 3086 引入的**元变量表达式**补的就是这个洞：

```rust,ignore
#![feature(macro_metavar_expr)]

macro_rules! with_capacity_vec {
    ($($x:expr),*) => {{
        let mut v = Vec::with_capacity(${count($x)});
        $( v.push($x); )*
        v
    }};
}
```

`${count($x)}` 会展开成一个**无后缀整数字面量**：在 `vec!` 这种单层重复里就是「一共几个元素」。可用的表达式（语法见 RFC 3086，语义以 1.98 的实现与测试为准）：

| 写法 | 语义 |
|---|---|
| `${count($x)}` | 当前展开位置下 `$x` 会重复多少次（等同 `${count($x, 0)}`） |
| `${count($x, depth)}` | 把统计深度限定在 `depth` 层：`0` 是最内层，数字越大越往外 |
| `${index()}` | 最内层重复的当前下标（从 0 开始；等同 `${index(0)}`） |
| `${index(depth)}` / `${len(depth)}` | 往外数第 `depth` 层的下标 / 该层的重复次数 |
| `${ignore($x)}` | 让 `$x` 参与重复计数，但自身展开为空 |
| `$$` | 展开成一个 `$`，用于「宏里定义宏」 |
| `${concat(…)}` | 由标识符/字面量拼出新标识符（另一个 gate） |

三处必须写清楚，因为它们**与 RFC 的写法或旧口径不一致**：

- **元变量要带 `$`，这是有意偏离 RFC。** RFC 3086 的示例写的是 `${count(x)}`，而 1.98 的实现要求 `${count($x)}`——少写 `$` 时编译器直说：`meta-variables within meta-variable expressions must be referenced using a dollar sign`（本机实测原话）。tracking issue 里 2023 年 10 月那份提案的原话是 "For unblocking purposes, the requirement of `$` is being suggested."——RFC 原本不带 `$`，是为了解除阻塞才改成要求它，lang team 随后同意推进。
- **实现里的名字是 `len`，RFC 文档里写的是 `length`。** 1.98 的解析分支只认 `count`、`ignore`、`index`、`len`、`concat` 这五个名字（诊断信息里列的就是这一串）；写 `${length()}` 会得到「无法识别的元变量表达式」。
- **depth 的方向与 RFC 的旧口径相反**，1.76 起才统一（PR #117050，milestone 1.76.0）：现在 `count` 与 `index`/`len` 同向，`0` 是最内层、往外递增，`${count($x)}` 与 `${count($x, 0)}` 等价；RFC 写的是反方向（默认统计所有深度、`${count(x, 1)}` 数最外层），其三层嵌套示例在 1.98 的测试里断言为 `[13, 4, 2]`，正对应 RFC 里的 `(2, 4, 13)`。这就是当年那段「方向不对称」争论（tracking issue 上最长的一串）的结论。

> [!WARNING]
> 1.98 的 stable 完全没有这套语法。写 `${count($x)}` 得到的是 `error[E0658]: meta-variable expressions are unstable`；写 `$$` 得到的是同一类 E0658——每处 `$$` 各报一条（note 都指向 issue #83527），并且在解析中断的地方还会跟着一条普通语法错误。本章示例因此只用 `ignore` 块展示，**不粘贴任何「运行结果」**——本机只有 stable。要真跑：`rustup toolchain install nightly` 加 `#![feature(macro_metavar_expr)]`。
>
> 另外，`${concat(…)}` 属于独立的 gate `macro_metavar_expr_concat`（tracking issue #124225），它解决的正是「声明宏造不出新标识符、只好引 `paste` 这类过程宏」的老问题。

它为什么至今没稳定？tracking issue 上记着两条：一是元变量表达式生成的字面量的**卫生性**曾不正确（需要专门的修复 PR），二是 `$$` 与 `$$crate` 该按谁的定义处解析没有结论（有人主张改成 `${crate()}`）。稳定一件语言特性要先定语义，这些没定完，它就继续挂在 nightly——这是本章的第一处「极限」。

### 39.2 组合形态：谁生成谁，谁先展开

真实代码里宏套宏很常见：函数式宏（`query!`、`include!` 这一类）的输出里可以挂属性宏，属性宏的输出里也可以；最常见的一类是把 `async fn main` 改写成普通 `fn main` 加运行时初始化的属性宏。但这些组合的展开顺序很少有人讲清楚，先给结论再给实测。

**规则一：同一个 item 上的多个宏属性，按书写顺序从外到内依次展开。** 每个宏拿到的 `item` 是「它下面的所有东西」——包括还没被消费的属性：

> The second TokenStream is the rest of the item, including other attributes on the item.
>
> （引自 Rust Reference）

**规则二：谁写在前面谁先跑——`derive` 也是这条规则里的普通一员。** `#[attr] #[derive(D)]` 先展开属性宏，`#[derive(D)] #[attr]` 先展开 derive；derive 拿到的输入 token 流里同样带着排在它下面的属性。

规则一值得配一张图：

<svg viewBox="0 0 700 250" width="100%" role="img" aria-label="属性宏叠加的展开顺序：外层先展开，它拿到的 item 里仍带着内层属性，内层随后展开">
<g fill="none" stroke="currentColor" stroke-width="1.5">
<rect x="20" y="8" width="420" height="46" rx="4"/>
<rect x="20" y="68" width="420" height="46" rx="4"/>
<rect x="20" y="128" width="420" height="46" rx="4"/>
<rect x="20" y="188" width="420" height="46" rx="4"/>
<path d="M230 54 V64"/>
<path d="M230 114 V124"/>
<path d="M230 174 V184"/>
</g>
<g fill="currentColor">
<polygon points="226,60 230,66 234,60"/>
<polygon points="226,120 230,126 234,120"/>
<polygon points="226,180 230,186 234,180"/>
</g>
<g fill="currentColor" font-size="12" font-family="monospace">
<text x="32" y="28">#[tag(OUTER_SAW)]</text>
<text x="32" y="46">#[tag(INNER_SAW)]  const STACKED: u8 = 1;</text>
<text x="32" y="88">const OUTER_SAW: &amp;str = "OUTER_SAW";</text>
<text x="32" y="106">#[tag(INNER_SAW)]  const STACKED: u8 = 1;</text>
<text x="32" y="142">const INNER_SAW: &amp;str = "INNER_SAW";</text>
<text x="32" y="158">const STACKED: u8 = 1;</text>
<text x="32" y="216">const OUTER_SAW … const INNER_SAW … const STACKED</text>
</g>
<g fill="currentColor" font-size="12">
<text x="460" y="27">① 外层先展开：</text>
<text x="460" y="43">item 里还带着内层属性</text>
<text x="460" y="87">② 内层随后展开：</text>
<text x="460" y="103">item 里只剩声明</text>
<text x="460" y="147">③ 最终代码：</text>
<text x="460" y="163">两个标记常量都生成</text>
<text x="460" y="207">三个名字共存</text>
<text x="460" y="223">于同一个模块</text>
</g>
<g fill="var(--links)" font-size="12" font-family="monospace">
<text x="460" y="66">attr = OUTER_SAW</text>
<text x="460" y="126">attr = INNER_SAW</text>
</g>
</svg>

证据来自 `snippets/ch39/attr-demo`：一个零依赖的过程宏 crate（手写 token 处理，没有 syn/quote）。里面的 `#[tag(NAME)]` 只做两件事——把输入打到编译日志（`eprintln!` 走编译器进程，cargo 会显示），并生成一个 `const NAME: &str` 标记；`Sniff` 这个 derive 生成 `const SNIFFED_<名字>: u8`。

```rust,ignore
{{#include ../snippets/ch39/attr-demo/src/lib.rs:8:29}}
```

消费它的 crate 这样写：

```rust,ignore
{{#include ../snippets/ch39/attr-demo-use/src/main.rs:7:36}}
```

编译这份代码，日志按发生顺序如下（已略去与顺序无关的链接器噪音）：

```text
[tag]   attr = <OUTER_SAW>
[tag]   item = <# [tag(INNER_SAW)] const STACKED : u8 = 1 ;>
[tag]   attr = <INNER_SAW>
[tag]   item = <const STACKED : u8 = 1 ;>
[tag]   attr = <RECORD_TAG>
[tag]   item = <# [derive(Sniff)] struct Record { id: u32, }>
[Sniff] input = <struct Record { id: u32, }>
[Sniff] input = <# [tag(LATE_TAG)] struct Late { id: u32, }>
[tag]   attr = <LATE_TAG>
[tag]   item = <struct Late { id: u32, }>
[tag]   attr = <FROM_FN_MACRO>
[tag]   item = <const MADE_BY_FN_MACRO : u8 = 2 ;>
[tag]   attr = <FROM_RULES>
[tag]   item = <const RULES_ITEM : u8 = 4 ;>
```

逐段读：

- **前两行**：`#[tag(OUTER_SAW)]` 先跑，它的 `item` 里还留着 `# [tag(INNER_SAW)]`。外层宏确实能「看见」内层属性，但那只是**没展开的 token**，不是内层的结果——想知道内层生成了什么，你得自己递归展开，这正是离开 syn 之后最难手写的那部分。
- **第四、五行**：`#[tag(RECORD_TAG)]` 跑完，`#[derive(Sniff)]` 才跑，而且 derive 的输入里已经没有 `#[tag(...)]`——属性宏展开时把它消费掉了。
- **第六行**：把顺序反过来写（`#[derive(Sniff)]` 在前、`#[tag(LATE_TAG)]` 在后），derive 先跑，并且**它的输入里带着 `# [tag(LATE_TAG)]`**。别指望 derive 看到干净的 item：它看到的是尚未展开的兄弟属性。
- **最后四行**：`make_item!()` 是函数式宏，它的输出里写着 `#[tag(FROM_FN_MACRO)]`；`gen_item!()` 是声明宏，展开结果里也有 `#[tag(FROM_RULES)]`。两条都照常被展开——**宏生成的属性宏调用不是特例**，输出会被重新扫描。

顺带一个实现细节：探针里标记名由调用方给（`const {name}: &str`），所以同一个 item 上叠两个 `#[tag]` 不会撞名。真实宏要生成自己的内部名字时同样得自己保证唯一——因为过程宏不卫生（见 39.5）。

### 39.3 `#[track_caller]`：把调用点变成参数

`unwrap` 失败时，报的位置是**你调用 `unwrap` 的那一行**，不是 `core` 里那份实现。这不是特例，而是 `#[track_caller]` 的日常效果：它让一个函数能读出「调用它的那行代码在哪儿」。

机制一句话：给函数的 ABI 加一个**隐式参数**。Reference 的说明点破了后面所有「能拿到 / 拿不到」：

> rustc implements track_caller in a codegen context by appending an implicit parameter to the function ABI, but this would be unsound for an indirect call because the parameter is not a part of the function's type and a given function pointer type may or may not refer to a function with the attribute.
>
> （引自 Rust Reference）

读调用点的 API 是 `std::panic::Location` 的 `caller()`：

```rust,ignore
#[track_caller]
fn located() -> &'static Location<'static> {
    Location::caller()
}
```

Reference 把它的语义定义为「沿栈往上走」：

> At the point of observation, an implementation behaves as if it walks up the stack from f's frame to find the nearest frame of an unattributed function outer, and it returns the Location of the tracked call in outer.
>
> （引自 Rust Reference）

`Location::caller` 的文档补了下半句：如果那个调用者自己也被标注了，就继续往上（"…and so on up the stack to the first call within a non-tracked function body."，引自 `std::panic::Location` 文档）。合起来就是下面这张表：

| 情形 | `Location::caller()` 报告哪儿 |
|---|---|
| 直接在调用点调用 `#[track_caller]` 函数 | 调用点那一行 |
| 中间夹一层**没标注**的函数 | 那层函数体内调用下一层的那一行（与调用处无关，是个常量） |
| 链条上每一层都标注 | 一路传到最外层被标注调用点的位置 |
| 泛型函数 / 方法 | 同上，标注与泛型无关 |

位置上的硬规则（都能在 Reference 的 `track_caller` 一节查到）：

- **只能用在「Rust ABI、且不是入口 `fn main`」的函数上**。给 `extern "C"` 函数标注会被拒（错误目录 E0737 的说法是「`#[track_caller]` requires functions to have the "Rust" ABI for implicitly receiving caller location.」）；
- **trait 声明上标注就对所有实现生效**，实现里可以不写；trait 提供带标注的默认实现时同理；
- **extern 块两边都要写**，只写一边是未定义行为；
- **闭包**在 1.98 还不能标注（`closure_track_caller`，tracking issue #87417）。

然后是失效场景：**一旦涉及函数指针，调用点信息会在 shim 那一层被替换成被调函数的定义处。** Reference 的原话是：

> In particular, coercing a function with `#[track_caller]` to a function pointer creates a shim which appears to observers to have been called at the attributed function's definition site, losing actual caller information across virtual calls. A common example of this coercion is the creation of a trait object whose methods are attributed.
>
> （引自 Rust Reference）

最后半句值得展开，因为它决定了一个容易踩的细节：**trait 声明上标了，经 trait 对象调用仍然正确；只在 impl 上标、声明没标的，经 trait 对象调用就退化成定义处。** 两种写法都能编译，行为不同。动手看看那一节会给出这两个数字。

### 39.4 诊断消息设计

第 28 章说过，过程宏报错有两条路：panic（一条错误，位置固定在 `#[derive(...)]`）和生成 `compile_error!`（位置由 span 决定，可以报多处）。到了极限这一层，要关心的是**编排**：一次编译能报几条、每条落在哪、`error:` 下面那几行是谁排的。

先看 `compile_error!` 在声明宏里的真实布局：

```rust,compile_fail
// 两条独立诊断：每条 compile_error! 各占一处
macro_rules! config {
    (name = $n:literal, threads = $t:literal) => {{
        let _name: &str = $n;
        let _threads: u32 = $t;
    }};
    (name = $n:literal) => {
        compile_error!(concat!("缺少 threads：只写了 name = ", $n));
    };
    (threads = $t:literal) => {
        compile_error!(concat!("缺少 name：只写了 threads = ", $t));
    };
}

fn main() {
    config!(name = "demo", threads = 4);
    config!(threads = 8);
    config!(name = "demo");
}
```

`compile_error!` 的参数必须是字符串字面量；`concat!` 之所以能用，是因为它更早一轮就展开成了字面量，所以消息里还能夹进出错的内容。以 Rust 1.98 为例，关键输出（两条错误都完整）：

```text
error: 缺少 name：只写了 threads = 8
  --> main.rs:11:9
   |
11 |         compile_error!(concat!("缺少 name：只写了 threads = ", $t));
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
17 |     config!(threads = 8);
   |     -------------------- in this macro invocation
   |
   = note: this error originates in the macro `config` (in Nightly builds, run with -Z macro-backtrace for more info)

error: 缺少 threads：只写了 name = demo
  --> main.rs:8:9
   |
 8 |         compile_error!(concat!("缺少 threads：只写了 name = ", $n));
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
18 |     config!(name = "demo");
   |     ---------------------- in this macro invocation
   |
   = note: this error originates in the macro `config` (in Nightly builds, run with -Z macro-backtrace for more info)
```

三点读法：

- **主 span 落在宏定义里的 `compile_error!`**，调用点由第二段标出（`in this macro invocation`）。声明宏没有换 span 的手段——这是它与过程宏的第一处能力差（过程宏可以把自己生成的 token 挂到用户输入那颗 token 的 span 上，第 28 章 28.3 有对照）。
- **两条错误一起出现。** 这是 `compile_error!` 相对 panic 的核心优势：panic 让宏在第一条错误处就返回，一次只报一条；`compile_error!` 只是往输出里塞一条诊断，所以「能查出来的问题一次列完」是可行的。写配置校验、DSL 检查类宏时，这条差别直接决定用户要编译几轮才能改对。
- **消息要带上下文。** 上例把出错的那个字面量拼进了消息（`name = "demo"`），读者不必回头数第几个参数写错了。`concat!` 里可以夹 `stringify!($x)`，也可以夹 `$n` 这种字面量片段。

再往上一步是 nightly 的 `proc_macro::Diagnostic`（gate `proc_macro_diagnostic`，tracking issue #54140）：宏可以直接构造**带级别的诊断**（`Level::Error`/`Warning`/`Note`/`Help`），每条能挂自己的 span，还能挂子诊断，最后 `emit()` 出去：

```rust,ignore
#![feature(proc_macro_diagnostic)]
use proc_macro::{Diagnostic, Level, Span, TokenStream};

#[proc_macro]
pub fn check(input: TokenStream) -> TokenStream {
    // …解析 input，找到出错的那颗 token
    let sp: Span = /* 出错 token 的 span */;
    Diagnostic::spanned(sp, Level::Error, "字段名和结构体重名")
        .help("换一个字段名，或给结构体改名")
        .emit();
    input
}
```

> [!WARNING]
> `Diagnostic`，以及同一 gate 下的 `Span::error/warning/note/help`，在 1.98 都标着 "nightly-only experimental API"（gate 就叫 `proc_macro_diagnostic`，tracking issue #54140）。本节因此只给 API 形状与用法，不贴「运行结果」。stable 上想让宏产生**警告**，只能绕道生成带 `#[deprecated]` 的项（第 28 章 28.3 提过）——难看但有效。

### 39.5 卫生性与 `$crate` 的极限情形

第 27 章讲清了声明宏的混合卫生，这里补边界上的四件事。

**一、item 名不在卫生范围内。** 宏体里的 `const $name`、`fn $name`、`struct $name` 用的是调用方传进来的标识符，落在**调用方的命名空间**里。同一个宏造两次同名就是硬错误 E0428（见报错现场二）——宏不提供改名机制，避免重名是调用方的责任。

**二、宏体不能「借用」调用处的名字，也不能在宏体里 `use`。** 展开式里的函数名、类型名按**调用处**解析：宏定义所在的模块里有没有这个名字无关紧要，调用处看不见才报错。于是可移植的宏体几乎只能写两种引用——调用处保证可见的路径，或者 `$crate::`。

**三、`$crate` 指向「定义宏的 crate」，而不是「重导出宏的 crate」。** 假设 `macros` crate 定义了宏 `m!`，`facade` 用 `pub use macros::*;` 把它重导出：`m!` 展开时 `$crate` 仍然指向 `macros`（Reference 的措辞是 "It refers to the crate defining the macro"）。对使用者没影响——`facade::m!` 照样能用——但对宏作者是陷阱：想「在自己的库里 `pub use` 别人的宏，然后让 `$crate` 指向自己」，做不到。

**四、过程宏完全没有卫生，nightly 上也没有现成答案。** Reference 建议的两个替代做法是写绝对路径（`::std::option::Option`）和给内部名字加不可能撞的前缀。更精细的 `Span::def_site` 还锁在 `proc_macro_def_site`（tracking issue #54724）里，而 39.1 的理由清单上也有同一条：`macro_metavar_expr` 生成 token 的卫生问题正是它未稳定的原因之一。**「名字属于谁」是宏系统至今没有完全解决的问题**——撞上它不怪你。

## 动手看看

三个示例都在仓库根的 `snippets/ch39/` 下。

**一、`#[track_caller]` 的传播与丢失**（`snippets/ch39/track_caller.rs`，完整文件）：

```rust
{{#rustdoc_include ../snippets/ch39/track_caller.rs}}
```

单独编译运行（Windows；输出里的行号就是这份文件的行号）：

```console
$ rustc --edition 2024 snippets/ch39/track_caller.rs -o track_caller.exe
$ .\track_caller.exe
located() → snippets/ch39/track_caller.rs:67
wrapper() 第一次 → snippets/ch39/track_caller.rs:13
wrapper() 第二次 → snippets/ch39/track_caller.rs:13
outer() → snippets/ch39/track_caller.rs:70
generic::<u8>() → snippets/ch39/track_caller.rs:71
S.method() → snippets/ch39/track_caller.rs:72
&dyn Declared（声明已标注） → snippets/ch39/track_caller.rs:75
&dyn Undeclared（只标实现） → snippets/ch39/track_caller.rs:57
函数指针调用 → snippets/ch39/track_caller.rs:7
panic 位置 → snippets/ch39/track_caller.rs:90
unwrap 的 panic 被捕获 = true
```

按上面那张表对号：

- `located()` 报第 67 行，那是 `main` 里调用它的位置；`S.method()`、`generic::<u8>()` 同理（第 72、71 行）；
- `wrapper()` 两次都报第 13 行——`wrapper` 自己没标注，往上一层就停住了，与「从哪儿调用它」无关，这是它作为**常量位置**的样子；
- `outer()` 报第 70 行：链条上每一层都标注，于是调用点一路传到 `main`；
- `&dyn Declared`（trait 声明上标注）报第 75 行的调用点，`&dyn Undeclared`（只在 impl 上标注）报第 57 行那个 impl 体内的定义位置——**这正是 Reference 说的 shim 造成的退化**，两种写法只差一行属性，行为不同；
- 函数指针调用报第 7 行，也就是被调函数自己的定义处；
- `unwrap()` 的 panic 报第 90 行——标准库同样靠这个属性。

**二、卫生边界的两面**（`snippets/ch39/hygiene_limits.rs`）：

```rust
{{#rustdoc_include ../snippets/ch39/hygiene_limits.rs}}
```

输出：

```console
$ rustc --edition 2024 snippets/ch39/hygiene_limits.rs -o hygiene_limits.exe
$ .\hygiene_limits.exe
call_helper!() = 10
LIMIT          = 1
CEILING        = 2
bump!(x)       = 11（调用处的 x 仍是 10）
```

`call_helper!()` 能在定义它的模块之外解析到 `use defs::helper;`——因为函数名按调用处解析；`bump!(x)` 里的 `let x` 却是独立绑定，调用处的 `x` 不受影响。同一条卫生规则的两个方向，都在这份输出里。

**三、属性宏的组合与顺序**：`snippets/ch39/attr-demo` 与 `snippets/ch39/attr-demo-use` 组成一个小 workspace（`snippets/ch39/Cargo.toml`），两个 crate 都不引外部依赖，也不在 `examples/` 那个 workspace 里：

```console
$ cd snippets/ch39
$ cargo test
（编译期打印 39.2 里那串 [tag]/[Sniff] 日志；测试结果节选）
test tests::markers_exist ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
$ cargo run -q -p ch39-attr-demo-use
STACKED            = 1
OUTER_SAW          = OUTER_SAW
INNER_SAW          = INNER_SAW
RECORD_TAG         = RECORD_TAG
LATE_TAG           = LATE_TAG
SNIFFED_RECORD     = 3
SNIFFED_LATE       = 3
MADE_BY_FN_MACRO   = 2
FROM_FN_MACRO      = FROM_FN_MACRO
RULES_ITEM         = 4
FROM_RULES         = FROM_RULES
Record.id          = 7
Late.id            = 8
```

（`cargo run` 触发重新编译时会再打一遍上面那串日志；这里只贴程序自己的输出。）

运行期这份输出是另一条证据：声明宏和函数式宏「生成」的那些属性宏调用**都真的跑过**——否则 `FROM_RULES`、`FROM_FN_MACRO` 这些常量不存在，代码根本编译不过；`tests::markers_exist` 里的断言同理。

## 报错现场

### 现场一：stable 上用元变量表达式

最小复现（也就是 39.1 里那个 `vec!` 改写的本体）：

```rust,compile_fail
macro_rules! with_capacity_vec {
    ($($x:expr),*) => {{
        let mut v = Vec::with_capacity(${count($x)});
        $( v.push($x); )*
        v
    }};
}

fn main() {
    let v = with_capacity_vec!(1, 2, 3);
    println!("{v:?}");
}
```

以 Rust 1.98 为例：

```text
error[E0658]: meta-variable expressions are unstable
 --> main.rs:3:41
  |
3 |         let mut v = Vec::with_capacity(${count($x)});
  |                                         ^^^^^^^^^^^
  |
  = note: see issue #83527 <https://github.com/rust-lang/rust/issues/83527> for more information
```

**定位路径**：`E0658` 是「用了未稳定的语言特性」，note 里的 issue 编号就是 tracking issue。这里两个信号都很干净：`${…}` 整段被标成错误范围，且消息直接点名 `meta-variable expressions`。

**修法有三条**，按推荐顺序：stable 上换回逐条 `push`（或先用 `Vec::new()` 再补容量）；要真用这个语法就切 nightly 并加 `#![feature(macro_metavar_expr)]`（那意味着你的 crate 只能跑 nightly）；或者用 TT muncher 手写计数器——第 27 章的递归技巧，能跑但难读。还有一条最容易踩的边缘：写成 `${count(x)}`（少 `$`）**不会**得到上面的 E0658，而是 `meta-variables within meta-variable expressions must be referenced using a dollar sign` 加一条 `expected expression, found $`——别被这两条错误带偏，加 `$` 即可。

### 现场二：宏造出的 item 名不卫生

最小复现：

```rust,compile_fail
macro_rules! make_const {
    ($name:ident, $value:expr) => {
        const $name: u32 = $value;
    };
}

make_const!(LIMIT, 1);
make_const!(LIMIT, 2);

fn main() {
    println!("{LIMIT}");
}
```

以 Rust 1.98 为例：

```text
error[E0428]: the name `LIMIT` is defined multiple times
 --> main.rs:3:9
  |
3 |         const $name: u32 = $value;
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^
  |         |
  |         `LIMIT` redefined here
  |         previous definition of the value `LIMIT` here
...
8 | make_const!(LIMIT, 2);
  | --------------------- in this macro invocation
  |
  = note: `LIMIT` must be defined only once in the value namespace of this module
  = note: this error originates in the macro `make_const` (in Nightly builds, run with -Z macro-backtrace for more info)
```

**定位路径**：`E0428` 是重定义；诊断把两处定义都指出来了——第一处 span 落在宏体里的 `$name`（因为名字来自调用方，两次展开指向同一颗 token），第二处在调用点。note 里的 `originates in the macro` 提示这不完全是你的手写项冲突，而是宏展开的结果。

**修法**：让名字唯一。要么由调用方负责（传不同的名字，或者调用前自己加前缀），要么把每个常量塞进各自的作用域——比如放在不同的 `impl` 里当关联常量、或者放进各自的模块。宏侧想自己造唯一名字，就需要过程宏（`Ident::new`）或 nightly 的 `${concat(…)}`——这也是 39.1 里 `macro_metavar_expr_concat` 存在的理由。

### 现场三：宏体里的函数按调用处解析

最小复现（两个 crate 时同样成立，这里用单文件里的模块模拟）：

```rust,compile_fail
mod defs {
    pub fn helper() -> u32 {
        10
    }
}

macro_rules! call_helper {
    () => {
        helper()
    };
}

fn main() {
    println!("{}", call_helper!());
}
```

以 Rust 1.98 为例：

```text
error[E0425]: cannot find function `helper` in this scope
  --> main.rs:9:9
   |
 9 |         helper()
   |         ^^^^^^ not found in this scope
...
14 |     println!("{}", call_helper!());
   |                    -------------- in this macro invocation
   |
   = note: this error originates in the macro `call_helper` (in Nightly builds, run with -Z macro-backtrace for more info)
help: consider importing this function
   |
 1 + use crate::defs::helper;
   |
```

**定位路径**：`helper` 在宏定义处根本不存在，可 rustc 没有抱怨宏定义，而是抱怨**调用点**——因为展开式里的函数名按调用处解析（39.5 第二条）。`help` 那一段也很能说明问题：编译器建议你在**调用点**加 `use`。

**修法**：在调用点把路径引进来（`use defs::helper;`，正是 `snippets/ch39/hygiene_limits.rs` 里的做法）；更稳妥的是不让宏体依赖调用点的可见性——把路径写成 `$crate::` 开头的绝对路径（同一个 crate 内）或者让调用方把路径作为参数传进来（`$path:path`）。这也是 serde 让用户写 `#[serde(crate = "…")]` 的原因。

## 小结

- **能力边界是设计选择的结果。** 接口是 token 流，换来长期稳定；代价是宏看不见结构，连「重复了几次」都得靠新增语法（`macro_metavar_expr`）来补——而它到 1.98 仍在 nightly。
- **组合形态的顺序是确定的规则，不是玄学。** 同一 item 上的宏属性从外到内依次展开；每个宏看到的 `item` 仍带着排在它后面的属性（derive 也一样）；宏生成的属性宏调用会被重新扫描、照常展开。
- **`#[track_caller]` 把调用点变成隐式参数。** `Location::caller()` 沿未标注的帧向上走；trait 声明上标注对所有实现生效；一旦经过函数指针（包括「只在 impl 上标注」的 trait 对象），位置退化成被调函数的定义处。
- **诊断设计就是三件事：位置、数量、措辞。** `compile_error!` 的主 span 落在宏定义里、调用点作第二段标注，但它能一次报多条；精确的 span 控制与警告级别要等 nightly 的 `proc_macro_diagnostic`。
- **卫生性是分层的。** 声明宏只有局部变量、标签、`$crate` 按定义处解析，item 名不在此列；过程宏完全不卫生，`Span::def_site` 尚未稳定。可移植的宏体靠全路径与 `$crate::`。

## 练习

**1. 预测展开顺序与「谁看到什么」。** 下面这个 item 上，三个宏的调用顺序是什么？`#[tag(FIRST)]` 拿到的 `item` 里有什么？`Sniff` 的输入里又有什么？

```rust,ignore
#[tag(FIRST)]
#[derive(Sniff)]
#[tag(SECOND)]
struct S;
```

**2. 预测报告的位置。** 两个函数都在 `main` 里被调用，各自报告哪一行？（它们都返回 `Location::caller()`。）

```rust
use std::panic::Location;

#[track_caller]
fn located() -> &'static Location<'static> {
    Location::caller()
}

fn quiet() -> &'static Location<'static> {
    located()
}

#[track_caller]
fn loud() -> &'static Location<'static> {
    located()
}
```

**3. 找错并修。** 下面这段为什么报 E0428？给出两种可行的修法，并说明为什么不推荐「让宏自己改名」。

```rust,compile_fail
macro_rules! register {
    ($name:ident) => {
        const $name: &str = stringify!($name);
    };
}

register!(TIMEOUT);
register!(TIMEOUT);
```

<details><summary>参考答案</summary>

**1.** 顺序是 `FIRST` → `Sniff` → `SECOND`：属性宏按书写顺序从外到内展开，`derive` 是这条规则里的普通一员，按它出现的位置参与排队。`#[tag(FIRST)]` 的 `item` 是 **`# [derive(Sniff)] # [tag(SECOND)] struct S ;`**——排在它后面的两条属性都还在，且都还没展开。`Sniff` 的输入是 **`# [tag(SECOND)] struct S ;`**：`#[derive(Sniff)]` 自己已被消费，而它下面的 `#[tag(SECOND)]` 仍以 token 形式躺在输入里。要点是：**看到属性 ≠ 看到它的结果**。

**2.** `quiet()` 报告 `quiet` 函数体内调用 `located()` 的那一行；`loud()` 报告 `main` 里调用 `loud()` 的那一行。原因就是「沿栈向上走到最近的未标注帧」：`quiet` 自己没标注，走到它的帧就停住，位置固定在那一行、与调用处无关；`loud` 标注了，调用点信息继续外传，一直传到 `main` 里的调用点。

**3.** `register!` 造出的 `const TIMEOUT` 用的是调用方传进来的标识符，落在调用方模块的值命名空间里；调用两次同名就是两次定义同一个常量，因此 E0428。两种修法：（a）由调用方给不同的名字，或自己加前缀/放进各自的模块；（b）把常量放进不同的 `impl` 当关联常量，靠类型区分命名空间。不推荐「让宏自己改名」是因为声明宏没有任何拼接标识符的手段——`$name` 后面不能再粘 `_A` 这种新标识符，`${concat(…)}`（nightly）或过程宏的 `Ident::new` 才行。稳定的替代是把唯一性交还给调用方，或者把这类生成工作改成过程宏（第 28 章）。

</details>

## 延伸阅读

- Rust Reference：<https://doc.rust-lang.org/reference/procedural-macros.html>（三种过程宏、token 流、过程宏不卫生）
- Rust Reference，`track_caller` 属性：<https://doc.rust-lang.org/reference/attributes/codegen.html#the-track_caller-attribute>
- Rust Reference，`macro_rules!` 与卫生性：<https://doc.rust-lang.org/reference/macros-by-example.html#hygiene>
- `std::panic::Location`：<https://doc.rust-lang.org/std/panic/struct.Location.html>
- `proc_macro` crate 文档：<https://doc.rust-lang.org/proc_macro/>
- Unstable Book，`macro_metavar_expr`：<https://doc.rust-lang.org/unstable-book/language-features/macro-metavar-expr.html>
- RFC 3086（元变量表达式）：<https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html>
- 跟踪 issue #83527（元变量表达式现状）：<https://github.com/rust-lang/rust/issues/83527>
