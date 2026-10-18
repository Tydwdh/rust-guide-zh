# 第 14 章 impl Trait 与 opaque type

## 这一章解决什么问题

第 13 章处理的是「运行时多态」：`Box<dyn Trait>` 把具体类型藏进 vtable，调用者只认 trait，代价是堆分配和间接调用。这一章处理另一半问题：**编译期就要定下类型时，怎么把具体类型从签名里藏掉**。

先看一个从第 10 章起就一直在用、却一直没解释的写法：

```rust
fn evens(xs: &[u32]) -> impl Iterator<Item = u32> {
    xs.iter().copied().filter(|x| x % 2 == 0)
}
```

`evens` 返回的是个真实类型，名字长这样：`Filter<Copied<std::slice::Iter<'_, u32>>, {closure@…}>`。其中的闭包类型由编译器生成，根本没有名字，你写不出来。`impl Iterator<Item = u32>` 让函数**说清它满足什么契约，而不说它是什么**——这就是不透明类型（opaque type）。

「返回一个迭代器」「返回一个 future」「接受任意可打印的值」——这类签名在真实代码里随处可见，它们背后的规则都归本章管。而且这套规则在 edition 2024 改过一次：同一段代码在 2021 与 2024 下可能一个报错、一个通过。

写完这一章你能做到：

- 分清参数位置与返回位置的 `impl Trait` 各自代表「谁选类型」；
- 在 `-> impl Trait`、`Box<dyn Trait>`、自定义 enum 之间做出有理由的选择；
- 读懂 `use<..>` 与 edition 2024 的捕获（capture）规则，明白生命周期错误为什么会凭空冒出来；
- 写出能在 `Send` 边界上使用的 trait 异步方法，并知道 `async fn in trait` 的 future 为什么不保证 `Send`。

## 为什么这样设计

### 同一处语法，两处相反的角色

`impl Trait` 出现在哪，含义完全不同：

- **参数位置**：`fn show(x: impl Debug)`。类型由**调用者**给出，函数只知道它满足 `Debug`。Rust Reference 把它叫匿名类型参数，并说它是「syntactic sugar for a generic type parameter like `<T: Trait>`, except that the type is anonymous and doesn’t appear in the `GenericParams` list」。
- **返回位置**：`fn make() -> impl Debug`。类型由**函数**给出，调用者只知道它满足 `Debug`。Reference 的原文是：「With `impl Trait`, unlike with a generic type parameter, the function chooses the return type, and the caller cannot choose the return type.」

一句话：参数位置是「存在一个类型，你给」；返回位置是「存在一个类型，我不说」。前者常写作 APIT（argument-position impl Trait），后者写作 RPIT（return-position impl Trait）。

### 藏掉返回类型买到了什么

第一层好处是「不用写出不可能写出的类型」——闭包、迭代器链。第二层更值钱：**具体类型不在签名里，它就不属于契约的一部分**。

拿本章开头那个 `evens` 说：返回类型写成 `Box<dyn Iterator<Item = u32>>` 也能用，但每次调用都要堆分配、每次 `next()` 都要查 vtable。写成 `impl Iterator<Item = u32>`，返回的仍是那个具体的 `Filter<...>` 值——没有分配，`next()` 还能被内联。

两种「隐藏」的分工：

| | `-> impl Trait` | `-> Box<dyn Trait>` |
|---|---|---|
| 隐藏发生在 | 编译期（调用者看不到名字） | 运行期（忘了具体类型） |
| 分派方式 | 静态：单态化后直接调用、可内联 | 动态：查 vtable |
| 额外分配 | 无 | 每次装箱一次堆分配 |
| 一个位置能装几种类型 | 一种（所有返回路径必须同一具体类型） | 任意多种 |
| 调用者能要求什么 | 写出来的 trait 约束，外加具体类型透传过来的 auto trait | 只有签名里显式写出来的那些（如 `dyn Trait + Send`） |

auto trait（自动 trait：`Send`、`Sync` 这类由编译器为每个类型自动推导的 trait）在两侧行为不同：`impl Trait` 会把隐藏类型的 auto trait 带出来，`dyn Trait` **不会**——它只承认写在类型里的那些。把 `Box<dyn Foo>` 交给 `fn needs_send<T: Send>(_: T)`，实测关键行是 ``error[E0277]: `dyn Foo` cannot be sent between threads safely`` 与 ``= help: the trait `Send` is not implemented for `dyn Foo` ``；写成 `Box<dyn Foo + Send>` 才能通过（第 13 章讲过 auto trait 在 trait 对象类型里的位置）。`impl Trait` 一侧的透传细节见「不透明的边界」。

（`dyn` 一侧的兼容性规则见第 13 章。）

### 不透明不等于「不可命名」

有一点容易混：闭包类型本来就不可命名，但**不透明类型是主动选择的结果**。「不告诉你具体类型」既能解决「写不出来」的问题，也能解决「不想把实现细节写进契约」的问题。

这也解释了两者的另一处差别。`fn make<T: Trait>() -> T` 里返回类型由**调用者**决定，`fn make() -> impl Trait` 里由**函数**决定。想让调用者挑实现，用泛型参数（第 9、12 章）；想让函数自己挑实现、把选择留在库内部，用 RPIT。

