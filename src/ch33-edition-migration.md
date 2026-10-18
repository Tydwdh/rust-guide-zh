# 第 33 章 Edition 与迁移

## 这一章解决什么问题

第 1 章给过一句结论：「edition 是语言方言，不是编译器版本」，当时只够你读懂 `Cargo.toml` 里的 `edition = "2024"`。本章把它拆开：方言是什么、谁在什么时候换、换了以后哪些代码会变，以及手上那堆 2021 的 crate 怎么搬过来。

读完这一章，你应该能：

- 说清 edition、rustc 版本号、crate 版本号三者的区别，解释为什么同一个 workspace 里能同时存在四种 edition；
- 拿到 Edition 2024 的完整改动清单，知道每条改动落在哪一层（语法、语义、lint 默认级别、工具链）；
- 用 `cargo fix --edition` 与 `rust-2024-compatibility` lint 组跑一次真实迁移，分清「机器能改的」和「必须人看一遍的」。

边界先画清楚：RPIT 捕获的机制在第 14 章；`unsafe extern`、`unsafe` 属性与符号名在第 26 章；`static mut` 与 `unsafe` 的契约在第 23 章；`Cargo.toml` 表结构与 lint 配置在第 30、32 章。本章只管：edition 这条线把这些改动串成了什么，以及怎么迁移。

## 为什么这样设计

Rust 1.0 立下过一条规矩：**stability without stagnation**。可稳定性承诺的另一面是「一个特性一旦稳定，就会在以后所有版本里继续被支持」——矛盾就在这里：有些改动**天生**做不到向后兼容。最典型的是新增关键字：Rust 早期没有 `async`，`let async = 1;` 完全合法；哪天把 `async` 认成关键字，所有叫 `async` 的变量、函数、字段立刻全废。`gen` 会被留作将来 `gen` block 的关键字，`dyn` 在 2015 里也只是普通标识符。

Edition 就是为这类改动准备的出口。Edition Guide 开篇的原话是：

> “Editions” are Rust's way of introducing changes into the language that would not otherwise be backwards compatible.

译：edition 是 Rust 引入那些本来无法向后兼容的改动的方式。

关键在于它**是 opt-in 的**：改动被推进下一个 edition，crate 停在原 edition 上时根本不生效——同一个 rustc 1.98 里，`gen` 只有在 edition 2024 的 crate 中才是关键字。

### 心智模型：三把互不相干的尺子

初学 edition 最容易搞混的，是把它当成「版本」。Rust 里同时有三把尺子：

| 尺子 | 写在哪 | 决定什么 | 谁升级 |
|---|---|---|---|
| rustc / Cargo 版本 | 你 `rustup` 装的工具链 | 支持哪些稳定特性、能编哪些 edition | 你 |
| edition | `Cargo.toml` 的 `edition` | 「方言」的语法与语义规则 | 每个 crate 自己 |
| crate 版本号 | `Cargo.toml` 的 `version` | 给下游的 API 兼容性承诺（semver） | 每个 crate 自己 |

三者互不绑定：同一个 workspace 里四个 crate 可以分别停在 2015、2018、2021、2024，换 edition 也不会动 `version`：

<svg viewBox="0 0 700 232" width="100%" role="img" aria-label="四个 crate 各选各的 edition，由同一个 rustc 编进同一个二进制" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">edition 是每个 crate 私有的选择</text>
<rect x="0" y="36" width="150" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<rect x="176" y="36" width="150" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<rect x="352" y="36" width="150" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<rect x="528" y="36" width="150" height="56" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="75" y="58" font-size="12" text-anchor="middle" fill="currentColor" font-family="monospace">crate legacy</text>
<text x="75" y="78" font-size="12" text-anchor="middle" fill="currentColor" font-family="monospace">edition = 2015</text>
<text x="251" y="58" font-size="12" text-anchor="middle" fill="currentColor" font-family="monospace">crate util</text>
<text x="251" y="78" font-size="12" text-anchor="middle" fill="currentColor" font-family="monospace">edition = 2018</text>
<text x="427" y="58" font-size="12" text-anchor="middle" fill="currentColor" font-family="monospace">crate net</text>
<text x="427" y="78" font-size="12" text-anchor="middle" fill="currentColor" font-family="monospace">edition = 2021</text>
<text x="603" y="58" font-size="12" text-anchor="middle" fill="var(--links)" font-family="monospace">crate app</text>
<text x="603" y="78" font-size="12" text-anchor="middle" fill="var(--links)" font-family="monospace">edition = 2024</text>
<line x1="75" y1="92" x2="75" y2="140" stroke="currentColor" stroke-width="1.5"/>
<line x1="251" y1="92" x2="251" y2="140" stroke="currentColor" stroke-width="1.5"/>
<line x1="427" y1="92" x2="427" y2="140" stroke="currentColor" stroke-width="1.5"/>
<line x1="603" y1="92" x2="603" y2="140" stroke="currentColor" stroke-width="1.5"/>
<rect x="75" y="140" width="528" height="40" rx="6" fill="none" stroke="currentColor" stroke-width="1.5"/>
<text x="339" y="165" font-size="12" text-anchor="middle" fill="currentColor">同一个 rustc 1.98 编出的同一个二进制</text>
<text x="0" y="204" font-size="12" fill="currentColor" opacity="0.7">crate 之间照样互相调用</text>
<text x="0" y="224" font-size="12" fill="currentColor" opacity="0.7">升级 app 不会连累 legacy</text>
</svg>

### 为什么不叫「大版本升级」

Edition 与「Python 2 到 3」那种断裂有本质区别。Edition Guide 用一整节讲这件事，标题就是 **Editions do not split the ecosystem**：

> crates in one edition must seamlessly interoperate with those compiled with other editions.
>
> changes found in new Rust editions tend to be ‘skin deep’. All Rust code - regardless of edition - will ultimately compile down to the same internal representation within the compiler.

译：不同 edition 的 crate 必须能无缝互操作；新 edition 里的改动往往只是「皮外伤」——所有 Rust 代码最终都编译成编译器内部同一套表示。

换句话说，**edition 只改前端**：分词、解析、名字解析、类型检查、默认开哪些 lint；它不改 ABI、不改布局、不给你两套标准库，所以 2015 与 2024 的 crate 可以互相传 `Vec<u8>`（第 25 章的布局规则与 edition 无关）。宏还多一条 **edition hygiene**：token 记着来自哪个 edition，展开时按其 edition 解析——2015 的宏被 2018 的 crate 调用照样没问题；反面是把定义宏的 crate 升级后 `cargo fix` 可能压根没检查过它（它没被本 crate 调用），这是迁移中最容易漏的坑，33.6 节再说。

## 机制与规则

### 33.1 edition 是谁的属性

一句话：**edition 是 Cargo 的每个 target 的属性**。

