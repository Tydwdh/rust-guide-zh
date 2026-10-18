# 第 9 章 泛型与 trait 入门

## 这一章解决什么问题

先看一段你多半已经写腻的代码：给 `i32` 求最大值写一个函数，给 `f64` 再写一个，给 `&str` 还要再写一个。三份代码逐字相同，只有类型不同。复制粘贴能跑，但每多支持一种类型就多一份要维护的副本——改一处 bug 得改三处，改漏一处就等着排查。

泛型（generics）把类型变成参数：一段代码，服务多种类型。

「同一段算法适配多种类型」只是这件事的一半。算法往往对类型有要求：求最大值要求类型能比较大小，打印要求类型能格式化。Rust 把这种要求写成显式的契约，就是 trait（行为契约）。用一句话分清楚：泛型回答「这段代码对哪些类型成立」，trait 回答「某类型能做什么」。

读完这一章，你能：

- 写泛型函数、泛型结构体，并在边界里写清要求；
- 定义 trait、为类型实现 trait、使用别人实现好的 trait；
- 说清泛型的收益与代价：单态化（monomorphization）发生在哪一步；
- 认识标准库里最常用的几个 trait；
- 认出三类最常见的相关报错，并知道怎么修。

> [!NOTE]
> trait 是「行为契约」，本书保留英文不译：「特征」「特质」在中文里歧义太大，社区也没有统一用法。

## 为什么这样设计

### 泛型在编译期就被替换掉

Java 的接口调用、C++ 的虚函数这类运行时多态，走的是函数指针表加一次间接调用。Rust 的泛型走另一条路——**单态化（monomorphization）**：编译器在你使用泛型的地方，把类型参数替换成具体类型，为每组具体类型生成一份专门的代码。程序跑起来时，机器码里根本没有「泛型函数」这种东西。

口说无凭，看编译产物。写一个最小的泛型函数，用三种类型各调一次：

```rust
use std::fmt::Display;

fn show<T: Display>(v: T) {
    println!("{v}");
}

fn main() {
    show(1u32);
    show(2.5f64);
    show("hi");
}
```

用 `--emit=llvm-ir` 让它输出 LLVM IR，再数一数 `show` 的定义：

```console
$ rustc --edition 2024 -C opt-level=0 --emit=llvm-ir show.rs
$ grep '^define' show.ll | grep '4show4show'
define internal void @_RINvCsrVwfFJZ6TP_4show4showReEB2_(ptr %0, i64 %1) …
define internal void @_RINvCsrVwfFJZ6TP_4show4showdEB2_(double %0) …
define internal void @_RINvCsrVwfFJZ6TP_4show4showmEB2_(i32 %0) …
```

同一份源码，三个 `show`（以 Rust 1.98 实测）。`Re`、`d`、`m` 是编译器对类型实参的编码，分别对应三个调用点的 `&str`、`f64`、`u32`；`@` 后面那一长串是编译器给这次实例化起的名字（含环境哈希），不必读懂，看参数类型就行：`&str` 是「指针 + 长度」两件套，`f64` 是 `double`，`u32` 显示成 `i32`（LLVM 的整数类型不区分有无符号）。

下图把这次单态化摊开（实例名已缩写）：

