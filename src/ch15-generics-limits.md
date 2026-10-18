# 第 15 章 泛型的边界：const 泛型与 GAT

## 这一章解决什么问题

第 9 章把泛型讲成「编译期参数化的类型与函数」，第 12 章把关联类型讲成「实现者替调用者选一个类型」。这两件工具覆盖了大多数场景，但有两件事它们默认表达不了。

**第一件是值。** `[i32; 3]` 和 `[i32; 4]` 是两种不同的类型，可长度为 `N` 的数组只能按具体数字书写：你没法写一个「对任意长度的定长数组都成立」的函数，因为泛型参数列表里没有地方放这个长度。

**第二件是借用了 `self` 的关联类型（associated type）。** `Iterator::Item` 是一个在实现时就定死的类型，`fn next(&mut self) -> Option<Self::Item>` 里的 `Self::Item` 与 `&mut self` 的生命周期无关。这从根上排除了「每次 `next` 借出 `self` 的一部分」的迭代器。

本章补上这两块：**const 泛型（const generics）**把值提升成参数，**泛型关联类型（generic associated types，简称 GAT）**让关联类型自己再接收参数。前者回答「泛型参数能不能是值」，后者回答「关联类型能不能带参数」。

读完这一章你能：读写带 `const N: usize` 的签名，知道 `[T; N]` 里的长度从哪来；知道 const 泛型的边界在哪、为什么 `N + 1` 到今天仍不行、遇到时怎么绕；会给关联类型加生命周期参数，知道那个必须写的 `where Self: 'a` 是从哪来的；最后，对「这两个工具什么时候值得用」有一套可执行的判断清单。

> [!NOTE]
> 关联类型的基础在第 12 章；本章只用到「关联类型是 trait 里的占位类型、由实现者填实」这一件事。异步迭代是另一条线（第 21、22 章展开）。

## 为什么这样设计

### const 泛型：让「值」进入类型

`[i32; 3]` 的 `3` 本来就是类型的一部分，const 泛型做的事是把这个位置**参数化**：`struct Buf<const N: usize>([u8; N])` 一次定义一族类型。数组长度在 reference 里定义得很直接：「The size is a constant expression that evaluates to a usize.」

但参数化一个值，和参数化一个类型，难度不同。类型参数只需要一个「符号」：比较两个类型是否相同，就是比较它们的符号是否一致。const 参数则要求编译器**求出值、再判断相等**——`Buf<3>` 与 `Buf<4>` 是不同的类型，判定依据是 `3 != 4`。这带来两条设计约束：

- **参数类型必须是能在编译期比较相等的东西。** 1.51 稳定时的原话是「Only values of primitive integers, bool, or char types are currently permitted.」；1.98 的 reference 仍是这样一张清单：整数、`char`、`bool`。想让别的类型当 const 参数，得在 nightly 上开对应的特性门，而且是分开的三条：结构体、枚举等自定义类型看 `adt_const_params`（#95174，配 `#[derive(ConstParamTy)]`）；数组 `[T; N]` 与 `()` 看 `min_adt_const_params`（#154042）；`str`、`[T]`、`&T` 看 `unsized_const_params`（#95174）。
- **参数只能是「孤立的常量」。** 在类型位置写 `N + 1`，编译器要证明两个表达式算出的值相等，这是一个通用的约束求解问题。官方选择先支持最小可用集，把算术留给不稳定的 `generic_const_exprs`。

记住这两条，本章后面的报错就都能预测：凡是编译器「求不出值」的写法，都会在 const 泛型上撞墙。

### GAT：把「实现者选定」升级成「随调用变化」

关联类型是把类型的选择权交给实现者，而**调用者**只知道它存在：写 `fn take(it: impl Iterator)` 时你不必说 `Item` 是什么。代价是这个类型在实现时就被钉死成一个具体类型——一个迭代器不可能「有时候产出 `&'a str`，有时候产出别的东西」。

于是 `Iterator` 表达不了借用式迭代（lending iterator）——每次 `next` 借出 `self` 的一部分。1.98 的编译器对一个直接尝试给出的诊断是（原话）：

```text
error: associated type `Iterator::Item` is declared without lifetime parameters, so using a borrowed type for them requires that lifetime to come from the implemented type
note: you can't create an `Iterator` that borrows each `Item` from itself, but you can instead create a new type that borrows your existing type and implement `Iterator` for that new type
```