最常规的写法是包级的 `edition` 键：

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2024"
```

几条规则：

- **不写就是 2015。** Edition Guide 的原话是「If there's no `edition` key, Cargo will default to Rust 2015.」——旧 `Cargo.toml` 没写 `edition`，方言就是 2015。
- **`cargo new` 会写当前最新 stable 的 edition**（1.98 上生成的就是 `edition = "2024"`）。
- **一个 workspace 里各包各写**：`[workspace]` 没有 `edition` 字段，edition 属于 `[package]`——这正是并行迁移的基础。
- **同一包内还能给单个 target 指定**（在 `[[bin]]` 下再写一行 `edition`），用于十几个 target 想一个个搬的极端情况。
- **edition 与 resolver 有关联但不绑定**：`edition = "2024"` 会蕴含 `resolver = "3"`（见 33.5）。

同一个编译器认得所有 edition，`rustc` 还能直接在命令行上换（`rustc --edition 2021 legacy.rs`）。下面这张表是本章全部「两版对照」实测的汇总——同一份源码分别用 `--edition 2021` 与 `--edition 2024` 编一遍（Windows x86-64、Rust 1.98）：

| 源码 | edition 2021 | edition 2024 |
|---|---|---|
| `#[no_mangle] extern "C" fn f() {}` | 通过 | 错误：`unsafe attribute used without unsafe` |
| `extern "C" { fn abs(x: i32) -> i32; }` | 通过 | 错误：`extern blocks must be unsafe` |
| `unsafe fn f(p: *const u8) -> u8 { *p }` | 通过（无提示） | 警告 `warning[E0133]` |
| `static mut` 上取 `&mut` | 警告 `warning: creating a mutable reference to mutable static` | 错误（deny） |
| `fn gen(n: u32) -> u32 { n + 1 }` | 通过 | 错误：`` expected identifier, found reserved keyword `gen` `` |
| `let [x, mut y] = &[(), ()];` | 通过 | 错误：`cannot mutably bind by value within an implicitly-borrowing pattern` |
| `if let Some(x) = v && x > 0 {}` | 错误：`let chains are only allowed in Rust 2024 or later` | 通过 |
| `for x in boxed_slice`（`Box<[u32]>`） | 通过（1.80 起按值） | 通过 |
| `*boxed_slice.into_iter().next().unwrap()` | 通过（出 `&u32`，警告 `boxed_slice_into_iter`） | 错误：`` type `u32` cannot be dereferenced `` |
| `impl Sized + use<>` | 通过（1.82 起） | 通过 |

### 33.2 Edition 2024 的完整改动清单

Edition 2024 随 **Rust 1.85.0（2025-02-20）** 稳定（**RFC 3501**），Edition Guide 把它拆成 26 个内容页：语言 14、标准库 3、Cargo 3、rustdoc 2、rustfmt 4。下面四张表逐条列出，最后一列给出「迁移时谁会报」。

**表 33-1　语言（14 项）**

| # | 改动 | 2024 起的行为 | 迁移 lint |
|---|---|---|---|
| 1 | RPIT 生命周期捕获（RFC 3498） | 无条件捕获作用域内的**全部**生命周期参数 | `impl_trait_overcaptures` |
| 2 | `if let` 临时值作用域 | 临时值在进入 `else` 前 drop | `if_let_rescope` |
| 3 | `if` / `while` 里的 let chains | 允许用 `&&` 串接 `let`（`\|\|` 不行） | 无（纯扩展） |
| 4 | 尾表达式临时值作用域 | 临时值可能早于局部变量 drop | `tail_expr_drop_order`（只提醒，无修复） |
| 5 | match ergonomics 保留 | 非 `move` 绑定模式下禁止写 `mut` / `ref` / `ref mut` / `&` | `rust_2024_incompatible_pat` |
| 6 | `unsafe extern` 块 | `extern` 块必须写 `unsafe` | `missing_unsafe_on_extern` |
| 7 | unsafe 属性 | `export_name` / `link_section` / `no_mangle` 要包进 `unsafe(...)` | `unsafe_attr_outside_unsafe` |
| 8 | `unsafe_op_in_unsafe_fn` 默认 warn | `unsafe fn` 体内操作要显式 `unsafe {}` | `unsafe_op_in_unsafe_fn` |
| 9 | 禁止对 `static mut` 取引用 | `static_mut_refs` 升为 **deny** | 无自动迁移 |
| 10 | never type fallback | 回退到 `!` 而非 `()` | `dependency_on_unit_never_type_fallback`（deny） |
| 11 | 宏片段说明符 | `expr` 也匹配 `const` 块与 `_`；新增 `expr_2021` | `edition_2024_expr_fragment_specifier` |
| 12 | 缺失片段说明符 | 由 lint 变硬错误（1.89 起所有 edition） | 无 |
| 13 | `gen` 成为保留关键字 | 冲突的标识符要写 `r#gen` | `keyword_idents_2024` |
| 14 | 保留语法 | 无前缀 `#"foo"#`、连续 ≥2 个 `#` | `rust_2024_guarded_string_incompatible_syntax` |

**表 33-2　标准库（3 项）**

| # | 改动 | 说明 | 迁移 lint |
|---|---|---|---|
| 1 | prelude 变更 | 新增 `Future` 与 `IntoFuture` | `rust_2024_prelude_collisions` |
| 2 | `Box<[T]>: IntoIterator` | impl 自 1.80 起在所有 edition 都在；`.into_iter()` 的解析在 2024 改成按值出 `T` | `boxed_slice_into_iter` |
| 3 | 若干函数新增 `unsafe` | `env::set_var`、`env::remove_var`、unix 的 `CommandExt::before_exec` | `deprecated_safe_2024` |

**表 33-3　Cargo（3 项）**

| # | 改动 | 说明 |
|---|---|---|
| 1 | Rust-version aware resolver | `edition = "2024"` 蕴含 `resolver = "3"`；虚拟 workspace 要显式写 |
| 2 | 表名/键名统一 | 移除 `[project]`、`default_features`、`crate_type`、`proc_macro`、`dev_dependencies`、`build_dependencies` |
| 3 | 拒绝无效的继承 `default-features` | workspace 依赖没关默认特性时，成员再写 `default-features = false` 直接报错 |

**表 33-4　rustdoc（2 项）与 rustfmt（4 项）**

| # | 改动 | 说明 |
|---|---|---|
| rustdoc 1 | doctest 合并编译 | 同一 crate 的 doctest 合成一个可执行文件；不兼容时加 `standalone_crate` 标签 |
| rustdoc 2 | 嵌套 `include!` 路径 | `#[doc=include_str!("...")]` 引入的 md 里的 `include!`/`include_str!`/`include_bytes!`，路径改为相对**该 md 文件** |
| rustfmt 1 | style edition | 新增 `style_edition` 配置项，默认跟随 Rust edition |
| rustfmt 2 | 格式化修正 | 十余处格式行为修正，见下方注释 |
| rustfmt 3 | raw identifier 排序 | 排序时用 `r#` 后面的标识符 |
| rustfmt 4 | version sort | 数字后缀按数值排序 |

> [!NOTE]
> rustdoc 的 doctest 合并**只在整包为 2024 及以上时生效**；用 `edition2024` 标签标在旧 edition 的 crate 里不会触发合并。写文档时若依赖 `std::panic::Location` 的行号或 `std::any::type_name` 的模块路径，升级后要复查。rustfmt 的十余处风格修正（不再缩进无关尾注释、嵌套元组索引 `.0.0`、match 里的 `return` 补分号等）由 `cargo fmt` 完成。

### 33.3 语言改动逐条

#### （1）RPIT 生命周期捕获规则（RFC 3498）

`impl Trait` 返回的是不透明类型（opaque type）；函数体里那个**隐藏类型**要用到某个泛型参数，前提是该参数被**捕获**。规则分两半：

