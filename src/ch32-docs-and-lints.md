# 第 32 章 文档与 lint 工程

## 这一章解决什么问题

前 31 章讲语言本身。这一章换一条线，讲两个围着代码转的工程设施：**rustdoc**（把注释变成文档，顺手把文档里的例子变成测试）和 **lint**（编译器的静态检查项，以及怎么配置它们的级别）。

第一类是「库写完了，别人怎么用」。如果你的 `pub` 面只有签名没有说明，使用者只能去读实现；如果你的文档里写着「这样调用」，半年后 API 改了文档没改，那段示例就在骗人。rustdoc 把这两件事绑在一起：文档与代码同源，示例是测试。

第二类是「什么算错不该只靠口头约定」。`a == a` 这种写法、`for i in 0..v.len()` 这种下标循环、`unwrap()` 到处飞——编译器都看得出来。lint 就是把这些判断做成可配置、可门禁的规则，让「我们的规矩」落在工具上，而不是落在人身上。

读完本章你应当能：

- 写 rustdoc 注释（`///`、`//!`、`#[doc]`），并用 intra-doc link 把文档连成一张网；
- 用隐藏行 `#`、`compile_fail`、`should_panic`、`no_run` 这些标注控制「文档里的例子怎么被验证」；
- 给库配上文档门禁：`missing_docs` + `rustdoc::broken_intra_doc_links` + `RUSTDOCFLAGS="-D warnings"`；
- 说清 `#[doc(cfg)]` 为什么在 stable 上不能直接用，以及 docs.rs 上的 `cfg_attr(docsrs, …)` 惯用法；
- 按 clippy 的分组做取舍，而不是「全开」或「全关」；
- 判断一条 lint 配置该写进源码、`[lints]` 表还是 CI 脚本里，并说得出 `deny(warnings)` 不该进库的理由。

**边界**：`[lints]` 表的语法细节、workspace 级继承与 `priority` 在第 30 章；测试的组织方式（单元测试、集成测试、文档测试怎么配合，以及基准测试）在第 31 章；Edition 迁移会牵动一批 lint 组，见第 33 章。本章只讲文档与 lint 这两条线本身。

## 为什么这样设计

**一、文档和代码同源。** rustdoc 不解析某种独立的文档格式，它直接读源码里的文档注释（doc comment）。文档注释里的代码块也不只是排版——它们进入测试集合，由 `cargo test` 编译并运行。rustdoc 官方书的说法是：

> rustdoc supports executing your documentation examples as tests. This makes sure that examples within your documentation are up to date and working.
> （引自 The rustdoc book）

这句是本章其它一切的前提。如果文档是另一份文件（wiki、独立站点），它不会因为代码改坏而报错，只会静静地过期；而把「示例还对吗」变成一条会失败的测试，文档腐化就从一个没人负责的问题变成了一个确定事件。

**二、lint 是分级、分组的。** 每条 lint 有级别：`allow`（不报）、`warn`（警告，继续编译）、`deny`（报错）、`forbid`（报错且禁止在更内层改回去）；还有专门的 `force-warn`。级别之外还有**组**：把同一倾向的 lint 打包（`clippy::style`、`clippy::pedantic`……），你可以整组配置，也可以单条覆盖。

这套分级不是在凑语法糖，它编码的是「一条 lint 有多确定」。看 clippy 的默认级别就明白了：`clippy::correctness` 组默认 `deny`，官方书对它的解释是这类 lint 命中时「your code is outright wrong or useless」；`clippy::style` 组默认 `warn`，因为「style is subjective」；`clippy::pedantic` 组默认 `allow`，因为它是刻意激进的。作者们把「有多确定」的判断写进了默认值，你只需要在这个基础上做取舍——确认性高的照单全收，主观的按项目口味谈。

**三、lint 政策的作用域决定后果。** 同一条 `deny`，写在不同地方，影响范围完全不同：

- 写在库源码里（`#![deny(...)]`），它跟着这份源码走。任何人编译这个 crate——你自己、你的 workspace 伙伴、把源码拉下去自己构建的人——都会吃到它。
- 写在 `Cargo.toml` 的 `[lints]` 表里，Cargo 官方说明是它只作用于当前包、不会传给依赖。
- 写在命令行或 CI 脚本里（`RUSTFLAGS`/`RUSTDOCFLAGS`），它只对**这一次**本机构建有效。

Cargo 对此还有一层保护：编译 registry 依赖时它传 `--cap-lints allow`，把依赖里的 lint 级别压到 `allow`。这层保护解释了「为什么依赖的警告不会刷屏」，但它**不是**万能护身符——本章「动手看看」里会实测：path 依赖与 workspace 成员不受这层保护，被依赖的库里一旦有 `#![deny(warnings)]`，下游构建会直接失败。

**顺手纠一个常见误解**：`-D warnings` 不是「更严格的检查」，它一个字都不多查。它只是把「现在会产生 warning 的东西」提升成 error。真正的新检查来自新 lint（新编译器版本、新 clippy 版本），而 `-D warnings` 的作用是让这些新检查在你升级工具链时立刻变成构建失败——这正是它适合放在 CI、不适合焊进库源码的原因（32.7 展开）。

## 机制与规则

### 32.1 文档注释的三种位置

rustdoc 的输入只有一种：**文档注释**，附在源码项上。写法有三种。

外层 `///`，挂在它下面的那个项上：

