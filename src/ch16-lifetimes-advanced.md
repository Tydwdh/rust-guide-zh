# 第 16 章 生命周期进阶：variance、HRTB 与 drop check

## 这一章解决什么问题

第 4 章给了你一套够用的直觉：借用有效到「最后一次使用」为止，别名规则决定谁能改谁不能改。但那套直觉解释不了下面三件事：

- 同一个 `&'static str`，传给要求 `&'a str` 的函数没问题，一旦出现在 `&mut` 后面（`&mut &'static str`），同样的「缩短」就编不过；
- 写一个迭代器结构体，编译器报 `E0392: lifetime parameter 'a is never used`，而加一个看起来什么都不干的 `PhantomData` 就好了；
- 一段本来能编译的代码，只要给某个类型补一个 `impl Drop`，就突然报借用错误。

这三件事背后是**三个不同的机制**，本章逐个拆开：

| 现象 | 机制 | 小节 |
|---|---|---|
| 有的位置能「把长生命周期当短的用」，有的位置不能 | 子类型（subtyping）与型变（variance） | 16.1、16.2 |
| 一个签名要「对任意 `'a` 都成立」 | 高阶 trait 约束（HRTB，`for<'a>`） | 16.3 |
| 析构代码可能访问借用数据 | 析构检查（drop check，dropck） | 16.4 |
| 未使用的参数、型变、dropck 都需要「声明」 | `PhantomData` | 16.5 |
| 什么时候可以不写生命周期 | 省略（elision）的精确规则 | 16.6 |

读完这一章，你应该能做到：

- 判断一个类型在某个位置是协变、逆变还是不变，从而预测赋值、传参、返回能不能通过借用检查；
- 认出「需要 `for<'a>`」的场合，知道为什么多数时候不必手写它，以及闭包为什么常常不满足；
- 解释 dropck 为什么必须保守，以及 `PhantomData` 到底「声明」了什么；
- 不靠猜地使用省略规则，并知道它在哪些位置根本不允许。

> [!NOTE]
> 本章所有「实测」输出都来自本机 `rustc 1.98.0 (88d9e12ae 2026-08-18)`，edition 2024，x86_64 Windows。正文提到 nightly-only 的地方会单独标注。报错输出按「最小复现 + 关键行节选」呈现，行号与该代码块自身的行号一致。

## 为什么这样设计

先回到根本矛盾：生命周期标注要同时让程序**灵活**和**安全**，而这两件事天然冲突。

灵活性要求「活得更久的值可以在要求更短的地方使用」。`&'static str` 传给一个只打算用一小会儿的函数，凭什么不行？如果每个位置都要求类型严格一致，几乎所有函数调用都得把生命周期写死，标注就会从帮助变成负担。Rust 的解法是引入**子类型（subtyping）**：`'static` 是任何更短生命周期的子类型，于是 `&'static str` 也能出现在需要 `&'a str` 的位置。

安全性要求「这种替换只在不会让短命的值冒充长寿值的地方发生」。判断标准就是**型变**：一个泛型类型把「参数的子类型关系」按什么方向传递下去。

然后还有一处编译器**看不见**的代码：析构函数。`Drop::drop` 是普通函数，里面可以读自己的字段，而字段可能持有借用；更麻烦的是跨 crate 时，编译器根本看不到 `Drop` 实现里写了什么。分析调用方时，它只能保守假设「析构时可能还会访问那些借用数据」。这就是**析构检查（dropck）**。

```text
生命周期检查的三层（从外到内）
  1. 借用检查：借用区间 + 别名规则                  → 第 4 章
  2. 子类型 / 型变：长的能不能当短的用               → 16.1 / 16.2
  3. 析构检查：被借用的数据必须严格比持有者活得久
             （持有者的析构点要落在借用区间里）      → 16.4

HRTB（16.3）与前两层正交：它说的是「一个签名同时对全部 'a 成立」，
既不涉及方向，也不涉及析构。三者的关系可以记成：
  variance 关心「方向」，HRTB 关心「范围」，dropck 关心「时机」。
```

## 机制与规则

### 16.1 子类型：`'long <: 'short`

子类型的意思是「一个类型的值可以顶替另一个类型」。Nomicon 给出的生命周期版判据只有一句：

> `'long <: 'short` if and only if `'long` defines a region of code that completely contains `'short`.

译文：`'long` 是 `'short` 的子类型，当且仅当 `'long` 覆盖的代码区间**完全包含** `'short`。注意 `'long` 可以比 `'short` 大，这不影响判据——我们要的正是「至少和你一样久」。

于是下面这段代码能编译：

```rust
fn debug<'a>(a: &'a str, b: &'a str) {
    println!("a = {a:?} b = {b:?}");
}

fn main() {
    let hello: &'static str = "hello";
    {
        let world = String::from("world");
        let world = &world; // 'world 比 'static 短
        debug(hello, world); // hello 被静默地当成 &'world str 使用
    }
}
```

`debug` 的签名要求两个参数**同一个** `'a`。如果编译器只接受类型完全一致，这里必须写 `&'static str`；因为 `&'static str` 是 `&'world str` 的子类型，调用才能成立。这次「降级」就是子类型的作用。

Nomicon 同时提醒：这套直觉好用到可以覆盖 99% 的日常判断——

> this simple rule is a very good 99% intuition. And unless you write unsafe code, the compiler will automatically handle all the corner cases for you.

剩下那 1% 就是本节余下部分与 16.2 的主题。

### 16.2 型变：三个方向

**型变** 描述「`F` 的子类型关系如何由参数的子类型关系决定」。设 `Sub` 是 `Super` 的子类型，Nomicon 的三条定义是：

> - `F` is **covariant** if `F<Sub>` is a subtype of `F<Super>` (the subtype property is passed through)
> - `F` is **contravariant** if `F<Super>` is a subtype of `F<Sub>` (the subtype property is “inverted”)
> - `F` is **invariant** otherwise (no subtyping relationship exists)