还要记住边界：RPIT 藏掉的是**名字与身份**，不是行为。调用者仍然按你写出来的约束使用返回值；函数内部把隐藏类型（hidden type，即不透明类型背后那个具体类型）换成一个满足同样约束的类型，调用方通常不用改——这正是 RPIT 在 API 设计里的主要卖点（例外是 auto trait，见「不透明的边界」）。

## 机制与规则

### 参数位置：匿名泛型参数

```rust
fn with_generic<T: std::fmt::Debug>(arg: T) {
    println!("{arg:?}");
}

fn with_apit(arg: impl std::fmt::Debug) {
    println!("{arg:?}");
}
```

两者「几乎等价」，差别有两处值得记住。

第一，匿名参数**不能**在调用点显式指定，因为它没有名字（E0107，见「报错现场」）。第二，`impl Trait` 只属于「参数或返回类型」这两个位置，别处一律不行。Reference 的原话是：「`impl Trait` can only appear as a parameter or return type of a non-extern function. It cannot be the type of a `let` binding, field type, or appear inside a type alias.」在参数里，它可以嵌在类型内部——`fn show_all(items: Vec<impl Debug>)` 是合法的。

> [!TIP]
> 调用点需要显式指定类型（`f::<u32>(...)`）、或想让读者一眼看出「这里有个类型参数」时，用命名的泛型参数；否则 `impl Trait` 更短，也省掉了给参数起名。

反过来，把参数类型从签名里删掉也有代价：函数体内只能用约束提供的能力。下面这个函数接受任意可打印的值，把它原样交还——返回的不透明类型捕获了 APIT 引入的匿名类型参数，所以调用者拿到的仍然是「某个实现 `Debug` 的值」，而不是某个具体类型：

```rust
fn identity(x: impl std::fmt::Debug) -> impl std::fmt::Debug {
    x
}

fn main() {
    let v = identity(vec![1, 2]);
    println!("{v:?}");
}
```

输出（Rust 1.98 实测）：

```text
[1, 2]
```

### 返回位置：不透明类型

最直接的理由就是「类型写不出来」：

```rust
fn adder() -> impl Fn(i32) -> i32 {
    |x| x + 1
}

fn main() {
    println!("{}", adder()(1));
}
```

输出（Rust 1.98 实测）：

```text
2
```

迭代器还多一层好处：调用者只见 `Iterator`，链中间那串包装类型既不进文档、也不进签名。

但有一条硬规则必须记住：**一个函数（一个泛型实例化）只对应一个隐藏类型**。Reference 的写法是：「Each possible return value from the function must resolve to the same concrete type.」所以两条路径类型不同就编译不过（E0308，见「报错现场」）；反过来，若两条路径给出同一个具体类型，就完全没问题：

```rust
fn one(flag: bool) -> impl Iterator<Item = u32> {
    if flag {
        std::iter::once(1u32)
    } else {
        std::iter::once(2u32)
    }
}

fn main() {
    for v in one(true) {
        println!("{v}");
    }
}
```

输出：

```text
1
```

「同一个不透明类型」还有一层含义：同一函数、同一泛型实参的两次调用返回同一类型，可以互相比较：

```rust
fn value() -> impl PartialEq + std::fmt::Debug {
    42u32
}

fn main() {
    let a = value();
    let b = value();
    println!("{:?}", a == b);
}
```

```text
true
```

而不同泛型实参就是不同的不透明类型（E0308，见「报错现场」第 6 条）。

三条路的选择：

| 情形 | 选什么 | 代价 |
|---|---|---|
| 只有一种实现 | `impl Trait` | 无 |
| 两类以上实现，运行期才知道是哪类 | `Box<dyn Trait>` | 堆分配 + 动态分派 |
| 有限且封闭的几类，想保留静态分派 | 自定义 enum | 每加一种都要改 enum 与 match |

### 捕获：隐藏类型能用到哪些泛型参数

不透明类型对外是「一个类型」，对内却是「某个具体类型」。隐藏类型想用某个泛型参数，前提是这个参数被不透明类型**捕获（capture）**了。规则分两半：

- **类型与 const 参数**：所有 edition 都自动捕获。Edition Guide 的说法是「In all editions, all in-scope type and const generic parameters are captured implicitly when the `use<..>` bound is not present.」固有 `impl` 的外层泛型参数也算「作用域内」。
- **生命周期参数**：这就是 2024 改动的那一半。

2021 及更早，普通函数与固有 `impl` 里的方法只捕获「出现在返回类型约束里的」生命周期。隐藏类型偷偷用了某个生命周期时，你得用 `+ '_`、`+ 'a`，或者社区那套 `Captures<..>` 戏法，把它「说出来」。2024 起无条件捕获**作用域内的全部**生命周期参数，与 RPITIT（return-position impl Trait in trait）和 AFIT（`async fn in trait`）的行为拉齐——两者在所有 edition 都是全捕获（详见「trait 里的 RPIT 与 `async fn`」）。