```rust
/// 计算两数之和。
///
/// 第二个参数会原样返回给调用方。
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

内层 `//!`，描述它所在的容器（crate 或模块），必须写在容器开头：

```rust
//! 这个模块负责解析配置文件。
//!
//! 入口是 [`parse`]。

/// 把文本解析成键值对。
pub fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}
```

第三种是属性形式 `#[doc]`，它挂在项上，内容用字符串给。价值在于**内容可以是表达式**：`#[doc = include_str!("../README.md")]` 能把 README 的正文直接当成 crate 文档。本机实测（Rust 1.98）：这样写能正常 `cargo doc`，而且 README 里的代码块会进入 doctest 集合——写错的断言会让 `cargo test --doc` 变红。同源可行，但两份文件里的例子从此一起被门禁管着。

三种位置合起来就得到库的文档面。下面这段片段（`snippets/ch32/doc_lib.rs`）同时演示了 crate 级文档、项文档、字段文档和 intra-doc link：

```rust
{{#rustdoc_include ../snippets/ch32/doc_lib.rs}}
```

两个容易踩的细节：`//!` 必须写在容器开头（写在中间会报错）；`missing_docs`（32.4）连**字段**、**枚举变体**都算，结构体写了文档但字段没写，一样会报。

### 32.2 doctest：注释里的例子是测试

文档注释里的代码块默认被当作 **文档测试（doctest）**：`cargo test` 的 `Doc-tests` 段会编译并运行它们（与单元测试、集成测试的配合见第 31 章）。

最省事的地方是 rustdoc 的预处理：例子不用写 `fn main`。官方书给出的算法里，它给每个例子插入一批 `allow`（`unused_variables`、`unused_assignments`、`unused_mut`、`unused_attributes`、`dead_code`），按需插入 `extern crate <mycrate>;`，最后「if the example does not contain `fn main`, the remainder of the text is wrapped in `fn main() { your_code }`」（引自 The rustdoc book）。

于是下面这个块看着是两条语句，实际会被包成完整程序跑起来：

```rust
let n = 6 * 7;
assert_eq!(n, 42);
```

**隐藏行 `#`。** 文档里常有「为了让例子能跑必须写、但读者不需要看」的样板。以 `#` 开头的行会在渲染时收起，但编译时照旧参与。官方书原话：

> Yes, that’s right: you can add lines that start with `#`, and they will be hidden from the output, but will be used when compiling your code.
> （引自 The rustdoc book）

```rust
# fn helper() -> i32 { 41 }
let n = helper();
assert_eq!(n + 1, 42);
```

上面这块在阅读时只会看到两行，`helper` 的定义被收起；但它真的被编译了——本机实测（Rust 1.98）：把隐藏行改成 `# let n: i32 = "不是数字";` 这种类型错误，`cargo test --doc` 立刻失败并指向那一行。**隐藏不等于不算数**，这是使用 `#` 时的底线。

要以 `#` 本身开头的内容（比如文档里贴一小段 shell），得写 `##` 转义：只有第一个 `#` 管隐藏。本书的代码块也用同一套约定。

**例子属性。** 在代码块的栅栏语言位（写 `rust` 的地方）可以带属性，用逗号或空格分隔：

```rust,should_panic
panic!("这个例子就是要 panic");
```

常用的一组：

| 属性 | 含义 |
|---|---|
| `should_panic` | 必须 panic 才算通过（与第 31 章 libtest 的 `#[should_panic]` 是同一意图的两种写法） |
| `no_run` | 只编译不运行（比如要联网、要写文件的例子） |
| `compile_fail` | **必须编译失败**才算通过，用来演示错误写法 |
| `ignore` | 跳过该块（最宽松，能不用就不用） |
| `edition2021` 等 | 指定该块用哪个 edition 编译 |

`compile_fail` 有个必须知道的限制：官方书提醒「code failing with the current Rust release may work in a future release, as new features are added」——它断言的是「现在编译不过」，不是「永远编译不过」。本书教学用的报错块统一用它。

**`?` 在例子里的用法。** rustdoc 包出来的 `fn main()` 返回 `()`，所以例子里的 `?` 过不了类型检查。两种解决：自己写一个返回 `Result` 的 `fn main`，或者用隐藏行补一个尾表达式——但**类型必须写全**：`# Ok::<(), 你的错误类型>(())`（官方书提醒 `(())` 要连写、中间不能有空白；只写 `# Ok(())` 会报 `` E0282: cannot infer type of the type parameter `E` ``，本机实测）。

```rust
use std::num::ParseIntError;
# fn main() -> Result<(), ParseIntError> {
let n: u32 = "42".parse()?;
assert_eq!(n, 42);
# Ok(())
# }
```

一个完整例子的生命周期大致如下：