<svg viewBox="0 0 640 218" width="100%" role="img" aria-label="单态化：一份泛型函数 show 按类型实参分别生成 &amp;str、f64、u32 三份实例代码" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">单态化：一份泛型源码 → 按类型实参各生成一份机器码</text>
<rect x="8" y="40" width="184" height="152" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="100" y="64" font-size="13" text-anchor="middle" fill="currentColor">一份源码（泛型函数）</text>
<g font-size="12" font-family="monospace" text-anchor="middle" fill="currentColor" xml:space="preserve" style="white-space:pre">
<text x="100" y="90">fn show&lt;T: Display&gt;</text>
<text x="100" y="108">  (v: T) {</text>
<text x="100" y="126">  println!("{v}");</text>
<text x="100" y="144">}</text>
</g>
<text x="100" y="174" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.7">类型参数在编译期被填实</text>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<line x1="194" y1="62" x2="238" y2="62" />
<line x1="194" y1="116" x2="238" y2="116" />
<line x1="194" y1="170" x2="238" y2="170" />
</g>
<polygon points="246,62 236,57 236,67" fill="currentColor" />
<polygon points="246,116 236,111 236,121" fill="currentColor" />
<polygon points="246,170 236,165 236,175" fill="currentColor" />
<rect x="248" y="40" width="384" height="44" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<rect x="248" y="94" width="384" height="44" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<rect x="248" y="148" width="384" height="44" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="260" y="59" font-size="12" font-family="monospace" fill="currentColor">show("hi")</text>
<text x="368" y="59" font-size="12" font-family="monospace" fill="currentColor">→ @…4show4showReEB2_</text>
<text x="260" y="77" font-size="12" fill="currentColor" opacity="0.75">签名 (ptr, i64)：&amp;str 是「指针 + 长度」两件套</text>
<text x="260" y="113" font-size="12" font-family="monospace" fill="currentColor">show(2.5f64)</text>
<text x="368" y="113" font-size="12" font-family="monospace" fill="currentColor">→ @…4show4showdEB2_</text>
<text x="260" y="131" font-size="12" fill="currentColor" opacity="0.75">签名 (double)：f64</text>
<text x="260" y="167" font-size="12" font-family="monospace" fill="currentColor">show(1u32)</text>
<text x="368" y="167" font-size="12" font-family="monospace" fill="currentColor">→ @…4show4showmEB2_</text>
<text x="260" y="185" font-size="12" fill="currentColor" opacity="0.75">签名 (i32)：u32（LLVM 的整数不区分有无符号）</text>
<text x="0" y="210" font-size="12" fill="currentColor" opacity="0.7">实例名含环境哈希，此处缩写；三份实例各自独立，调用点直接落到对应那一份</text>
</svg>

> [!TIP]
> 这里加 `-C opt-level=0`，是因为开了优化之后这三个函数很可能被内联进 `main`、从产物里整个消失——那本身也是「零成本」的一部分。想看实例数量，就用不优化的构建。

这就是**零成本抽象（zero-cost abstraction）**的含义：你写泛型，拿到的运行时代码与手写三份一样——没有类型标签、没有查表、没有额外跳转。代价不在运行时，而在别处：每一组具体类型都要生成一份代码，用到的类型越多，二进制越大，编译时间与指令缓存（I-cache）的压力也跟着涨。

对比另一条路：`Box<dyn Trait>` 把「具体类型」藏到运行时，只有一份代码，但每次调用都要经虚函数表（vtable）间接跳转、顺着指针追一次数据。泛型是「多份代码、零间接」，trait 对象是「一份代码、一次间接」。这是个真取舍，第 13 章展开。

> [!WARNING]
> 别把单态化当成运行时多态：机器码里没有「这次调用用的是哪个类型」这种隐含信息，也不能在运行时按类型分支。类型参数在编译期就被填实了。（`std::any::TypeId` 是显式请求类型标识的另一条路，需要 `T: 'static`。）

### trait 是行为契约

泛型函数体里能对 `T` 做的事，严格受边界限制。`T` 是「某个类型」，编译器不假设它会比较、会克隆、会打印——除非你写下来。

trait 就是这种要求的具名表达：名字下面列出方法签名，实现者按签名提供方法，使用者按签名调用方法。三类角色分工明确：

- **定义方**：写下契约（方法名、签名、可选默认实现）。
- **实现方**：`impl Trait for Type`，兑现契约。
- **使用方**：写边界 `T: Trait`，然后在函数体里放心调用 trait 的方法。

下图把这三类角色的分工摊开：