- **类型参数与 const 参数**：所有 edition 都自动捕获。Edition Guide 的原话是「In all editions, all in-scope type and const generic parameters are captured implicitly when the `use<..>` bound is not present.」
- **生命周期参数**：2021 及更早只捕获「出现在返回类型约束里的」那些；**2024 起无条件捕获作用域内的全部生命周期参数**，与 RPITIT、`async fn` 拉齐（后两者在所有 edition 都是全捕获）。

机制、`Captures<..>` 戏法与 `use<..>` 都在第 14 章。这里只记一条结论：`impl_trait_overcaptures` 会给受影响的返回类型插上 `use<>`，把捕获集合维持成 2021 的样子——它保住了语义，但**也可能保住了一个你本来想修的限制**，逐条看一遍，想扩的就去掉 `use<>`。

`use<..>` 在 1.82 起**所有 edition** 都能写，所以自动修出来的代码两边都能编：整套迁移策略都是「先改成两边都合法的写法，再换 edition」。

#### （2）`if let` 的临时值作用域

同一个 `if let`，两版 edition 里 scrutinee 的临时值活到哪一刻不同：

```rust
use std::sync::RwLock;

fn ensure_true(value: &RwLock<Option<bool>>) {
    if let Some(x) = *value.read().unwrap() {
        println!("value is {x}");
    } else {
        // 拿写锁：如果读锁还没还，这里就死锁
        let mut v = value.write().unwrap();
        if v.is_none() {
            *v = Some(true);
        }
    }
}
```

2021 里 `value.read()` 产生的读锁守卫是临时值，而 `if let` 的临时值要活到整个表达式结束——包括 `else` 块执行完，于是 `else` 里拿写锁会**死锁**；2024 把它缩短到「进 `else` 之前」。本机把上例 `else` 分支的 `value.write()` 换成 `value.try_write()`（否则会真的挂住），两个 edition 下实测：

```text
=== edition 2021 ===
inside `else`: write lock available = false
final = None
=== edition 2024 ===
inside `else`: write lock available = true
final = Some(true)
```

修法上有个反直觉之处：`cargo fix --edition` 会把 `if let` **改写成 `match`**，因为 `match` 的 scrutinee 临时值活到表达式结束、等价于 2021 的 `if let`。可在这个例子里那正是**你不想保留的**行为（会死锁），所以目标就是 2024 作用域时要把这次改写**回退成 `if let`**——演练里我们就是这么做的。

#### （3）`if` / `while` 里的 let chains

2021 里 `let` 只能紧跟 `if` / `while`，构成 `if let` / `while let`；2024 起允许在条件位置用 `&&` 串起多个 `let`：

```rust
fn sum_first_two(nums: &[u8]) -> Option<u8> {
    let mut iter = nums.iter();
    if let Some(first) = iter.next()
        && let Some(second) = iter.next()
    {
        first.checked_add(*second)
    } else {
        None
    }
}
```

两条限制：只能用 `&&`（`||` 不行）；`let` 必须出现在顶层，括号里的 `if (let Some(a) = x || …)` 不允许。这是纯粹的**扩展**，不需要迁移；它仍挂在 edition 上是因为实现依赖上一条的重定作用域。2021 里写会得到：

```text
error: let chains are only allowed in Rust 2024 or later
```

#### （4）尾表达式的临时值作用域

这一条最容易被忽略：它在**能编译的代码**上悄悄改运行时行为——块（函数体、闭包体）尾表达式里临时造出的值，2021 里被延长到**局部变量之后** drop，2024 起可能就在块尾 drop、**早于**局部变量。`snippets/ch33/drop_order.rs` 把这件事摆到台面上：

```rust
{{#rustdoc_include ../snippets/ch33/drop_order.rs}}
```

同一份文件，两个 edition 下的输出（本机实测）：

```text
=== edition 2021 ===
begin
  drop 0
  drop 1
  got 1
=== edition 2024 ===
begin
  drop 1
  drop 0
  got 1
```

2021 先 drop 局部变量（`drop 0`）再 drop 尾表达式的临时值（`drop 1`），2024 反过来。它只在类型带**自定义 `Drop`**（significant dtor）且析构有可观察副作用时才要紧——`String`、`Vec`、`Box`、`Rc` 这类「只是释放内存」的析构不算。

更硬的一档：作用域**收窄**会让原本能编的代码编不过。

```rust,compile_fail
fn main() {
    let x = { &String::from("1234") }.len();
}
```

2021 里那个 `String` 临时值被延长到语句结束，`len()` 借用得下去；2024 里它到块尾就 drop，于是报「临时值在借用期间被 drop」。修法是把块表达式提成局部变量，让寿命延长规则接管：

```rust
fn main() {
    let s = { &String::from("1234") };
    let x = s.len();
}
```

`tail_expr_drop_order` 只做一半事：指出「哪个临时值会提前 drop、它和哪个局部变量的顺序会变」，但**不给修复建议**。cargo 同一次运行的提示是：

```text
   = note: most of the time, changing drop order is harmless; inspect the `impl Drop`s for side effects like releasing locks or sending messages
```

按这句话做：逐个看受影响类型的 `Drop`，只放宽确认没有副作用的。

#### （5）match ergonomics 的保留

match ergonomics（RFC 2005）让模式匹配「不写 `&` 就穿过引用」，也留下几个说不通的角落——2024 把它们**保留（reserve）**掉了。规则收成一句：**默认绑定模式（default binding mode）不是 `move` 时，不许写会重置它的语法**——隐含借用时绑定上不能写 `mut`；同样条件下不能写 `ref` / `ref mut`（那时它是冗余的）；同样条件下模式里不能出现 `&` / `&mut` 引用模式。

| 2021 允许的写法 | 它在 2021 的含义 | 2024 的等价写法 |
|---|---|---|
| `let [x, mut y] = &[(), ()];` | `x: &()`,`y: ()`（`mut` 顺手把绑定模式重置回 `move`） | `let &[ref x, mut y] = &[(), ()];` |
| `let [ref x] = &[()];` | `x: &()`（`ref` 冗余） | `let [x] = &[()];` |
| `let [&x, y] = &[&(), &()];` | `x: ()`,`y: &&()`（一个 `&` 消掉了两层引用） | `let &[&x, ref y] = &[&(), &()];` |

理由 Edition Guide 说得很直白：`mut` 会影响类型、一个 `&` 会消掉两层引用，都不直观；保留它们是为了给将来修掉留空间。这三种写法在 2024 各有一条诊断（`cannot mutably bind by value within an implicitly-borrowing pattern` 等）。`rust_2024_incompatible_pat` 的处理分两种：由 `mut` 与 `&` 触发的那两条改成「全显式」（`let &[ref x, mut y] = …`、`let &[&x, ref y] = …`），冗余 `ref` 那条只是**删掉 `ref`**；两条路都保证在 2024 与之前所有 edition 下行为一致。日常表现见第 5 章。

#### （6）`unsafe extern` 块

从 2024 起，`extern` 块必须写 `unsafe`：

```rust
unsafe extern "C" {
    // 任何 f64 都是合法输入，可以标 safe
    safe fn sqrt(x: f64) -> f64;
    // 需要有效指针，标 unsafe
    unsafe fn strlen(p: *const std::ffi::c_char) -> usize;
    // 不写限定符 = unsafe
    fn free(p: *mut std::ffi::c_void);
}
```