<svg viewBox="0 0 720 200" width="100%" role="img" aria-label="doctest 流程：rustdoc 从文档注释里抽出代码块，补上 allow 并包裹 fn main，然后编译运行，最后把每个块报告成 ok 或 FAILED" xmlns="http://www.w3.org/2000/svg">
<rect x="12" y="56" width="124" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="74" y="80" font-size="12" text-anchor="middle" fill="currentColor">源码里的</text>
<text x="74" y="98" font-size="12" text-anchor="middle" fill="currentColor">文档注释代码块</text>
<rect x="152" y="56" width="124" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="214" y="80" font-size="12" text-anchor="middle" fill="currentColor">rustdoc 抽出</text>
<text x="214" y="98" font-size="12" text-anchor="middle" fill="currentColor">每个代码块</text>
<rect x="292" y="56" width="124" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="354" y="80" font-size="12" text-anchor="middle" fill="currentColor">补 allow、包 fn main</text>
<text x="354" y="98" font-size="12" text-anchor="middle" fill="currentColor">（隐藏行也在内）</text>
<rect x="432" y="56" width="124" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="494" y="80" font-size="12" text-anchor="middle" fill="currentColor">编译并运行</text>
<text x="494" y="98" font-size="12" text-anchor="middle" fill="currentColor">断言 / panic</text>
<rect x="572" y="56" width="124" height="56" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="634" y="80" font-size="12" text-anchor="middle" fill="var(--links)">报告 ok</text>
<text x="634" y="98" font-size="12" text-anchor="middle" fill="var(--links)">或 FAILED</text>
<line x1="136" y1="84" x2="148" y2="84" stroke="currentColor" stroke-width="1.5"/>
<line x1="276" y1="84" x2="288" y2="84" stroke="currentColor" stroke-width="1.5"/>
<line x1="416" y1="84" x2="428" y2="84" stroke="currentColor" stroke-width="1.5"/>
<line x1="556" y1="84" x2="568" y2="84" stroke="currentColor" stroke-width="1.5"/>
<text x="12" y="150" font-size="12" fill="var(--quote-border)">属性（should_panic / no_run / compile_fail / ignore）在第 3 步决定这个块怎么被对待。</text>
<text x="12" y="172" font-size="12" fill="var(--quote-border)">隐藏行只影响渲染，不影响第 4 步的编译。</text>
</svg>

### 32.3 intra-doc link：文档之间连成网

**intra-doc link** 就是写在文档注释里、指向另一个 Rust 项的相对链接。rustdoc 官方书的定义是：它用「item 的路径」当链接目标。四种常见写法等价：

```rust
/// 见 [Table]，或者写成 [`Table`]（反引号会被去掉，渲染成等宽）。
/// 也可以自定义显示文本：[键值表](Table)。
/// 引用式也行：[table][Table]。
pub struct Table;
```

解析规则要点：

- 能引用**作用域内**的任何东西，包括 `Self`、`self`、`super`、`crate` 路径；关联项（方法、常量）也支持。
- 名字有歧义时（同名类型与函数都叫 `Foo`），rustdoc 会提示你消歧：加 `struct@`/`fn@`/`enum@` 这类前缀，或者给函数加 `()`、给宏加 `!`。
- 名字空间不同不冲突，但**歧义会被报出来**——这说明链接解析是确定的。
- 链接在**定义处**的作用域解析，即使该项后来被重导出；跨 crate 的失败链接不会报警告。

把这类链接纳入门禁的 lint 是 `rustdoc::broken_intra_doc_links`，默认 `warn`。官方书对它的描述是「This lint warns by default. This lint detects when an intra-doc link fails to be resolved.」。实测输出（Rust 1.98，探针 crate）：

```console
$ cargo doc --no-deps
…
warning: unresolved link to `NotAType`
  --> src\lib.rs:10:16
   |
10 | /// 链接到不存在的类型 [NotAType]。
   |                         ^^^^^^^^ no item named `NotAType` in scope
   |
   = help: to escape `[` and `]` characters, add '\' before them like `\[` or `\]`
   = note: requested on the command line with `-W rustdoc::broken-intra-doc-links`
…
```

注意最后一行：级别不是我写在源码里的，而是 `[lints.rustdoc]` 表让 Cargo 在命令行上加了 `-W`。也就是说 `[lints]` 表对 **rustdoc 的 lint 一样有效**（32.7 再展开）。

还有一种「沉默」值得知道：rustdoc 官方书说，如果一个方括号写法**不像**链接（例如里面含 `/` 或 `[]`），它会被忽略、不解析也不警告。所以写错形式的链接不会报错，它只是不成为链接。

### 32.4 文档门禁：`missing_docs` 与 `RUSTDOCFLAGS`

rustdoc 的 lint 都挂在 `rustdoc::` 名字空间下，**除了一个**：官方书明确写道「Note that, except for `missing_docs`, these lints are only available when running `rustdoc`, not `rustc`.」。所以门禁要分两处配：

- `missing_docs`（rustc 的 lint，默认 `allow`）管「有没有写文档」，`cargo build` 时也生效。
- 其余 `rustdoc::*`（`broken_intra_doc_links`、`missing_crate_level_docs` 等）只在 `cargo doc` 时生效。

给库加文档门禁，最小配置是在 `Cargo.toml` 里声明级别：

```toml
[lints.rust]
missing_docs = "warn"

[lints.rustdoc]
broken_intra_doc_links = "warn"
```

然后在 CI 里把它们变成硬失败：用 `RUSTDOCFLAGS` 而不是 `RUSTFLAGS`——门禁属于 rustdoc 那次调用。

```console
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
 Documenting docprobe v0.1.0 (C:\work\docprobe)
error: missing documentation for a function
 --> src\lib.rs:8:1
  |
8 | pub fn undocumented() {}
  | ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `-D missing-docs` implied by `-D warnings`
  = help: to override `-D warnings` add `#[allow(missing_docs)]`