<svg viewBox="0 0 640 204" width="100%" role="img" aria-label="trait 的三类角色：定义方写契约、实现方兑现契约、使用方只依赖契约" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">trait 的三类角色：定义方写契约，实现方兑现，使用方只依赖契约</text>
<rect x="5" y="40" width="190" height="116" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<rect x="225" y="40" width="190" height="116" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<rect x="445" y="40" width="190" height="116" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" />
<text x="100" y="62" font-size="13" text-anchor="middle" fill="currentColor">定义方</text>
<text x="320" y="62" font-size="13" text-anchor="middle" fill="currentColor">实现方</text>
<text x="540" y="62" font-size="13" text-anchor="middle" fill="currentColor">使用方</text>
<g font-size="12" font-family="monospace" text-anchor="middle" fill="currentColor" xml:space="preserve" style="white-space:pre">
<text x="100" y="86">trait Summary {</text>
<text x="100" y="104">  fn author(&amp;self)</text>
<text x="100" y="122">      -&gt; String;</text>
<text x="100" y="140">}</text>
<text x="320" y="86">impl Summary</text>
<text x="320" y="104">    for Comment {</text>
<text x="320" y="122">  fn author(&amp;self)</text>
<text x="320" y="140">      -&gt; String }</text>
<text x="540" y="86">fn notify&lt;T: Summary&gt;</text>
<text x="540" y="104">    (x: &amp;T) {</text>
<text x="540" y="122">  x.summarize();</text>
<text x="540" y="140">}</text>
</g>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<line x1="197" y1="98" x2="214" y2="98" />
<line x1="417" y1="98" x2="434" y2="98" />
</g>
<polygon points="223,98 215,93.5 215,102.5" fill="currentColor" />
<polygon points="443,98 435,93.5 435,102.5" fill="currentColor" />
<text x="100" y="176" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.75">契约：方法名 + 签名 + 默认实现</text>
<text x="320" y="176" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.75">兑现：impl Trait for Type</text>
<text x="540" y="176" font-size="12" text-anchor="middle" fill="currentColor" opacity="0.75">调用：只依赖边界 T: Trait</text>
<text x="0" y="196" font-size="12" fill="currentColor" opacity="0.7">使用方看不到具体类型：它能调用的只有契约里列出的方法</text>
</svg>

与其他语言的接口相比，多出来的一条是：trait 可以为**你拥有的类型**、或**你定义的 trait** 而实现，包括为别人（比如标准库）的类型实现你自己的 trait，也包括为自己的类型实现标准库的 trait。这条自由度有边界，就是后面的孤儿规则。

### 语言对照角

前言承诺过，泛型与编译期检查会配 C++/Java/Python 的对照（第 9、10 章现场建立）。这里先把泛型与 trait 这两处跨语言差异最大的概念对照掉，每个语言只用几句。

**泛型：类型参数在哪里被处理？**

- **C++ 模板**：与 Rust 最接近——同样在编译期实例化；但模板参数没有随语言自带的契约，约束靠 C++20 concepts，否则错误会在模板深处炸开。
- **Java 泛型**：类型参数在运行时被擦除（erasure），`List<int>` 这种写法不存在、泛型数组不能创建；Rust 单态化则给每个具体类型分别生成一份代码。
- **Python**：不做静态参数化，鸭子类型——有没有这个方法，运行到那一行才知道。Rust 把这类检查全部前移到编译期。

**契约：能力写在哪里？**

- **Rust**：trait 是独立于类型声明的实体。你可以为自己的类型实现别人的 trait，也可以为别人的类型实现自己定义的 trait——只要 trait 或类型有一侧在本地（下一节讲这条边界）。
- **Java**：`interface` 提供契约，但实现关系写在类的声明处（`class X implements Y`）；想给已有类型补接口，只能包一层适配器。
- **C++**：抽象基类 + 虚函数是契约机制，要进入继承层次才满足它；C++20 concepts 能表达编译期约束，但不是「给已有类型补实现」的手段。
- **Python**：契约靠约定与文档，跑到调用处才发现不满足；`typing.Protocol` 与 `abc` 能补上部分静态检查。

## 机制与规则

### 泛型函数

```rust
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}
```

三处新东西：`<T: PartialOrd>` 声明一个类型参数 `T` 并要求它满足 `PartialOrd`（能用 `>` 比大小）；参数与返回值里出现 `T`；调用时**不需要**手写类型——编译器从实参推断。

类型参数的命名是约定，不是语法（The Book 第 10 章同款说明）：短、常是单个字母、用 UpperCamelCase。习惯上第一个用 `T`，第二个 `U`；键值对用 `K`/`V`，错误类型用 `E`。名字可以自取，`<T: PartialOrd>` 与 `<Item: PartialOrd>` 等价，但不要发明与这组惯例冲突的名字。