自 1.82 起这个语法在所有 edition 可用，所以迁移只是「加一个词」，不改语义。为什么必须有它见第 26 章：`extern` 块里每条声明都是「当前 crate 之外存在这么个东西」的承诺，编译器没法核对，签名写错就是 UB 来源。

#### （7）unsafe 属性

同理，这三个属性要包进 `unsafe(...)`：`export_name`、`link_section`、`no_mangle`。

```rust
// SAFETY: 全局只有这一个叫 drill_add 的符号。
#[unsafe(no_mangle)]
pub extern "C" fn drill_add(a: i32, b: i32) -> i32 {
    a + b
}
```

它们动的是**全局符号命名空间**：正常函数名会被符号修饰成唯一的，撞名几乎不可能；一旦关掉修饰或自定名字，撞名就变成「你负责别撞」。符号与 ABI 的完整讨论见第 26 章。

#### （8）`unsafe_op_in_unsafe_fn` 默认 warn

`unsafe fn` 曾经身兼两职：对调用者声明「调用需要 unsafe」，同时在**函数体内自动开放** unsafe 能力。2024 起第二职被取消，函数体里每个 unsafe 操作都要自己写 `unsafe {}`：

```rust
pub unsafe fn first_byte(p: *const u8) -> u8 {
    // 需要显式 unsafe 块
    unsafe { *p }
}
```

默认级别是 **warn**（不是 deny）：它不改语义，只是把「这几行真的在冒险」画出来。想收紧可以自己写 `#![deny(unsafe_op_in_unsafe_fn)]`；`unsafe {}` 的契约见第 23 章。

#### （9）禁止对 `static mut` 取引用

`static_mut_refs` 在 2024 升为 **deny**。它禁的不是「用 `static mut`」，而是「对 `static mut` **取引用**」，包括看不出来的隐式取引用（如 `println!("{NUMS:?}")`）。Edition Guide 说清了为什么这是错误：

> Merely taking such a reference in violation of Rust's mutability XOR aliasing requirement has always been instantaneous undefined behavior, even if the reference is never read from or written to.

译：造出这样一个引用就违反了「可变 XOR 别名」要求——**即便它从未被读取或写入，也立刻是未定义行为**。而对 `static mut` 维持别名规则要**全局**推理，实践上几乎做不到，正确做法是换成能在局部推理的抽象：

| 需求 | 换成 |
|---|---|
| 计数/标志位 | `AtomicU32` 等原子类型（第 20 章） |
| 复杂结构 | `Mutex` / `RwLock`（第 19 章） |
| 一次性初始化 | `OnceLock` / `LazyLock` |
| 必须传裸指针给 C | `&raw const` / `&raw mut`，不产生中间引用（第 24 章） |
| 有外部同步保证 | 自建 `UnsafeCell` 包装并 `unsafe impl Sync`（第 17 章），或尽量窄地 `#[allow(static_mut_refs)]` |

**这条没有自动迁移**：换成哪个是设计决定。本章演练里剩下的唯一手工修就是它。

#### （10）never type fallback

`!`（never type，无值类型）出现在强制转换位置时，编译器会插入一个「万能转换」`absurd::<T>(x: !) -> T`；`T` 推不出来时就用一个**回退类型**顶上。历史上它是 `()`，2024 起改成 `!`。后果是：过去「靠 `()` 顶着」才编过的代码，现在会推到 `!` 然后报错。最常见的形状是 `f()?;`（`?` 的展开让 `Ok` 部分被推断成 `()`）：

```rust,compile_fail
#![allow(dependency_on_unit_never_type_fallback)]
fn outer<T>(x: T) -> Result<T, ()> {
    fn f<T: Default>() -> Result<T, ()> {
        Ok(T::default())
    }
    f()?;      // 2021 下 T 被推成 ()
    Ok(x)
}
```

以 Rust 1.98、edition 2024 为例（文件名是探针文件名，下同；本节探针都是单文件 `rustc --edition <ed> <文件>` 编译），报的是类型错误而不是 lint：

```text
error[E0277]: the trait bound `!: Default` is not satisfied
 --> never_fallback.rs:6:5
  |
6 |     f()?;
  |     ^^^ the trait `Default` is not implemented for `!`
  |
  = help: you might have intended to use the type `()` here instead
note: required by a bound in `f`
 --> never_fallback.rs:3:13
  |
3 |     fn f<T: Default>() -> Result<T, ()> {
  |             ^^^^^^^ required by this bound in `f`
```

修法是把类型写死：`f::<()>()?;` 或 `() = f()?;`。另一个常见形状是 `run(|| panic!())` 里闭包的返回类型从 `()` 变成 `!`，写成 `|| -> () { panic!() }`。另外，`never_type_fallback_flowing_into_unsafe` 与 `dependency_on_unit_never_type_fallback` 在 1.98 已是 **deny**——第（9）条那种「2024 才 deny」是 edition 相关的，这两条不是。

#### （11）宏片段说明符：`expr` 与 `expr_2021`

`const` 表达式（1.79 稳定）与 `_` 表达式（1.59 稳定）在 2021 及更早**不被** `$e:expr` 匹配，这是为了保持向后兼容：

```rust
macro_rules! example {
    ($e:expr) => { println!("first rule"); };
    (const $e:expr) => { println!("second rule"); };
}
```

`example!(const { 1 + 1 })` 在 2021 下走第二条规则；若当年就让 `expr` 匹配 `const`，它会走第一条——破坏性改动，只能等 edition。2024 起 `expr` 也匹配 `const` 块与 `_`；想让旧行为不变的宏，把 `expr` 写成新增的 **`expr_2021`**。

`edition_2024_expr_fragment_specifier` 偏保守：把**所有** `expr` 改成 `expr_2021` 先保住旧行为。但大多数宏其实想用新行为，把 lint 的改动**回退**回去才对；判断依据是「有没有别的规则会同样匹配 `const` / `_`」。

#### （12）缺失的片段说明符

`macro_rules!` 里写了元变量却忘了片段说明符：

```rust,ignore
macro_rules! foo {
    () => {};
    ($name) => { };   // 少了 :expr 之类的说明符
}
```

它在 1.17 起是 lint、1.20 起 deny、1.82 起连依赖里的这种写法也警告，最终在 **1.89 变成所有 edition 的硬错误**。Edition Guide 特意提醒了这处例外：

> NOTE: This was originally made a hard error only for the 2024 Edition. In Rust 1.89, released after Rust 2024, the lint was made into a hard error in all editions.

所以这条**不能**靠「先不改 edition」躲过去；没有自动迁移，只能删掉用不上的那条匹配规则。这类写法极其罕见。

#### （13）`gen` 成为保留关键字

`gen` 被 RFC 3513 保留下来，给将来的 `gen` block 让路。拿它当标识符的代码要改成 raw identifier（例如 `pub fn r#gen(n: u32) -> u32`）。`keyword_idents_2024` 会做这个机械替换，包括调用点。注意 raw identifier 是**标识符**层面的逃生门，不是字符串层面的——`r#gen` 和 `gen` 是同一个名字，只是不会被当成关键字。

#### （14）保留语法

2024 保留了两类 token 序列：一个或多个 `#` 紧跟一个字符串字面量且**没有**前缀（`#"foo"#`），以及连续两个及以上 `#`（中间没有空白）。2021 只保留了带前缀的形式（`ident##"foo"##`），2024 扩展到没有前缀的情况。挂 edition 是因为它和分词、宏的 `tt` 匹配纠缠在一起：