第二段是官方给的正统出路：**再造一个迭代器类型**，让 `Item` 的生命周期来自它借用的数据，而不是来自 `self`。标准库的 `slice::ChunksMut` 就是这样——`Item = &'a mut [T]`，`'a` 来自被切片的那个切片，与迭代器自身无关。

当「再造一个类型」不可行时——比如被迭代的数据由迭代器自己持有——GAT 就是出口：关联类型自己接收生命周期参数，每次调用得到一个**新的、与本次借用绑定的类型**。RFC 2996 给异步迭代设计 `AsyncIterator` 时就说清了这层依赖（原文）：

> Lending async iterators depend on [Generic Associated Types](https://rust-lang.github.io/rfcs/1598-generic_associated_types.html), which are not (at the time of this RFC) stable.

那句话里的括注是 2020 年的时点：GAT 已在 1.65 稳定。它留下的结论今天仍然成立——**标准库没有 `LendingIterator`**，想要借用式迭代得自己按本章的写法定义 trait。

## 机制与规则

### const 泛型的语法

```rust
struct Buf<const N: usize>([u8; N]);

impl<const N: usize> Buf<N> {
    // 当作关联常量用
    const CAPACITY: usize = N * 4;
    // 当作运行期表达式的值用
    fn is_full(&self) -> bool {
        self.0.len() == N
    }
}
```

`const` 这个名字进入的是**值命名空间**，所以 `N` 在函数体里就是一个普通常量，可以参与任意运行期表达式（`N * 4` 没问题）。它还能出现在：签名里（`fn f<const N: usize>(xs: [u8; N])`）、字段类型里、关联常量的定义里、关联类型里。

允许的 const 参数类型只有：`u8`/`u16`/`u32`/`u64`/`u128`/`usize`、`i8`/`i16`/`i32`/`i64`/`i128`/`isize`、`char`、`bool`。

三条参数对照起来看：

| | 类型参数 `T` | 生命周期参数 `'a` | const 参数 `const N: usize` |
|---|---|---|---|
| 参数是什么 | 类型 | 借用区间 | 编译期常量值 |
| 可选的种类 | 任意满足 `Sized` 的类型 | 不适用 | 只有整数、`char`、`bool` |
| 实参写法 | 类型 | 生命周期名 | 字面量、单段路径、`{ 表达式 }`、`_` |
| 默认值 | 类型定义上可以写 | 不能 | 类型定义上可以写（1.98 实测） |
| 在值命名空间里 | 否 | 否 | **是** |
| 单态化后 | 换成具体类型 | 擦除 | 换成具体值 |

### 实参怎么写

const 实参可以是字面量、带负号的字面量、单段路径，或者一个块：

```rust
struct S<const N: i64>;
const C: i64 = 1;

fn f<const N: i64>() -> S<N> {
    S
}

fn main() {
    let _ = f::<1>();        // 字面量
    let _ = f::<-1>();       // 负字面量
    let _ = f::<{ 1 + 2 }>(); // 块表达式
    let _ = f::<C>();        // 单段路径
}
```

算式必须包在块里，块外面的 `f::<1 + 2>()` 会在解析阶段被拒（1.98 实测）：

```text
error: expressions must be enclosed in braces to be used as const generic arguments
  | fn main() { let _ = f::<1 + 2>(); }
  |                         ^^^^^
help: enclose the `const` expression in braces
  | fn main() { let _ = f::<{ 1 + 2 }>(); }
```

还有一条容易被忽略的规则：当一个实参既可以解释成类型、又可以解释成 const 时，**一律按类型解**。所以下面这段会报 E0747：

```rust,compile_fail
type N = u32;
struct Foo<const N: usize>;

fn foo<const N: usize>() -> Foo<N> {
    Foo
}

fn main() {}
```

```text
error[E0747]: type provided when a constant was expected
 --> src/main.rs:4:33
  |
4 | fn foo<const N: usize>() -> Foo<N> {
  |                                 ^
  |
help: if this generic argument was intended as a const parameter, surround it with braces
  |
4 | fn foo<const N: usize>() -> Foo<{ N }> {
  |                                 +   +
```

以 Rust 1.98 为例。编译器不仅报错，还直接给出修法：加花括号强制按 const 解释。

1.89 起，实在推不出值时也可以写 `_` 请编译器代填：

```rust
fn make<const N: usize>() -> [u8; N] {
    [0; N]
}

fn main() {
    // 由左侧标注推出 N = 1024
    let buf: [u8; 1024] = make::<_>();
    assert_eq!(buf.len(), 1024);
}
```

`_` 只能从上下文推，推不出来就是 E0284（`type annotations needed`），并不会给你一个默认值。想在类型定义上省掉实参，得写默认值——并且只在类型定义（struct/enum/trait/type alias）上合法：

```rust
struct Buf<const N: usize = 4>([u8; N]);

fn main() {
    let a: Buf = Buf([0; 4]); // 省略实参：N 取默认值 4
    let b = Buf::<8>([0; 8]); // 显式写出
    assert_eq!(a.0.len() + b.0.len(), 12);
}
```

函数与 `impl` 上写默认值是编译错误，报 `defaults for generic parameters are not allowed here`（1.98 实测）。

### 边界：算术与非整数参数类型都还只有 nightly

这是本章最需要记住的一条：**在类型或数组重复表达式里，const 参数只能孤立出现**。reference 的说法是，此时它「may only be used as a single segment path expression, possibly inside a block (such as `N` or `{N}`)」——不能与其他表达式组合。

```rust,compile_fail
fn bad<const N: usize>() -> [u8; N + 1] {
    [0; N + 1]
}

fn main() {}
```

```text
error: generic parameters may not be used in const operations
 --> src/main.rs:1:34
  |
1 | fn bad<const N: usize>() -> [u8; N + 1] {
  |                                  ^ cannot perform const operation using `N`
  |
  = help: const parameters may only be used as standalone arguments here, i.e. `N`
```

以 Rust 1.98 为例。注意报的是普通错误而不是「特性未稳定」——因为这段代码在 stable 上连门都进不去。

> [!WARNING]
> const 泛型的这两块「边界外」能力至今仍是 nightly-only，要用 `#![feature(...)]` 打开，而且这些特性都被官方标为 incomplete（设计可能变）：
> - `generic_const_exprs`（tracking issue #76560）：允许签名里出现需要求值的泛型常量表达式，如 `[u8; N + 1]`。
> - `min_generic_const_args`（#132980）：Unstable Book 的描述是「Enables the generic const args MVP (paths to type const items and constructors for ADTs and primitives).」——它是 `generic_const_exprs` 的最小化替代方案，路线不同。
> - `adt_const_params`（#95174）、`min_adt_const_params`（#154042）、`unsized_const_params`（#95174）：放开「哪些类型能当 const 参数」。`adt_const_params` 管结构体、枚举等自定义类型（配 `#[derive(ConstParamTy)]`），`min_adt_const_params` 管数组 `[T; N]` 与 `()`，`unsized_const_params` 管 `str`、`[T]`、`&T`——三条互相独立，不要混成一条。
>
> 在 stable 1.98 上写 `#![feature(generic_const_exprs)]` 会直接得到（原文）：
>
> ```text
> error[E0554]: `#![feature]` may not be used on the stable release channel
> ```
>
> 本章正文的所有示例都只用 stable 能力。

`N + 1` 写不了，但在**需要求值**的上下文里就可以对 `N` 做运算——内联 const 块（inline const，1.79 起稳定）正是这样一个上下文。于是有个实用技巧：把断言放进 `const { ... }`，让它变成编译期检查。

```rust
fn first<const N: usize>(xs: [i32; N]) -> i32 {
    const { assert!(N > 0, "数组不能为空") };
    xs[0]
}

fn main() {
    assert_eq!(first([7, 8, 9]), 7);
}
```

一旦有人写 `first::<0>([])`，编译就失败在常量求值阶段：

```rust,compile_fail
fn first<const N: usize>(xs: [i32; N]) -> i32 {
    const { assert!(N > 0, "数组不能为空") };
    xs[0]
}

fn main() {
    println!("{}", first::<0>([]));
}
```

```text
error[E0080]: evaluation panicked: 数组不能为空
 --> src/main.rs:2:13
  |
2 |     const { assert!(N > 0, "数组不能为空") };
  |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ evaluation of `first::<0>::{constant#1}` failed here

note: the above error was encountered while instantiating `fn first::<0>`
 --> src/main.rs:7:20
  |
7 |     println!("{}", first::<0>([]));
  |                    ^^^^^^^^^^^^^^
```

以 Rust 1.98 为例。`N` 只是参数，`const { ... }` 会为每个实例化单独求值，所以断言变成了一条针对每个具体长度的编译期检查——比运行期的 `assert!` 更早一步。

### 几个容易踩的 const 泛型规则

**未使用的 const 参数**：类型定义上可以不使用（`struct Foo<const N: usize>;` 合法），但 `impl` 里不行——const 参数必须在 trait、self 类型或谓词里被「约束」：

```rust,compile_fail
struct Unconstrained;

impl<const N: usize> Unconstrained {}

fn main() {}
```

```text
error[E0207]: the const parameter `N` is not constrained by the impl trait, self type, or predicates
 --> src/main.rs:3:6
  |
3 | impl<const N: usize> Unconstrained {}
  |      -^^^^^^^^^^^^^^-
  |      ||
  |      |unconstrained const parameter
  |
  = note: expressions using a const parameter must map each value to a distinct output value
```

以 Rust 1.98 为例。最后一句 note 是理解 const 泛型的钥匙：每个 const 取值必须对应一份不同的实例。

**穷尽性不参与 trait 判定**：`bool` 只有两个值，你可能以为「`Foo<true>` 与 `Foo<false>` 都实现了 `Bar`」就等于「所有 `Foo<B>` 都实现 `Bar`」。编译器不这么看：

```rust,compile_fail
struct Flag<const B: bool>;

trait Enabled {}
impl Enabled for Flag<true> {}
impl Enabled for Flag<false> {}

fn need(_: impl Enabled) {}

fn generic<const B: bool>() {
    need(Flag::<B>);
}

fn main() {}
```

```text
error[E0277]: the trait bound `Flag<B>: Enabled` is not satisfied
  --> src/main.rs:10:10
   |
10 |     need(Flag::<B>);
   |     ---- ^^^^^^^^^ unsatisfied trait bound
   |     |
   |     required by a bound introduced by this call
```

以 Rust 1.98 为例。reference 明确写了这条规则：求解 trait 约束时**不考虑 const 参数的穷尽性**。所以别指望用 `bool` 或小整数 const 参数搭「类型级状态机」还让编译器替你穷尽匹配；要分派就为每个取值手工实现（或用 trait + 具体类型建模，见第 12 章）。

**函数体内不能再定义使用 `N` 的 item**：reference 的示例里，函数体内写 `const BAD_CONST: [usize; N] = [1; N];`、`static`、内部 `fn`、类型别名、内部 `struct` 都不允许——只有签名与运行期表达式能用 `N`。

### GAT 的语法与 `where Self: 'a`

一个借用式迭代器的 trait 长这样：

```rust
trait LendingIterator {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}
```

`type Item<'a>` 就是 GAT：关联类型自己带一个生命周期参数。`fn next` 也带一个 `'a`，并且要求返回的 `Item<'a>` 与**这一次**借用的 `'a` 绑定。下图把「每次调用一个新类型」摊开：

```text
next(&'a1 mut self) -> Option<Item<'a1>>   ← 第 1 次调用
next(&'a2 mut self) -> Option<Item<'a2>>   ← 第 2 次调用（'a1 已结束）
next(&'a3 mut self) -> Option<Item<'a3>>   ← 第 3 次调用
```

`'a1`/`'a2`/`'a3` 是三次**不同**的借用区间，`Item<'a1>` 与 `Item<'a2>` 是不同的类型——普通关联类型做不到这一点。而 `where Self: 'a` 的意思是：要谈 `Item<'a>`，先要求 `Self` 活得比 `'a` 长（否则 `Item<'a>` 里的借用没有来源）。

那为什么这个 `where` 必须写？reference 给了理由（原话）：「these where clauses are required in order to maximize the allowed definitions of the associated type in impls.」——这个 bound 不是给调用者加的，而是为了让 impl 里能写出更多合法定义。实现侧也必须原样重复，否则报 E0477：

```rust,compile_fail
trait Lending {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct S<'s>(&'s mut [u8]);

impl<'s> Lending for S<'s> {
    type Item<'a> = &'a mut [u8];
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>> {
        Some(self.0)
    }
}

fn main() {}
```

```text
error[E0477]: the type `S<'s>` does not fulfill the required lifetime
 --> src/main.rs:9:21
  |
2 |     type Item<'a> where Self: 'a;
  |     ------------- definition of `Item` from trait
...
9 |     type Item<'a> = &'a mut [u8];
  |                     ^^^^^^^^^^^^
  |
note: type must outlive the lifetime `'a` as defined here
 --> src/main.rs:9:15
  |
9 |     type Item<'a> = &'a mut [u8];
  |               ^^
help: copy the `where` clause predicates from the trait
  |
9 |     type Item<'a> = &'a mut [u8] where Self: 'a;
  |                                  ++++++++++++++
```

以 Rust 1.98 为例。修法就是编译器说的那样：把 `where Self: 'a` 抄过来。

在**约束**的位置引用 GAT，要显式给出那个生命周期参数。写单次调用用具体生命周期：

```rust
trait Lending {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

fn take_one<'i, L: Lending>(it: &'i mut L) -> Option<<L as Lending>::Item<'i>> {
    it.next()
}
```

要表达「任意生命周期借出来的 `Item` 都满足某个约束」，用 HRTB（higher-ranked trait bound，`for<'a>`，第 16 章展开）：

```rust
trait Lending {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

fn count_all<L>(it: &mut L) -> usize
where
    L: Lending,
    for<'a> L::Item<'a>: AsRef<str>,
{
    let mut n = 0;
    while let Some(item) = it.next() {
        n += item.as_ref().len();
    }
    n
}
```

两类写法都能编译（1.98 实测），区别是：前者把某个具体生命周期写进签名，后者要求对**所有** `'a` 都成立。

### GAT 的限制与标准库现状

**带 GAT 的 trait 不能当 trait 对象。** reference 的 dyn 兼容性规则里有一条：「It must not have any associated types with generics.」，即关联类型带泛型参数就不满足 dyn 兼容（`dyn` 兼容性的完整规则与机制见第 13 章）。实测：

```rust,compile_fail
trait LendingIterator {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

fn use_it(it: &mut dyn LendingIterator) {
    let _ = it.next();
}

fn main() {}
```

```text
error[E0038]: the trait `LendingIterator` is not dyn compatible
 --> src/main.rs:6:24
  |
6 | fn use_it(it: &mut dyn LendingIterator) {
  |                        ^^^^^^^^^^^^^^^ `LendingIterator` is not dyn compatible
  |
1 | trait LendingIterator {
  |       --------------- this trait is not dyn compatible...
2 |     type Item<'a> where Self: 'a;
  |          ^^^^ ...because it contains generic associated type `Item`
  = help: consider moving `Item` to another trait
```

以 Rust 1.98 为例（中间省略的是指向 reference 的 note 行）。想动态分派，常见的做法是让 GAT 具体化（为需要的那几种生命周期各写一个关联类型），或者干脆放弃 `dyn`、用泛型 + 单态化。

**标准库里没有稳定可用的公开 GAT。** 在 1.98 的 `core`/`alloc`/`std` 源码里按 `type <名字><'…` 扫一遍，带生命周期参数的关联类型只出现在 nightly 门控处：`core::str::pattern::Pattern::Searcher<'a>` 所在的 pattern API 整个模块是 `#![unstable]`；`AsyncFn` 家族的关联类型虽然所属 trait 从 1.85 起稳定，但那个关联类型本身与方法在 1.98 仍标 `#[unstable(feature = "async_fn_traits", issue = "none")]`（标准库文档页对它们标「nightly-only experimental API」）：

```rust,ignore
// std::ops::AsyncFnMut 的声明（摘自 1.98 标准库文档）
#[stable(feature = "async_closure", since = "1.85.0")]
pub trait AsyncFnMut<Args>: AsyncFnOnce<Args>
where
    Args: Tuple,
{
    // 这个关联类型本身仍标 #[unstable(feature = "async_fn_traits")]
    type CallRefFuture<'a>: Future<Output = Self::Output>
    where
        Self: 'a;
    // 其余的内容从略
}
```

`Iterator` 至今没有 GAT，也没有 `LendingIterator`（`core::async_iter::AsyncIterator` 的 `type Item;` 不带生命周期参数，且本身仍是 nightly-only）——想要借用式迭代，就按本章的写法自己定义。

### 判断清单：什么时候值得用

**const 泛型值得用**，当：

- 长度或维度是**类型的语义一部分**：定长缓冲、矩阵维度、固定大小窗口；`[T; N]` 参与的类型运算、`Copy` 大数组、无堆分配的保证都依赖它。
- 你要在**编译期**排除「长度不匹配」这类错误（本章的 `dot([1, 2], [1, 2, 3])` 直接不通过编译），而不是运行期断言。
- 你要针对不同长度做不同实现/分派（常见做法是为少量具体长度 `impl` 若干次）。

**不要用**，当：

- 长度运行时才知道，或者本来就打算换成 `Vec`/切片——那用 `&[T]`、`Vec<T>`、`Box<[T]>`，别把类型空间炸开。
- 你只是想让「长度是编译期已知」——`&[T]` 加 `let len = xs.len()` 一样零成本。
- 你需要的正是**长度算术**（`N + 1`、拼两个数组）——目前只有 nightly 的 `generic_const_exprs` 能做；stable 上的替代是：改写数据结构（用切片/`Vec`）、把算式挪到运行期（`Vec::with_capacity` + 断言）、或把有限几个长度用宏展开成具体实现。

**GAT 值得用**，当：

- 关联类型的值**借用 `self`**，且迭代器自己持有被借用的数据（本章的 `Lines`）；这是 GAT 的主场，纯 `Iterator` 表达不了。
- 关联类型随**调用**变化：视图/游标（`type View<'a> = &'a [T]`）、解析器借出的 token（`type Token<'a> = &'a str`）、解码器每次取出的切片（`type Chunk<'a> = &'a [u8]`）。
- 你要把「生命周期该由谁选」这件事明确交给调用方——GAT 的 `Item<'a>` 就是这个意思。

**不要用**，当：

- 普通关联类型就够：`type Item = T` 钉死一个类型是好事，别为了「看起来通用」加生命周期参数——加了之后 impl、`where`、bound 都要跟着变复杂。
- 参数其实属于 **trait 本身**：`trait View<T>` 往往比 `trait View { type Item<'a>; }` 更简单；只有当类型必须随借用的生命周期变化时才轮到 GAT。
- 你能接受「收集一份 `Vec` 再迭代」（简单、可 `dyn`），或者把迭代状态交给调用者（`while let Some((head, tail)) = slice.split_first()`）——这两种替代方案比引入 GAT 便宜得多。

## 动手看看

第一个程序只用稳定能力：长度进入类型的矩阵转置，以及一个长度不匹配就编译不过的点积。

```rust
{{#rustdoc_include ../snippets/ch15/fixed_size.rs}}
```

```console
$ cd snippets/ch15
$ rustc --edition 2024 fixed_size.rs -o fixed_size.exe
$ .\fixed_size.exe
转置 = [[1, 4], [2, 5], [3, 6]]
点积 = 32
```

`transpose` 的签名 `fn transpose<const R: usize, const C: usize>(m: [[i32; C]; R]) -> [[i32; R]; C]` 值得盯着看：返回类型的行列是**由输入类型算出来的**，不需要你写 `Vec<Vec<i32>>`，也不需要运行期检查。`dot` 则把「两个数组长度必须相同」交给类型系统——`N` 是同一个参数，写 `dot([1, 2], [1, 2, 3])` 直接编译失败，报 `error[E0308]: mismatched types` 加一行 `expected an array with a size of 2, found one with a size of 3`（1.98 实测）。

第二个程序是本章的重点：数据由迭代器自己持有，`Item` 必须借用 `self`，于是关联类型需要自己的生命周期参数。

```rust
{{#rustdoc_include ../snippets/ch15/lending_lines.rs}}
```

```console
$ cd snippets/ch15
$ rustc --edition 2024 lending_lines.rs -o lending_lines.exe
$ .\lending_lines.exe
[alpha]
[be]
[gamma]
最长一行 5 个字符
```

三处呼应前面的规则：trait 声明 `type Item<'a> where Self: 'a;`；impl 里原样重复 `where Self: 'a`；消费函数 `longest` 用 `for<'a> I::Item<'a>: AsRef<str>` 说明「不管哪个生命周期借出来的行都能当 `&str` 用」。`Lines::next` 内部先记下 `start`，用 `find` 找到行尾，更新 `self.pos`，最后借出 `&self.buf[start..end]`——借的正是 `self`，这正是 `Iterator` 做不到的那一步。

## 报错现场

**现场一：在类型位置写算术。**（完整代码与输出见上文「边界」一节）

最小复现是 `fn bad<const N: usize>() -> [u8; N + 1]`。关键行是：

```text
error: generic parameters may not be used in const operations
  |                                  ^ cannot perform const operation using `N`
  = help: const parameters may only be used as standalone arguments here, i.e. `N`
```

定位路径：看到「cannot perform const operation using `N`」，就不要在原地找语法错误了——这是能力边界，不是写错。修法按代价从小到大：(1) 把长度改成参数之外的常量（`const M: usize` 另开一个 const 参数，让调用方显式给出 `M = N + 1`）；(2) 换成切片或 `Vec`，长度在运行期传递；(3) 确实需要算法则，上 nightly 的 `generic_const_exprs`，并接受它是 incomplete 特性。

**现场二：用 `Iterator` 实现借用式迭代。**

```rust,compile_fail
struct Lines {
    buf: String,
    pos: usize,
}

impl Iterator for Lines {
    type Item = &str;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

fn main() {}
```

```text
error: associated type `Iterator::Item` is declared without lifetime parameters, so using a
borrowed type for them requires that lifetime to come from the implemented type
 --> src/main.rs:7:17
  |
7 |     type Item = &str;
  |                 ^
```

以 Rust 1.98 为例。`&str` 缺生命周期，而 `Item` 没办法从 `&mut self` 拿到一个——这就是本章开头那段诊断。修法有两条：按官方建议**另造一个类型**并让它借用数据源（`slice::ChunksMut` 的路线）；或者像本章的 `Lines` 那样改用 GAT。

**现场三：GAT 漏了 `where Self: 'a`。**

```rust,compile_fail
trait Lending {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct S<'s>(&'s mut [u8]);

impl<'s> Lending for S<'s> {
    type Item<'a> = &'a mut [u8];
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>> {
        Some(self.0)
    }
}

fn main() {}
```

```text
error[E0477]: the type `S<'s>` does not fulfill the required lifetime
 --> src/main.rs:9:21
  |
9 |     type Item<'a> = &'a mut [u8];
  |                     ^^^^^^^^^^^^
  |
help: copy the `where` clause predicates from the trait
  |
9 |     type Item<'a> = &'a mut [u8] where Self: 'a;
  |                                  ++++++++++++++
```

以 Rust 1.98 为例。这个报错的位置有点迷惑（指向关联类型的**定义**而不是签名），但 help 已经把答案说完了：trait 里的 bound 在 impl 里要照抄。

## 小结

- **泛型参数分三类**：类型、生命周期、const 值。const 参数进入值命名空间，但只允许整数、`char`、`bool`，因为在类型层面比较两个 const 参数靠的是**求值后判断相等**。
- **const 泛型的价值是让长度/维度成为类型的一部分**：编译期就能拒绝「长度不匹配」，`[T; N]` 有关的类型运算由编译器算。代价是类型空间被展开；**算术**与**非整数参数类型**至今都是 nightly（算术：`generic_const_exprs` #76560、`min_generic_const_args` #132980；参数类型：`adt_const_params` #95174、`min_adt_const_params` #154042、`unsized_const_params` #95174）。
- **写 const 实参**：字面量/单段路径优先，算式要包进 `{ }`，歧义时用花括号强制成 const；1.89 起可用 `_` 推断；默认值只写在类型定义上。
- **GAT 解决的是「关联类型要借用 `self`」**：`type Item<'a> where Self: 'a`，`where` 是为了最大化 impl 的可用定义；impl 侧必须重复它，约束位置要显式给出生命周期或写 `for<'a>`。
- **GAT 与 `dyn` 不兼容**（关联类型带泛型参数就不满足 dyn 兼容）；`Iterator` 没有 GAT，`LendingIterator` 也不存在，而标准库里现存的 GAT 全在 nightly 门控之下（`Pattern::Searcher<'a>`、`AsyncFn*::CallRefFuture<'a>`）——想用就自己定义 trait。
- **判断标准**：长度/维度是类型语义的一部分时才上 const 泛型；数据归属在迭代器自身、且必须借出时才是 GAT 的主场。两者都要为「多一层复杂度」付出签名与约束的代价。

## 练习

**1. 预测编译结果：这两个块分别能编译吗？为什么？**

```rust
const M: usize = 3;
struct A<const N: usize>;

fn f() -> A<M> {
    A
}
```

```rust,compile_fail
type N = u32;
struct A<const N: usize>;

fn g<const N: usize>() -> A<N> {
    A
}
```

（两块各自独立，`A` 的定义都写全了；第二块里多了一个类型别名 `N`。）

**2. 找错：下面这段想实现「每次借出一个元素」，为什么编译不过？**

```rust,compile_fail
struct Drain<'a, T> {
    items: &'a mut Vec<T>,
    idx: usize,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.items.len() {
            return None;
        }
        self.idx += 1;
        Some(&mut self.items[self.idx - 1])
    }
}
```

**3. 判断：什么时候不该引入 GAT？**

（a）关联类型只需要是一个固定类型，比如 `type Output = String`。
（b）迭代器借用外部传入的切片来产出元素。
（c）迭代器自己持有一份缓冲区，每次产出的切片都指向这份缓冲区。

<details><summary>参考答案</summary>

**1.** 第一块可以编译：`M` 是一个真正的常量（`const M: usize = 3;`），在类型位置是合法的单段路径，`A<M>` 就是 `A<3>`。第二块不能编译：作用域里有 `type N = u32;`，于是 `A<N>` 里的 `N` 被优先解释成**类型**，编译器报 E0747「type provided when a constant was expected」，并提示改成 `A<{ N }>`。对照记忆：**常量可以直接当 const 实参，参数在存在同名类型时需要花括号才不会被误读**。这条规则的边界值得实测一次：把 `type N = u32;` 删掉后，`A<N>` 反而能编译——没有歧义时它就按 const 实参解释；花括号是给歧义准备的，不是给参数准备的。

**2.** 编译不过，关键行是 `type Item = &'a mut T;` 配 `fn next(&mut self)`。`'a` 是结构体字段的生命周期，而 `next` 借的是 `&mut self`（编译器内部叫 `'1`），二者不同：`self.items[...]` 取出的借用绑在 `'1` 上，却对外宣称是 `'a`，于是报「method was supposed to return data with lifetime `'a` but it is returning data with lifetime `'1`」这类 `lifetime may not live long enough`。想借出 `self` 的**内部状态**（这里是 `idx` 与 `items` 的组合），正是 GAT 的场景：把 `Item` 改成 `type Item<'b> = &'b mut T where Self: 'b;`，`next` 写成 `fn next<'b>(&'b mut self) -> Option<Self::Item<'b>>`。注意：这个例子还有第二层问题——真正的 `Vec::drain` 产出的 `&mut T` 指向的是被移出 `Vec` 的元素，若改为 `split_first_mut` 拿走所有权再切片，则属于「另造类型」的路线，用普通 `Iterator` 也能写。

**3.** （a）不该：固定类型就用普通关联类型，GAT 只会让签名和约束变复杂。（b）一般不该：如果被借用的数据来自调用方传入的切片，可以让 `Item` 的生命周期来自那个切片（`slice::ChunksMut` 的路线），普通 `Iterator` 就够。（c）该用：这正是 GAT 的主场——数据由迭代器持有，只有 `&mut self` 的借用能作为 `Item` 的来源，普通关联类型无法表达这种「每次调用借一次」的类型。

</details>

## 延伸阅读

- const 泛型的语法、允许的类型与实参规则：Rust Reference 的 [Generic parameters](https://doc.rust-lang.org/reference/items/generics.html)（Const generics 一节）
- 数组类型的长度约束：[Array types](https://doc.rust-lang.org/reference/types/array.html)
- 关联类型与 GAT、必需的 `where` 子句：[Associated items](https://doc.rust-lang.org/reference/items/associated-items.html)
- dyn 兼容性（含「关联类型不得带泛型参数」）：[Traits — Dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)
- 不稳定特性状态：[The Unstable Book](https://doc.rust-lang.org/unstable-book/) 的 `generic_const_exprs`、`min_generic_const_args`、`adt_const_params`
- lending 与 async iterator 的关系：[RFC 2996 · async-iterator](https://rust-lang.github.io/rfcs/2996-async-iterator.html)
- 稳定版本节点：[Rust 1.51.0 发布说明（const 泛型 MVP）](https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html)、[1.65.0（GAT）](https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html)