> [!NOTE]
> `&list[0]` 在空切片上会 panic（第 7 章）。教学里省略这层防御；真要写库，应返回 `Option<&T>`。

### 泛型结构体与方法

```rust,ignore
struct Pair<T> {
    first: T,
    second: T,
}

impl<T> Pair<T> {
    fn new(first: T, second: T) -> Self { /* … */ }
}

impl<T: PartialOrd + std::fmt::Display> Pair<T> {
    fn larger(&self) -> &T { /* … */ }
}
```

（两个方法的完整实现见「动手看看」的 `snippets/ch09/pair.rs`。）

`impl<T> Pair<T>` 里的 `<T>` 不是重复——它声明「这段实现对任意 `T` 有效」。同一个类型可以有多段 `impl`，各自要求不同的边界：第一段只要能把两个值装进去，第二段才需要 `T` 能比较、能打印。方法只在满足对应 `impl` 边界时存在，这是第 5 章「方法定义在 `impl` 块里」的自然延伸，也是后面「按能力分层组织 API」的雏形。

### 定义 trait 与实现 trait

```rust,ignore
trait Summary {
    fn author(&self) -> String;               // 只有签名：实现者必须提供

    fn summarize(&self) -> String {           // 有默认实现：可以不写
        format!("（{} 的内容摘要待补充）", self.author())
    }
}

impl Summary for Comment {
    fn author(&self) -> String { self.user.clone() }
    // summarize 省略 → 用默认实现
}
```

`impl Summary for Comment` 是契约的兑现处：`author` 必须给，`summarize` 省略就等于采用默认实现；想改行为就把它写出来覆盖掉。调用方——无论是具体的 `Comment`，还是边界为 `T: Summary` 的泛型函数——看到的只是契约里的方法。

trait 里还可以有关联类型、常量、关联函数等成员，本章只用方法。关联类型与 trait 设计的取舍在第 12 章，const 泛型与 GAT 在第 15 章。

### 边界怎么写

同一条要求有三种写法。多个边界用 `+` 串起来：

```rust
fn show_twice<T: std::fmt::Display + Clone>(v: T) {
    println!("第一次：{v}");
    println!("第二次：{}", v.clone());
}
```

边界多、参数长时用 `where`，读起来像一份要求清单：

```rust
fn describe<T>(values: &[T]) -> String
where
    T: std::fmt::Display + PartialOrd,
{
    let mut max = &values[0];
    for v in values {
        if v > max {
            max = v;
        }
    }
    format!("共 {} 个，最大的是 {}", values.len(), max)
}
```

参数位置用 `impl Trait`，写法最短：

```rust
fn print_it(x: &impl std::fmt::Display) {
    println!("{x}");
}
```

`<T: Trait>` 与 `where T: Trait` 完全等价，`where` 只是为了读起来更像一份要求清单；团队里常见约定是「一个边界且短就内联，两个及以上或带泛型参数就 `where`」。

边界是函数体内能用什么的全部依据。去掉 `PartialOrd`，函数体里的 `>` 立刻编译不过：

```rust,compile_fail
fn largest<T>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn main() {
    println!("{}", largest(&vec![1, 2, 3]));
}
```

```text
error[E0369]: binary operation `>` cannot be applied to type `&T`
 --> e0369.rs:4:17
  |
4 |         if item > largest {
  |            ---- ^ -------
...
help: consider restricting type parameter `T` with trait `PartialOrd`
  |
1 | fn largest<T: std::cmp::PartialOrd>(list: &[T]) -> &T {
  |             ++++++++++++++++++++++
```

（以 Rust 1.98 为例，下同；行号随代码布局变化。）编译器不猜你的意图，但它会给出修法：加上 `PartialOrd`。`E0277` 与 `E0369` 这对组合是本章最值得记住的两条报错。

### `impl Trait`：参数位置与返回位置

参数位置的 `impl Trait` 是语法糖：`fn print_it(x: &impl Display)` 就是 `fn print_it<T: Display>(x: &T)`。甜在写法短，代价是这个类型在函数体里没有名字，调用方也不能显式指定它：