```rust,ignore
macro_rules! demo {
    ( $a:tt ) => { println!("one token") };
    ( $a:tt $b:tt $c:tt ) => { println!("three tokens") };
}

demo!(#"foo"#);
```

2021 下 `#"foo"#` 被切成三个 token、走第二条规则；2024 下它是**一个**保留 token，直接编译错误。`rust_2024_guarded_string_incompatible_syntax` 会指出这些 token 并建议插空格分开。

### 33.4 标准库改动

#### prelude 新增 `Future` 与 `IntoFuture`

prelude 是「每个模块都自动导入」的模块。往里加东西要小心，因为方法名会撞车：

> However, adding a trait to the prelude can break existing code in a subtle way.

译：往 prelude 里加 trait，可能以很微妙的方式破坏已有代码——`x.poll()` 在 `MyPoller` 与 `std::future::Future` 同时可见时就变歧义。加类型不会有事（手动导入的项优先），加 **trait** 会。2024 的 prelude 与旧的唯一差别就是多了 `Future` 和 `IntoFuture`。`rust_2024_prelude_collisions` 会把歧义调用改写成限定形式：

```rust
trait MyPoller {
    fn poll(&self) -> &'static str;
}

impl<T: ?Sized> MyPoller for T {
    fn poll(&self) -> &'static str {
        "my poller"
    }
}

fn main() {
    // 完全限定写法：`Future` 是否进了 prelude 都不影响这一行
    println!("{}", <_ as MyPoller>::poll(&core::pin::pin!(async {})));
}
```

```text
my poller
```

本机实测这条 lint 确实会响（写作 `fut.poll()` 时），`cargo fix` 的改写是 `MyPoller::poll(&fut)`；上面写成 `<_ as MyPoller>::poll(…)` 效果一样。

#### `Box<[T]>` 的 `IntoIterator`

`Box<[T]>` 从 1.80 起实现了 `IntoIterator`（按值出 `T`），并且**在所有 edition 都生效**——所以 `for x in boxed_slice`（不写 `.into_iter()`）哪一版都对。麻烦出在**方法调用**上：`.into_iter()` 会自动 deref 到 `&[T]` 出 `&T`；1.80 之后方法解析有两解，为了不破坏旧代码，它在 2024 **之前**仍解析成「借用的那个」，2024 起才按值：

```rust
fn main() {
    let boxed_slice: Box<[u32]> = vec![1, 2, 3].into_boxed_slice();
    // 2021：x 是 &u32（方法调用先 deref 到 &[u32]）
    // 2024：x 是 u32（解析到 Box<[u32]> 自己的 IntoIterator）
    for x in boxed_slice.into_iter() {
        println!("{x}");
    }
}
```

```text
1
2
3
```

`boxed_slice_into_iter` 默认就是 warn（在所有 edition），看到就顺手改：保持旧语义写 `.iter()`，想按值就删掉 `.into_iter()`。

#### 新标记为 `unsafe` 的函数

三个函数在 2024 起是 `unsafe`：`std::env::set_var`、`std::env::remove_var`（多线程下操作进程环境本来就不健全），以及 unix 的 `CommandExt::before_exec`（早已废弃）。旧 edition 不改。`deprecated_safe_2024` 会自动把它们包进 `unsafe` 块，并留一行提示：

```rust
// FIXME: Audit that the environment access only happens in single-threaded code.
unsafe { std::env::set_var("FOO", "123") };
```

> [!WARNING]
> `unsafe` 只是语法包装：`set_var` 的前提是「此刻没有别的线程在跑」。机器加完那行之后，你得自己去看调用点是否真满足前提——不满足就换机制。

### 33.5 Cargo / rustdoc / rustfmt

**Cargo 三项。** `edition = "2024"` 蕴含 `resolver = "3"`（解析依赖时把 `package.rust-version` 算进去）；resolver 是 **workspace 级**且只对顶层包生效，虚拟 workspace 要在 `[workspace]` 里自己写，没有自动迁移。表名/键名统一后只剩一种拼法，`cargo fix --edition` 会直接改 `Cargo.toml`。继承依赖里无效的 `default-features = false`（workspace 层没关默认特性时）从警告变成错误。三项都属 Cargo 工程化，见第 30 章。

**rustdoc 与 rustfmt** 见表 33-4。它们不需要你改 Rust 代码，但会造成 diff：doctest 合并可能改变 `Location` / `type_name` 的取值，rustfmt 会重排一部分格式。建议迁移的代码改动单独提交，再跑 `cargo fmt` 单独提交格式改动。

### 33.6 迁移流程：`cargo fix --edition` 与 lint 组

Edition Guide 给的标准步骤只有五步：

1. `cargo update`——先更新依赖（过程宏与构建期生成代码的依赖可能有兼容问题），更新完先跑一遍测试。
2. `cargo fix --edition`——让 Cargo 自动改代码。
3. 手动编辑 `Cargo.toml`，把 `edition` 改成新的值。
4. `cargo build` / `cargo test` 验证。
5. `cargo fmt` 按新风格重排。

第 2 步是核心。Edition Guide 的「Advanced migration strategies」写得很清楚：

> `cargo fix --edition` works by running the equivalent of `cargo check` on your project with special lints enabled which will detect code that may not compile in the next edition. These lints include instructions on how to modify the code to make it compatible on both the current and the next edition.

译：`cargo fix --edition` 等价于带着一组特殊 lint 跑 `cargo check`，这些 lint 能发现「下个 edition 里编不过」的代码，并给出「同时兼容当前与下个 edition」的修改建议。

关键就是最后半句：**改动后的代码在两个 edition 下都合法**。你可以停在原 edition 反复迭代直到诊断清空，再动 `Cargo.toml`——源码里那些 `use<>`、`expr_2021`、`unsafe extern` 全是这个策略的产物。

那组 lint 叫 **`rust-2024-compatibility`**。本机 `rustc -Whelp` 列出的成员如下（1.98；换行只为排版），共 16 条：

```text
rust-2024-compatibility  keyword-idents-2024, edition-2024-expr-fragment-specifier,
boxed-slice-into-iter, impl-trait-overcaptures, if-let-rescope, static-mut-refs,
dependency-on-unit-never-type-fallback, deprecated-safe-2024, missing-unsafe-on-extern,
never-type-fallback-flowing-into-unsafe, rust-2024-guarded-string-incompatible-syntax,
rust-2024-incompatible-pat, rust-2024-prelude-collisions, tail-expr-drop-order,
unsafe-attr-outside-unsafe, unsafe-op-in-unsafe-fn
```

**这个组默认是关的，各条的默认级别也不同**（1.98 实测）：

| 默认级别 | 成员 |
|---|---|
| `deny` | `dependency-on-unit-never-type-fallback`、`never-type-fallback-flowing-into-unsafe` |
| `warn` | `static-mut-refs`、`boxed-slice-into-iter` |
| `allow`（不写 `-W` 就完全不响） | 其余 12 条 |

> 表里是 1.98 在 **2021 语境**下的实测值：两条 never type lint 在所有 edition 都是 deny，而 `static-mut-refs` 只在 2024 才是 deny（见 33.3 第（9）条）。

