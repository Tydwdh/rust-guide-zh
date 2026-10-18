# 项目 2：表达式求值器

项目 1 做的是**应用**：参数进、文件读写、退出码出。这一章做的是**库**：`examples/expr-eval/` 只负责把一条表达式字符串变成 `f64`，做不到时把「哪儿出了问题」讲清楚。零依赖、只用标准库；库根上写着 `#![forbid(unsafe_code)]`，所以「不用 unsafe」是编译器替你守的规矩。

第二篇（第 12–18 章）的概念在这里各有落点：trait 是唯一的扩展点，dyn 兼容性决定操作符怎么写，生命周期决定谁借用谁拥有，`Box` 决定树怎么摆，`#[non_exhaustive]` 与 `#[must_use]` 决定公开面长什么样。

## 这一章解决什么问题

### 要做的东西

一条表达式走三步：**词法分析**切记号，**语法分析**接成树，**求值**沿树算出数：

<svg viewBox="0 0 720 214" width="100%" role="img" aria-label="expr-eval 三步流水线：词法借源串，语法树拥有数据，求值查环境后得到 f64" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">一条表达式的三步：借用 → 拥有 → 求值</text>
<g fill="none" stroke="currentColor" stroke-width="1.5">
<rect x="8" y="36" width="150" height="58" rx="6" />
<rect x="190" y="36" width="170" height="58" rx="6" />
<rect x="392" y="36" width="150" height="58" rx="6" />
<rect x="574" y="36" width="138" height="58" rx="6" />
</g>
<g font-size="13" fill="currentColor" text-anchor="middle">
<text x="83" y="58">源串</text>
<text x="275" y="58">词法分析</text>
<text x="467" y="58">语法分析</text>
<text x="643" y="58">求值</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle" font-family="monospace">
<text x="83" y="80">&amp;str</text>
<text x="275" y="80">Lexer → Token&lt;'a&gt;</text>
<text x="467" y="80">Parser → Expr</text>
<text x="643" y="80">Env → f64</text>
</g>
<g stroke="var(--links)" stroke-width="2" fill="none">
<line x1="158" y1="65" x2="180" y2="65" />
<line x1="360" y1="65" x2="382" y2="65" />
<line x1="542" y1="65" x2="564" y2="65" />
</g>
<g fill="var(--links)">
<polygon points="190,65 180,60 180,70" />
<polygon points="392,65 382,60 382,70" />
<polygon points="574,65 564,60 564,70" />
</g>
<g font-size="12" fill="currentColor" text-anchor="middle">
<text x="83" y="114">调用者给的</text>
<text x="275" y="114">切片指回源串，不分配</text>
<text x="467" y="114">String / Box / Vec</text>
<text x="643" y="114">变量与函数查 Env</text>
</g>
<g font-size="12" fill="var(--links)" text-anchor="middle">
<text x="275" y="132">借来的</text>
<text x="467" y="132">自己的</text>
</g>
<text x="0" y="168" font-size="12" fill="currentColor">三步里任何一步出错，回来的都是同一个 Error，并带着字节偏移；src/bin/eval.rs 把它变成一行中文加一个脱字符，退出码 2。</text>
<text x="0" y="188" font-size="12" fill="currentColor" opacity="0.75">整个 crate 只用标准库：没有词法生成器，没有解析器组合子，没有正则。</text>
</svg>

公开面全在 `src/lib.rs` 的几行 `pub use` 里：

| 项 | 干什么 |
|---|---|
| `parse` / `Expr::eval` / `eval_str` | 源串 → 树 → 数值 |
| `Env` / `Bindings` / `call_builtin` | 变量从哪儿来、函数怎么调（`call_builtin` 给自定义环境兜底） |
| `Error` / `Result` | 三档错误与统一的 `Result` 别名 |
| `tokenize` / `Lexer` / `Token` / `TokenKind` | 词法层的公开视图（做工具时用） |
| `BinOp` / `Expr` | 树的形状 |

先看它跑起来的样子：

```console
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- "1 + 2 * 3"
7
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- --ast "1 + 2 * 3"
(1 + (2 * 3))
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- --var x=3 "x^2 + 1"
10
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- -2^2
-4
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- "sqrt(abs(-16))"
4
```

`--ast` 打印**完全括号化**的树，输出可以直接读回来。出错的三典型：

```console
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- "1 / 0"
错误：1 / 0 的结果不是有限数（除零、负数开方、上溢都会这样）
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- "2 + * 3"
错误：偏移 4 处应该是一个值，实际看到 `*`

2 + * 3
    ^
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- "x + 1"
错误：变量 `x` 没有绑定
```

### 验收标准

做完要满足下面五条，缺一条都算没做完：