```rust,compile_fail
fn print_it(x: &impl std::fmt::Display) {
    println!("{x}");
}

fn main() {
    print_it::<i32>(&1);
}
```

```text
error[E0107]: function takes 0 generic arguments but 1 generic argument was supplied
 --> e0107.rs:6:5
  |
6 |     print_it::<i32>(&1);
  |     ^^^^^^^^------- help: remove the unnecessary generics
...
  = note: `impl Trait` cannot be explicitly specified as a generic argument
```

返回位置的 `impl Trait` 语义不同：它声明「返回值的类型实现了这个 trait，但具体是什么类型是隐藏的」。

```rust
fn version_label() -> impl std::fmt::Display {
    String::from("1.2")
}
```

调用方拿到一个只能当 `Display` 用的值，写 `let s: String = version_label();` 会被拒绝（E0308：expected `String`, found opaque type）。更实际的一条限制是：这样的函数只能返回**一种**具体类型。想按条件返回两种类型，会得到另一个 E0308，编译器还会顺便建议改用 `Box<dyn Display>`——那正是第 13 章的主题。返回位置的 `impl Trait`（RPIT）还有生命周期捕获等细节，第 14 章展开。

> [!WARNING]
> `impl Trait` 不是「写出泛型」的替代品：函数体内不能给它命名，无法直接调用它没有列出的方法。想要命名，就用显式类型参数 `<T: Trait>`。

### 标准库里的常用 trait（使用者视角）

不必背定义，先记住「什么时候会要求它」。下面这些你几乎每周都会碰到：

| trait | 提供什么 | 能 derive 吗 | 典型出现位置 |
| --- | --- | --- | --- |
| `Debug` | `{:?}`、`{:#?}` | 能 | 调试打印、测试断言、`dbg!` |
| `Display` | `{}` | **不能** | 面向用户的输出、`to_string()` |
| `Clone` | `.clone()` | 能 | 显式复制（深浅由实现决定） |
| `Copy` | 赋值/传参即复制（转移不失效） | 能（字段都 `Copy`、无 `Drop`，还要连 `Clone` 一起 derive） | 小整数型数据 |
| `PartialEq` | `==`、`!=` | 能 | 比较、`assert_eq!` |
| `PartialOrd` | `<`、`>`、排序 | 能（按字段声明顺序逐字段比较） | 排序、取最大 |
| `From` | `T::from(x)` | 不能 | 类型转换的入口 |
| `Into` | `x.into()`，由 `From` 自动获得 | 不能 | 接受「能转成 X」的参数 |

`derive` 你在第 5 章已经见过：它让编译器按字段逐个生成实现，生成的正是一段 `impl`——`#[derive(Debug)]` 展开出来就是 `impl Debug for Version { … }`。能做到的事都很机械：每个字段都能 `Debug`，整个结构体就能 `Debug`；`PartialEq`、`PartialOrd` 同理。`Copy`/`Clone` 第 4 章讲移动时见过：`Copy` 意味着赋值不再转移所有权，所以它要求所有字段都 `Copy`（这也是「大数组别随便 `derive Copy`」的原因）。另外 `Copy` 是 `Clone` 的子 trait（`pub trait Copy: Clone`）：只写 `#[derive(Copy)]` 会报 E0277，两个要一起 derive。

`Display` 必须手写——第 5 章预告过这一点，原因就在这里：「怎么显示」是人的决定，不是字段的机械组合。所以「实现 trait」不总是打一行 `derive` 就算完；自己写 `impl` 与 `derive` 生成的 `impl`，在语言里地位完全相同。

`From`/`Into` 的关系值得单独记：标准库有一条覆盖所有类型的实现——「只要 `T: From<U>`，就有 `U: Into<T>`」。所以**只实现 `From`，`Into` 自动到手**；反过来不成立，惯例是永远实现 `From`、在边界里用 `Into`。在边界里写 `T: Into<Fahrenheit>`，比直接写具体的源类型更宽松：任何「能转成华氏度」的东西都能传进来，包括将来才加入的类型。