这解释了两件事。其一，**升级工具链本身就会出问题**：默认开着的那 4 条在所有 edition 都生效，不改 edition 也可能编不过。其二，`cargo fix --edition` 必须用 `--force-warn` 把这组拉起来（证据在它自己的输出里）：

```text
= note: `--force-warn tail-expr-drop-order` implied by `--force-warn rust-2024-compatibility`
```

（`rustc --help` 里写作 `--force-warn <LINT>`，是单数；Edition Guide 写的 `--force-warns` 在 1.98 不被接受。）想手动复现：

```console
$ rustc --edition 2021 --crate-type lib --force-warn=rust-2024-compatibility src/lib.rs
```

属性写法用下划线：`#![warn(rust_2024_compatibility)]`。顺带一个本机测出来的区别：换成 `-W` 时那组 lint 大多照样报，但**报不了已被代码里 `#![allow(...)]` 掉的条目**——同一个探针，`--force-warn` 24 条、`-W` 23 条。`--force-warn` 就是为「压过 allow」而存在的。

**机器改不了的几类：**

- **`static_mut_refs`**：换成原子、锁还是裸指针，是设计决定。
- **`tail_expr_drop_order`**：只提示，不给建议。
- **宏里的语法**：`macro_rules!` 只在被同 crate 调用时才可能被自动改，过程宏一律改不了；宏定义改了 edition 之后 token 的 edition 也跟着变，`cargo fix` 可能连警告都没出。
- **文档测试**（可以给单个块加 `edition2018` 标签）、**构建脚本生成的代码**、**非 Cargo 项目**（可手动开这组 lint，或用 rustfix 库自己写迁移工具）。

**几个有用的开关。** `--broken-code`：自动修复把代码改坏时 Cargo 默认会回滚，加它可保留现场接着修。只跑一条 lint：在 crate 根加 `#![warn(if_let_rescope)]` 再跑不带 `--edition` 的 `cargo fix`。按 target / feature 分批：`--all-features`、`--target`（默认 `--all-targets`，每次只处理一种配置）。

## 动手看看

本节是一次完整的真实迁移，用一个一次性的 crate（`edition-drill`）完成：它故意写满 2021 风格的代码，覆盖能触发的全部迁移 lint。

```text
edition-drill/
├── Cargo.toml          # edition = "2021"
├── src/
│   ├── lib.rs
│   ├── envvars.rs      # std::env::set_var
│   ├── ffi.rs          # extern 块 + #[no_mangle]
│   ├── generic.rs      # RPIT 捕获
│   ├── globals.rs      # static mut
│   ├── iters.rs        # Box<[T]>::into_iter()
│   ├── legacy.rs       # gen 标识符、unsafe fn
│   ├── macros.rs       # macro_rules! 的 $e:expr
│   ├── neverf.rs       # never type fallback
│   ├── patterns.rs     # match ergonomics
│   ├── prelude_collision.rs
│   ├── rescope.rs      # if let 的读锁
│   └── tail.rs         # 尾表达式临时值
└── tests/drill.rs      # 行为断言
```

**第一步：先看现状。** 在 2021 上直接 `cargo check`：

```console
$ cargo check --all-targets
warning: creating a mutable reference to mutable static
 --> src\globals.rs:7:17
  |
7 |         let c = &mut COUNTER;
  |                 ^^^^^^^^^^^^ mutable reference to mutable static
...
  = note: `#[warn(static_mut_refs)]` (part of `#[warn(rust_2024_compatibility)]`) on by default
warning: this method call resolves to `<&Box<[T]> as IntoIterator>::into_iter` (due to backwards
compatibility), but will resolve to `<Box<[T]> as IntoIterator>::into_iter` in Rust 2024
 --> src\iters.rs:5:16
...
warning: `edition-drill` (lib) generated 5 warnings (2 duplicates)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
```

能编过，但已经响了 5 条警告：3 条来自这组里默认 warn 的两条 lint（`static_mut_refs` ×2、`boxed_slice_into_iter` ×1），另 2 条是无关的 unused；还有一条 never type 警告被 crate 根的 `#![allow(...)]` 压着。

**第二步：强制开一遍这组 lint。** cargo 内部用的开关是 `--force-warn`（33.6 节解释过为什么不是 `-W`），本机跑出来 24 条：

```console
$ rustc --edition 2021 --crate-type lib --force-warn=rust-2024-compatibility src/lib.rs
```

| 条数 | 诊断 |
|---|---|
| 3 | the `expr` fragment specifier will accept more expressions in the 2024 edition |
| 2 | creating a mutable reference to mutable static |
| 2 | `gen` is a keyword in the 2024 edition |
| 2 | `impl Sized` / `impl Copy` will capture more lifetimes than possibly intended in edition 2024 |
| 1 | extern blocks should be unsafe |
| 1 | unsafe attribute used without unsafe |
| 1 | cannot mutably bind by value within an implicitly-borrowing pattern in Rust 2024 |
| 1 | cannot explicitly dereference within an implicitly-borrowing pattern in Rust 2024 |
| 1 | cannot explicitly borrow within an implicitly-borrowing pattern in Rust 2024 |
| 1 | trait method `poll` will become ambiguous in Rust 2024 |
| 1 | call to deprecated safe function `std::env::set_var` is unsafe and requires unsafe block |
| 1 | `if let` assigns a shorter lifetime since Edition 2024 |
| 1 | relative drop order changing in Rust 2024 |
| 1 | this method call resolves to `<&Box<[T]> as IntoIterator>::into_iter` … |
| 1 | this function depends on never type fallback being `()` |
| 1 | dereference of raw pointer is unsafe and requires unsafe block |
| 3 | 与迁移无关的 `unused_variables` / `unused_assignments` |

后 3 条是噪声，**前面 21 条**才是这组 lint 报的。

**第三步：`cargo fix --edition`。** 探针不在版本库里，要加 `--allow-no-vcs`：

```console
$ cargo fix --edition --allow-no-vcs
   Migrating Cargo.toml from 2021 edition to 2024
   Migrating src\lib.rs from 2021 edition to 2024
       Fixed src\macros.rs (3 fixes)
       Fixed src\envvars.rs (1 fix)
       Fixed src\patterns.rs (3 fixes)
       Fixed src\iters.rs (1 fix)
       Fixed src\generic.rs (2 fixes)
       Fixed src\neverf.rs (1 fix)
       Fixed src\rescope.rs (1 fix)
       Fixed src\prelude_collision.rs (1 fix)
       Fixed src\legacy.rs (3 fixes)
       Fixed src\ffi.rs (2 fixes)
```

10 个文件、18 处改写。逐条看它改了什么（下面每一格都是本机真实 diff）：