| 类型 | 对生命周期参数 | 对类型参数 | 一句解释 |
|---|---|---|---|
| `&'a T` | 协变（covariant） | 协变 | 只读访问，长借用的值可以只被用一小会儿 |
| `&'a mut T` | 协变 | **不变**（invariant） | 能往里面写，所以不允许偷偷换成长生命周期的值 |
| `Box<T>`、`Vec<T>` | — | 协变 | 按值拥有，下面的 `Box` 论证适用 |
| `Cell<T>`、`UnsafeCell<T>` | — | **不变** | 有内部可变性，等价于 `&mut T` |
| `fn(T) -> U` | — | **逆变**（contravariant）／U 协变 | 参数方向反转，见下文 |
| `*const T` | — | 协变 | 跟随 `&T` |
| `*mut T` | — | **不变** | 跟随 `&mut T` |
| `PhantomData<T>` | 由 `T` 决定 | 由 `T` 决定 | 见 16.5 |

#### 为什么 `&mut T` 必须对 `T` 不变

看这个「最自然的写法」：

```rust
fn assign<T>(input: &mut T, val: T) {
    *input = val;
}
```

`assign` 的签名制造了一个类型等式约束：`*input` 的类型必须与 `val` 完全一致。如果 `&mut T` 对 `T` 协变，那么把 `&world`（`&'world str`）写进一个 `&'static str` 的位置就会被允许——`world` 先被析构，`hello` 随后被使用，典型的 use-after-free。编译器拒绝它（完整报错见「报错现场」现场 2）：

```rust,compile_fail
fn assign<T>(input: &mut T, val: T) {
    *input = val;
}

fn main() {
    let mut hello: &'static str = "hello";
    {
        let world = String::from("world");
        assign(&mut hello, &world);
    }
    println!("{hello}");
}
```

对照 `Box<T>` 为什么可以协变：把 `Box<&'static str>` 移动到一个只知道 `Box<&'b str>` 的变量里是安全的，因为 Nomicon 的说法是——

> we destroyed the only thing in the universe that remembered it lived for longer

译文：那个「记得它活得更久」的唯一记录者已经被析构了。按值移动时，长寿那个「身份」随移动一起消失，剩下的只有更短的约束；而 `&mut T` 只是借用，多方仍可能记得原来那个更长的类型，所以不能降级。

#### 为什么 `fn(T)` 对 `T` 逆变

设想一个函数，它要把参数原样存进一个全局的 `Vec<&'static str>`：

```rust,compile_fail
fn call_with_local(f: fn(&str)) {
    let s = String::from("local");
    f(&s);
}

fn store(input: &'static str) -> &'static str {
    input
}

fn main() {
    call_with_local(store);
}
```

`store` 只接受 `&'static str`，而 `call_with_local` 要拿一个局部变量去调用它。一旦允许这种替换，`f` 在别的代码里就能把局部数据当成 `'static` 存下来。反过来是允许的：**能接受任意生命周期的函数，当然能接受 `'static`**。所以函数类型在参数位置是逆变——方向被翻了过来。（这条报错信息很有教学价值，完整输出见「报错现场」现场 3。）

两个方向的实测对照：

```rust
type TakeShort = for<'a> fn(&'a str);
type TakeStatic = fn(&'static str);

fn print_it(s: &str) {
    println!("print_it: {s}");
}

fn main() {
    // 逆变方向：接受任意生命周期的函数，放到「只接受 'static」的位置上
    let f: TakeShort = print_it;
    let g: TakeStatic = f;
    g("static string");
}
```

#### 自己定义的类型：型变由字段决定

结构体的型变不是声明出来的，而是**从字段推导**：某个参数在字段里全部协变，结构体就协变；全部逆变就逆变；出现一处不变（或方向冲突）就整体不变——Nomicon 在示例代码里的措辞是 “invariance wins all conflicts”（不变赢得所有冲突）。

```rust
use std::cell::Cell;

struct Mixed<'a, T, U> {
    r: &'a T,        // 'a 协变、T 协变
    m: &'a mut U,    // U 不变
    c: Cell<u8>,     // 与 T、U 无关
}
// 结论：Mixed 对 'a 协变，对 T 协变，对 U 不变
```

动手验证一遍三个方向（用 `PhantomData` 精确指定我们想要的型变，理由见 16.5）：

```rust
use std::marker::PhantomData;

struct Co<'a> {
    _marker: PhantomData<&'a ()>, // 协变
}

fn takes_co<'a>(_local: &'a String, _x: Co<'a>) {}

fn main() {
    let s = String::from("x");
    let co: Co<'static> = Co { _marker: PhantomData };
    takes_co(&s, co); // 'static 的包装用在更短的 'a 位置：通过
}
```

```rust,compile_fail
use std::marker::PhantomData;

struct Inv<'a> {
    _marker: PhantomData<*mut &'a ()>, // 不变
}

fn takes_inv<'a>(_local: &'a String, _x: Inv<'a>) {}

fn main() {
    let s = String::from("x");
    let inv: Inv<'static> = Inv { _marker: PhantomData };
    takes_inv(&s, inv); // 不变：不通过
}
```

```rust
use std::marker::PhantomData;

struct Contra<'a> {
    _marker: PhantomData<fn(&'a ())>, // 逆变
}

fn takes_contra_static(_x: Contra<'static>) {}

fn make_contra<'a>(_local: &'a String) -> Contra<'a> {
    Contra { _marker: PhantomData }
}

fn main() {
    let s = String::from("x");
    takes_contra_static(make_contra(&s)); // 更短的包装用在 'static 位置：通过
}
```

> [!NOTE]
> 1.98 给 `&mut` 的缩短开了一个小口子。release notes 原文：**“Allow shortening lifetime of `&mut` when unsize-coercing, even in an invariant position.”** 例子是把 `Cell<&'long mut i32>` 变成 `Cell<&'short mut dyn Send>`；同一段说明还指出，「把 `&mut` 强转成 `&`」与「把 `&` 强转成 `&`」时这种缩短一直是被允许的。本机 1.98 实测这个例子可编译。上面的不变性结论针对的是**没有 unsize 强转的普通子类型**，两者不要混起来。

### 16.3 HRTB：`for<'a>`

先看一个不写 `for<'a>` 也能工作的例子：

```rust
fn count<F>(f: F) -> usize
where
    F: Fn(&str) -> usize,
{
    let s = String::from("local");
    f(&s)
}

fn main() {
    println!("{}", count(|s| s.len()));
}
```

`F: Fn(&str) -> usize` 看起来只说了「接受一个 `&str`」，但按 Reference 的省略规则，闭包 trait 签名里的省略生命周期同样会被补成高阶形式，也就是 `for<'a> Fn(&'a str) -> usize`。所以 `f` 可以对**任意**生命周期的 `&str` 调用，包括函数体内的局部 `s`。HRTB 里的 `for<'a>`，Nomicon 的读法是：