…
```

`-D warnings` 的完整效果在「动手看看」里看。这里的要点是：文档门禁和小节 32.6 的代码门禁是**两条独立的流水线**——只跑 `cargo clippy` 而从不跑 `cargo doc`，你永远不会知道文档里的链接断了。

### 32.5 `#[doc(cfg)]` 与 docs.rs 上的惯用法

问题：一个库常有「只在某个 feature 打开时存在」的 API，你想在文档里给它标一个「需要 extra」的标签。这就是 `#[doc(cfg(...))]` 的用途。官方 unstable book 的说法是：这个 feature「allows an API be documented as only available in some specific platforms」，并在被标注项的文档里显示一句「Available on (platform) only」。

但它在 stable 上**不可用**。本机实测（Rust 1.98）：

```console
$ rustc --edition 2024 --crate-type lib bare.rs
error[E0658]: `#[doc(cfg)]` is experimental
 --> bare.rs:1:1
  |
1 | #[doc(cfg(feature = "extra"))]
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: see issue #43781 <https://github.com/rust-lang/rust/issues/43781> for more information
```

在 stable 上写 `#![feature(doc_cfg)]` 也不行，会得到 ``error[E0554]: `#![feature]` may not be used on the stable release channel``（本机实测）。跟踪 issue 是 rust-lang/rust#43781。

那 stable 上的库怎么办？惯用写法是**把它包在 `cfg_attr` 里，条件用 `docsrs`**：

```rust
{{#rustdoc_include ../snippets/ch32/gated.rs}}
```

这行属性的效果完全取决于 `docsrs` 这个 cfg 成立不成立：

- 你本地 `cargo build` / `cargo doc`：`docsrs` 不成立，`cfg_attr` 整条不生效，编译器根本看不到 `doc(cfg(...))`——所以 stable 上照样编译通过（上面这段片段在本书 `mdbook test` 里就是这么被编译的）。
- docs.rs 上：那里用 nightly 构建，并给最终那次 rustdoc 调用传 `--cfg docsrs`。官方 Builds 页的说法是「All crates are built in a sandbox using the nightly release of the Rust compiler.」，并且

> The `docsrs` cfg only applies to the final rustdoc invocation (i.e. the crate currently being documented). It does not apply to dependencies (including workspace ones).
> （引自 docs.rs, Builds）

于是**只有 docs.rs 的页面**会出现那个「需要 feature」的标签；本地 `cargo doc` 看不见徽章是正常的。想本地验证 docs.rs 行为，官方页面推荐第三方工具 `cargo docs-rs` 或 docs.rs 仓库自带的本地构建脚本。

配套的 docs.rs 元数据，常见几项（官方 Metadata 页）：

```toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

`all-features = true` 让文档覆盖全部 features；`rustdoc-args` 是给 rustdoc 追加参数的地方，早期项目常在这里传 `--cfg docsrs`。Builds 页只讲了「`docsrs` cfg 只作用于最终那次 rustdoc 调用」；本节据此推断多数 crate 今天不必自己传（官方 Metadata 页没有对应文字）。

顺带一个常被混进来的属性：`#[cfg(doc)]`。它**不是** `doc(cfg)`，作用相反——标记「只在生成文档时才存在」的项。本机实测：`cfg(doc)` 在 stable 1.98 上可以直接用，不需要任何 feature 开关。

### 32.6 clippy：分组与取舍

clippy 与编译器同版本发布，用法就是 `cargo clippy`；它除了自己的 lint，也跑 rustc 的全部 lint。

clippy 的分组（默认级别为本机 1.98 实测）：

| 组 | 默认 | 大致覆盖面 |
|---|---|---|
| `clippy::correctness` | deny | 明显错误或没用的代码（`a == a`） |
| `clippy::suspicious` | warn | 看着像笔误的写法 |
| `clippy::style` | warn | 更惯用的写法（`s.push_str("a")`） |
| `clippy::complexity` | warn | 能写得更短的写法（`x * 1`） |
| `clippy::perf` | warn | 优化器不好处理的写法（遍历 `vec![1, 2, 3]` 而不直接用数组） |
| `clippy::pedantic` | allow | 刻意激进，会有误报 |
| `clippy::restriction` | allow | 禁掉语言/标准库的某些用法 |
| `clippy::nursery` | allow | 还在打磨，可能有问题 |
| `clippy::cargo` | allow | `Cargo.toml` 的元数据 |

`clippy::all` 这个名字听起来像「全部」，其实只涵盖默认就启用的那几组——clippy 官方页对它的定义是「all lints that are on by default (correctness, suspicious, style, complexity, perf)」，也正是 `cargo clippy` 默认跑的东西。下面这台探针 crate 每个组跑出一条（真实输出，仅节选诊断头）：

```console
$ cargo clippy
error: equal expressions as operands to `==`
  = note: `#[deny(clippy::eq_op)]` on by default
warning: calling `push_str()` using a single-character string literal
  = note: `#[warn(clippy::single_char_add_str)]` on by default
warning: the loop variable `i` is only used to index `v`
  = note: `#[warn(clippy::needless_range_loop)]` on by default
warning: this operation has no effect
  = note: `#[warn(clippy::identity_op)]` on by default
warning: useless use of `vec!`
  = note: `#[warn(clippy::useless_vec)]` on by default