1. `cargo run -p expr-eval -- "1 + 2 * 3"` 打印 `7`（在 `examples/` 里跑；仓库根出发写 `cargo run --manifest-path examples/Cargo.toml -p expr-eval -- …`）。`--ast` 打印树，`--var 名字=值` 绑变量（可重复），`-` 从标准输入读；
2. 出错一律给一行说得清位置的话并以退出码 2 结束，绝不 panic（嵌套超过 128 层报 `TooDeep`，不是把栈撑爆）：词法错、语法错、变量没绑定、函数不认识、参数个数不对、结果不是有限数，各有各的消息；
3. 公开面只有 `lib.rs` 那几行 `pub use`；库上标了 `#![forbid(unsafe_code)]`，`Cargo.toml` 的 `[dependencies]` 是空的；
4. `cargo test --manifest-path examples/Cargo.toml -p expr-eval` 全绿：单元测试、`tests/cli.rs` 的端到端测试、文档测试；
5. 正文里每段引用的代码都是 crate 里的原文，一个字不改。

本次验证用稳定版 rustc 1.98.0、edition 2024。下面所有终端记录都真跑过；只省掉 cargo 自己的 `Compiling` / `Finished` / `Running` 行，以及错误路径上它追加的 `error: process didn't exit successfully … (exit code: 2)`。

## 设计取舍

动手之前先定几件影响全局的事，每条都给「为什么不选另一个」。

**1. 词法阶段借用源串，语法树拥有自己的数据。** 标识符在记号里是切片（`TokenKind::Ident(&'a str)`），直接指向输入：词法分析除出错路径外零分配；到树里换成 `Expr::Var(String)`。分界线是「谁需要活得比源串长」：记号不必，树必须。让树也借用省一次拷贝，但 `Expr<'a>` 会传染整条公开 API；让记号也拥有则相反：每扫到一个名字分配一次，而它下一行就被丢掉。

**2. 操作符是枚举，优先级是一张表。** 五种中缀操作符写死在这门小语言的语法里，所以用 `enum BinOp`；「谁比谁紧」不写进文法，挂在操作符身上：

| 操作符 | 左绑定力 | 右绑定力 | 得到什么 |
|---|---|---|---|
| `+` `-` | 1 | 2 | 左结合：`1 - 2 - 3` = `(1 - 2) - 3` |
| `*` `/` | 3 | 4 | 左结合，比加减紧 |
| 前缀 `-` `+` | — | 5 | 比 `*` 松、比 `^` 紧 |
| `^` | 7 | 7 | 右结合：`2 ^ 3 ^ 2` = `2 ^ (3 ^ 2)` |

空档是故意的：以后要插新操作符，位置已经留好。不做成 trait 对象表，是因为那要给每个操作符一个类型、给 trait 一个关联类型，再在 `dyn` 上把它钉死（`dyn Infix<Value = f64>` —— 第 13 章：trait 对象必须知道关联类型的具体值，否则定不出大小、排不出 vtable）。**能被枚举穷举的，优先枚举**；哪天允许用户注册操作符，再反过来。

**3. 子节点装在 `Box` 里，不上 `Rc`。** `Expr` 直接包含 `Expr`，没有间接层算不出大小；`Box` 是够用的那一层。树单向、无共享、无环，`Rc` 只会白白加计数还丢掉「谁拥有这棵树」；求值只读也让 `RefCell` 用不上（第 17 章）。

**4. 解析用 Pratt，而不是按优先级层层递归。** 经典递归下降要写 `expr` / `term` / `factor` 三层函数，加一级优先级就得加一个函数；Pratt 把「谁更紧」压成两个数字，解析器只剩一个循环加一个前缀函数。代价是先得接受「绑定力」这个约定 —— 它写进了 `BinOp::binding_power` 的文档注释。

**5. 一个错误枚举装下三档错误，另给一个 `offset()`。** 拆成三个类型只会逼出一层没用的 `match`：调用者几乎总是做同一件事（打印给人看）。`offset()` 把「哪个变体带位置」留在库里，调用者拿 `Option<usize>` 就能画出脱字符；枚举第一天就标 `#[non_exhaustive]`，`Result<T, E = Error>` 让签名短一截（第 18 章）。数值约定同理：`1 / 0` 在 `f64` 里是 `inf`，我们一律报 `NotFinite` —— 用户要的是一句话，不是 `inf`。

**6. 扩展点只有 `Env` 一个。** `var` 查变量、`call` 调函数（默认即内置那五个）。查询参数收 `&str`：调用者手上本来就有这个借用。

## 逐块实现

下面每块都是从 `examples/expr-eval/src/*.rs` 切出来的原文，按依赖顺序：错误类型 → 词法 → 语法树 → 语法分析 → 求值 → 公开面 → CLI。

> [!NOTE]
> 这些块大多是跨模块的片段：`mdbook test` 把每个块当独立程序编译，单独的片段自然编译不过（例如 `impl Error` 那块没有 `Error` 的定义）。所以大部分标了 `ignore`，片段本身与文件逐字一致；整份代码由下面 `cargo test` 真正跑过。

### 1. 错误类型：`src/error.rs`