> `for<'a>` can be read as “for all choices of `'a`”

译文：对所有可能的 `'a` 都成立——相当于给 `F` 生成一份无穷长的约束清单。

那么什么时候必须手写 `for<'a>`？

**第一类：生命周期出现在非闭包 trait 的参数里。** 省略规则只覆盖函数项、函数指针、闭包 trait 签名，这个位置不沾边。下面这段把 trait 参数里的生命周期写成了 `&str`：

```rust,compile_fail
trait Parser<T> {
    fn parse(&self) -> T;
}

struct StrParser;

impl<'a> Parser<&'a str> for StrParser {
    fn parse(&self) -> &'a str { "parsed" }
}

fn use_parser<P>(p: P) -> usize where P: Parser<&str> {
    p.parse().len()
}

fn main() { println!("{}", use_parser(StrParser)); }
```

实测报错（关键行）：

```text
error[E0637]: `&` without an explicit lifetime name cannot be used here
  --> src/main.rs:11:49
   |
11 | fn use_parser<P>(p: P) -> usize where P: Parser<&str> {
   |                                                 ^ explicit lifetime name needed here
   |
help: consider introducing a higher-ranked lifetime here
   |
11 | fn use_parser<P>(p: P) -> usize where P: for<'a> Parser<&'a str> {
   |                                          +++++++         ++
```

编译器甚至直接给出了修法：写成 `for<'a> Parser<&'a str>`，意思是「对任意 `'a`，`P` 都实现 `Parser<&'a str>`」。

**第二类：省略规则表达不出输入与输出之间的绑定关系时。** 函数指针类型和闭包 trait 签名确实能自动补出 `for<'a>`，但前提是「输出跟随唯一那个输入」。两个输入时省略无法决定输出跟谁，编译器会直接把你推向 HRTB：

```rust,compile_fail
fn pick_first<F>(f: F, a: &str, b: &str) -> usize
where
    F: Fn(&str, &str) -> &str,
{
    f(a, b).len()
}

fn main() {
    let a = String::from("aaa");
    let b = String::from("bb");
    println!("{}", pick_first(|x: &str, _y: &str| x, &a, &b));
}
```

实测报错（关键行）：

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:3:26
  |
3 |     F: Fn(&str, &str) -> &str,
  |           ----  ----     ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from argument 1 or argument 2
  = note: for more information on higher-ranked polymorphism, visit https://doc.rust-lang.org/nomicon/hrtb.html
help: consider making the bound lifetime-generic with a new `'a` lifetime
  |
3 |     F: for<'a> Fn(&'a str, &'a str) -> &'a str,
  |        +++++++     ++       ++          ++
```

要表达「输出只跟随第一个参数」，就得把两个生命周期都写成高阶的：

```rust
fn pick_first<F>(f: F, a: &str, b: &str) -> usize
where
    F: for<'x, 'y> Fn(&'x str, &'y str) -> &'x str,
{
    f(a, b).len()
}

fn main() {
    let a = String::from("aaa");
    let b = String::from("bb");
    println!("{}", pick_first(|x: &str, _y: &str| x, &a, &b));
}
```

输出：

```text
3
```

**第三类：需要给闭包补一个「对任意生命周期成立」的签名时。** 这是日常开发里最常撞到的一类。看这段：

```rust,compile_fail
fn main() {
    let f = |s: &str| -> &str { s.split_whitespace().next().unwrap_or("") };
    let owned = String::from("alpha beta");
    let local = String::from("gamma delta");
    println!("{} {}", f(&owned), f(&local));
}
```

实测报错（关键行）：

```text
error: lifetime may not live long enough
 --> src/main.rs:2:33
  |
2 |     let f = |s: &str| -> &str { s.split_whitespace().next().unwrap_or("") };
  |                 -        -      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ returning this value requires that `'1` must outlive `'2`
  |                 |        |
  |                 |        let's call the lifetime of this reference `'2`
  |                 let's call the lifetime of this reference `'1`
```

闭包的类型是**推断**出来的：这里没有任何「期望签名」告诉编译器它该是 `for<'a>`，于是它被推断成某个具体的生命周期，调用两次生命周期不同的字符串就不行了。补救办法是给它一个期望签名——写个只做标注的辅助函数：

```rust
fn annotate<F>(f: F) -> F
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    f
}

fn main() {
    let f = annotate(|s: &str| -> &str { s.split_whitespace().next().unwrap_or("") });
    let owned = String::from("alpha beta");
    let local = String::from("gamma delta");
    println!("{} {}", f(&owned), f(&local));
}
```

输出：

```text
alpha gamma
```

（另一个办法是直接用**函数项**：`fn first_word(s: &str) -> &str` 这类签名里的生命周期是晚绑定的，天生满足 `for<'a>`，不需要标注。）

> [!TIP]
> 判断口诀：**闭包只有在「有期望签名」时才可能被推断成高阶的。** 直接传给 `F: for<'a> Fn(...)` 参数没问题；但先 `let f = |..| ..;`、装进结构体字段、或存进集合，就没有期望签名了，要么用 `annotate`，要么换成函数项。

同一个根因还有一副更容易认错的面孔：如果这个闭包塞进结构体字段、再调用带 `for<'a>` 约束的 `impl` 方法，报错会变成 ``E0599: no method named `run` found for struct ...``——具体闭包类型不满足 `impl` 上的高阶约束，于是方法「看起来不存在」（本机实测；note 里会写 “the method was found for `FirstWord<F>`”）。遇到「泛型结构体上方法找不到」时，先把 `where` 子句里的 `for<'a>` 与实参的闭包签名对一遍。

最后划清一条边界：HRTB 与型变是**互相独立**的两套机制。Nomicon 在讲逆变时特意说明，函数指针上的 `for<'a>` 属于 「higher rank lifetimes, which work independently of subtyping」。`for<'a>` 管的是「一个签名对整个生命周期范围成立」，型变管的是「方向」，不要用一个去解释另一个。

### 16.4 析构检查：析构里还会用借用

先补齐析构顺序（Reference 的 `destructors` 页给了完整清单，实测见「动手看看」）：