error: could not compile `lintprobe` (lib) due to 1 previous error; 4 warnings emitted
```

`eq_op` 是 `deny`，所以它直接把这次 `cargo clippy` 变成了非零退出；四条 `warn` 不影响退出码。这就是 32.7 里那套「默认级别 → 你的策略」的原始材料。

**取舍原则**（取自 clippy 官方书各组说明）：

- `correctness`：命中就改，不要 allow——官方对它的描述是「code that is outright wrong or useless」，而且「just `#[allow]`ing those lints is not recommended」。
- `suspicious`：可能是有意的写法，但要**就地** allow 并写清为什么。
- `style` / `complexity`：默认 `warn` 已经够用；不接受某条就 crate 级 allow，别整组关。
- `perf`：通常好改，建议照单全收。
- `pedantic`：官方建议「cherry-pick lints out of the pedantic group」，不要整组开；开了就要预期大量就地 allow。开启方式是 `cargo clippy -- -W clippy::pedantic` 或 `[lints.clippy] pedantic = "warn"`，两者都能让 `cast_possible_truncation`、`missing_panics_doc` 出现。
- `restriction`：整组开启会被 clippy 自己劝退——实测第一条就是 ``warning: `clippy::restriction` is not meant to be enabled as a group``，并建议 `enable the restriction lints you need individually`。这个组是「菜单」，不是「套餐」。

### 32.7 `-D warnings` 放哪儿：策略、`#[expect]` 与 `[lints]`

先看一张表，它把「配置写给谁看」讲清楚：

| 写在哪 | 影响谁 | 适合放什么 |
|---|---|---|
| 源码里的 `#![deny(...)]` / `#[deny(...)]` | 所有编译这份源码的人 | 少量语义明确的 lint |
| `Cargo.toml` 的 `[lints]` | 当前包（不传给依赖） | 本地分级策略；workspace 用继承统一发（第 30 章） |
| `RUSTFLAGS` / `RUSTDOCFLAGS`（本地脚本、CI） | 这一次构建的本地 crate | `-D warnings` 这类门禁 |
| `cargo clippy -- -W ...` | 这一次 clippy 运行 | 试验新 lint 组 |

**为什么 `deny(warnings)` 不该进库。** 先给出经常被引用的机制。rustc 官方书在讲 `--cap-lints` 时写道：

> This feature is used heavily by Cargo; it will pass `--cap-lints allow` when compiling your dependencies, so that if they have any warnings, they do not pollute the output of your build.
> （引自 The rustc book）

听起来像「依赖里的 `deny(warnings)` 会被压住，那写不写无所谓」。本机实测（Rust 1.98）显示这条保护有边界：

- registry 依赖：`cargo build -v` 里能看到 Cargo 给它的 rustc 传了 `--cap-lints allow`（用 `cfg-if` 实测）。加了 `RUSTFLAGS="-D warnings"` 之后依然如此，所以 CI 的 `-D warnings` 不会因为依赖而爆。
- **path 依赖与 workspace 成员：没有这层保护。** 探针里被依赖的库写了 `#![deny(warnings)]`，内部有一处 `unused_variables`；下游 `cargo build` 的真实输出（exit 101）：

```text
error: unused variable: `unused`
 --> C:\work\wscap\dep\src\lib.rs:6:9
  |
6 |     let unused = 1;
  |         ^^^^^^ help: if this is intentional, prefix it with an underscore: `_unused`
  |
note: the lint level is defined here
 --> C:\work\wscap\dep\src\lib.rs:2:9
  |
2 | #![deny(warnings)]
  |         ^^^^^^^^
  = note: `#[deny(unused_variables)]` implied by `#[deny(warnings)]`

error: could not compile `dep` (lib) due to 1 previous error
```

也就是说：库源码里的 `deny(warnings)` 会卡住「以源码方式使用你」的下游——同 workspace 的伙伴、path 依赖、把源码拉去自己构建的分发方。Cargo 官方书在 SemVer 章里也把这条讲成了兼容性风险：新的 lint 会随时间出现（新编译器版本、依赖更新都会引入），

> Denying warnings should be done with care and the understanding that new lints may be introduced over time.

而且「Transitive dependencies which introduce new lints should not usually cause a failure because Cargo uses `--cap-lints` to suppress all lints in dependencies.」——反过来说，**直接**依赖和本地代码没有这层兜底。

结论落成三条可执行的规矩：

1. 库源码里不用 `deny(warnings)`；要 `deny` 就 `deny` 语义稳定的具体 lint（如 `missing_docs`、`unsafe_code`），它们不随版本漂移。
2. 「所有警告都不能放过」放在 CI：clippy 官方书推荐「run Clippy on CI with `-Dwarnings`」，`RUSTFLAGS="-Dwarnings"` 覆盖所有 `cargo` 命令。按 Cargo 官方书的提醒，用 `RUSTFLAGS` 时补 `-A` 兜住容易误伤的 lint（它举的例子是 `-Adeprecated`）。
3. 更细的分级写进 `[lints]` 表——它只作用于当前包，比焊在源码里干净。

**`#[expect]`：比 `allow` 多一层保险。** `#[allow(...)]` 的问题是它一直沉默：lint 哪天不再命中了（代码重构、lint 升级），没人知道那条 allow 已经过期。`#[expect(...)]` 从 **Rust 1.81.0** 起稳定（1.81 发布说明：Stabilize `#[expect]` for lints (RFC 2383)），语义是「我预期这里会有这条 lint」——没命中时 `unfulfilled_lint_expectations` 会提醒你。本机实测：