先把可能出错的地方列全 —— 第 7 章的老习惯：

```rust
{{#include ../examples/expr-eval/src/error.rs:8:69}}
```

八个变体按阶段分三组：词法 2（不认识的字符、坏数字）、语法 2（位置不对的记号、嵌套太深）、求值 4（未绑定、未知函数、参数个数、非有限数）。带字节偏移的只有前三个，所以另给一个访问器，把「哪些带位置」这件事收在库里：

```rust,ignore
{{#include ../examples/expr-eval/src/error.rs:71:88}}
```

`Display` 把变体渲染成人话，每句都带够定位的上下文：

```rust,ignore
{{#include ../examples/expr-eval/src/error.rs:90:117}}
```

`UnexpectedToken` 的 `expected` 是 `&'static str`（解析器写死的短句），`found` 是 `String`（记号文本或「输入的末尾」）：能静态的都静态、只有来自输入的部分才拥有，是错误类型里常见的形状。

**这块用到第 7 章的自定义错误与 `?`、第 9 章的 `Display`、第 18 章的 `#[non_exhaustive]` 与 `Result` 别名。**

### 2. 词法分析：`src/lexer.rs`

记号的种类是一张封闭的表。`Num` 里存的已经是 `f64` 而不是原文，数字的合法性在词法阶段就判完：

```rust
{{#include ../examples/expr-eval/src/lexer.rs:10:36}}
```

`Token` 在种类之外多带一个字节偏移，并派生 `Copy`：

```rust,ignore
{{#include ../examples/expr-eval/src/lexer.rs:55:65}}
```

`Copy` 不是随手加的：解析器要「看一眼再决定吃不吃」，若记号只能通过引用看，借用会活到决定之后、和「吃掉它」那步撞车 —— 见报错现场一。扫描数字的规则：`.5` 和 `1.` 都收，`e`/`E` 后面必须真跟着（可带符号的）数字，否则整段算作坏的数字字面量：

```rust,ignore
{{#include ../examples/expr-eval/src/lexer.rs:91:137}}
```

最后那个 `else` 对应 `2e`、`1e+`：`2e` 本可静默切成 `2` 加变量 `e`，但用户这么写时想的多半是「指数没写全」，所以宁可报错 —— 把「贪心吸收 `e`」写成规则，而不是让解析器去猜。标识符的扫描短得多，借用就发生在这里：

```rust,ignore
{{#include ../examples/expr-eval/src/lexer.rs:139:151}}
```

`Lexer` 不自己切完整个输入，它是个迭代器，由解析器拉着走：

```rust,ignore
{{#include ../examples/expr-eval/src/lexer.rs:170:189}}
```

`type Item = Result<Token<'a>>` 是我们自己的类型第一次填关联类型 —— 只不过填的是标准库的 trait。想一次切完的人用外面那个便捷函数：

```rust,ignore
{{#include ../examples/expr-eval/src/lexer.rs:191:207}}
```

**这块用到第 4 章的切片与借用、第 5 章的枚举与 `match`、第 9 章的 `FromStr`（`parse::<f64>`）、第 10 章的 `Iterator` 与关联类型。**

### 3. 语法树：`src/ast.rs`

`BinOp` 除了变体，还带着它在解析器里的坐标（绑定力）和在 `Display` 里的写法：

```rust
{{#include ../examples/expr-eval/src/ast.rs:5:47}}
```

两个数字这样用：**左**绑定力决定「还进不进主循环」，**右**绑定力决定「右操作数解析到哪一档停」。右比左大 1 就是左结合 —— `+` 是 (1, 2)，所以 `1 - 2 - 3` 里第二个 `-` 不会粘到右边的 `3` 上；`^` 是 (7, 7)，同一个操作符允许继续往右套，于是 `2 ^ 3 ^ 2` = `2 ^ (3 ^ 2)`。

节点本身：

```rust,ignore
{{#include ../examples/expr-eval/src/ast.rs:49:78}}
```

`Neg` 用元组变体，`Binary` 与 `Call` 用带名字的字段；`Call` 的实参是 `Vec<Expr>`，顺序即书写顺序，个数交给求值阶段。树还要能打印，而且要打印得**能读回来**：

```rust,ignore
{{#include ../examples/expr-eval/src/ast.rs:80:97}}
```

每个运算都套括号，于是 `ast.to_string()` 的结果是合法表达式，重新解析得到同一棵树。这个性质不只是好看：优先级测试因此可以只比较字符串，不必把文法再抄一遍。

**这块用到第 5 章的枚举与模式、第 6 章的 `Vec` 与 `String`、第 9 章的 `Display`、第 17 章的 `Box`。**

### 4. 语法分析：`src/parser.rs`

解析器对外只有一个函数：解析到底，并且要求输入被用干净：

```rust,ignore
{{#include ../examples/expr-eval/src/parser.rs:24:46}}
```