| 对象 | 顺序 |
|---|---|
| 局部变量、临时值 | 离开作用域时**逆序**：变量按声明、临时值按创建（原文：dropped in reverse order of declaration (for variables) or creation (for temporaries)） |
| 结构体 / 枚举变体的字段 | 按声明顺序（declaration order） |
| 元组元素 | 按顺序（in order） |
| 数组或拥有的切片 | 从第一个到最后一个 |
| 闭包按 move 捕获的变量 | 顺序未指定（unspecified order） |
| 模式里的绑定 | 按模式内声明顺序**逆序** |

那为什么需要一门**独立**的检查？因为据 Nomicon：

> Implementing `Drop` lets the `Inspector` execute some arbitrary code during its death. This means it can potentially observe that types that are supposed to live as long as it does actually were destroyed first.

译文：实现 `Drop` 意味着这个类型在「死亡」时可以执行任意代码，于是它可能观察到「本应活到和自己一样久的类型其实先被析构了」。而且这个问题只对带生命周期参数的类型出现：

> If they aren't generic, then the only lifetimes they can harbor are `'static`, which will truly live forever.

译文：如果类型不是泛型的，它能容纳的生命周期只有 `'static`，那是真·永远活着。所以这被称为**健全的泛型析构（sound generic drop）**，由 drop checker（dropck）负责。Nomicon 把它归结为一条大规则：

> For a generic type to soundly implement drop, its generics arguments must strictly outlive it.

译文：一个泛型类型要能健全地实现析构，它的泛型参数必须**严格地**比它活得更久。注意是「严格」：`'a: 'b` 允许两者同时结束，而析构要求析构点落在被借用数据的借用区间**内部**。

之所以要「独立检查」，核心在于信息不可见：分析调用方时，编译器看不到 `Drop::drop` 的函数体（跨 crate 时更不可能看到），只能假设它可能访问字段里的借用。所以下面这段代码——析构函数明确不碰借用数据、实际完全安全——**照样被拒**：

```rust
struct Inspector<'a>(&'a u8, &'static str);

impl<'a> Drop for Inspector<'a> {
    fn drop(&mut self) {
        println!("Inspector(_, {}) knows when *not* to inspect.", self.1);
    }
}
```

Nomicon 对此的说明是：`main` 的借用检查分析并不知道每个 `Inspector` 的 `Drop` 实现里写了什么，只能保守认为「析构函数可能会访问那份借用数据」，于是要求所有被借用的数据严格比持有者活得更久。真正带析构的完整失败案例见「报错现场」现场 1。

> [!WARNING]
> 标准库用 nightly 的 `#[may_dangle]` 属性给自己开了后门：它放在 `Drop` 实现的泛型参数上，向编译器**承诺**「析构代码不会访问这个参数里的（可能过期的）数据」。属性名来自 RFC 1327，写法是 `unsafe impl<#[may_dangle] T> Drop for Vec<T>`——注意必须写成 `unsafe impl`，因为这个承诺编译器不验证。它需要 nightly 的特性门 `dropck_eyepatch`（Nomicon 例子首行是 `#![feature(dropck_eyepatch)]`），**stable 上用不了**；本节只需要记住它的名字与用途。

不过 `#[may_dangle]` 的效果在 stable 上就能观察到，因为标准库已经用它了：`Vec<T>` 的 `Drop` 就带着 `#[may_dangle] T`（本机 rust-src：`library/alloc/src/vec/mod.rs:4295`）。于是下面这段「往 `Vec` 里存一个 `&s`，随后手动 `drop(s)`」的代码**能编译**——`Vec` 的析构只需要释放缓冲区，不会访问那些引用，所以这份承诺成立：

```rust
fn main() {
    let mut v: Vec<&str> = Vec::new();
    let s: String = "Short-lived".into();
    v.push(&s);
    drop(s);
}
```

换成普通的 `impl<T> Drop for Vec<T>`，同样的代码会被 dropck 拒绝——Nomicon 的 `phantom-data` 页对这个对照有详细推导（第 34 章重写 `Vec` 时会正面处理）。

### 16.5 `PhantomData` 的三重身份

`PhantomData` 常被当成「为了骗过编译器而加的空字段」，其实它是在**声明一个不存在的字段**。Nomicon 的定位是：

> `PhantomData` consumes no space, but simulates a field of the given type for the purpose of static analysis.

译文：`PhantomData` 不占空间，但对静态分析模拟了一个给定类型的字段。它是零尺寸类型（本机实测 `size_of::<PhantomData<u64>>()` 与 `size_of::<PhantomData<&'static str>>()` 都是 `0`）。它的三个身份：

**身份一：占位，让泛型参数「被用到」。** 结构体声明了参数就必须使用它；只有裸指针字段时，参数就「没用上」：

```rust,compile_fail
struct Iter<'a, T> {
    ptr: *const T,
    end: *const T,
}

fn main() {}
```

报错（关键行）：

```text
error[E0392]: lifetime parameter `'a` is never used
 --> src/main.rs:1:13
  |
1 | struct Iter<'a, T> {
  |             ^^ unused lifetime parameter
  |
  = help: consider removing `'a`, referring to it in a field, or using a marker such as `PhantomData`
```

类型参数版本同样报 `E0392`（``type parameter `T` is never used``）。编译器给了三条出路：删掉、在字段里真正用到、用标记类型。要保留参数就只能选第三条，而 `Iter` 逻辑上「含有一堆 `&'a T`」，于是照着写：

```rust
use std::marker;

struct Iter<'a, T: 'a> {
    ptr: *const T,
    end: *const T,
    _marker: marker::PhantomData<&'a T>,
}

fn main() {
    let _ = core::mem::size_of::<Iter<'static, u8>>();
}
```

**身份二：决定型变。** `PhantomData<T>` 的型变就跟着 `T` 走，于是选哪种写法就等于声明哪种型变：

| 写法 | 模拟的字段 | 型变 |
|---|---|---|
| `PhantomData<T>` | 一个 `T` | 协变 |
| `PhantomData<&'a T>` | `&'a T` | `'a`、`T` 都协变 |
| `PhantomData<&'a mut T>` | `&'a mut T` | `'a` 协变、`T` 不变 |
| `PhantomData<fn(T)>` | `fn(T)` | **逆变** |
| `PhantomData<fn() -> T>` | `fn() -> T` | 协变 |
| `PhantomData<*mut T>` | `*mut T` | **不变** |

16.2 里的三个验证块就是这张表的前三行与后两行的实例：`PhantomData<&'a ()>` 协变、`PhantomData<fn(&'a ())>` 逆变、`PhantomData<*mut &'a ()>` 不变。Nomicon 认为这种「用字段模拟」的做法比「直接声明想要的型变」更不容易出错：