```console
$ cargo build
warning: this lint expectation is unfulfilled
  --> src\lib.rs:11:10
   |
11 | #[expect(unused_variables, reason = "演示 expect 未兑现时告警")]
   |          ^^^^^^^^^^^^^^^^
   |
   = note: 演示 expect 未兑现时告警
   = note: `#[warn(unfulfilled_lint_expectations)]` on by default
…
```

注意 `reason` 的内容会被原样当成诊断的 note 打出来——这就是「就地说明为什么」的规范写法：`#[expect(clippy::too_many_arguments, reason = "参数都是纯数据，拆结构体反而更难读")]`。同一探针里另一处 `#[expect(unused_variables)]`（确实有未使用变量）完全沉默，即「兑现了」。所以建议 **`#[expect]` 优先于 `#[allow]`**，除非你预期这条 lint 长期不命中。

**`[lints]` 表。** Cargo 官方书对它的规则：把 lint 名 `::` 前的那一段当作工具名（没有 `::` 就是 `rust`），于是 `clippy::enum_glob_use` 写成 `[lints.clippy] enum_glob_use = "deny"`，`rustdoc::broken_intra_doc_links` 写成 `[lints.rustdoc] broken_intra_doc_links = "deny"`（本机 1.98 实测有效）。两种写法 `foo = "deny"` 与 `foo = { level = "deny", priority = 0 }` 等价。

与源码属性最大的区别还是作用域：Cargo 官方书说这些设置「only affect local development of the current package」，不作用于依赖。所以「团队统一 lint 策略」的做法是 workspace 级继承（`[workspace.lints]` + 成员 `[lints] workspace = true`），细节在第 30 章。

## 动手看看

用两个小 crate 把上面的规则跑一遍。先看文档探针，`src/lib.rs` 故意留了两处问题（漏文档的函数、指向不存在项的链接）：

```rust
//! 文档探针 crate：故意留两处问题。
//!
//! 正常写文档时这里会链接到 [`Point`]。

/// 已经写了文档的函数，链接到 [`Point`]。
pub fn documented() {}

pub fn undocumented() {}

/// 链接到不存在的类型 [NotAType]。
pub fn broken_link() {}

/// 一个点。
pub struct Point {
    /// 横坐标。
    pub x: i32,
}
```

`Cargo.toml` 里用 `[lints]` 表声明级别：

```toml
[lints.rust]
missing_docs = "warn"

[lints.rustdoc]
broken_intra_doc_links = "warn"
```

先看默认构建——`warn` 级别只是提醒，退出码是 0：

```console
$ cargo doc --no-deps
 Documenting docprobe v0.1.0 (C:\work\docprobe)
warning: missing documentation for a function
 --> src\lib.rs:8:1
  |
8 | pub fn undocumented() {}
  | ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: requested on the command line with `-W missing-docs`

warning: unresolved link to `NotAType`
  --> src\lib.rs:10:16
   |
10 | /// 链接到不存在的类型 [NotAType]。
   |                         ^^^^^^^^ no item named `NotAType` in scope
   |
   = help: to escape `[` and `]` characters, add '\' before them like `\[` or `\]`
   = note: requested on the command line with `-W rustdoc::broken-intra-doc-links`

warning: `docprobe` (lib doc) generated 2 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.32s
   Generated C:\work\docprobe\target\doc\docprobe\index.html
$ echo $?
0
```

再把它当门禁用（`-D warnings` 把所有 warning 提升为 error 一起检查），退出码 101：

```console
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
 Documenting docprobe v0.1.0 (C:\work\docprobe)
error: missing documentation for a function
 --> src\lib.rs:8:1
  |
8 | pub fn undocumented() {}
  | ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `-D missing-docs` implied by `-D warnings`
  = help: to override `-D warnings` add `#[allow(missing_docs)]`

error: unresolved link to `NotAType`
  --> src\lib.rs:10:16
   |
10 | /// 链接到不存在的类型 [NotAType]。
   |                         ^^^^^^^^ no item named `NotAType` in scope
   |
   = help: to escape `[` and `]` characters, add '\' before them like `\[` or `\]`
   = note: `-D rustdoc::broken-intra-doc-links` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(rustdoc::broken_intra_doc_links)]`

error: could not document `docprobe`
$ echo $?
101
```

修法三条，选最合适的：给缺文档的项补文档（`missing_docs` 的意图就是「让漏网变成可见的工作量」）；把链接改成能解析的形式（`[NotAType]` 改名字，或字面方括号转义成 `\[NotAType\]`）；确实要留的用 `#[expect(..., reason = "...")]` 说明。改完再跑同样的命令，输出里只剩 ` Documenting …`、`Finished …`、`Generated …/index.html` 三行，退出码回到 0。

第二个探针每个组埋一条诊断，用来对比 clippy 的分组：

```rust
pub fn same(a: i32) -> bool { a == a }

pub fn push_char(s: &mut String) { s.push_str("a"); }

pub fn sum_index(v: &[i32]) -> i32 {
    let mut s = 0;
    for i in 0..v.len() { s += v[i]; }
    s
}

pub fn ident(x: i64) -> i64 { x * 1 }

pub fn iterate() {
    for x in vec![1, 2, 3] { println!("{x}"); }
}

// 下面两条只在 pedantic 组里报警
pub fn to_i32(x: f64) -> i32 { x as i32 }

pub fn parse(s: &str) -> i32 { s.parse().unwrap() }
```