结尾那个 `if let Some(tok) = …` 是「尾巴要收干净」的检查：`1 2` 会在第二个记号上停下来，而不是悄悄忽略后半截。操作符表是一张 `match`：

```rust,ignore
{{#include ../examples/expr-eval/src/parser.rs:48:58}}
```

主循环就是整个优先级机制；每一层进来先查深度上限 —— 递归下降每进一层括号或前缀号都要多压几帧调用栈，`((((1))))` 写到几百层能把栈撑爆（那时进程直接 abort，连 `Result` 都来不及给）：

```rust,ignore
{{#include ../examples/expr-eval/src/parser.rs:106:137}}
```

拿 `1 + 2 * 3` 走一遍：`expr_bp(0)` 取到 `1`，看到 `+` 的 (1, 2)，`1 ≥ 0` 所以吃进来，右操作数按 `min_bp = 2` 继续；那一层取到 `2`，看到 `*` 的 (3, 4)，`3 ≥ 2` 也吃进来，于是先算出 `2 * 3`，回到外层才拼成 `1 + (2 * 3)`。反过来，内层若遇到更松的操作符（`min_bp = 2` 时遇到 `-` 的 1），`1 < 2`，循环当场退出，把它留给外层。

前缀位置复杂一点，因为它要分辨「变量」和「函数调用」：

```rust,ignore
{{#include ../examples/expr-eval/src/parser.rs:139:173}}
```

`-` 用 `PREFIX_BP`（5）继续解析操作数：比 `*` 紧，所以 `-2 * 3` 是 `(-2) * 3`；比 `^` 松，所以 `-2 ^ 2` 是 `-(2 ^ 2)`，等于 `-4`。这两个例子在测试里各占一条。括号就是「递归调用 `expr_bp(0)`」，因为括号里是一个完整的表达式。实参列表单独一个函数：

```rust,ignore
{{#include ../examples/expr-eval/src/parser.rs:175:209}}
```

进来时 `(` 已被吃掉，出去时 `)` 已被吃掉 —— 把「谁负责吃掉哪个记号」写进文档注释，是解析器里最省事的约定。空参数表（`f()`）也收，个数交给求值阶段判断：语法不该替语义做决定。

**这块用到第 3 章的函数与控制流、第 5 章的 `match` 与 `let … else`、第 10 章的 `Peekable`、第 13 章的「能被枚举穷举的，优先枚举」。**

### 5. 求值：`src/eval.rs`

求值环境是 crate 唯一的扩展点：

```rust,ignore
{{#include ../examples/expr-eval/src/eval.rs:9:24}}
```

两个方法都收 `&str`，`call` 还带默认实现：什么都不做的实现者照样能用内置函数；想加自己的函数，就在 `call` 里认完自己的名字，最后退回 `call_builtin`。内置函数表：

```rust,ignore
{{#include ../examples/expr-eval/src/eval.rs:67:101}}
```

`match (name, args)` 用了切片模式，一个分支同时表达「名字是 `abs`」和「恰好一个参数」；「名字对得上、个数不对」由下面两组分支专门接住，最后才是「这名字我根本不认识」。`Bindings` 是一张 `HashMap` 的包装：

```rust,ignore
{{#include ../examples/expr-eval/src/eval.rs:26:65}}
```

`with` 收 `impl Into<String>` 而不是 `&str`：它要把名字**存起来**，所以必须拿到所有权，但没必要逼调用者写 `.to_owned()`。返回值标了 `#[must_use]` —— 写在链上的方法，丢掉结果等于什么都没做（第 18 章）；`From<HashMap<String, f64>>` 让 CLI 攒完表后直接 `.into()`。数值运算收口在一个卫兵里：

```rust,ignore
{{#include ../examples/expr-eval/src/eval.rs:103:124}}
```

`finite` 是唯一的卫兵：把 `NaN` 和 `±inf` 变成 `NotFinite`，消息里带上出问题的运算（用户看不懂 `NaN`，但看得懂 `sqrt(-1)`）。最后是遍历本身：

```rust,ignore
{{#include ../examples/expr-eval/src/eval.rs:126:157}}
```

`match self` 是共享借用上的匹配，`op.apply(...)` 通过自动解引用拿到 `BinOp`（它是 `Copy`）。`eval` 收 `&dyn Env` 而不是 `impl Env`：求值是递归的，写成泛型会为每个环境类型各编译一份；而那次间接调用落在每个节点上各一次，跟递归本身的成本比可以忽略（第 14 章的取舍，这里选 dyn 那一侧）。

**这块用到第 4 章的借用、第 5 章的切片模式、第 6 章的 `HashMap`、第 9 章的 trait 与默认方法、第 13/14 章的 `dyn` 与 `impl Trait`、第 18 章的 `#[must_use]` 与参数类型选择。**

### 6. 公开面：`src/lib.rs`

模块全是私有的，能看到什么由这一个文件说了算：

```rust,ignore
{{#include ../examples/expr-eval/src/lib.rs:22:32}}
```