### 孤儿规则

trait 实现有个硬性限制：**只有当 trait 或类型至少有一个定义在你当前 crate 里时，才能写这个 `impl`**。给外部类型实现外部 trait 会被拒绝（最小复现与真实报错见本章「报错现场」的第三例）。

这条规则叫**孤儿规则**。理由很实际：`Display` 和 `Vec` 分属标准库与你的 crate，而下游另一个 crate 可能也想给 `Vec<i32>` 实现 `Display`。两个实现同时存在，`println!("{}", v)` 该调用哪个？编译器无法裁决，于是干脆从源头禁止——只有一个「本地」位置时才允许。

绕过办法就是报错建议的 newtype：包一层自己的类型，就有了本地类型的位置（第 12 章展开相关设计）：

```rust,ignore
struct WordList(Vec<String>);

impl std::fmt::Display for WordList { /* … */ }
```

## 动手看看

以下六段是完整程序。在仓库里跑：

```console
$ rustc --edition 2024 snippets/ch09/largest.rs
$ ./largest         # Linux / macOS
$ .\largest.exe     # Windows
```

不写 `-o` 时，rustc 按源文件名命名产物、并自动补上平台后缀（Windows 上是 `.exe`）；一旦写了 `-o 名字`，名字就原样使用、不会再补后缀——在 Windows 上 `-o largest` 得到的是名为 `largest` 的文件，`.\largest.exe` 会报找不到。

**1. 一个函数服务三种类型**（`snippets/ch09/largest.rs`）：

```rust
{{#rustdoc_include ../snippets/ch09/largest.rs}}
```

```console
整数最大值: 100
字符最大值: y
字典序最大: zebra
```

同一个 `largest` 服务 `i32`、`char`、`&str`——只要它们满足 `PartialOrd`。三种类型对应三份实例，就是前面 LLVM IR 里看到的机制。

**2. 泛型结构体与两段 `impl`**（`snippets/ch09/pair.rs`）：

```rust
{{#rustdoc_include ../snippets/ch09/pair.rs}}
```

```console
3 与 7 中较大的是 7
apple 与 pear 中较大的是 pear
```

注意 `Pair::new` 对任何 `T` 都可用，而 `larger` 只对满足 `T: PartialOrd + Display` 的 `Pair<T>` 存在。

**3. 定义 trait、默认实现与覆盖**（`snippets/ch09/summary.rs`）：

```rust
{{#rustdoc_include ../snippets/ch09/summary.rs}}
```

```console
新内容：《RISC-V 的十年》，作者 林一
新内容：（qiao 的内容摘要待补充）
```

`Article` 覆盖了 `summarize`，`Comment` 用默认实现；`notify` 是调用方视角——它只知道「参数实现了 `Summary`」。

**4. `derive` 与手写 `Display`**（`snippets/ch09/std_traits.rs`）：

```rust
{{#rustdoc_include ../snippets/ch09/std_traits.rs}}
```

```console
1.2 < 1.10: true
1.2 和 1.2 都还在
Version { major: 1, minor: 2 } / Version {
    major: 1,
    minor: 2,
}
```

`1.2 < 1.10` 为真，因为 `derive(PartialOrd)` 按字段声明顺序逐字段比较：先比 `major`，相等再比 `minor`——这是字典序。

**5. `From` 一实现，`Into` 自动到手**（`snippets/ch09/from_into.rs`）：

```rust
{{#rustdoc_include ../snippets/ch09/from_into.rs}}
```

```console
25 °C = 77.0 °F
32.0 °F
212.0 °F
```

`report` 的边界是 `T: Into<Fahrenheit>`；`Celsius` 只实现了 `From`，`Into` 由标准库补齐。

**6. newtype 绕过孤儿规则**（`snippets/ch09/newtype.rs`）：

```rust
{{#rustdoc_include ../snippets/ch09/newtype.rs}}
```

```console
3 个词：alpha / beta / gamma
```

## 报错现场

三类错误，各自看「编译器在说什么、它在指哪个位置、往哪修」。

**一、边界不满足：E0277。** 类型没实现边界要求的 trait：