`cargo clippy` 默认集合报出 1 error + 4 warnings（`correctness` 那条是 `deny`，整次运行以 101 结束）；加 `-W clippy::pedantic` 后变成 11 条警告——默认那 4 条之外多出 `must_use_candidate` ×5、`cast_possible_truncation`、`missing_panics_doc`；`-W clippy::restriction` 的第一行是 clippy 自己劝你别整组开。要让 clippy 变成门禁，官方 CI 示例给的是 `cargo clippy --all-targets --all-features`，再配 `RUSTFLAGS="-Dwarnings"`（或 `-- -D warnings`）。

## 报错现场

**报错一：`#[doc(cfg)]` 在 stable 上根本编不过。**

最小复现，一个文件一行属性：

```rust,compile_fail
#[doc(cfg(feature = "extra"))]
pub fn always_present() {}
```

构建时报错（本机 Rust 1.98，`rustc --edition 2024 --crate-type lib bare.rs`）：

```text
error[E0658]: `#[doc(cfg)]` is experimental
 --> bare.rs:1:1
  |
1 | #[doc(cfg(feature = "extra"))]
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: see issue #43781 <https://github.com/rust-lang/rust/issues/43781> for more information

error: aborting due to 1 previous error
```

定位路径：错误码 `E0658` 的含义是「用了未稳定的语言特性」，note 里的 issue 编号（#43781）就是跟踪单。它不是语法错误，也不是你写错了参数——是这个属性还没稳定，stable 工具链直接拒绝。`#![feature(doc_cfg)]` 补救不了：在 stable 上会先撞上 `error[E0554]: #![feature] may not be used on the stable release channel`。

修法：**不要裸写**。把它包进 `cfg_attr`，条件用 `docsrs` 这个只属于 docs.rs 的 cfg：

```rust
#[cfg(feature = "extra")]
#[cfg_attr(docsrs, doc(cfg(feature = "extra")))]
pub mod gated {}
```

这样普通构建里这行属性根本不展开（`docsrs` 不成立），只有 docs.rs 的 nightly 构建会真用上 `doc(cfg)`。副作用要接受：本地 `cargo doc` 里不会出现那个徽章。

**报错二：`-D warnings` 下 rustdoc 直接失败。**

最小复现就是上面文档探针那两处。关键行是 `error: missing documentation for a function`、``error: unresolved link to `NotAType` ``，以及随后的解释行 ``= note: `-D missing-docs` implied by `-D warnings` ``——注意这个 note：它明确告诉你**级别是 `-D warnings` 带出来的**，不是这条 lint 默认就这么严。定位路径：`--> src\lib.rs:8:1` 指到缺文档的项，链接那条指到方括号的位置。修法见上一节的三种；还有个「临时」修法是 `RUSTDOCFLAGS="-D warnings -A rustdoc::broken_intra_doc_links"` 这类局部豁免，但只在排障时用，别写进仓库配置。

## 小结

- 文档注释有三种位置：`///`（给下一项）、`//!`（给所在容器）、`#[doc = ...]`（内容可以是表达式）；它们是源码的一部分，不是另一份文件。
- 文档里的代码块是 doctest：默认被包进 `fn main` 并跑起来；`#` 开头的行渲染时收起但照常编译；`should_panic`/`no_run`/`compile_fail`/`ignore` 决定这个块怎么被验证。
- intra-doc link 用项路径连接文档；歧义要消解（`struct@`/`fn@`、`()`/`!`）；门禁是默认 `warn` 的 `rustdoc::broken_intra_doc_links`。
- `missing_docs` 是唯一同时被 rustc 与 rustdoc 使用的文档 lint，默认 `allow`；文档门禁用 `RUSTDOCFLAGS="-D warnings" cargo doc`，与 clippy 是两条独立流水线。
- `#[doc(cfg)]` 仍不稳定（E0658，issue #43781）；stable 上的惯用法是 `#[cfg_attr(docsrs, doc(cfg(...)))]`，只在 docs.rs 的 nightly 构建里生效。
- clippy 分组按确定性分级：`correctness` deny；`suspicious`/`style`/`complexity`/`perf` warn；`pedantic`/`restriction`/`nursery`/`cargo` allow。整组开的只有 `clippy::all`；`restriction` 要单点。
- lint 政策按作用域放置：`deny(warnings)` 不进库（path 依赖与 workspace 成员没有 `--cap-lints` 兜底，直接卡下游构建）；「全警告门禁」放 CI（`RUSTFLAGS`/`RUSTDOCFLAGS`）；分级写 `[lints]`；就地例外用 `#[expect(..., reason = "...")]`。

## 练习

**1. 找错：** 一个库的 `src/lib.rs` 开头是这样，`cargo build` 以 101 退出，而触发失败的只是一条 `unused_variables` 警告。为什么一条警告变成了错误？说出至少两种正确的改法。

```rust,ignore
//! mini 库。
#![deny(warnings)]

pub fn greet(name: &str) -> String {
    let prefix = "你好，";
    let unused = 0;
    format!("{prefix}{name}")
}
```

**2. 预测：** 下面这段文档注释，渲染出的文档里读者看到几行代码？`cargo test --doc` 会编译几行？如果把 `let enabled = settings == "enabled";` 这行改成 `# let enabled: bool = "yes";`，`cargo test --doc` 会怎样？