想让某个内部类型「顺便公开」，得先动这个文件 —— `pub use` 那一步会逼你想一下「它凭什么是对外 API」。根上还有两句属性：

```rust,ignore
{{#include ../examples/expr-eval/src/lib.rs:19:20}}
```

`#![warn(missing_docs)]` 让漏写文档注释在编译时就吱声；`#![forbid(unsafe_code)]` 是本项目的边界，写进代码比写在文档里可靠 —— 想破例的人必须先删掉这一行。

### 7. 薄 CLI：`src/bin/eval.rs`

二进制是同一份 `Cargo.toml` 里的另一个 crate，对库来说就是普通使用者。它有自己的错误类型，把「库的错误」和「进程的 I/O」装在一起：

```rust,ignore
{{#include ../examples/expr-eval/src/bin/eval.rs:13:47}}
```

两个 `From` 实现是 `?` 的燃料：`read_to_string(...)?` 借 `From<io::Error>`，`eval_str(...)?` 借 `From<EvalError>` —— 库只说清「表达式错在哪儿」，翻译成人话、决定退出码是应用的事（第 18 章 §18.1）。主体只做派发：

```rust,ignore
{{#include ../examples/expr-eval/src/bin/eval.rs:89:122}}
```

`--ast` 与求值走两个分支，但都返回 `Result<String, Error>`，因为两边各自把结果变成了要打印的那一行 —— 打印只发生在一个地方。参数解析沿用项目 1 的约定：选项只认表达式之前的位置，第一个不是选项的参数之后全是表达式文本：

```rust,ignore
{{#include ../examples/expr-eval/src/bin/eval.rs:124:162}}
```

最后是那个脱字符 —— 库给的是**字节偏移**，壳负责把它换算成屏幕上的**列**（宽字符占两列、制表符推进到下一个 8 的倍数）：

```rust,ignore
{{#include ../examples/expr-eval/src/bin/eval.rs:174:207}}
```

**这块用到第 7 章的退出码与错误转换、第 8 章的二进制目标、第 9 章的 `From`、第 18 章的「库错误与应用错误分开」。**

### 概念落点对照

| 章 | 概念 | 落点 |
|---|---|---|
| 12 | trait 设计（默认方法、扩展点） | `Env` 的 `call` 默认实现；`From<HashMap<String, f64>>` 构造 |
| 13 | dyn 兼容性与 trait 对象 | `&dyn Env`；`dyn Infix<Value = f64>` 必须钉住关联类型；类型参数会传染签名，所以 AST 不泛型化 |
| 14 | `impl Trait` 与不透明类型 | 参数位置收 `impl Into<String>`；`eval` 收 `&dyn Env` |
| 15 | 泛型的边界（const 泛型、GAT） | 用不上：没有定长缓冲，也没有要借出 `&self` 的迭代器 |
| 16 | 生命周期进阶 | `Token<'a>` 与 `Lexer<'a>` 的协变；`peek_token` 返回拥有值 |
| 17 | 智能指针与内部可变性 | `Box<Expr>`；为什么不用 `Rc`/`RefCell` |
| 18 | 错误与 API 设计 | `#[non_exhaustive]`、`Result<T, E = Error>`、`#[must_use]`、`offset()` |

## 报错现场

### 现场一：看一眼的同时把它吃掉（E0499）

解析器要「peek 一下，再决定吃不吃」。最直觉的写法是这样：

```rust,compile_fail
use std::str::Chars;

#[derive(Debug, Clone, Copy)]
enum Token {
    Ident,
    Plus,
}

struct Tokens<'a>(Chars<'a>);

impl<'a> Iterator for Tokens<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        self.0
            .next()
            .map(|ch| if ch == '+' { Token::Plus } else { Token::Ident })
    }
}

fn main() {
    let mut tokens = Tokens("a+".chars()).peekable();
    if let Some(tok) = tokens.peek() {
        let taken = tokens.next();
        println!("{tok:?} {taken:?}");
    }
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0499]: cannot borrow `tokens` as mutable more than once at a time
  --> main.rs:24:21
   |
23 |     if let Some(tok) = tokens.peek() {
   |                        ------ first mutable borrow occurs here
24 |         let taken = tokens.next();
   |                     ^^^^^^ second mutable borrow occurs here
25 |         println!("{tok:?} {taken:?}");
   |                    --- first borrow later used here
```

`Peekable::peek` 要 `&mut self`（它得先把记号取出来存着），返回的引用活到第 25 行，而第 24 行又要一次可变借用 —— 问题不在 `peek`，而在「借来的引用要跨过下一步修改」这个模式。修法两种：让借用早点结束（先拷贝一份再看），或者把拷贝藏进接口 —— 本项目选后者：`Token` 派生 `Copy`，`peek_token` 返回**拥有值**而不是引用，借用在那次调用结束时就还回去了；代价是每个记号多复制十几个字节。

### 现场二：记号的借用不能比源串活得久（E0597）