> This was deemed to be less error-prone than explicitly telling the type-system the kind of variance that you want, while also providing other useful things such as auto traits and the information needed by drop check.

**身份三：参与析构检查与 auto trait。** 上面的引文提到两件事：`PhantomData<T>` 会像真字段一样影响 `Send`/`Sync` 这类 auto trait，也会影响 dropck。前者很直观——`PhantomData<*const T>` 让类型带上裸指针的 `!Send`：

```rust,compile_fail
use std::marker::PhantomData;

struct NotSend<T> {
    _marker: PhantomData<*const T>,
}

fn needs_send<T: Send>(_: T) {}

fn main() {
    needs_send(NotSend::<u8> { _marker: PhantomData });
}
```

实测报错（关键行）：`` `*const u8` cannot be sent between threads safely ``，紧跟的 note 是 `` the trait `Send` is not implemented for `*const u8` ``，并附了一条「required because it appears within the type `PhantomData<*const u8>`」。换成 `PhantomData<T>` 就通过了。

dropck 这一侧有个现代结论值得记住：**如果一个类型已经写了 `Drop` 实现，再加 `_owns_T: PhantomData<T>` 对 dropck 是多余的。** Nomicon 原文：

> adding an extra `_owns_T: PhantomData<T>` field is thus *superfluous* and accomplishes nothing

理由来自 RFC 1238：`impl<T> Drop for Vec<T>` 的存在本身就让编译器认为「它拥有 `T`」——因为在析构实现里可能用到 `T`。所以自己写集合、或写自家这类泛型类型时，不再需要模仿老式 `Vec` 的写法；`PhantomData` 在该处的价值回到型变与 auto trait。

一手证据就在标准库源码里：`Unique<T>` 带着一个这样的字段，注释写着 “this marker has no consequences for variance, but is necessary for dropck to understand that we logically own a `T`”（本机 1.98：`struct Unique` 的定义从 `library/core/src/ptr/unique.rs:35` 开始，这条注释在 37–38 行）；`RawVec<T, A>` 里也是同一手笔（`library/alloc/src/raw_vec/mod.rs:75`）。

这句注释解决的是 16.4 那个 `#[may_dangle]` 的「例外的例外」：`Vec` 的 `Drop` 声明了 `#[may_dangle] T`，但 `#[may_dangle]` 会在参数**以拥有的方式被字段持有**时自动失效——只要 `T` 带着 drop glue，它仍需被正确析构（drop glue 指编译器自动生成的递归析构代码：一个类型即使没写 `Drop`，只要字段需要析构就带得有它）。`PhantomData<T>` 正是用来声明「我们逻辑上拥有一个 `T`」的。Nomicon 的 `phantom-data` 页把这条推导写得很完整。

### 16.6 省略规则的精确表

第 4 章见过 `E0106`。这一节把规则钉死。Reference 的 `lifetime-elision` 页给出三条函数侧规则（适用于函数项、函数指针、闭包 trait 签名）：

> - Each elided lifetime in the parameters becomes a distinct lifetime parameter.
> - If there is exactly one lifetime used in the parameters (elided or not), that lifetime is assigned to *all* elided output lifetimes.
> - If the receiver has type `&Self` or `&mut Self`, then the lifetime of that reference to `Self` is assigned to all elided output lifetime parameters.

译文与例子：

| 规则 | 例子 |
|---|---|
| 每个省略的输入生命周期各是一个独立参数 | `fn f(a: &str, b: &str)` → `fn f<'a, 'b>(a: &'a str, b: &'b str)` |
| 输入里**恰有一个**生命周期 → 输出全部用它 | `fn substr(s: &str, until: usize) -> &str` → `<'a>(s: &'a str, until: usize) -> &'a str` |
| 方法接收者是 `&self`/`&mut self` → 输出全部用它 | `fn get_mut(&mut self) -> &mut dyn T` → `<'a>(&'a mut self) -> &'a mut dyn T` |

「恰有一个」是字面意思：`&Words` 其实含**两个**生命周期（引用的、以及 `Words` 自己的），所以下面这种写法无法推断：

```rust,compile_fail
struct Words<'a>(&'a str);

fn first(w: &Words) -> &str {
    w.0
}

fn main() {}
```

实测报错（关键行）：

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:3:24
  |