````rust,ignore
/// 判断开关是否打开。
///
/// ```
/// # let settings = "enabled";
/// let enabled = settings == "enabled";
/// assert!(enabled);
/// ```
pub fn is_enabled() -> bool { true }
````

**3. 说明为什么：** 团队约定「CI 里必须 `-D warnings`」，于是有人建议顺手在 `src/lib.rs` 顶上写 `#![deny(warnings)]`，理由是这样本地开发也能享受到门禁。请说出这个建议的问题（至少两条），并给出你推荐的替代写法。

<details><summary>参考答案</summary>

**1.** 因为 `#![deny(warnings)]` 把**所有**警告提升成了错误：`unused` 从没被用过，本来只是一条 `unused_variables` 警告，被一提升就成了编译错误（诊断会给出级别来源：`= note: #[deny(unused_variables)] implied by #[deny(warnings)]`）。

但真正该讨论的是：**这条配置不该出现在库源码里**。本机实测（Rust 1.98）：被依赖的库写了 `#![deny(warnings)]` 且内部有一处 `unused_variables`，下游 `cargo build` 以 101 退出——path 依赖与 workspace 成员都不受 `--cap-lints` 保护。正确改法（任选）：

- 删掉 `let unused = 0;`——大多数 `unused_variables` 就是真没用上；
- 删掉 `#![deny(warnings)]`，把门禁放到 CI：`RUSTFLAGS="-Dwarnings" cargo build`（需要时加 `-A deprecated` 之类兜底）、`cargo clippy --all-targets --all-features -- -D warnings`；
- 要就地约束，只 `deny` 语义稳定的具体 lint（如 `missing_docs`），例外用 `#[expect(..., reason = "...")]` 说明。

**2.** 渲染时读者看到 **2 行**（块内共 3 行代码，以 `#` 开头的第 1 行被收起）；`cargo test --doc` 编译的是**完整 3 行**——隐藏行参与编译，只是不显示。把 `let enabled = settings == "enabled";` 改成 `# let enabled: bool = "yes";` 之后，渲染出的内容只剩 `assert!(enabled);` 一行，而 `"yes"` 与 `bool` 类型不匹配，这个 doctest 会编译失败，`cargo test --doc` 变红并指向那一行。这正是 `#` 的用法边界：可以藏起样板，不能藏起错误。

**3.** 两条问题：

- **作用域错**：写在库源码里，影响的是「所有编译这份源码的人」。`--cap-lints allow` 压住的只是依赖，而本机实测 path 依赖与 workspace 成员不受保护——同 workspace 的伙伴、vendor 你代码的人都会因为新 lint 而构建失败，而且他们改不了你的源码。
- **语义错**：`warnings` 是「当前所有会报警的东西」，集合随工具链版本漂移；Cargo 官方书在 SemVer 章提醒「Denying warnings should be done with care and the understanding that new lints may be introduced over time.」。把漂移的集合设成硬失败，等于把「升级工具链」变成一次赌博。

推荐写法：源码里只 `deny` 语义稳定的具体 lint（`missing_docs`、`unsafe_code`）；团队分级写 `Cargo.toml` 的 `[lints]`（workspace 用继承统一发，见第 30 章）；「全警告门禁」只放在 CI 的 `RUSTFLAGS` / `RUSTDOCFLAGS` 里；就地例外用 `#[expect(lint, reason = "...")]`，让过期豁免自己暴露出来。

</details>

## 延伸阅读

- The rustdoc book：[https://doc.rust-lang.org/rustdoc/](https://doc.rust-lang.org/rustdoc/)
- rustdoc book, Documentation tests：[https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html)
- rustdoc book, Linking to items by name：[https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)
- rustdoc book, Lints：[https://doc.rust-lang.org/rustdoc/lints.html](https://doc.rust-lang.org/rustdoc/lints.html)
- rustdoc book, unstable features（`doc_cfg`、`doc(auto_cfg)`）：[https://doc.rust-lang.org/rustdoc/unstable-features.html](https://doc.rust-lang.org/rustdoc/unstable-features.html)
- The rustc book, Lint levels：[https://doc.rust-lang.org/rustc/lints/levels.html](https://doc.rust-lang.org/rustc/lints/levels.html)
- The rustc book, Lint listing：[https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html)
- Rust Reference, Lint attributes 与 `#[expect]`：[https://doc.rust-lang.org/reference/attributes/diagnostics.html](https://doc.rust-lang.org/reference/attributes/diagnostics.html)
- Clippy 文档：[https://doc.rust-lang.org/clippy/](https://doc.rust-lang.org/clippy/)
- Clippy, Lint configuration：[https://doc.rust-lang.org/clippy/lint_configuration.html](https://doc.rust-lang.org/clippy/lint_configuration.html)
- Clippy, Continuous Integration：[https://doc.rust-lang.org/clippy/continuous_integration/](https://doc.rust-lang.org/clippy/continuous_integration/)
- The Cargo Book, The `[lints]` section：[https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section)
- The Cargo Book, SemVer compatibility：[https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
- docs.rs, Builds（`docsrs` cfg、nightly 构建）：[https://docs.rs/about/builds](https://docs.rs/about/builds)
- docs.rs, Metadata（`[package.metadata.docs.rs]`）：[https://docs.rs/about/metadata](https://docs.rs/about/metadata)