借用源串有个直接后果：源串活不下来，记号也活不下来。下面这段想「在局部把表达式处理完」：

```rust,compile_fail
#[derive(Debug)]
struct Token<'a> {
    ident: &'a str,
}

fn lex(src: &str) -> Vec<Token<'_>> {
    vec![Token { ident: src }]
}

fn main() {
    let tokens = {
        let owned = String::from("x");
        lex(&owned)
    };
    println!("{}", tokens.len());
}
```

以 Rust 1.98 为例：

```text
error[E0597]: `owned` does not live long enough
  --> main.rs:13:13
   |
11 |     let tokens = {
   |         ------ borrow later stored here
12 |         let owned = String::from("x");
   |             ----- binding `owned` declared here
13 |         lex(&owned)
   |             ^^^^^^ borrowed value does not live long enough
14 |     };
   |     - `owned` dropped here while still borrowed
```

定位路径看第一行注释「borrow later stored here」：`vec!` 把切片存进了结果，结果又要活到第 15 行，切片的借用要求因此被拉到源串之外 —— 设计真实的约束是**词法切片不能比源串活得久**。修法三条：把源串放进更长命的作用域；立刻把名字复制成 `String`（`Expr::Var` 干的就是这件事）；或者让词法层直接产出拥有所有权的记号（放弃零分配）。选哪条取决于数据下一步要活多久 —— 这正是第 4 章所有借用规则背后的那个问题。

### 现场三：两条值得认识的警告

第一条来自 `#[must_use]`，它挡住的是「链式写了一半」这种错：

```rust,ignore
use expr_eval::Bindings;

fn main() {
    let _ok = Bindings::new().with("x", 1.0);
    // 忘了接住返回值：绑定没生效。
    Bindings::new().with("y", 2.0);
}
```

```text
warning: unused return value of `Bindings::with` that must be used
 --> src\main.rs:6:5
  |
6 |     Bindings::new().with("y", 2.0);
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: with 返回更新后的绑定表，丢掉它等于没绑定
```

第二条和生命周期有关，但**不是 edition 的问题**：下面这个 `fn lex(src: &str) -> Vec<Token>` 能编译通过，只给一条警告 —— 2015/2018/2021/2024 四个 edition 的输出完全一样。

```rust,ignore
#[derive(Debug)]
struct Token<'a> {
    ident: &'a str,
}

fn lex(src: &str) -> Vec<Token> {
    vec![Token { ident: src }]
}

fn main() {
    println!("{}", lex("x").len());
}
```

```text
warning: hiding a lifetime that's elided elsewhere is confusing
 --> main.rs:6:13
  |
6 | fn lex(src: &str) -> Vec<Token> {
  |             ^^^^         ^^^^^ the same lifetime is hidden here
  |             |
  |             the lifetime is elided here
  |
  = help: the same lifetime is referred to in inconsistent ways, making the signature confusing
  = note: `#[warn(mismatched_lifetime_syntaxes)]` on by default
help: use `'_` for type paths
  |
6 | fn lex(src: &str) -> Vec<Token<'_>> {
  |                               ++++
```

这条 lint（`mismatched_lifetime_syntaxes`）从 1.89 起默认打开，管的是「同一件事写成了两种样子」：参数与返回类型都省略了生命周期，读的人不容易把两者联到一起。E0106 只在省略规则判不出来时才出现 —— 两个输入生命周期 `fn lex(a: &str, b: &str) -> Vec<Token>` 就会让编译器问「借的是 `a` 还是 `b`」。本项目仍写成 `Vec<Token<'_>>`：写出来才知道这里挂着一个借用。

## 测试

测试分三层，对应三种「什么算对」：

- **单元测试**在各模块的 `#[cfg(test)] mod tests` 里：词法层断言记号序列与偏移（含用 `std::ptr::eq` 验的「切片指着源串内存」）；语法层断言框起来的字符串；求值层断言数值、错误消息与 `offset()`；CLI 里单测列宽换算；
- **端到端测试**在 `examples/expr-eval/tests/cli.rs`，把编译出来的 `eval` 当黑盒跑：

```rust,ignore
{{#include ../examples/expr-eval/tests/cli.rs:21:34}}
```

```rust,ignore
{{#include ../examples/expr-eval/tests/cli.rs:43:60}}
```

```rust,ignore
{{#include ../examples/expr-eval/tests/cli.rs:62:76}}
```

`env!("CARGO_BIN_EXE_eval")` 是 cargo 预置给集成测试的环境变量，指向刚编好的二进制；`.output()` 一次拿到退出码与两路输出 —— 脱字符落在哪一列，只有看屏幕才算测过。

- **文档测试**在几个模块的文档注释里，覆盖公开面的典型用法。

（调试时两个手段：`cargo test -- --nocapture` 看打印；把出错的表达式原样贴给 `eval`，看脱字符落在哪儿。）

跑一遍：