这次改动的收益，看本章开头那个签名就够了：它在 edition 2024 下直接编译通过，在 2021 下报错：

```text
error[E0700]: hidden type for `impl Iterator<Item = u32>` captures lifetime that does not appear in bounds
 --> src/main.rs:2:5
  |
1 | fn evens(xs: &[u32]) -> impl Iterator<Item = u32> {
  |                         ------------------------- opaque type defined here
2 |     xs.iter().copied().filter(|x| x % 2 == 0)
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: hidden type `Filter<Copied<std::slice::Iter<'_, u32>>, {closure@src/main.rs:2:31: 2:34}>` captures lifetime `'_`
```

（这段是 edition 2021 下的输出，以 Rust 1.98 为例。同一段代码在 2024 下不需要任何额外写法。）

捕获是双向的：捕多了也出错（见「报错现场」第 1 条）。**捕获集合越宽，调用者受到的约束越多；越窄，隐藏类型能借用的东西越少。**

> [!NOTE]
> 史前写法：在 1.82 引入精确捕获（precise capturing，也就是 `use<..>`）之前，社区靠两条戏法把生命周期「塞进」隐藏类型。一条是 **`Captures<..>` 戏法**——自己定义一个 `pub trait Captures<T: ?Sized> {}` 加一个全覆盖实现，再把它写进返回类型的约束；另一条是 **outlives 戏法**——给返回类型加 `+ 'a`，同时给相关类型参数加 `T: 'a`。Edition Guide 的原话是：「With the `use<..>` bound syntax, the `Captures` trick is no longer needed and can be replaced with the following in all editions.」outlives 戏法的额外代价是给调用者强加了一个本不需要的 `T: 'a`。两条戏法今天都还能编译，但新代码应当写 `use<..>`。

### `use<..>`：精确捕获

要逐项指定捕获哪些参数，用 1.82 起稳定的 `use<..>`（所有 edition 都可用）：

```rust
fn capture<'a, T>(x: &'a (), y: T) -> impl Sized + use<'a, T> {
    (x, y)
}

fn main() {
    let unit = ();
    let captured = capture(&unit, 5u32);
    println!("{}", std::any::type_name_of_val(&captured));
}
```

输出：

```text
(&(), u32)
```

（顺带证实：`type_name_of_val` 打印的正是隐藏类型，见「不透明的边界」。）

给 `capture` 换成 `use<>`（什么都不捕获），一个 `'static` 场景就通了：

```rust
fn capture<'a>(_: &'a ()) -> impl Sized + use<> {}

fn test<'a>(x: &'a ()) -> impl Sized + 'static {
    capture(x)
}

fn main() {}
```

`use<..>` 的规则不多，但几条容易撞：

- **只能出现一个** `use<..>`，且**必须列出作用域内全部类型与 const 参数**（生命周期可以少列）。漏了就报 `` error: `impl Trait` must mention all type parameters in scope in `use<...>` ``。
- 生命周期参数要写在类型与 const 参数之前；单个被省略的生命周期可以写成 `use<'_>`，例如 `fn f(x: &str) -> impl Iterator<Item = u32> + use<'_>`。
- **不能与参数位置的 `impl Trait` 同时出现在一个签名里**：APIT 的参数是匿名的，没法写进 `use<..>`（实测报的正是上面那条「must mention all type parameters」）。
- 在 trait 定义里还必须把 `Self` 一起列上（`use<'a, T, Self>`），因为 trait 的 `Self` 也在作用域内。trait 内的 `use<..>` 自 1.87 起可用。

这几条约束里最容易忘的是 `Self`。实测会按顺序连报两条错误：

```rust,compile_fail
trait Repo<T> {
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T> + use<'a, T>;
}

fn main() {}
```

```text
error[E0309]: the parameter type `T` may not live long enough
 --> src/main.rs:2:45
  |
2 |     fn items<'a>(&'a self) -> impl Iterator<Item = &'a T> + use<'a, T>;
  |                                             ^^^^^^^^--^^
  |                                             |       |
  |                                             |       the parameter type `T` must be valid for the lifetime `'a` as defined here...
  |                                             ...so that the reference type `&'a T` does not outlive the data it points at
  |
help: consider adding an explicit lifetime bound
  |
2 |     fn items<'a>(&'a self) -> impl Iterator<Item = &'a T> + use<'a, T> where T: 'a;
  |                                                                        +++++++++++

error: `impl Trait` must mention the `Self` type of the trait in `use<...>`
 --> src/main.rs:2:31
  |
1 | trait Repo<T> {
  | ------------- `Self` type parameter is implicitly captured by this `impl Trait`
2 |     fn items<'a>(&'a self) -> impl Iterator<Item = &'a T> + use<'a, T>;
  |                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: currently, all type parameters are required to be mentioned in the precise captures list
```

第一条是 `&'a T` 要求 `T: 'a`（隐藏类型要用 `&'a T`，就得让 `T` 活到 `'a`），第二条才是 `Self` 没被列出。两条一起处理：

```rust
trait Repo<T> {
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T> + use<'a, T, Self>
    where
        T: 'a;
}