```rust,compile_fail
struct Point {
    x: i32,
    y: i32,
}

fn show<T: std::fmt::Display>(v: T) {
    println!("{v}");
}

fn main() {
    let p = Point { x: 1, y: 2 };
    show(p);
}
```

```text
error[E0277]: `Point` doesn't implement `std::fmt::Display`
  --> e0277.rs:12:10
   |
12 |     show(p);
   |     ---- ^ unsatisfied trait bound
   |     |
   |     required by a bound introduced by this call
   |
help: the trait `std::fmt::Display` is not implemented for `Point`
  --> e0277.rs:1:1
   |
 1 | struct Point {
   | ^^^^^^^^^^^^
note: required by a bound in `show`
  --> e0277.rs:6:12
   |
 6 | fn show<T: std::fmt::Display>(v: T) {
   |            ^^^^^^^^^^^^^^^^^ required by this bound in `show`
```

读法：第一行说**谁**不满足**什么**（`Point` 缺 `Display`）；`-->` 指向**调用点**（因为类型是这里确定下来的）；`help` 给出缺失的具体 trait；最后的 `note` 回指**边界声明的位置**。四行合起来就是一条完整链路：调用点 → 缺的能力 → 边界。修法二选一：给 `Point` 实现 `Display`（手写 `impl`），或者换一个实现了它的类型——`Debug` 有 `derive` 可用，`Display` 没有。

**二、trait 方法不在作用域：E0599。** 方法存在、类型也实现了，但你没把 trait 引进当前作用域：

```rust,compile_fail
mod media {
    pub trait Summary {
        fn summarize(&self) -> String;
    }

    pub struct Article {
        pub title: String,
    }

    impl Summary for Article {
        fn summarize(&self) -> String {
            format!("《{}》……", self.title)
        }
    }
}

fn main() {
    let a = media::Article {
        title: String::from("RISC-V 的十年"),
    };
    println!("{}", a.summarize());
}
```

```text
error[E0599]: no method named `summarize` found for struct `Article` in the current scope
  --> e0599.rs:21:22
   |
 3 |         fn summarize(&self) -> String;
   |            --------- the method is available for `Article` here
...
 6 |     pub struct Article {
   |     ------------------ method `summarize` not found for this struct
...
21 |     println!("{}", a.summarize());
   |                      ^^^^^^^^^ method not found in `Article`
   |
   = help: items from traits can only be used if the trait is in scope
help: trait `Summary` which provides `summarize` is implemented but not in scope; perhaps you want to import it
   |
 1 + use crate::media::Summary;
```

这正是「方法从哪来」的补课：方法语法 `a.summarize()` 只在编译器能「看见」提供它的 trait 时有效。第 2 章讲的诊断思路在这里完整复用——先读第一行的「no method named」，再看 `help` 里给出的 `use`。加上那行导入即可通过。

**三、孤儿规则：E0117。** 给外部类型实现外部 trait：

```rust,compile_fail
use std::fmt;

impl fmt::Display for Vec<i32> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn main() {}
```

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
 --> e0117.rs:3:1
  |
3 | impl fmt::Display for Vec<i32> {
  | ^^^^^^^^^^^^^^^^^^^^^^--------
  |                       |
  |                       `Vec` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
  = note: for more information see https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
  = note: define and implement a trait or new type instead
```

报错已经把两条出路写在 `note` 里：**定义并实现自己的 trait**，或**用 new type** 把 `Vec<i32>` 包起来。这不是「编译器小气」，而是保证「同一个实现只可能有一个」的机制——细节与设计影响在第 12 章。

## 小结

1. 泛型是编译期参数化：单态化把每个具体类型替换进函数体，各生成一份代码。运行时没有泛型，也没有额外开销——代价在产物体积。
2. `Box<dyn Trait>` 是另一条路：一份代码、运行时间接跳转。泛型 vs trait 对象是取舍，第 13 章展开。
3. trait 是行为契约，有定义、实现、使用三种视角。默认实现可以少写重复代码，方法是否可用取决于「提供它的 trait 是否在当前作用域」。
4. 边界写法：`<T: Trait>`、多个用 `+`、复杂时用 `where`；`impl Trait` 在参数位置是匿名泛型参数的糖，在返回位置表示「类型隐藏但实现了该 trait」。
5. 泛型函数体内只能用边界承诺过的能力——所以边界写错了，报错会出现在函数体里（E0369），调用方类型不满足要求则出现在调用点（E0277）。
6. 孤儿规则只允许「本地 trait」或「本地类型」参与 `impl`；需要给外部类型加外部 trait 时，用 newtype 包装（第 12 章）。

## 练习

**1. 读报错。** 下面这段代码为什么编译不过？除了加上 `PartialOrd`，还有别的修法吗？

```rust,compile_fail
fn max_of<T>(values: &[T]) -> &T {
    let mut max = &values[0];
    for v in values {
        if v > max {
            max = v;
        }
    }
    max
}
```

**2. 找错。** 下面这段代码有一处编译不过，指出位置并给出两种修法。

```rust,compile_fail
use std::fmt;

struct Meters(f64);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} m", self.0)
    }
}