```console
$ cargo test --manifest-path examples/Cargo.toml -p expr-eval

running 39 tests
test ast::tests::display_puts_the_minus_inside_its_own_parentheses ... ok
test error::tests::unexpected_char_reports_the_offset_and_the_char ... ok
test eval::tests::division_by_zero_and_other_non_finite_results_are_errors ... ok
test lexer::tests::full_width_spaces_are_whitespace_and_offsets_stay_byte_based ... ok
test parser::tests::nesting_past_the_limit_is_an_error_instead_of_a_crash ... ok
test parser::tests::long_left_associative_chains_stay_iterative ... ok
…
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 3 tests
test tests::ascii_prefixes_count_one_column_each ... ok
test tests::wide_characters_count_two_columns ... ok
test tests::a_tab_advances_to_the_next_tab_stop ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 11 tests
test evaluates_an_expression_given_as_several_arguments ... ok
test a_wide_space_does_not_shift_the_caret_column ... ok
test deeply_nested_input_is_an_error_not_a_stack_overflow ... ok
…
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

running 6 tests
test expr-eval\src\parser.rs - parser::parse (line 28) ... ok
…
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

（取自一次真跑；顺序每次可能不同，`…` 处都是 `ok`。合计：库单元测试 39 个、CLI 单元测试 3 个、端到端 11 个、文档测试 6 个。）三类路径都有覆盖：

- **正常路径**：优先级、括号、调用、变量、内置函数、三种入口、`Display` 往返；
- **边界**：`.5` 与 `1.`、指数写法、空参数表、`+-1`、全角空白的偏移与列宽、`2 ^ 1024` 溢出、2000 项链与 64 层括号仍能解析；
- **错误路径**：不认识的字符、坏数字、缺操作数、缺 `)`、多余记号、缺逗号、未知函数、参数个数不对、变量未绑定、除零、嵌套超限 —— 分别断言消息、`offset()` 与退出码 2。

调试时两个手段：`cargo test -- --nocapture` 看打印；把出错的表达式原样贴给 `eval`，看脱字符落在哪儿。

## 收尾：用法、错误与下一步

### 用法

```console
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- --help
eval —— 算一条表达式

用法：
    eval [选项] <表达式>
    eval [选项] -            从标准输入读表达式

选项：
    --ast               只打印语法树，不求值
    --var <名字>=<值>    绑定一个变量，可以写多次
    --help, -h          打印这份说明

表达式：
    数字、变量、+ - * / ^、括号、一元正负号
    函数：abs(x)  sqrt(x)  pow(x, y)  min(x, y)  max(x, y)

退出码：
    0  算出来了
    2  参数或表达式有问题（错误信息写到标准错误）
```

表达式可以拆成多个参数写（`eval 1 + 2` 会被用空格拼成一条 —— 参数里有 `*` 这类 shell 元字符时按 shell 的规矩加引号），也可整条用引号包起来；`-` 表示从标准输入读，方便接管道：

```console
$ echo "2 ^ 10" | cargo run --manifest-path examples/Cargo.toml -p expr-eval -- -
1024
```

### 错误与退出码

所有失败都走同一条路：`run` 把它们收进 `CliError` → 打印 → 退出码 2（`?` 只在参数解析与读标准输入两处做自动转换）。真跑出来的几种：

```console
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- "pow(2)"
错误：函数 `pow` 需要 2 个参数，实际给了 1 个
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- "1 @ 2"
错误：偏移 2 处有不认识的字符 `@`

1 @ 2
  ^
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval -- --var
错误：参数错误：`--var` 后面要跟 `名字=值`
$ cargo run --manifest-path examples/Cargo.toml -p expr-eval --
错误：参数错误：没给表达式
```

有位置的错误多打两行（源串和脱字符）；没有位置的只有一行 —— 它跟「哪一列」无关。

### 怎么继续扩展

想接着练，最直接的一步是加一个操作符（比如取余 `%`）：`BinOp` 变体、绑定力、求值分支、词法表四处都要动，漏掉哪处，编译器都会用「match 没穷尽」把你按回原地。再往前一步是给值加类型（`Value::Num` / `Value::Bool`）：解析、求值、错误都得跟着长，那正是 `#[non_exhaustive]` 派上用场的地方。

完整代码在 `examples/expr-eval/`，跑法只有一句：`cargo test --manifest-path examples/Cargo.toml -p expr-eval`。

## 小结

- 谁活得更久，谁就拥有数据：记号借用源串（零分配），树拥有自己的 `String`。
- 优先级是数据，不是文法层级：每个操作符两个数，解析器只有一个循环。
- 能被枚举穷举的优先枚举：操作符用枚举，求值环境用 trait（第 13 章）。
- `Box` 是递归类型最小的间接层；没有共享时，`Rc` 只会白白计数。
- 错误是一个枚举加一个 `offset()`；公开面收在一个文件里，库与应用各有各的错误类型。

## 练习