fn main() {}
```

> [!WARNING]
> `impl Trait` 出现在**关联类型位置**仍不稳定。以最小复现 `type Out = impl Iterator<Item = u32>;`（写在 `impl` 里）为例，Rust 1.98 报 `` error[E0658]: `impl Trait` in associated types is unstable ``，并附一条指向 issue #63063 的 note（换一种写法还会连带出 `E0562` 之类的错误）。需要「带关联类型的迭代器」时，目前只有 GAT（第 15 章）与 `Box<dyn Iterator>` 两条路。

**迁移提示**：`impl_trait_overcaptures` lint（下划线是规范名，命令行上也接受连字符写法）属于 `rust-2024-compatibility` 组，默认 allow，`cargo fix --edition` 会启用它。它在 2021 下就能指出「这个 `impl Trait` 到 2024 会多捕生命周期」，并常自动补上 `use<>`：

```rust
fn f<'a>(x: &'a ()) -> impl Sized {
    *x
}
```

上面这段在 2024 下合法；换成 edition 2021、加 `-W impl_trait_overcaptures` 编译，输出是（已略去一条 `f` 未使用的 dead_code 警告）：

```text
warning: `impl Sized` will capture more lifetimes than possibly intended in edition 2024
 --> src/main.rs:1:24
  |