impl fmt::Display for Vec<Meters> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} 段距离", self.len())
    }
}

fn main() {}
```

**3. 预测编译结果。** 下面三个调用，哪些合法，为什么？

```rust,ignore
fn a(v: impl std::fmt::Display) {}
fn b<T: std::fmt::Display>(v: T) {}

a(1u8);
a::<u8>(1);
b::<u8>(1);
```

<details><summary>参考答案</summary>

**1.** 报错是 E0369：`binary operation `>` cannot be applied to type `&T``——这里声明了类型参数 `T`，函数体里却在没有边界的情况下使用 `>`。编译器的 `help` 直接给了解法：给 `T` 加上 `PartialOrd`。其他修法：(a) 去掉类型参数，只写服务某一种具体类型的版本（那就不是泛型函数了）；(b) 如果本意是「任意可比较类型」，还要想清楚需要多强的比较：`PartialOrd` 只保证偏序，`f64` 满足它但有 `NaN`，需要全序得用 `Ord`，而 `f64` 没有实现 `Ord`。这道题的重点是：**报错出现在函数体里，往往意味着边界写少了**；报错出现在调用点（E0277），才是类型不够格。

**2.** 第二段 `impl fmt::Display for Vec<Meters>` 违反孤儿规则（E0117）：`Vec` 是外部类型，`Display` 是外部 trait。修法一：newtype——`struct Distances(Vec<Meters>);` 然后 `impl fmt::Display for Distances`。修法二：把它改成自己的 trait，例如 `trait Render { fn render(&self) -> String; }`，再为 `Vec<Meters>` 实现 `Render`（本地 trait 可以给外部类型实现）。注意第二段里的 `self.len()` 只有第一种修法能直接用：`Distances` 需要 `Deref` 或显式访问 `.0`（第 17 章）。

**3.** `a(1u8)` 合法：参数位置的 `impl Trait` 就是匿名的类型参数，`u8` 实现 `Display`，类型由实参推断。`a::<u8>(1)` **不合法**（E0107）：`impl Trait` 不能被显式指定为泛型实参——它写着方便，代价是无法命名。`b::<u8>(1)` 合法：`b` 的类型参数有名字 `T`，显式指定它是普通操作。记这两行就够：参数位置的 `impl Trait` = 匿名且不可命名；返回位置的 `impl Trait` = 类型隐藏、且只能对应一个具体类型。

</details>

## 延伸阅读

- The Book 第 10 章「泛型、trait 与生命周期」：<https://doc.rust-lang.org/book/ch10-00-generics.html>
- The Rust Reference，泛型参数与 trait：<https://doc.rust-lang.org/reference/items/generics.html>、<https://doc.rust-lang.org/reference/items/traits.html>
- 孤儿规则（Reference，实现一节）：<https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules>
- `std::fmt` 的 `Debug` 与 `Display`：<https://doc.rust-lang.org/std/fmt/index.html>
- `std::convert` 的 `From` 与 `Into`：<https://doc.rust-lang.org/std/convert/index.html>
- 错误索引（E0277、E0369、E0599、E0117 各有专页）：<https://doc.rust-lang.org/error_codes/index.html>