1. 本项目里 `Token<'a>` 借用源串、`Expr` 拥有 `String`。如果把两者对调 —— 记号拥有 `String`、语法树借用（`Expr<'a>`）—— 你会多付什么、少付什么？请从「分配次数」「公开 API 上的生命周期」「把树存下来」三个角度说。
2. `-2 ^ 2` 求出来是 `-4`。把 `PREFIX_BP` 从 5 依次改成 8、3、1，各会改变哪些表达式的解析结果？每种情况举一个例子（值变了，或者值没变但树变了）。
3. `BinOp::apply` 把「结果不是有限数」一律报成 `NotFinite`。如果想让 `1 / 0` 报一个专门的「除以零」、而 `sqrt(-1)` 仍报 `NotFinite`，最小改法是什么？这样做值得吗？

<details><summary>参考答案</summary>

**1.** 多付的是**分配**：每个标识符在词法阶段就要复制一次（数字此时已经变成 `f64`，不涉及字符串），`Lexer` 从「零分配」变成「每个记号一次分配」。少付的是**语法树上的生命周期参数**：树不再借用源串，可以存进结构体、从函数返回、塞进 `Vec` 长期保留。反过来（树借用）代价对调：拷贝降到零，但 `Expr<'a>` 会让 `parse`、`eval`、CLI 里每个 `let` 都带上 `'a`，连「打印出来再读回来」也得先保证源串活着。本项目输入是一次性字符串、结果要活得更久，所以选「记号借用、树拥有」；若这个库要服务「同一份表达式反复求值」（缓存 AST）而源串常驻，树借用更划算。

**2.** `PREFIX_BP` 是前缀负号解析操作数时的 `min_bp`，够了才会把中缀操作符吃进操作数里。**改成 8**（比 `^` 的左绑定力 7 还大）：`^` 不再够紧，`-2 ^ 2` 变成 `(-2) ^ 2` = `4`（`-2 ^ 2 ^ 3` 也随之变成 `((-2) ^ (2 ^ 3))` = 256）；`-1 + 1` 和 `-2 * 3` 都没变。**改成 3**（和 `*` 的左绑定力持平）：`*` 不再比它紧，`-2 * 3` 的树从 `((-2) * 3)` 变成 `-(2 * 3)`（值仍是 `-6`，树变了）；`-1 + 1` **仍然不变** —— `+` 的左绑定力是 1，比 3 小，负号照样只吃 `1`。**改成 1**（不比 `+` 的 1 紧）：`+` 也被吃进去，`-1 + 1` 变成 `-(1 + 1)` = `-2`，`-1 - -1` 变成 `-(1 - (-1))` = `-2`。这几个数字直接决定语言长什么样 —— 改之前先用测试把预期钉住（上面每个取值都真跑过）。

**3.** 最小改法是给除零单独加一个变体，并在 `apply` 里先判：`BinOp::Div` 且除数为零时返回 `Error::DivisionByZero`，其余分支照旧走 `finite`。`finite` 拦的是「算完才发现不是有限数」（负数开方、上溢）；除零**算之前**就能知道，所以放在前面，`0 / 0` 也因此归到「除以零」。值不值得看调用者：只是一律打印的话，收益是消息更准；要做「除以零就显示 ∞」这类交互就必须区分变体 —— 第 18 章的判断标准是「调用者会不会按种类分流」。粒度原则：**能预先判定的原因值得有自己的变体**；只能事后从结果里看出来的，共用一个就够。

</details>

## 延伸阅读

- The Rust Reference《Expressions》（语言自己的运算符与优先级表，可以跟本项目那张表对照）：<https://doc.rust-lang.org/reference/expressions.html>
- `std::iter::Peekable`（解析器里那个「看一眼」）：<https://doc.rust-lang.org/std/iter/struct.Peekable.html>
- `std::str::FromStr`（`"1.5".parse::<f64>()` 背后）：<https://doc.rust-lang.org/std/str/trait.FromStr.html>
- `f64`（`is_finite`、`powf`、`abs` 这些方法）：<https://doc.rust-lang.org/std/primitive.f64.html>
- `std::boxed::Box`（递归类型的那一层间接）：<https://doc.rust-lang.org/std/boxed/struct.Box.html>
- `std::collections::HashMap`（`Bindings` 的底层）：<https://doc.rust-lang.org/std/collections/struct.HashMap.html>
- `std::error::Error` 与 `std::process::ExitCode`：<https://doc.rust-lang.org/std/error/trait.Error.html>、<https://doc.rust-lang.org/std/process/struct.ExitCode.html>
- The Rust Reference《Type system attributes》（`#[non_exhaustive]`）与《Diagnostics attributes》（`#[must_use]`）：<https://doc.rust-lang.org/reference/attributes/type_system.html>、<https://doc.rust-lang.org/reference/attributes/diagnostics.html>
- The Cargo Book《Workspaces》（`examples/Cargo.toml` 这种多 crate 布局）：<https://doc.rust-lang.org/cargo/reference/workspaces.html>