3 | fn first(w: &Words) -> &str {
  |             ------     ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but the signature does not say which one of `w`'s 2 lifetimes it is borrowed from
help: consider introducing a named lifetime parameter
  |
3 | fn first<'a>(w: &'a Words<'a>) -> &'a str {
  |         ++++     ++      ++++      ++
```

正确写法是显式写明两处的关系：`fn first<'a>(w: &'a Words<'a>) -> &'a str`。同理，`fn get_str() -> &str` 与 `fn frob(s: &str, t: &str) -> &str` 都报 `E0106`——前者的 help 提醒「函数的返回类型包含借用值，但没有可借的来源」，后者会说「签名没有说明借用自 `s` 还是 `t`」。（Reference 的原话是 “It is an error to elide lifetime parameters that cannot be inferred.”）

**trait object 的默认生命周期**是另一套规则（不属于上面的函数省略）：

- 若 trait object 是某个泛型类型的类型参数，先用**外层类型**推出的唯一 bound；有多个 bound 就必须显式写；
- 否则若 trait 只声明了一个生命周期 bound，用那个；
- 出现 `'static` 就用 `'static`；
- trait 没有任何生命周期 bound 时，在表达式里推断、在表达式之外是 `'static`。

最容易踩的是最后一条：

```rust,compile_fail
fn to_display(x: &str) -> Box<dyn std::fmt::Display> {
    Box::new(x)
}

fn main() {
    println!("{}", to_display("hi"));
}
```

实测报错（关键行）：

```text
error: lifetime may not live long enough
 --> src/main.rs:2:5
  |
1 | fn to_display(x: &str) -> Box<dyn std::fmt::Display> {
  |                  - let's call the lifetime of this reference `'1`
2 |     Box::new(x)
  |     ^^^^^^^^^^^ returning this value requires that `'1` must outlive `'static`
  |
help: to declare that the trait object captures data from argument `x`, you can add an explicit `'_` lifetime bound
  |
1 | fn to_display(x: &str) -> Box<dyn std::fmt::Display + '_> {
  |                                                     ++++
```

`Box<dyn Display>` 就是 `Box<dyn Display + 'static>`，所以返回借用自参数的 `x` 不行；写成 `Box<dyn Display + '_>` 即可。反过来，放在引用后面时默认跟随引用：`&'a dyn Foo` 等于 `&'a (dyn Foo + 'a)`。

最后两个边角：

- `const`/`static` 里的引用类型生命周期**隐式是 `'static`**（`const STRING: &str = "…";` 即 `&'static str`）；若其中包含函数或闭包引用，编译器先按上面的标准规则解析，解不出来才报错；
- `'_` 是「这里省略、请推断」的占位写法，在路径类型里（`Words<'_>`）比裸省略更清楚，Reference 明确推荐。

## 动手看看

### 实验 A：析构顺序

源码在 `snippets/ch16/drop_order.rs`，程序把每次析构记进一个 `thread_local` 的 `Vec`，最后断言顺序。它属于「读者看 5 行、实际是完整程序」那类，所以放进文件：

```rust
{{#rustdoc_include ../snippets/ch16/drop_order.rs}}
```

实测输出（`rustc --edition 2024 drop_order.rs -o drop_order` 后运行）：

```text
drop 内层作用域变量
drop 变量 z
drop 模式绑定 y
drop 模式绑定 x
drop 元组 .0
drop 元组 .1
drop 结构体字段 first
drop 结构体字段 second
总顺序：内层作用域变量 -> 变量 z -> 模式绑定 y -> 模式绑定 x -> 元组 .0 -> 元组 .1 -> 结构体字段 first -> 结构体字段 second
```

对照输出看两点：`_inner` 所在的内层作用域先结束，所以它最先析构；`run` 的局部变量整体逆序（`z` → 模式绑定 `y`、`x` → 元组 → 结构体），而结构体字段与元组元素各自**正序**。

`y` 先于 `x` 析构这一点依赖 1.91 起的行为。release notes 的条目原文是 “Lower pattern bindings in the order they're written and base drop order on primary bindings' order”——模式里的绑定按书写顺序登记，析构顺序以主绑定的书写顺序为基准。

### 实验 B：把回调存进结构体

源码在 `snippets/ch16/hrtb_callback.rs`：

```rust
{{#rustdoc_include ../snippets/ch16/hrtb_callback.rs}}
```

实测输出：

```text
alpha
delta
```

两处值得注意：一是 `impl` 的 `where` 子句让 `run` 能对任意生命周期的 `&str` 调用，并把返回值绑到入参上（`fn run<'s>(&self, s: &'s str) -> &'s str`）；二是同一个实例先用更长的 `owned`、再用更短的 `local` 调用——闭包那路靠 `annotate` 拿到期望签名，`second_word` 作为函数项本来就满足高阶约束。

### 实验 C：亲手造一个协变 / 不变 / 逆变的包装

把 16.2 的三个结构体放进同一个文件，逐个看编译结果：`Co`（`PhantomData<&'a ()>`）与 `Contra`（`PhantomData<fn(&'a ())>`）的两个方向都能过，`Inv`（`PhantomData<*mut &'a ()>`）的两个方向都过不了。这不是「编译器脾气古怪」，而是型变表在起作用；把三个结构体的 `_marker` 类型互换，能通过的方向也会跟着换。

## 报错现场

读法与第 2 章一致：先看第一行 `error[Exxxx]:` 的结论，再看 `-->` 的位置，然后看 `^`/`-` 标出的对应关系，最后读 `help:`。

### 现场 1：`E0597` 析构检查（dropck）

```rust,compile_fail
struct Inspector<'a>(&'a u8);

impl<'a> Drop for Inspector<'a> {
    fn drop(&mut self) {
        println!("I was only {} days from retirement!", self.0);
    }
}

struct World<'a> {
    inspector: Option<Inspector<'a>>,
    days: Box<u8>,
}

fn main() {
    let mut world = World {
        inspector: None,
        days: Box::new(1),
    };
    world.inspector = Some(Inspector(&world.days));
}
```

真实输出（关键几行）：

```text
error[E0597]: `world.days` does not live long enough
  --> src/main.rs:19:38
   |
15 |     let mut world = World {
   |         --------- binding `world` declared here
...
19 |     world.inspector = Some(Inspector(&world.days));
   |                                      ^^^^^^^^^^^ borrowed value does not live long enough
20 | }
   | -
   | |
   | `world.days` dropped here while still borrowed
   | borrow might be used here, when `world` is dropped and runs the destructor for type `World<'_>`
```

**定位路径**：结论行说的是 `world.days` 活不够久，但两个标记点是「`world.days` 在这里被借用」与「`world` 在这里被析构」，而且最后一句直接点出「当 `world` 被析构、为 `World<'_>` 运行析构函数时，这份借用可能被使用」。也就是说，问题不在借用的使用点，而在**析构点**——这正是 dropck 在接管。

**修法**：析构时确实要用到那份数据，就必须让它**严格**比持有者活得久。把 `days` 移到 `World` 外面，并且声明在 `world` **之前**——局部变量逆序析构，于是 `world` 先析构，`days` 那时还在：

```rust
struct Inspector<'a>(&'a u8);

impl<'a> Drop for Inspector<'a> {
    fn drop(&mut self) {
        println!("I was only {} days from retirement!", self.0);
    }
}

struct World<'a> {
    inspector: Option<Inspector<'a>>,
}

fn main() {
    let days = Box::new(1);
    let mut world = World { inspector: None };
    world.inspector = Some(Inspector(&days));
}
```

把 `days` 留在 `World` 里、只是换个声明位置是没用的：那样借用指向的是被析构者自己的字段（本机实测仍报同一条 `E0597`）。关键是**谁先析构**，而不是借用写在哪儿。

顺带一提，把 `Inspector` 的 `Drop` 实现删掉，原版也能编译：没有析构函数就没有「析构时访问借用」的可能。这正是 16.4 说的「保守性来自信息不可见」。

### 现场 2：`E0597` `&mut T` 的不变性

```rust,compile_fail
fn assign<T>(input: &mut T, val: T) {
    *input = val;
}

fn main() {
    let mut hello: &'static str = "hello";
    {
        let world = String::from("world");
        assign(&mut hello, &world);
    }
    println!("{hello}");
}
```

真实输出（关键几行）：

```text
error[E0597]: `world` does not live long enough
 --> src/main.rs:9:28
  |
6 |     let mut hello: &'static str = "hello";
  |                    ------------ type annotation requires that `world` is borrowed for `'static`
7 |     {
8 |         let world = String::from("world");
  |             ----- binding `world` declared here
9 |         assign(&mut hello, &world);
  |                            ^^^^^^ borrowed value does not live long enough
10 |     }
   |     - `world` dropped here while still borrowed
```

**定位路径**：类型标注 `&'static str` 是第一标记点（「要求 `world` 被借用为 `'static`」），`&world` 是第二标记点，`}` 上的一笔说明 `world` 在这里被析构却仍被借用。`assign` 本身没问题，问题是 `&mut T` 对 `T` 不变：`T` 被钉死为 `&'static str`，导致实参 `&world` 必须满足 `'static`。

**修法**：看你要什么。要让 `hello` 真正存下短命引用，就把它的类型改成能表达的关系（用泛型函数或 `&mut &'a str`，别再写死 `'static`）；如果只是想在局部用一下，写成 `&mut &'w str` 或干脆传值。

### 现场 3：`E0308` 逆变与 HRTB

```rust,compile_fail
fn call_with_local(f: fn(&str)) {
    let s = String::from("local");
    f(&s);
}

fn store(input: &'static str) -> &'static str {
    input
}

fn main() {
    call_with_local(store);
}
```

真实输出（关键几行）：

```text
error[E0308]: mismatched types
  --> src/main.rs:11:21
   |
11 |     call_with_local(store);
   |     --------------- ^^^^^ one type is more general than the other
   |     |
   |     arguments to this function are incorrect
   |
   = note: expected fn pointer `for<'a> fn(&'a _) -> ()`
                 found fn item `fn(&'static _) -> &'static str {store}`
note: function defined here
  --> src/main.rs:1:4
   |
1 | fn call_with_local(f: fn(&str)) {
   |    ^^^^^^^^^^^^^^^ -----------
```

**定位路径**：note 里把两个类型并排展示了：期望的是 `for<'a> fn(&'a _) -> ()`（对任意生命周期都成立），拿到的是 `fn(&'static _)`。`call_with_local` 的参数写 `fn(&str)`，按省略规则就是 `for<'a> fn(&'a str)`；而 `store` 只接受 `'static`，是「更不通用」的那个。`one type is more general than the other` 这句话可以直接读成「一方比另一方更泛」。

**修法**：要么让 `store` 通用（把参数改成 `&str`），要么调用方别要求任意生命周期（把 `f` 的类型改成 `fn(&'static str)` 并相应调整调用方式）。这条报错也是「函数类型在参数位逆变」的直接体现：反方向（通用 → 只接受 `'static`）是允许的，正方向不允许。

### 现场 4：`E0392` 未使用的生命周期参数

```rust,compile_fail
struct Iter<'a, T> {
    ptr: *const T,
    end: *const T,
}

fn main() {}
```

真实输出（关键行）：

```text
error[E0392]: lifetime parameter `'a` is never used
 --> src/main.rs:1:13
  |
1 | struct Iter<'a, T> {
  |             ^^ unused lifetime parameter
  |
  = help: consider removing `'a`, referring to it in a field, or using a marker such as `PhantomData`
```

**定位路径**：报错指向参数声明本身而不是用法，help 给出三条出路。裸指针 `*const T` 不会「持有」`'a`，所以这个参数在类型里无处安放。

**修法**：`_marker: PhantomData<&'a T>`（16.5 身份一）。选 `PhantomData<&'a T>` 而不是 `PhantomData<T>`，是因为这里要表达的是「逻辑上含有一批 `&'a T`」，顺带把 `'a` 的协变也定下来。

### 报错码速查

| 码 | 结论行 | 本章对应机制 | 先去查什么 |
|---|---|---|---|
| `E0597` | `` `x` does not live long enough `` | variance 或 dropck | 两个标记点是不是「析构点」？是 → dropck；否 → 看是不是不变位置 |
| `E0308` | `mismatched types` + `one type is more general than the other` | 逆变 / HRTB | note 里期望类型是不是 `for<'a> …` |
| `E0637` | `` `&` without an explicit lifetime name cannot be used here `` | HRTB | 报错位置是不是 trait 参数里的 `&str` |
| `E0392` | `lifetime parameter 'a is never used` | `PhantomData` 占位 | 参数是否只在裸指针或 `PhantomData` 之外无处出现 |
| `E0106` | `missing lifetime specifier` | 省略规则 | 输入里生命周期是否「恰有一个」 |
| `E0599` | ``no method named `run` found for struct `FirstWord<…>` ``（且 impl 带 `for<'a>`） | 闭包没有期望签名 | 闭包是不是先被 `let` 绑定了/装进结构体字段了 |

## 小结

- **子类型**负责灵活性：`'long <: 'short` 当且仅当 `'long` 完全包含 `'short`；`&'a T` 对 `'a` 协变，所以「长的当短的用」在只读位置成立。
- **型变**负责安全：`&mut T` 对 `T` 不变（能写就不能降级），`Cell`/`UnsafeCell` 同理；`fn(T)` 是语言里**唯一**的逆变来源；结构体的型变由字段推导，不变赢得冲突。1.98 给 `&mut` 的 unsize 强转开了缩短的小口子。
- **HRTB** 负责范围：`for<'a> Fn(&'a str) -> &'a str` 表示「对所有 `'a` 成立」。闭包 trait 签名里的省略会自动变成这种形式；非闭包 trait 的参数、输入多于一个而要指定输出跟随哪一个、以及需要给闭包补签名的场合，才要手写 `for<'a>`。闭包没有期望签名时会退化成某个具体生命周期。
- **dropck** 负责时机：泛型类型的析构实现可能访问借用数据，所以它的泛型参数必须**严格**比它活得久。检查是保守的（看不到 `Drop` 函数体），必要时标准库用 nightly 的 `#[may_dangle]` + `unsafe impl` 自行承诺。
- **`PhantomData`** 是「声明一个不存在的字段」：占位（避开 `E0392`）、指定型变（`&'a T` / `fn(T)` / `*mut T` 三种写法对应三个方向）、参与 dropck 与 auto trait。已有 `Drop` 实现时，它对 dropck 是多余的（RFC 1238）。
- **省略规则**是「恰好一个输入生命周期」这一条硬条件加两个特例（唯一输入、`&self` 接收者）；`&Words` 这类含两个生命周期的写法推不出来。trait object 有自己的默认规则，`Box<dyn Trait>` 就是 `+ 'static`。

## 练习

**题 1.** 下面三个包装类型，分别判断它对 `'a` 是协变、逆变还是不变；然后预测：把 `Co<'static>`、`Inv<'static>`、`Contra<'static>` 分别传给 `fn takes<'a>(_: &'a String, _: 对应类型<'a>)`，哪些能编译？

```rust,ignore
use std::marker::PhantomData;

struct Co<'a> { _marker: PhantomData<&'a ()> }
struct Inv<'a> { _marker: PhantomData<*mut &'a ()> }
struct Contra<'a> { _marker: PhantomData<fn(&'a ())> }
```

**题 2.** 下面这个函数的本意是「随便传个计数回调」，但调用方只要拿局部变量去调就会失败。改哪里？为什么不能靠加生命周期参数 `<'a>` 解决？

```rust,ignore
fn count<'a, F>(f: F) -> usize
where
    F: Fn(&'a str) -> usize,
{
    let s = String::from("local");
    f(&s)
}
```

**题 3.** 下面这个程序能编译。回答三问：它的析构顺序是什么？为什么 `data` 必须声明在 `bag` 之前？如果把 `data` 挪回 `Bag` 内部（现场 1 的 `World` 形态）会怎样，此时再删掉 `Holder` 的 `impl Drop` 又会怎样？

```rust
struct Holder<'a>(&'a u8);

impl<'a> Drop for Holder<'a> {
    fn drop(&mut self) {
        println!("drop Holder({})", self.0);
    }
}

struct Bag<'a> {
    holder: Option<Holder<'a>>,
}

fn main() {
    let data = Box::new(1);
    let mut bag = Bag { holder: None };
    bag.holder = Some(Holder(&data));
}
```

<details><summary>参考答案</summary>

**题 1.** `Co` 协变，`Inv` 不变，`Contra` 逆变。

- `Co<'static>` → 更短的 `'a`：**通过**（协变方向，16.2 第一个验证块）；
- `Inv<'static>` → 更短的 `'a`：**不通过**（不变，报 `E0597`，报错里会有「type annotation requires that `s` is borrowed for `'static`」这类提示）；
- `Contra<'static>` → 更短的 `'a`：**不通过**；反过来「更短的 `Contra` → `'static` 位置」**通过**（逆变，16.2 第三个验证块）。

理由是 `PhantomData` 模拟字段的型变：`&'a ()` 对 `'a` 协变，`*mut &'a ()` 里 `'a` 落在不变位置，`fn(&'a ())` 则把方向翻了过来。

**题 2.** 把 `F: Fn(&'a str) -> usize` 换成 `F: for<'a> Fn(&'a str) -> usize`（或者直接省略成 `F: Fn(&str) -> usize`，它会自动变成高阶形式）。

不能靠自己的 `<'a>` 解决，是因为函数签名里的 `<'a>` 是**调用方**选择的：调用方传入 `'static` 时，`'a` 就被钉成 `'static`，函数体里那个局部 `String` 显然满足不了。`for<'a>` 放在 trait 约束里，表达的是「对**每一个** `'a` 都成立」，于是函数体可以拿自己造的短命值去调用。实测报错是 `E0597`，关键行是 `` argument requires that `s` is borrowed for `'a` ``——配上 `` lifetime `'a` defined here ``，正好说明 `'a` 是外部给的。

**题 3.** 实测输出只有一行：`drop Holder(1)`。顺序与 16.4 的表一致——

1. 局部变量逆序析构：`bag` 比 `data` 晚声明，所以 `bag` 先走，连带析构它的字段 `holder`（`Option<Holder<'a>>` 里的那个值），于是打印 `drop Holder(1)`；
2. 然后才轮到 `data`。

`data` 必须声明在 `bag` 之前，是因为 `bag.holder` 借用了 `data`，而这个借用就写在 `Bag<'a>` 的类型参数里；`Holder` 又实现了 `Drop`，于是 dropck 要求被借用的数据**严格**比持有者活得久——「局部变量逆序析构」正好保证持有者先走。

把 `data` 挪回 `Bag` 内部（现场 1 的 `World` 形态）会得到同一条 `E0597`：那时借用指向的是被析构者自己的字段，`holder` 可能先于 `data` 被析构。在这个形态下再删掉 `Holder` 的 `impl Drop`，代码又能编译——dropck 的保守性建立在「析构函数里可能有任意代码」的假设上：没有 `Drop` 实现，就没有「析构时访问借用」的可能，这一层检查不再需要提出「严格活得更久」的要求。（这不代表「没有 `Drop` 就一定安全」，只说明这项检查不适用。）
</details>

## 延伸阅读

- Nomicon：子类型与型变（<https://doc.rust-lang.org/nomicon/subtyping.html>）、HRTB（<https://doc.rust-lang.org/nomicon/hrtb.html>）、drop check（<https://doc.rust-lang.org/nomicon/dropck.html>）、`PhantomData`（<https://doc.rust-lang.org/nomicon/phantom-data.html>）
- Reference：生命周期省略（<https://doc.rust-lang.org/reference/lifetime-elision.html>）、子类型与型变（<https://doc.rust-lang.org/reference/subtyping.html>）、析构（<https://doc.rust-lang.org/reference/destructors.html>）
- 标准库：`PhantomData`（<https://doc.rust-lang.org/std/marker/struct.PhantomData.html>）、`Drop`（<https://doc.rust-lang.org/std/ops/trait.Drop.html>）、`RefCell`（<https://doc.rust-lang.org/std/cell/struct.RefCell.html>）
- 版本与发布说明：<https://doc.rust-lang.org/stable/releases.html>（本章引用的 1.91 模式绑定析构顺序、1.98 `&mut` 缩短两条都在这里）
- RFC：dropck 的 `#[may_dangle]`（<https://github.com/rust-lang/rfcs/blob/master/text/1327-dropck-param-eyepatch.md>）、非参数化 dropck（<https://github.com/rust-lang/rfcs/blob/master/text/1238-nonparametric-dropck.md>）、析构顺序（<https://github.com/rust-lang/rfcs/blob/master/text/1857-stabilize-drop-order.md>）
- 报错码逐条解释：把错误码换成 `E0597`、`E0308`、`E0637`、`E0392`、`E0106`（<https://doc.rust-lang.org/error_codes/E0597.html>）