1 | fn f<'a>(x: &'a ()) -> impl Sized {
  |                        ^^^^^^^^^^
  |
note: specifically, this lifetime is in scope but not mentioned in the type's bounds
 --> src/main.rs:1:6
  |
1 | fn f<'a>(x: &'a ()) -> impl Sized {
  |      ^^
  = note: all lifetimes in scope will be captured by `impl Trait`s in edition 2024
  = warning: this changes meaning in Rust 2024
  = note: for more information, see <https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html>
  = note: requested on the command line with `-W impl-trait-overcaptures`
help: use the precise capturing `use<...>` syntax to make the captures explicit
  |
1 | fn f<'a>(x: &'a ()) -> impl Sized + use<> {
  |                                   +++++++
```

不想让调用者被生命周期绑住，也可以按编译器建议把 `+ 'static` 放宽成 `+ 'a`。

### trait 里的 RPIT 与 `async fn`（RPITIT / AFIT）

trait 的方法也能写 `-> impl Trait`（1.75 起稳定，常写作 RPITIT）。Reference 说它是语法糖：「Every `impl Trait` in the return type of an associated function in a trait is desugared to an anonymous associated type.」

```rust
trait Source {
    fn nums(&self) -> impl Iterator<Item = u32>;
}

struct S(Vec<u32>);

impl Source for S {
    fn nums(&self) -> impl Iterator<Item = u32> {
        self.0.iter().copied()
    }
}

fn main() {
    for v in S(vec![1, 2, 3]).nums() {
        println!("{v}");
    }
}
```

输出（Rust 1.98 实测）：

```text
1
2
3
```

同一版本还稳定了 `async fn in trait`（AFIT）。`async fn f()` 返回一个不透明的 `Future`，所以 AFIT 可以看成 RPITIT 的特例。

两个限制先记住：

**一、含 RPITIT 或 AFIT 的 trait 不能做 trait 对象。** Reference 的 dyn 兼容性规则里明确要求方法「Not have an opaque return type」，并列出了 `async fn`（隐藏 `Future` 类型）与 `fn example(&self) -> impl Trait` 两种情形。实测：

```rust,compile_fail
trait Job {
    async fn run(&self) -> u32;
}

fn use_it(j: &dyn Job) -> u32 {
    let _ = j.run();
    0
}

fn main() {}
```

```text
error[E0038]: the trait `Job` is not dyn compatible
 --> src/main.rs:5:15
  |
5 | fn use_it(j: &dyn Job) -> u32 {
  |               ^^^^^^^ `Job` is not dyn compatible
  |
note: for a trait to be dyn compatible it needs to allow building a vtable
      for more information, visit <https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility>
 --> src/main.rs:2:14
  |
1 | trait Job {
  |       --- this trait is not dyn compatible...
2 |     async fn run(&self) -> u32;
  |              ^^^ ...because method `run` is `async`
  = help: consider moving `run` to another trait
```

**二、公开 trait 里的 `async fn` 会被 lint 提醒。** 原因正是下一节的坑：future 上没地方写 `Send`。

```rust
pub trait Greeter {
    async fn greet(&self) -> String;
}
```

```text
warning: use of `async fn` in public traits is discouraged as auto trait bounds cannot be specified
 --> src/lib.rs:2:5
  |
2 |     async fn greet(&self) -> String;
  |     ^^^^^
  |
  = note: you can suppress this lint if you plan to use the trait only in your own code, or do not care about auto traits like `Send` on the `Future`
  = note: `#[warn(async_fn_in_trait)]` on by default
help: you can alternatively desugar to a normal `fn` that returns `impl Future` and add any desired bounds such as `Send`, but these cannot be relaxed without a breaking API change
  |
2 -     async fn greet(&self) -> String;
2 +     fn greet(&self) -> impl std::future::Future<Output = String> + Send;
  |
```

### AFIT 的 `Send` 坑

把 AFIT 和「要求 `Send`」的代码放在一起，就是本章最值得记住的那个坑（第 21 章的执行器正是这么要求的）：

```rust,compile_fail
trait Job {
    async fn run(&self) -> u32;
}

fn needs_send<F: Send>(_: F) {}

async fn use_it<J: Job>(j: J) {
    needs_send(j.run());
}

fn main() {}
```

```text
error[E0277]: `impl Future<Output = u32>` cannot be sent between threads safely
 --> src/main.rs:8:16
  |
8 |     needs_send(j.run());
  |     ---------- ^^^^^^^ `impl Future<Output = u32>` cannot be sent between threads safely
  |     |
  |     required by a bound introduced by this call
  |
  = help: the trait `Send` is not implemented for `impl Future<Output = u32>`
note: required by a bound in `needs_send`
 --> src/main.rs:5:18
  |
5 | fn needs_send<F: Send>(_: F) {}
  |                  ^^^^ required by this bound in `needs_send`
help: `Send` can be made part of the associated future's guarantees for all implementations of `Job::run`
  |
2 -     async fn run(&self) -> u32;
2 +     fn run(&self) -> impl std::future::Future<Output = u32> + Send;
  |
```

（编译器写的是全路径 `std::future::Future`；2024 的 prelude 里已经有 `Future`，所以本书正文与示例都写简称。）

原因：trait 里的 `async fn` 只承诺「返回某个 `Future<Output = u32>`」，没承诺它是 `Send`。泛型代码不能凭空假设实现者的 future 能跨线程。编译器给的修法就是把返回位置显式写出来，把 `Send` 写进契约：

```rust
trait Job {
    fn run(&self) -> impl Future<Output = u32> + Send;
}

struct Add(u32, u32);

impl Job for Add {
    fn run(&self) -> impl Future<Output = u32> + Send {
        let (a, b) = (self.0, self.1);
        async move { a + b }
    }
}

fn main() {
    let job = Add(1, 2);
    let _pending = job.run();
    println!("ok");
}
```

```text
ok
```

两个细节：

- 这里**不需要** `+ '_`。trait 里的 RPIT 在所有 edition 都会捕获作用域内的生命周期参数，返回类型已经绑定在 `&self` 上。
- 代价是「一刀切」：trait 上写了 `Send`，**所有**实现都必须给出 `Send` 的 future。只想对部分实现加要求时，要么另加约束（第 12 章的关联类型），要么退回 `Pin<Box<dyn Future<Output = T> + Send + '_>>`（future 装箱、有分配，换来「不必改 trait 其它方法的语义」，第 13 章同一口径）。

### 不透明的边界

「不透明」是**编译器前端**的概念，不是加密。边界上有三件事值得记住。

**一、auto trait 会透传。** 隐藏类型是 `Send`，不透明类型就也是 `Send`：

```rust
fn counter() -> impl Iterator<Item = u32> {
    (0..3).map(|x| x * 2)
}

fn needs_send<T: Send>(_: T) {}

fn main() {
    needs_send(counter()); // 编译通过：隐藏类型的 Send 透传出来
    for v in counter() {
        println!("{v}");
    }
}
```

输出（Rust 1.98 实测）：

```text
0
2
4
```

这是好事（调用者能放心把返回值丢给线程），但也意味着**换隐藏类型可能悄悄改变 auto trait**：原来返回的实现是 `Send`，换成内部用 `Rc` 的实现后就不再 `Send`，调用方会突然编译不过。

**二、写出来的约束会泄露关联信息。** `impl Iterator<Item = u32>` 里的 `Item = u32` 是契约的一部分；`impl Trait + Clone` 允许调用者克隆返回值。没写的就不算数。

**三、名字还能被打印出来。** `std::any::type_name_of_val` 会给出隐藏类型的名字：

```rust
fn values() -> impl Iterator<Item = u32> {
    (0..3).map(|x| x * 2)
}

fn main() {
    println!("{}", std::any::type_name_of_val(&values()));
}
```

输出（Rust 1.98 实测）：

```text
core::iter::adapters::map::Map<core::ops::range::Range<u32>, main::values::{{closure}}>
```

（前缀 `main` 是 crate 名，会随 crate 名与模块路径变化。）

但别把它当 API。std 文档对 `type_name` 的说明是：「This is intended for diagnostic use. The exact contents and format of the string returned are not specified, other than being a best-effort description of the type.」并且「The returned string must not be considered to be a unique identifier of a type as multiple types may map to the same type name.」

## 动手看看

下面是一段能跑的完整程序（`snippets/ch14/pipeline.rs`）。它把「解析 → 过滤 → 求和」拆成三个小函数：前两个返回 `impl Iterator`，最后一个把结果收成一个数。调用者完全不知道中间套了多少层适配器。

```rust
{{#rustdoc_include ../snippets/ch14/pipeline.rs}}
```

输出（Rust 1.98 实测）：

```text
总数：42
≥10 的行：["beta", "delta"]
alpha => 3
beta => 12
gamma => 7
delta => 20
```

两处 `+ use<'_>` 在 2024 下其实是多余的（生命周期本来就会被捕获）；写出来是想说明：这个签名精确捕获那一个被省略的输入生命周期，别的一个都不要。

第二个例子（`snippets/ch14/afit_send.rs`）把上一节的修法变成可运行的代码。`assert_send` 在**编译期**验证 `run()` 返回的 future 满足 `Send`；main 里不驱动它，因为我们还没讲执行器（第 21 章）。

```rust
{{#rustdoc_include ../snippets/ch14/afit_send.rs}}
```

输出（Rust 1.98 实测）：

```text
Add::run 的 future 满足 Send
```

> [!TIP]
> 想给「future 必须 `Send`」加个编译期断言，`fn assert_send<T: Send>(_: T) {}` 这种「吞掉参数」的写法最省事：调用点写 `assert_send(some_async_call())`，类型不满足就直接报 E0277。

## 报错现场

以下都是 Rust 1.98 实测，只保留关键行。

### 1. 多捕了生命周期

```rust,compile_fail
fn capture<'a>(_: &'a ()) -> impl Sized {}

fn test<'a>(x: &'a ()) -> impl Sized + 'static {
    capture(x)
}

fn main() {}
```

关键行：

```text
error: lifetime may not live long enough
 --> src/main.rs:4:5
  |
3 | fn test<'a>(x: &'a ()) -> impl Sized + 'static {
  |         -- lifetime `'a` defined here
4 |     capture(x)
  |     ^^^^^^^^^^ returning this value requires that `'a` must outlive `'static`
  |
help: consider changing `impl Sized + 'static`'s explicit `'static` bound to the lifetime of argument `x`
  |
3 - fn test<'a>(x: &'a ()) -> impl Sized + 'static {
3 + fn test<'a>(x: &'a ()) -> impl Sized + 'a {
  |
```

（后面还有一条「给这个引用加显式 `'static`」的 help，略。）

定位路径：报错在 `test` 的**调用点**，根因在 `capture` 的**签名**。2024 下 `capture` 的返回类型捕获了 `'a`，哪怕隐藏类型 `()` 根本用不到它。这不是借用检查器在挑剔：`capture` 返回的值**有可能**借用 `'a`（比如你以后把隐藏类型改成 `&'a ()`），所以签名必须按最宽的口径处理。修法两种：把 `capture` 的返回类型写成 `impl Sized + use<>`（明确不捕获），或把 `test` 的返回类型放宽成 `+ 'a`。

### 2. 两条返回路径类型不同（E0308）

```rust,compile_fail
fn pick(flag: bool) -> impl std::fmt::Display {
    if flag { 1u32 } else { "hello" }
}

fn main() {
    println!("{}", pick(true));
}
```

关键行：

```text
error[E0308]: `if` and `else` have incompatible types
 --> src/main.rs:2:29
  |
2 |     if flag { 1u32 } else { "hello" }
  |               ----          ^^^^^^^ expected `u32`, found `&str`
  |               |
  |               expected because of this
  |
help: you could change the return type to be a boxed trait object
  |
1 - fn pick(flag: bool) -> impl std::fmt::Display {
1 + fn pick(flag: bool) -> Box<dyn std::fmt::Display> {
```

定位路径：`impl Trait` 只绑定**一个**隐藏类型，`if`/`else` 两支必须给出同一类型。编译器提示的 `Box<dyn>` 是三条路之一（有堆分配，但不用给每种返回类型各写一段代码）。改用 `Box<dyn std::fmt::Display>` 后同一程序编译通过，实测输出 `1 hello`。

### 3. 返回借用了局部数据（E0597）

```rust,compile_fail
fn src(flag: bool) -> impl Iterator<Item = u32> {
    let a = vec![1u32, 2];
    let b = vec![3u32, 4];
    if flag {
        a.iter().copied()
    } else {
        b.iter().copied()
    }
}

fn main() {
    for v in src(true) {
        println!("{v}");
    }
}
```

关键行：

```text
error[E0597]: `a` does not live long enough
 --> src/main.rs:5:9
  |
2 |     let a = vec![1u32, 2];
  |         - binding `a` declared here
...
5 |         a.iter().copied()
  |         ^ borrowed value does not live long enough
6 |     } else {
7 |         b.iter().copied()
  |         ----------------- opaque type requires that `a` is borrowed for `'static`
8 |     }
9 | }
  | - `a` dropped here while still borrowed
```

（`b` 会再报一条同类的 E0597，略。）

定位路径：这个函数**没有生命周期参数**，2024 的「捕获作用域内全部生命周期」也就无从捕起——返回的不透明类型要求 `'static`，而 `a`、`b` 活不到那么久。修法：把数据从参数借进来，让生命周期成为签名的一部分：

```rust
fn src<'a>(flag: bool, a: &'a [u32], b: &'a [u32]) -> impl Iterator<Item = u32> + use<'a> {
    let chosen: &'a [u32] = if flag { a } else { b };
    chosen.iter().copied()
}

fn main() {
    let x = [1u32, 2];
    let y = [3u32, 4];
    let v: Vec<u32> = src(true, &x, &y).collect();
    println!("{v:?}");
}
```

实测输出 `[1, 2]`。另一条路是让函数返回拥有的数据（`collect` 成 `Vec` 再返回）。

这条错误也是「惰性求值」的边界：迭代器可以惰性，但它借用的数据必须活得比它久。要么让调用者把数据借进来（生命周期进签名，零拷贝），要么把数据收集成拥有所有权的集合（一次分配）。

### 4. AFIT 的 future 不满足 `Send`（E0277）

复现与关键行见「AFIT 的 `Send` 坑」一节。那几句 help 就是修法：把 `async fn run(&self) -> u32;` 换成 `fn run(&self) -> impl Future<Output = u32> + Send;`。

定位路径：`async fn in trait` 的返回值没有地方声明 auto trait。只要下游要求 `Send`（跨线程执行器、`tokio::spawn` 一类），就必须在 trait 签名里显式写出来。

### 5. `impl Trait` 用错位置（E0562）

```rust,compile_fail
fn main() {
    let x: impl std::fmt::Debug = 1;
    println!("{x:?}");
}
```

关键行：

```text
error[E0562]: `impl Trait` is not allowed in the type of variable bindings
 --> src/main.rs:2:12
  |
2 |     let x: impl std::fmt::Debug = 1;
  |            ^^^^^^^^^^^^^^^^^^^^
  |
  = note: `impl Trait` is only allowed in arguments and return types of functions and methods
  = note: see issue #63065 <https://github.com/rust-lang/rust/issues/63065> for more information
```

字段类型（`struct S { it: impl Iterator<Item = u32> }`）报同样的 E0562，`= note:` 行不变。需要「把迭代器存进结构体」时，用 `Box<dyn Iterator<Item = u32>>`、具名类型，或者泛型参数 `struct S<I: Iterator<Item = u32>> { it: I }`。

### 6. 不同泛型实参是不同类型（E0308）

```rust,compile_fail
fn generic<T: 'static>() -> impl PartialEq + std::fmt::Debug {
    1u32
}

fn main() {
    let c = generic::<u8>();
    let d = generic::<i8>();
    println!("{:?}", c == d);
}
```

关键行：

```text
error[E0308]: mismatched types
 --> src/main.rs:8:27
  |
1 | fn generic<T: 'static>() -> impl PartialEq + std::fmt::Debug {
  |                             --------------------------------
  |                             |
  |                             the expected opaque type
  |                             the found opaque type
...
8 |     println!("{:?}", c == d);
  |                           ^ expected `u8`, found `i8`
  |
  = note: expected opaque type `impl PartialEq + Debug` (`u8`)
             found opaque type `impl PartialEq + Debug` (`i8`)
  = note: distinct uses of `impl Trait` result in different opaque types
```

定位路径：不透明类型是「每个泛型实例化各一份」。`generic::<u8>()` 与 `generic::<i8>()` 的返回类型互不相同，哪怕函数体返回的都是 `u32`。想比较两者，就得先擦除类型（`Box<dyn Trait>`）或把它们转成同一具体类型再比。

### 附：调用点指定 APIT 的泛型实参（E0107）

```rust,compile_fail
fn show(arg: impl std::fmt::Debug) {
    println!("{arg:?}");
}

fn main() {
    show::<u32>(1);
}
```

```text
error[E0107]: function takes 0 generic arguments but 1 generic argument was supplied
 --> src/main.rs:6:5
  |
6 |     show::<u32>(1);
  |     ^^^^------- help: remove the unnecessary generics
  |     |
  |     expected 0 generic arguments
  |
note: function defined here, with 0 generic parameters
 --> src/main.rs:1:4
  |
1 | fn show(arg: impl std::fmt::Debug) {
  |    ^^^^
  = note: `impl Trait` cannot be explicitly specified as a generic argument
```

## 小结

- `impl Trait` 在参数位置是匿名泛型参数（调用者选类型，不能 turbofish）；在返回位置是不透明类型（函数选类型，调用者只见约束）。两者「几乎等价」，差别在调用点能否显式给出类型。
- 一个函数（一个泛型实例化）只有一个隐藏类型；要返回多种类型就得换 `Box<dyn Trait>`（运行期、有分配）或 enum（编译期、无分配）。
- 不透明类型必须**捕获**隐藏类型用到的泛型参数：类型与 const 参数各 edition 都自动捕获；生命周期参数 2024 起无条件捕获作用域内全部，2021 只捕获写进约束的那些。
- `use<..>`（1.82+，所有 edition）精确指定捕获集合；不能与 APIT 同用，trait 里必须带 `Self`。
- trait 里的 `-> impl Trait`（RPITIT）与 `async fn`（AFIT）自 1.75 起稳定，但这样的 trait **不能做 trait 对象**；AFIT 的 future **不保证 `Send`**，要跨线程就显式改写签名。
- auto trait 会从隐藏类型透传出来——这既是保证，也是你更换实现时要留意的接口变化。
- 「不透明」是编译期概念：隐藏类型照样被单态化、被内联，也能被 `type_name` 打印出来。它挡住的是名字与身份，不是行为，更不是安全边界。

## 语言对照角

Swift 有一个几乎同构的机制：返回类型写 `some Shape`。Swift 官方书对它的解释是：「You can think of an opaque type like being the reverse of a generic type.」——泛型让调用者选类型，不透明类型让实现者选类型，与 Rust 的 RPIT 是同一句话。Swift 书还强调「Opaque types preserve type identity」（大意：隐藏类型在模块内部仍然唯一确定、静态分派照旧），并把 `any Shape` 单列为另一套机制：那个才对应 Rust 的 `Box<dyn Trait>`，运行期才知道具体类型、需要装箱。写法上的差别是 Swift 用关键字 `some`、Rust 用 `impl`，而 Swift 的 `any` 与 Rust 的 `dyn` 都表示「运行期才知道类型」。

## 练习

**1.** 下面两个签名，对调用者来说差别在哪？什么时候必须选后者？

```rust
fn a(x: impl std::fmt::Debug) {}
fn b<T: std::fmt::Debug>(x: T) {}
```

**2.** 判断下面的程序能否编译，并说明理由：

```rust,compile_fail
fn make(flag: bool) -> impl Iterator<Item = u32> {
    if flag {
        std::iter::once(1u32)
    } else {
        (0..3).map(|x| x * 2)
    }
}

fn main() {
    println!("{}", make(true).count());
}
```

**3.** 这个 trait 有什么问题？给出两种修法。

```rust,compile_fail
trait Loader {
    async fn load(&self) -> Vec<u8>;
}

async fn run_all<L: Loader>(l: L) {
    let f = l.load();
    requires_send(f);
}

fn requires_send<F: Send>(_: F) {}
```

<details><summary>参考答案</summary>

**1.** 语义上「几乎等价」（Reference 说 APIT 是「syntactic sugar」），但有两点实际差别。其一，`b` 的类型参数有名字，调用点可以写 `b::<u32>(1)`；`a` 不行——实测报 `` error[E0107] ... = note: `impl Trait` cannot be explicitly specified as a generic argument ``。其二，两种写法之间的切换会改变泛型实参个数，对调用方是可见变化。所以：需要显式指定类型、或想让读者看出「这里有个类型参数」时选 `b`；否则 `a` 更短，也省掉了给参数起名。

**2.** 编译失败，报 ``error[E0308]: `if` and `else` have incompatible types``（以 Rust 1.98 为例）。两支的具体类型不同，实测诊断写的是 `std::iter::Once<u32>` 与 `Map<std::ops::Range<{integer}>, {closure@src/main.rs:5:20: 5:23}>`（`0..3` 的元素类型此时还没定下来，所以显示 `{integer}`），而 `impl Iterator` 只绑定一个隐藏类型。修法：两边统一成同一具体类型（例如都用 `std::iter::once`，或都用同一个 `map` 闭包），或改成 `Box<dyn Iterator<Item = u32>>`，或自定义 enum 并手写 `Iterator`。

**3.** 问题在 `requires_send(f)`：`async fn load` 只承诺返回某个 `Future<Output = Vec<u8>>`，没承诺 `Send`，所以报 ``error[E0277]: `impl Future<Output = Vec<u8>>` cannot be sent between threads safely``。

修法一，把承诺写进 trait 签名：

```rust
trait Loader {
    fn load(&self) -> impl Future<Output = Vec<u8>> + Send;
}
```

修法二，把 future 装箱进 `Pin<Box<…>>`，`Send` 写在 trait 对象类型上：

```rust
use std::pin::Pin;

trait Loader {
    fn load(&self) -> Pin<Box<dyn Future<Output = Vec<u8>> + Send + '_>>;
}
```

`Pin` 不能省：`dyn Future` 是 `!Unpin`，裸的 `Box<dyn Future<…>>` 连 `.await` 都用不了（实测：``error[E0277]: `dyn Future<Output = Vec<u8>> + Send` cannot be unpinned``，并附带 ``= note: required for `Box<dyn Future<Output = Vec<u8>> + Send>` to implement `Future``）；`Pin<Box<dyn Future<…>>>` 自己就实现了 `Future`，`.await` 正常工作（第 21、22 章把这条链路讲全）。

取舍：修法一零分配，但要求**所有**实现都给出 `Send` 的 future；修法二每次调用一次堆分配，换来「不在 trait 层面强加 `Send`」。两种写法都实测编译通过，修法二的 `.await` 路径也实测跑通（`Pin<Box<dyn Future>>` + 一个用 `Waker::noop()` 的极简 `block_on`，取值得到 `[1, 2, 3]`）。

</details>

## 延伸阅读

- `impl Trait`（语法、捕获、限制）：<https://doc.rust-lang.org/reference/types/impl-trait.html>
- 2024 的 RPIT 生命周期捕获规则与迁移（Edition Guide）：<https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html>
- dyn 兼容性（哪些 trait 能做 trait 对象）：<https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility>
- `Future` trait：<https://doc.rust-lang.org/std/future/trait.Future.html>
- The Book 第 10 章「泛型、trait 与生命周期」：<https://doc.rust-lang.org/book/ch10-00-generics.html>