| 文件 | lint | 机器改写 |
|---|---|---|
| `ffi.rs` | `missing_unsafe_on_extern`、`unsafe_attr_outside_unsafe` | `extern "C" {` → `unsafe extern "C" {`；`#[no_mangle]` → `#[unsafe(no_mangle)]` |
| `globals.rs` | —（`static_mut_refs` 无自动迁移） | 不改 |
| `generic.rs` | `impl_trait_overcaptures` | `impl Sized` → `impl Sized + use<>`；`impl Sized + Copy` → `impl Sized + Copy + use<>` |
| `legacy.rs` | `keyword_idents_2024`、`unsafe_op_in_unsafe_fn` | `fn gen` / `gen(1)` → `r#gen`；函数体套上 `unsafe { }` |
| `macros.rs` | `edition_2024_expr_fragment_specifier` | `$e:expr` → `$e:expr_2021`（**先保旧行为**） |
| `patterns.rs` | `rust_2024_incompatible_pat` | `let [x, mut y] = &[…]` → `let &[ref x, mut y] = &[…]`；`let [ref z] = &[()]` → `let [z] = &[()]`；`let [&a, b] = …` → `let &[&a, ref b] = …` |
| `iters.rs` | `boxed_slice_into_iter` | `b.into_iter()` → `b.iter()` |
| `envvars.rs` | `deprecated_safe_2024` | 包成 `unsafe { … }`，并加一行 `// FIXME: Audit that the environment access only happens in single-threaded code.` |
| `neverf.rs` | `dependency_on_unit_never_type_fallback` | `f()?;` → `f::<()>()?;` |
| `prelude_collision.rs` | `rust_2024_prelude_collisions` | `fut.poll()` → `MyPoller::poll(&fut)` |
| `rescope.rs` | `if_let_rescope` | `if let … else` → `match … { … _ => { … } }` |
| `tail.rs` | `tail_expr_drop_order` | 不改，只打印诊断 |

第 2 步那 24 条诊断，到这一步只剩 3 条还响（`static_mut_refs` ×2、`tail_expr_drop_order` ×1）——正是「无自动迁移」清单上的两项。

**第四步：改 `Cargo.toml`。** `cargo fix --edition` **不会**替你加这一行，必须手工做：

```toml
[package]
name = "edition-drill"
version = "0.1.0"
edition = "2024"
```

**第五步：在新 edition 上编。** 只剩 `static_mut_refs`，级别从 warn 变成 deny：

```console
$ cargo check --all-targets
error: creating a mutable reference to mutable static
 --> src\globals.rs:7:17
  |
7 |         let c = &mut COUNTER;
  |                 ^^^^^^^^^^^^ mutable reference to mutable static
...
  = note: `#[deny(static_mut_refs)]` (part of `#[deny(rust_2024_compatibility)]`) on by default
help: use `&raw mut` instead to create a raw pointer
  |
7 |         let c = &raw mut COUNTER;
  |                  +++
```

**第六步：手工修三处。**

1. **`static mut` 的引用 → `&raw mut`**（编译器 help 给的最小改动，不产生引用、也不改语义）：

```rust
{{#rustdoc_include ../snippets/ch33/static_mut_raw.rs}}
```

2. **`if let` 改回原样。** 机器为「两个 edition 都对」把它换成 `match`，那恰好保住了 2021 的死锁行为；既然已站在 2024 上，应当回退：

```rust
{{#rustdoc_include ../snippets/ch33/if_let_scope.rs}}
```

（这段代码用 edition 2021 编再跑会**真的挂住**——读锁没还就去拿写锁；本机实测 2021 那份进程不返回、只能杀掉，2024 正常跑完。）

3. **删掉 crate 根的 `#![allow(dependency_on_unit_never_type_fallback)]`。** 它是「留在 2021」的补丁；现在 `f::<()>()?` 已经写死类型，这个豁免不该继续留着掩盖诊断。

**第七步：验证。** 行为断言全部通过：

```console
$ cargo test --all-targets
running 8 tests
test boxed_slice_sum_still_works ... ok
test exported_macro_still_works ... ok
test gen_table_still_works ... ok
test first_byte_still_works ... ok
test ffi_add_still_works ... ok
test if_let_scope_still_works ... ok
test prelude_collision_still_works ... ok
test raw_pointer_roundtrip_still_works ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**但测试通过不等于语义没变。** 第（4）条讲的尾表达式 drop 顺序：迁移前后都能编、都能过测试，行为却不同。

```text
=== edition 2021 ===
begin
  drop 0
  drop 1
  got 1
=== edition 2024 ===
begin
  drop 1
  drop 0
  got 1
```

（同一份源码 `tailorder.rs`，两个 edition 各编一次。）这正是 `tail_expr_drop_order` 的意义：**它提醒的东西测试抓不到**，只能靠人看 `Drop` 实现——有释放锁、发消息、写文件这类副作用时，就得显式调整顺序（最常见的是把临时值提成局部变量）。收尾还有两件事：`cargo fmt`（见 33.5），以及把 `cargo fix` 留下的 `// FIXME: Audit …` 逐条落实。

## 报错现场

输出里的文件名是本章探针的文件名（单文件 `rustc --edition <ed> <文件>` 编译）；均为 1.98 的原文节选。

### 现场一：忘了 `unsafe extern` 与 `unsafe` 属性

```rust,compile_fail
#[no_mangle]
extern "C" fn drill_add(a: i32, b: i32) -> i32 {
    a + b
}

extern "C" {
    fn abs(input: i32) -> i32;
}
```

以 Rust 1.98、edition 2024 为例：

```text
error: extern blocks must be unsafe
 --> c1_unsafe_extern.rs:6:1
  |
6 | / extern "C" {
7 | |     fn abs(input: i32) -> i32;
8 | | }
  | |_^

error: unsafe attribute used without unsafe
 --> c1_unsafe_extern.rs:1:3
  |
1 | #[no_mangle]
  |   ^^^^^^^^^ usage of unsafe attribute
  |
help: wrap the attribute in `unsafe(...)`
  |
1 | #[unsafe(no_mangle)]
  |   +++++++         +
```

两条都是硬错误（同一份代码在 2021 下干净通过），`help` 直接给出机械改法。`cargo fix --edition` 能补上这两处，但**不会**核对签名——写错了编译器照样沉默，那正是 `unsafe` 要你负责的部分。

### 现场二：`static_mut_refs` 从 warn 变 deny

```rust,compile_fail
static mut COUNTER: u32 = 0;

pub fn bump() {
    unsafe {
        let c = &mut COUNTER;
        *c += 1;
    }
}
```

edition 2021：

```text
warning: creating a mutable reference to mutable static
 --> c2_static_mut.rs:5:17
  |
5 |         let c = &mut COUNTER;
  |                 ^^^^^^^^^^^^ mutable reference to mutable static
  |
  = note: `#[warn(static_mut_refs)]` (part of `#[warn(rust_2024_compatibility)]`) on by default
help: use `&raw mut` instead to create a raw pointer
  |
5 |         let c = &raw mut COUNTER;
  |                  +++

warning: 1 warning emitted
```

edition 2024，同一份代码、同一个位置，只差 `warn` 与 `deny` 一个词：

```text
error: creating a mutable reference to mutable static
 --> c2_static_mut.rs:5:17
  |
5 |         let c = &mut COUNTER;
  |                 ^^^^^^^^^^^^ mutable reference to mutable static
  |
  = note: `#[deny(static_mut_refs)]` (part of `#[deny(rust_2024_compatibility)]`) on by default
help: use `&raw mut` instead to create a raw pointer
  |
5 |         let c = &raw mut COUNTER;
  |                  +++

error: aborting due to 1 previous error
```

**注意 `note` 里的 `part of ...` 括号**：这是判断「这条 lint 归哪一组、谁打开的」的最快入口。修法只有一条路：别造引用——最小改动是 `&raw mut`（见 `snippets/ch33/static_mut_raw.rs`），但真正该做的是回到 33.3 第（9）条那张表。**随手 `#[allow]` 掉这条，等于把一个全局推理的 UB 风险留在代码里。**

## 小结

- **edition 是语言方言，不是版本号。** 它写在 `Cargo.toml` 的 `edition` 字段里（缺省 2015），是**每个包**自己的选择；同一个 workspace 可以混用四种 edition，同一个 rustc 都认。它只改前端（分词、解析、名字解析、类型检查、默认 lint），不改 ABI 与布局，所以不同 edition 的 crate 能无缝互操作。
- **Edition 2024 随 Rust 1.85.0 稳定（RFC 3501），共 26 项改动**：语言 14、标准库 3、Cargo 3、rustdoc 2、rustfmt 4。按后果分三类：新增许可（let chains）、语义改变（RPIT 捕获、两处临时值作用域、never type fallback、`Box<[T]>` 的 `.into_iter()`）、edition 相关默认级别提高（`static_mut_refs` 升 deny、`unsafe_op_in_unsafe_fn` 升 warn）；另有两条 never type 相关 lint 在所有 edition 都已是 deny。
- **`unsafe` 家族那四项改动都是「让契约显形」**：关键字不改运行时行为，它标出的是编译器核对不了、必须由你负责的部分；前两项（`unsafe extern`、`unsafe` 属性）是纯语法迁移，后两项（`unsafe_op_in_unsafe_fn`、`static_mut_refs`）要你真的去看代码。
- **`cargo fix --edition` 的本质是「带着 `rust-2024-compatibility` 组跑 `cargo check` 再应用建议」**，建议都是「两个 edition 都合法」的写法，好处是你可以停在原 edition 反复迭代。那 16 条 lint 的默认级别并不统一（2 deny、2 warn、12 allow），所以升级工具链本身就可能先撞上 `static_mut_refs`。
- **机器改不了四件事**：`static_mut_refs`、`tail_expr_drop_order`（只提醒不修复）、宏（过程宏完全改不了，`macro_rules!` 常被漏检）、文档测试与构建期生成的代码；`cargo fix` 留下的 `FIXME` / `TODO` 还得逐条落实。
- **测试绿不代表迁移完成。** 尾表达式 drop 顺序这类改动两个 edition 都能编、都能过测试，行为却不同；`Drop` 里有副作用时必须逐条人工确认。

## 练习

**一、判断 edition。** 下面四段片段各自在哪个 edition 下能编译？

```rust,ignore
// (a)
unsafe extern "C" { fn abs(x: i32) -> i32; }

// (b)
fn gen(n: u32) -> u32 { n + 1 }

// (c)
fn f(v: Option<i32>) -> bool {
    if let Some(x) = v && x > 0 { true } else { false }
}

// (d)
fn len_of<'a>(x: &'a str) -> impl Sized + use<> { x.len() }
```

**二、诊断分诊。** 在 2021 的 crate 上跑完 `cargo fix --edition` 再 `cargo check`，剩下这四条：每条能不能自动修？不能的话你打算怎么办？

```text
① error: creating a mutable reference to mutable static
② warning: relative drop order changing in Rust 2024
③ warning: `gen` is a keyword in the 2024 edition
④ warning: extern blocks should be unsafe
```

**三、预测输出。** 下面这段程序在 edition 2021 与 edition 2024 下各打印什么？为什么？

```rust,ignore
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn f() -> usize {
    let local = Noisy("local");
    Noisy("temp").0.len() + local.0.len()
}

fn main() {
    println!("-> {}", f());
}
```

<details><summary>参考答案</summary>

**一、**

| 片段 | 结论 | 依据 |
|---|---|---|
| (a) | **两者都行** | `unsafe extern` 的**语法**自 1.82 起在所有 edition 可用；2024 只是把它变成必需。 |
| (b) | **只有 2021** | `gen` 在 2024 是保留关键字；想两边都行就写 `fn r#gen`。 |
| (c) | **只有 2024** | let chains 需要 2024；2021 报「only allowed in Rust 2024 or later」。 |
| (d) | **两者都行** | `use<..>` 自 1.82 起在所有 edition 可用——这也正是 `cargo fix` 会插 `use<>` 的原因：改完两边都能编。 |

一般规律：**「新语法」往往在所有 edition 都能用（只受工具链版本限制），而「默认行为」与「必需性」才挂在 edition 上。**

**二、**

| 诊断 | `cargo fix` 能自动修吗 | 处理 |
|---|---|---|
| ① `static_mut_refs` | **不能** | Edition Guide 明说没有自动迁移。换成原子 / `Mutex` / `OnceLock` / `&raw mut`；只求编过可以 `#[allow]`，但要写清为什么安全。 |
| ② `tail_expr_drop_order` | **不能**（不给修复建议） | 逐个看被影响的 `Drop` 有没有副作用；有就把临时值提成局部变量，显式控制顺序。 |
| ③ `gen` 关键字 | **能** | `keyword_idents_2024` 会把标识符与调用点都改成 `r#gen`。 |
| ④ `unsafe extern` | **能** | `missing_unsafe_on_extern` 补关键字；签名正确性仍要人工核对。 |

③④ 会一次性搞定，①② 则在流程最后一步等着你——**排计划时要把手工环节算进去**：`cargo fix` 跑完 ≠ 迁移完成。

**三、**

`f()` 的局部变量 `local` 带自定义 `Drop`，尾表达式里又临时造出一个 `Noisy("temp")`。两版编译都通过，`-> 9` 也相同，**差别只在两个 `drop` 谁先谁后**：

- **edition 2021：** 临时值被延长到局部变量之后 drop，先 `drop local` 再 `drop temp`。
- **edition 2024：** 临时值在块尾先 drop，即先 `drop temp` 再 `drop local`。

```text
=== edition 2021 ===      === edition 2024 ===
drop local                drop temp
drop temp                 drop local
-> 9                      -> 9
```

（两列都是本机实测。）重点是：**返回值一样、编译都过、`cargo test` 也抓不到**——行为改变全藏在 `Drop` 的副作用顺序里。这正是 `tail_expr_drop_order` 只「提醒」不给建议的原因。

</details>

## 延伸阅读

- 全部改动的权威清单，按语言/标准库/Cargo/rustdoc/rustfmt 分节：[The Rust Edition Guide — Rust 2024](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
- edition 是什么、为什么不分裂生态（「为什么这样设计」一节的出处），以及迁移一篇到底改了什么：[What are editions?](https://doc.rust-lang.org/edition-guide/editions/index.html)
- 标准迁移流程（`cargo update` → `cargo fix --edition` → 改 `Cargo.toml` → 验证 → `cargo fmt`）：[Transitioning an existing project to a new edition](https://doc.rust-lang.org/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html)
- `cargo fix` 背后的机制、lint 组、`--broken-code`、部分迁移与非 Cargo 项目：[Advanced migration strategies](https://doc.rust-lang.org/edition-guide/editions/advanced-migrations.html)
- 模式绑定模式（33.3 第（5）条的对照表与三条诊断名）：[Patterns — Binding modes](https://doc.rust-lang.org/reference/patterns.html#binding-modes)
- 全部内置 lint 的默认级别与所属组，迁移前值得扫一遍：[Lint listing](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html)
- Cargo 的 `edition` 字段与 resolver：[The Cargo Book — The manifest format](https://doc.rust-lang.org/cargo/reference/manifest.html)
