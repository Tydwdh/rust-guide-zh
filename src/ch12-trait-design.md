# 第 12 章 trait 设计

## 这一章解决什么问题

第 9 章给了你 trait 的第一层认识：它是行为契约，`impl Trait for Type` 是签署契约，泛型边界是「我要求参数实现某个契约」。那层认识足够你使用标准库，但还不够你**设计**自己的 trait。

设计 trait 时才冒出来的问题是这一章的五个主题：

- trait 里那个类型占位符，该写成关联类型（associated type），还是写成 trait 自己的泛型参数（generic parameter）？`Iterator` 和 `Add` 各选了一种，为什么？
- 怎么表达「要实现 A 就必须先实现 B」？这就是父 trait（supertrait）。
- 我发布的 trait，怎么阻止下游 crate 实现它？这就是密封 trait（sealed trait）。
- 我想给 `Vec<T>` 加一个方法，但 `Vec` 不是我定义的——能做吗？这就是扩展 trait（extension trait）。
- 编译器为什么有时干脆不让我写某条 `impl`？这就是一致性（coherence）与孤儿规则（orphan rule）的细节。

读完这一章，你能对上面五个问题各给出一个判断，并且知道判断依据落在语言里的哪条规则上；见到 E0117、E0210、E0119 时，能立刻说出是规则里的哪一句被违反了。

## 为什么这样设计

把 trait 想成一张**由实现者填写的表格**。trait 的声明决定表格有哪些格子；`impl` 就是一份填好的表。

`Iterator` 的表里有一格叫 `Item`：对 `Counter` 这个类型（The Book 第 20 章的计数器例子），这一格只能填一次，因为一个迭代器只有一种元素类型。`Add` 的表里有**两**格：右操作数 `Rhs` 和结果 `Output`。如果 `Meters` 既想和 `Meters` 相加、又想和 `f64` 相加，那么「右操作数」这一格就必须允许填多种答案。

于是第一个设计判据是：**这个抽象对每个类型是否只可能有一种解释？**

- 只有一种 → 用关联类型。它把答案钉在类型上：`Counter` 是 `Iterator<Item = u32>`，没有第二种说法。
- 可能有多种 → 用泛型参数。它把答案变成 trait 名字的一部分：`Meters: Add<Meters>` 与 `Meters: Add<f64>` 是两条互不冲突的契约。

第二个设计问题是**裁决**。`impl` 可以分布在整个依赖图里的任何 crate。`x + y` 或 `v.len()` 出现时，编译器必须能唯一确定调用哪一份实现。一致性就是「每个 (trait, 类型) 组合只有一份实现」这条全局纪律，孤儿规则是它最外在的那道检查。

代价很直接：你没法像 Python 那样给任何类型打补丁（monkey patch），也没法随随便便为「所有类型」实现一个外部 trait。换来的东西是：行为在编译期就确定，改依赖不会偷偷改变语义。

## 机制与规则

### 关联类型还是泛型参数

先看标准库里这两极的原型。这是 1.98 文档中 `Iterator` 的定义（节选）：

```rust
pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

这是 `Add`：

```rust
pub trait Add<Rhs = Self> {
    type Output;

    fn add(self, rhs: Rhs) -> Self::Output;
}
```

注意这两处的分工：`Add` 把**右操作数**做成泛型参数（带默认值 `Rhs = Self`），把**结果**做成关联类型。标准库文档对 `Rhs` 的说明原文是：*"Note that `Rhs` is `Self` by default, but this is not mandatory. For example, `std::time::SystemTime` implements `Add<Duration>`, which permits operations of the form `SystemTime = SystemTime + Duration`."*——默认值让「同类相加」这种常见情况少写一个类型参数；而 `SystemTime + Duration` 说明「右操作数可以不是自己」为什么必须留出一个泛型参数的位置。

判据可以背下来，The Book 写得很直白：*"In other words, when a trait has a generic parameter, it can be implemented for a type multiple times, changing the concrete types of the generic type parameters each time."* 反过来就是：*"With associated types, we don’t need to annotate types, because we can’t implement a trait on a type multiple times."*

落到设计上，先问一句「这个类型会有几种解释」，再问「这个类型是输入侧还是输出侧」：

| 你想要的 | 该用 | 标准库例子 |
|---|---|---|
| 每个类型只有一种解释，且这个类型常要被调用方命名或约束 | 关联类型 | `Iterator::Item`、`Add::Output` |
| 同一类型可能有多种解释，由调用方挑选 | 泛型参数 | `Add::Rhs`、`From<T>` |
| 同一类型可能有多种解释，但常见情况只有一种 | 泛型参数 + 默认值 | `Add<Rhs = Self>`、`PartialEq<Rhs = Self>` |

把这两种写法落到「表」上，差别就是同一个类型能在同一张表里填几次：

<svg viewBox="0 0 660 232" width="100%" role="img" aria-label="关联类型与泛型参数对照：关联类型每个类型只有一格，再填就被判重叠报 E0119；泛型参数让同一类型并存多份实现，如 Add&lt;Meters&gt; 与 Add&lt;f64&gt;" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">同一张表能填几次：关联类型钉死一格，泛型参数可开多份</text>
<rect x="0" y="32" width="318" height="190" rx="6" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/>
<text x="14" y="58" font-size="13" fill="var(--links)">关联类型：type Item</text>
<text x="14" y="86" font-size="12" font-family="monospace" fill="currentColor">impl Iterator for Counter</text>
<rect x="14" y="98" width="290" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="159" y="118" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">type Item = u32; ← 只有一格</text>
<rect x="14" y="138" width="290" height="30" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="5 3"/>
<text x="159" y="158" font-size="12" fill="var(--warning-border)" text-anchor="middle">再填一格 → 重叠，E0119</text>
<text x="14" y="196" font-size="12" fill="currentColor" opacity="0.7">一个类型只有一种解释，答案钉在类型上</text>
<rect x="342" y="32" width="318" height="190" rx="6" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/>
<text x="356" y="58" font-size="13" fill="var(--links)">泛型参数：Add&lt;Rhs&gt;</text>
<rect x="356" y="78" width="290" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="501" y="98" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">impl Add&lt;Meters&gt; for Meters</text>
<rect x="356" y="116" width="290" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="501" y="136" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">impl Add&lt;f64&gt; for Meters</text>
<text x="356" y="170" font-size="12" fill="currentColor">同一类型的两份 impl 并存，互不冲突</text>
<text x="356" y="196" font-size="12" fill="currentColor" opacity="0.7">可能有多种解释，选哪份由调用方决定</text>
</svg>

还有三条实践差异值得记住（下面用到 `Report`/`Detail`/`ReportAs`，它们出自本章「动手看看」第 1 个程序）：

- **关联类型能在边界里被钉住。** 你可以写 `fn f<T: Report<Detail = String>>(x: &T)`，把「报告类型必须是 `String`」写进约束。泛型参数版本只能写 `T: ReportAs<String>`，看起来差不多，但它是「挑一种解释」，不是「断言唯一答案」。
- **关联类型可以自带边界。** 声明写成 `type Detail: Display;` 之后，任何拿到 `T: Report` 的函数体都能直接 `format!("{}", x.report())`，不必在每处 `where` 里重复 `T::Detail: Display`。这是「把契约写在一处」的另一个例子。
- **泛型参数的代价落在使用处。** `Meters: Add<Meters>` 与 `Meters: Add<f64>` 并存之后，`m + 5.0` 依赖字面量被推断成 `f64`；推断不出来时，你得用类型标注或 turbofish 把答案写出来。关联类型没有这个问题——答案是类型的一部分，不需要每次挑。

最后，「一次只能有一种解释」不是建议，是语言强制的：给同一个类型写两份关联类型不同的 `impl`，会被判为重叠，报 E0119——报错现场第二例是同一类冲突的另一种形态（blanket impl 与专门实现撞车）。

### 父 trait（supertrait）：前提条件，不是继承

`trait Circle: Shape` 的意思是「要实现 `Circle`，你的类型必须先实现 `Shape`」。Reference 的原文是：*"Supertraits are traits that are required to be implemented for a type to implement a specific trait. Furthermore, anywhere a generic or trait object is bounded by a trait, it has access to the associated items of its supertraits."*——后一句才是父 trait 真正给你的东西：**只要边界里有子 trait，你就能用父 trait 的关联项（associated item）**。

「不是继承」在三个地方看得见：

1. `impl Circle for UnitCircle` 不会顺带给你 `impl Shape`。父 trait 的实现要单独写，漏了就 E0277。
2. 父 trait 不会因为出现了子 trait 而多出方法。`Shape` 始终是 `Shape`。
3. 子 trait 的默认方法里可以直接调用 `self.area()`（父 trait 的方法）——这叫「父 trait 的关联项可用」，不叫「方法被继承」。

写法上，`trait Circle: Shape` 与 `trait Circle where Self: Shape` 等价，后者在父 trait 需要更复杂的表达式时更好读。trait 也不能是自己的父 trait（Reference：*"It is an error for a trait to be its own supertrait."*）。

> [!NOTE]
> Reference 在 dyn 兼容性一节里写明，旧名字「对象安全（object safety）」已经改叫「dyn 兼容性（dyn compatibility）」。你在旧文章里看到的 object safety 就是它。第 13 章展开。

trait 对象与父 trait 的关系这里只留一句：以 `dyn Circle` 为边界时，父 trait 的方法一样可用；把 `&dyn Circle` 转成 `&dyn Shape` 的向上转型（upcasting）从 1.86 起稳定。细节在第 13 章。

### 密封 trait（sealed trait）：让外部不能实现你的 trait

问题出在发布之后。你写了 `pub trait Greet`，下游任何 crate 都能 `impl Greet for MyType`。某天你发现该加一个没有默认实现的方法——这会破坏所有下游实现。SemVer 上这是破坏性变更，可你只是「加了个方法」。

修法是**密封（seal）**：让 trait 的可命名性与可实现性分开。trait 本身公开，下游能命名、能当边界；但实现者集合被封死在本 crate 内。

```rust
mod private {
    pub trait Sealed {}
}

pub trait Greet: private::Sealed {
    fn greet(&self) -> String;
}
```

机制只有一句话：`Greet` 要求实现者同时实现 `private::Sealed`，而 `private` 模块在 crate 外部不可命名——那个 `impl Sealed` 根本写不出来。

同一句话换成模块可见性的分层图，那道墙的位置就清楚了：

<svg viewBox="0 0 680 252" width="100%" role="img" aria-label="密封 trait 的分层：外部 crate 能命名 pub trait Greet、能把它当边界，却不能命名 private 模块里的 trait Sealed，所以写不出 impl Greet，报 E0277；虚线那道墙隔开可实现区与不可实现区" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">密封 trait：名字照样公开，实现者集合锁在 crate 内</text>
<rect x="0" y="42" width="176" height="196" rx="6" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/>
<text x="88" y="64" font-size="13" fill="currentColor" text-anchor="middle">外部 crate（下游）</text>
<text x="10" y="92" font-size="12" fill="var(--links)">✓ 命名 Greet、当边界</text>
<text x="10" y="114" font-size="11" font-family="monospace" fill="currentColor">fn f&lt;G: Greet&gt;(&amp;g)</text>
<text x="10" y="172" font-size="12" fill="var(--warning-border)">✗ impl Greet for</text>
<text x="10" y="192" font-size="12" fill="var(--warning-border)">MyType → E0277</text>
<line x1="182" y1="36" x2="182" y2="244" stroke="var(--warning-border)" stroke-width="2" stroke-dasharray="7 5"/>
<text x="182" y="30" font-size="12" fill="var(--warning-border)" text-anchor="middle">墙：外部不可实现</text>
<rect x="190" y="42" width="470" height="196" rx="6" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/>
<text x="202" y="64" font-size="13" fill="currentColor">crate greet</text>
<rect x="206" y="78" width="440" height="60" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="218" y="102" font-size="12" font-family="monospace" fill="currentColor">pub trait Greet: private::Sealed</text>
<text x="218" y="124" font-size="12" fill="var(--links)">外部可命名、可当边界</text>
<rect x="206" y="152" width="440" height="72" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.55"/>
<text x="218" y="176" font-size="12" font-family="monospace" fill="currentColor">mod private（不是 pub）</text>
<text x="218" y="198" font-size="12" font-family="monospace" fill="currentColor">└ trait Sealed</text>
<text x="218" y="218" font-size="12" fill="var(--warning-border)">← 外部不可命名 → impl Sealed 写不出来</text>
</svg>

几条容易踩的细节：

- **这是编译期强制，不是文档约定。** 外部 crate 尝试实现会得到 E0277，外加一条 rustc 专门为这个模式写的 note（报错现场第三例）。
- **和 `pub(crate) trait` 不是一回事。** `pub(crate)` 的 trait 外部根本看不见，连 `fn f<G: Greet>(g: &G)` 都写不出来。密封要的正是「看得见、用得上，但只有我能实现」。
- **它管的是实现者，不是使用者。** 下游照样可以 `fn describe<G: Greet>(g: &G)`。
- **全局 blanket impl 有同样的封锁效果，但更粗暴。** 你写下 `impl<T: Debug> Describe for T` 的那一刻，下游就再也无法 `impl Describe for Point`——跨 crate 会报 E0119，理由节选如下，而且这条 impl 会把你自己的外部类型全部卷进来：

  ```text
    = note: conflicting implementation in crate `blanket`:
            - impl<T> Describe for T
              where T: Debug;
  ```
- **它和 `#[non_exhaustive]` 是两件事。** 后者管的是构造与匹配（外部 crate 不能字面构造、不能穷尽匹配），而且只能用在结构体、枚举与枚举变体上（Reference）。想「以后加字段不破坏下游」用它；想「以后加方法不破坏下游」用密封。

标准库自己也这么干。`std` 源码里有一个私有模块，注释原文是：*"This trait being unreachable from outside the crate prevents outside implementations of our extension traits. This allows adding more trait methods in the future."*——大意是：这个 trait 在 crate 外部不可达，从而阻止外部实现我们的扩展 trait，这让我们以后还能加方法。

代价要说清：对下游而言，密封 trait 只是「一组封闭类型的标签」，不能拿来自定义类型。所以只在两种情况下用——实现者集合确实封闭（所有合法实现都由你提供），或者这个契约的正确性要求实现者必须由你控制。如果你能承诺「以后新增的方法一律带默认实现」，那就不需要密封。

### 扩展 trait（extension trait）：给外部类型加方法

场景：`Vec<T>` 缺一个 `second()`，或者你想给所有 `Iterator` 加一个组合子。

你**不能**给外部类型加固有方法（inherent method；报错文本以 Rust 1.98 为例）：

```rust,compile_fail
impl Vec<i32> {
    fn second(&self) -> Option<&i32> {
        self.get(1)
    }
}

fn main() {}
```

```text
error[E0116]: cannot define inherent `impl` for a type outside of the crate where the type is defined
 --> e0116.rs:1:1
  |
1 | impl Vec<i32> {
  | ^^^^^^^^^^^^^ impl for type defined outside of crate
  |
  = help: consider defining a trait and implementing it for the type or using a newtype wrapper like `struct MyType(ExternalType);` and implement it
```

但**可以**定义自己的 trait，再实现给外部类型——这条 impl 里 trait 是「本地的」，孤儿规则满足。这就是扩展 trait：

```rust
trait VecExt<T> {
    fn second(&self) -> Option<&T>;
}

impl<T> VecExt<T> for Vec<T> {
    fn second(&self) -> Option<&T> {
        self.get(1)
    }
}
```

用起来有两点与普通 trait 一样、但新手容易忘：

1. **trait 必须在作用域里。** 下游要写 `use my_crate::VecExt;` 才能 `v.second()`，否则 E0599 会提示 *"items from traits can only be used if the trait is in scope"*。
2. **与固有方法同名时，固有方法优先。** `v.len()` 调用的永远是 `Vec::len`；想调扩展 trait 的那个，用全限定写法 `VecExt::len(&v)`（等价的长写法是 `<Vec<i32> as VecExt<i32>>::len(&v)`）。两个扩展 trait 提供同名方法时同理，编译器会要求你消歧——写 `TraitA::m(&x)` 或 `TraitB::m(&x)`，它不会替你猜。

什么时候值得写扩展 trait：你想给**一片**已有类型补同一组方法，而且这些方法能靠 blanket impl 覆盖（`impl<T> VecExt<T> for Vec<T>`、`impl<I: Iterator> IterExt for I`）。什么时候不值得：只有一两个类型需要，或者方法名很容易与别的 crate 撞车——扩展方法只有 `use` 之后才存在，读者看到 `v.second()` 时未必知道它从哪来。

> [!TIP]
> 覆盖越宽的 blanket impl，越接近「密封」：`impl<I: Iterator> IterExt for I` 之后，下游别想再为任何迭代器实现 `IterExt`。封死不总是缺点，但要知道自己签了什么。

### 一致性（coherence）与孤儿规则的细节

孤儿规则（orphan rule）的表述是：

> The orphan rule states that a trait implementation is only allowed if either the trait or at least one of the types in the implementation is defined in the current crate.

一句话概括：**trait 与类型，至少有一个得是「你的」**。理由第 9 章讲过——如果 `Display for Vec<i32>` 允许任何 crate 来写，两份互不相容的实现就没人能裁决。

泛型出现后，「属于你」需要精确化。Reference 给的判据是：对 `impl<P1..=Pn> Trait<T1..=Tn> for T0`，这条 impl 合法当且仅当

1. `Trait` 是本地的；或者
2. 同时满足：`T0..=Tn` 中至少有一个本地类型（记其中最靠前的那个为 `Ti`），并且**没有「未覆盖（uncovered）」的类型参数出现在 `T0..Ti` 中（`Ti` 本身除外）**。

「未覆盖」的意思是：这个类型参数没有被包在某个本地类型里面。实践里记住这张表就够用：

| 情形 | 例子 | 结果 |
|---|---|---|
| trait 本地 | `impl<T> Describe for T` | 允许（这等于替所有人签了契约，别人再也实现不了） |
| 类型本地 | `impl fmt::Display for WordList` | 允许（第 9 章的 newtype） |
| 都外部、且没有本地类型 | `impl fmt::Display for Vec<i32>` | E0117（第 9 章报错现场见过） |
| 都外部、类型参数裸露 | `impl<T> fmt::Display for T` | E0210 |

把「trait 的位置」与「类型的位置」当成两个轴，四种组合的去留就摊成一张 2×2：

<svg viewBox="0 0 660 264" width="100%" role="img" aria-label="孤儿规则 2×2：trait 本地时两格都允许；trait 外部而类型本地也允许；trait 与类型都外部则写不出来——具体外部类型报 E0117，裸露类型参数报 E0210；本地类型由 newtype 提供" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">孤儿规则的两个维度：trait 与类型，至少有一个是本地（你的）</text>
<text x="8" y="42" font-size="12" fill="currentColor" opacity="0.7">行 trait ／ 列 类型</text>
<text x="263" y="42" font-size="13" fill="currentColor" text-anchor="middle">类型：本地（你的）</text>
<text x="525" y="42" font-size="13" fill="currentColor" text-anchor="middle">类型：外部（别人的）</text>
<text x="8" y="102" font-size="13" fill="currentColor">trait：本地</text>
<text x="8" y="196" font-size="13" fill="currentColor">trait：外部</text>
<rect x="134" y="52" width="258" height="90" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="263" y="92" font-size="14" fill="var(--links)" text-anchor="middle">✓ 允许</text>
<text x="263" y="118" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">impl Describe for WordList</text>
<rect x="396" y="52" width="258" height="90" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="525" y="92" font-size="14" fill="var(--links)" text-anchor="middle">✓ 允许</text>
<text x="525" y="118" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">impl&lt;T&gt; Describe for T</text>
<rect x="134" y="146" width="258" height="90" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5"/>
<text x="263" y="186" font-size="14" fill="var(--links)" text-anchor="middle">✓ 允许</text>
<text x="263" y="212" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">impl fmt::Display for WordList</text>
<rect x="396" y="146" width="258" height="90" rx="6" fill="none" stroke="var(--warning-border)" stroke-width="1.5"/>
<text x="525" y="186" font-size="14" fill="var(--warning-border)" text-anchor="middle">✗ 不能写</text>
<text x="525" y="206" font-size="12" fill="var(--warning-border)" text-anchor="middle">具体外部类型 → E0117</text>
<text x="525" y="224" font-size="12" fill="var(--warning-border)" text-anchor="middle">裸露类型参数 → E0210</text>
<text x="0" y="254" font-size="12" fill="currentColor" opacity="0.7">角注：本地类型由 newtype 提供——包一层本地类型，同时给出本地类型与覆盖类型参数的位置</text>
</svg>

把第二行与第三行连起来看，newtype 为什么能当逃生门就清楚了：「包一层本地类型」同时提供了本地类型**和**覆盖类型参数的位置。`impl<T> fmt::Display for Wrapper<T>` 合法（`Wrapper` 本地、`T` 被它覆盖）；而 `impl<T> fmt::Display for Box<T>` 不合法——`Box` 本身是外部的，这个外壳不算本地类型（实测报 E0210）。

这里要引入一类特例：**基础类型构造器（fundamental type constructors）**。Reference 术语表的原文是：*"A fundamental type constructor is a type where implementing a blanket implementation over it is a breaking change. `&`, `&mut`, `Box`, and `Pin` are fundamental."* 后面紧接着一句：*"Any time a type `T` is considered local, `&T`, `&mut T`, `Box<T>`, and `Pin<T>` are also considered local. Fundamental type constructors cannot cover other types."*

翻译成可操作的规则，就是两句话：

- 本地性会「穿透」这四个包装：`Local` 是你的，`&Local`、`&mut Local`、`Box<Local>`、`Pin<Local>` 也**算**你的。所以 `impl fmt::Display for Box<Local>` 合法——本章实测通过。
- 但反过来，这四个包装**不能覆盖**类型参数：`Box<T>` 里的 `T` 仍然算裸露，所以 `Box<T>` 不能充当那个「本地落脚点」。

设计意图也说得通：`Box`/`&` 是透明的转发包装，不该挡住本地性；但它们一旦成为「通用覆盖器」，就会把下游的实现空间整个吞掉，所以不能。

一致性的另一半是「没有重叠实现」。同 crate 内两条 impl 覆盖到同一个具体类型即冲突（报错现场第二例）；跨 crate 的情形由孤儿规则预防——那条 impl 你根本写不出来。但要注意：你自己的 blanket impl 会替下游制造冲突，这一点在「密封 trait」与「扩展 trait」两节都出现过。

> [!WARNING]
> 「trait 或类型有一个是本地」不等于「随便写」。`impl<T> MyTrait for T` 合法，但它意味着全宇宙的类型都被你这一条 impl 覆盖：你既封死了下游，也让自己以后无法为某个类型写更专门的实现（那是 E0119）。写 blanket impl 前先想清楚要不要留这个口子。

### 让报错说人话：`#[diagnostic::on_unimplemented]`

trait 的作者能顺手决定别人看到什么报错。`#[diagnostic::on_unimplemented]` 从 1.78 起稳定，三个键：`message`（顶层错误文案）、`label`（指向出错位置的短标签）、`note`（附加说明，可写多条）。文案里 `{Self}` 会被替换成实现者类型的名字。

```rust,compile_fail
#[diagnostic::on_unimplemented(
    message = "类型 `{Self}` 不能当作配置源",
    label = "缺少 `ConfigSource` 实现",
    note = "要么给该类型手写 `impl ConfigSource`，要么改用 `ConfigStore`"
)]
trait ConfigSource {
    fn get(&self, key: &str) -> Option<String>;
}

struct Env;

fn load<S: ConfigSource>(src: &S) -> usize {
    src.get("count").map_or(0, |s| s.len())
}

fn main() {
    let env = Env;
    println!("{}", load(&env));
}
```

```text
error[E0277]: 类型 `Env` 不能当作配置源
  --> diag.rs:18:25
   |
18 |     println!("{}", load(&env));
   |                    ---- ^^^^ 缺少 `ConfigSource` 实现
   |                    |
   |                    required by a bound introduced by this call
   |
help: the trait `ConfigSource` is not implemented for `Env`
  --> diag.rs:10:1
   |
10 | struct Env;
   | ^^^^^^^^^^
   = note: 要么给该类型手写 `impl ConfigSource`，要么改用 `ConfigStore`
```

对照你在第 9 章报错现场见过的那个裸 E0277，差别就是「编译器只知道边界没满足」与「你的库告诉使用者下一步该做什么」。写在 trait 声明上；解析失败或键名写错只会变成警告，不会让编译失败。

## 动手看看

四个程序。在仓库里跑（以第一个为例，其余把文件名换掉）：

```console
$ rustc --edition 2024 snippets/ch12/assoc_vs_generic.rs
$ ./assoc_vs_generic       # Linux / macOS
$ .\assoc_vs_generic.exe   # Windows
```

**1. 关联类型与泛型参数各来一份**（`snippets/ch12/assoc_vs_generic.rs`）：

```rust
{{#rustdoc_include ../snippets/ch12/assoc_vs_generic.rs}}
```

```console
Meters(3.5)
Meters(6.5)
1.50 m
3
1.50 m
1.50 m
1.5
```

前两行是 `Add<Rhs>` 的泛型参数在起作用：`a + b` 走 `Add<Meters>`，`a + 5.0` 走 `Add<f64>`，同一类型两种解释。中间三行是关联类型：`Report` 里 `Detail` 只能填一次。最后两行把同一个抽象改用泛型参数写（`ReportAs<T>`），于是 `Meters` 能同时给出 `String` 和 `f64` 两种报告。

**2. 父 trait 不是继承**（`snippets/ch12/supertrait.rs`）：

```rust
{{#rustdoc_include ../snippets/ch12/supertrait.rs}}
```

```console
area = 3.1416, radius = 1.0000
```

`UnitCircle` 分别写了 `impl Shape` 与 `impl Circle` 两段；`describe` 的边界只要求 `Circle`，函数体里却能调 `c.area()`——那是父 trait 的关联项可用。

**3. 密封 trait**（`snippets/ch12/sealed.rs`）：

```rust
{{#rustdoc_include ../snippets/ch12/sealed.rs}}
```

```console
hello
```

`lib::Greet` 是公开的，`lib::private::Sealed` 不是。这个文件里 `English` 能实现，是因为 `impl` 写在 `lib` 内部；换到别的 crate 再试就报错（报错现场第三例）。

**4. 扩展 trait 与同名冲突**（`snippets/ch12/extension_trait.rs`）：

```rust
{{#rustdoc_include ../snippets/ch12/extension_trait.rs}}
```

```console
Some(20)
固有 len: 3
扩展 len: 999
```

`VecExt` 是本地的，所以能给外部的 `Vec<T>` 补方法；`second()` 直接可用，`len()` 则因为与固有方法同名，必须用全限定写法才能调到 trait 里那份（返回 `999` 而不是 `3`）。这个程序里 `VecExt` 与 `main` 同处一个模块，所以不需要 `use`；一旦它被移到别的模块，`v.second()` 就会要求你先把它 `use` 进来。

## 报错现场

三例，都按「最小复现 → 报错关键行 → 定位 → 修法」看。报错文本以 Rust 1.98 为例（下同），输出的行号对应紧跟其上的清单。

**一、E0210：类型参数没被本地类型覆盖。**

```rust,compile_fail
use std::fmt;

impl<T> fmt::Display for T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "某个 T")
    }
}

fn main() {}
```

```text
error[E0210]: type parameter `T` must be used as an argument to some local type (e.g., `MyStruct<T>`)
 --> e0210.rs:3:6
  |
3 | impl<T> fmt::Display for T {
  |      ^ uncovered type parameter
  |
  = note: implementing a foreign trait is only possible if at least one of the types for which it is implemented is local
  = note: only traits defined in the current crate can be implemented for a type parameter
```

`Display` 是外部的，`T` 是类型参数——没有任何本地类型出现。报错把 `T` 标成 "uncovered type parameter"（未覆盖的类型参数），并给出了标准解法：让它当某个本地类型的参数。修法是 newtype：

```rust
use std::fmt;

struct Wrapper<T>(T);

impl<T> fmt::Display for Wrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "wrapper")
    }
}
```

**二、E0119：两条 impl 重叠。**

```rust,compile_fail
trait Describe {
    fn describe(&self) -> String;
}

struct Widget;

impl<T> Describe for T {
    fn describe(&self) -> String {
        String::from("某种类型")
    }
}

impl Describe for Widget {
    fn describe(&self) -> String {
        String::from("一个 Widget")
    }
}

fn main() {}
```

```text
error[E0119]: conflicting implementations of trait `Describe` for type `Widget`
  --> e0119.rs:13:1
   |
 7 | impl<T> Describe for T {
   | ---------------------- first implementation here
...
13 | impl Describe for Widget {
   | ^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Widget`
```

`Describe` 是本地的，两条 impl 都合法地满足了孤儿规则——但它们重叠了：`Widget` 既被第一条 blanket impl 覆盖，又被第二条专门覆盖。一致性要求每个 (trait, 类型) 组合唯一，于是冲突。修法是二选一：要么删掉 blanket impl，让每个类型自己写；要么保留 blanket impl，删掉特例（并接受「谁也别想再实现」这个后果）。**不要**试图让编译器替你分优先级——Rust 没有这个机制，专门化（specialization）至今仍不稳定。

> [!WARNING]
> 专门化（specialization）允许为 blanket impl 提供更具体的版本，但它**仍未稳定**，stable 1.98 上写 `default fn` 会报 E0658。别在 stable 项目里设计依赖它的 API。

**三、密封 trait：父 trait 不可达。**

```rust,compile_fail
mod lib {
    mod private {
        pub trait Sealed {}
    }

    pub trait Greet: private::Sealed {
        fn greet(&self) -> String;
    }
}

struct MyGreeter;

impl lib::Greet for MyGreeter {
    fn greet(&self) -> String {
        String::from("hi")
    }
}

fn main() {}
```

```text
error[E0277]: the trait bound `MyGreeter: Sealed` is not satisfied
  --> sealed.rs:13:21
   |
13 | impl lib::Greet for MyGreeter {
   |                     ^^^^^^^^^ unsatisfied trait bound
   |
help: the trait `Sealed` is not implemented for `MyGreeter`
  --> sealed.rs:11:1
   |
11 | struct MyGreeter;
   | ^^^^^^^^^^^^^^^^
help: this trait has no implementations, consider adding one
  --> sealed.rs:3:9
   |
 3 |         pub trait Sealed {}
   |         ^^^^^^^^^^^^^^^^
note: required by a bound in `Greet`
  --> sealed.rs:6:22
   |
 6 |     pub trait Greet: private::Sealed {
   |                      ^^^^^^^^^^^^^^^ required by this bound in `Greet`
   = note: `Greet` is a "sealed trait", because to implement it you also need to implement `lib::private::Sealed`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
```

这个最小复现只用了一个 crate：`MyGreeter` 在 `lib` 模块**外面**，而 `private` 模块在 `lib` 里面且不是 `pub`，所以它也够不到 `Sealed`。修法就是「不实现」——这正是设计意图：拿 `lib::English` 用就好。

把同一份代码搬到真正的下游 crate（`greet` 与 `user` 两个包）后，报错是同一形状，只是类型路径前面多了 crate 名。帮助信息则从「上游视角」换成了「下游视角」：单 crate 版在「父 trait 未实现」那条 help 之后跟的是 `help: this trait has no implementations, consider adding one`；跨 crate 版同一位置换成「the trait `greet::private::Sealed` is implemented for `English`」，直接点出哪个类型已经实现了它。末尾再多一条 `= help`，把使用者引向那个类型（下面是节选）：

```text
error[E0277]: the trait bound `MyGreeter: greet::private::Sealed` is not satisfied
 --> user\src\main.rs:5:16
  |
5 | impl Greet for MyGreeter {
  |                ^^^^^^^^^ unsatisfied trait bound
  |
…
   = note: `Greet` is a "sealed trait", because to implement it you also need to implement `greet::private::Sealed`, which is not accessible; this is usually done to force you to use one of the provided types that already implement it
   = help: the following type implements the trait:
             greet::English
```

## 小结

1. **关联类型 = 每个类型只有一个答案；泛型参数 = 同一类型可以有多个答案。** 对照 `Iterator::Item` 与 `Add<Rhs>`：先问「几种解释」，再问「是输入侧还是输出侧」，答案就出来了。
2. **父 trait 是前提条件，不是继承。** 实现要分两次写；得到的好处是「以子 trait 为边界时，父 trait 的关联项可用」。`where Self: Shape` 是等价写法。
3. **密封 trait 用 `pub trait X: private::Sealed` 实现**：外部看得见、当得了边界，但实现不了，因为父 trait 不可命名。rustc 会专门提示这是 "sealed trait"（诊断原文）。全局 blanket impl 有同样的封死效果，但会连带吞掉所有类型。
4. **扩展 trait 是孤儿规则的合法应用**：trait 本地，就能实现给外部类型。使用时 trait 必须进作用域；与固有方法同名时固有方法优先，要调 trait 里那份得写全限定路径。
5. **一致性 = 每个 (trait, 类型) 组合唯一实现。** trait 本地或类型本地就能写 impl；类型参数必须被本地类型覆盖（`impl<T> Display for T` 是 E0210，包一层 `Wrapper<T>` 就好）。`&`、`&mut`、`Box`、`Pin` 是基础类型构造器：本地性会穿透它们，而它们不能覆盖类型参数。

## 练习

**1. 预测编译结果。** 下面三条 `impl` 哪条能过？过不了的是哪个错误码？

```rust,ignore
use std::fmt;

struct Wrapper<T>(T);
type Alias = Vec<i32>;

// A
impl<T> fmt::Display for Wrapper<T> { /* ... */ }

// B
impl<T> fmt::Display for Box<T> { /* ... */ }

// C
impl fmt::Display for Vec<i32> { /* ... */ }
```

<details><summary>参考答案</summary>

A 能过。`Wrapper` 是本地类型，`T` 被它覆盖，满足孤儿规则。

B 报 **E0210**（"type parameter `T` must be used as an argument to some local type"）。`Box` 在标准库里，所以 `Box<T>` 不是本地类型；又因为基础类型构造器不能覆盖类型参数，`T` 依然算裸露。

C 报 **E0117**（"only traits defined in the current crate can be implemented for types defined outside of the crate"）。`Display` 与 `Vec` 都在外部，这条 impl 里没有本地落脚点。

顺带说明 `type Alias = Vec<i32>;` 那行：类型别名不改变「本地性」——给外部类型起个别名，`impl Display for Alias` 依然是 E0117（E0210 的错误页里也明确写了别名不算）。

</details>

**2. 解释。** 为什么密封 trait 要写成 `pub trait Greet: private::Sealed`，而不是直接把 trait 声明成 `pub(crate) trait Greet`？

<details><summary>参考答案</summary>

`pub(crate)` 让外部 crate 完全看不见这个 trait：既不能命名它，也不能写 `fn describe<G: Greet>(g: &G)`。而密封 trait 的目标是「公开、可用，但只有我能实现」——`Greet` 必须是 `pub`，才能出现在你的公开 API 签名里、才能被下游当作边界使用。

封住实现入口靠的是父 trait：`private::Sealed` 在外部不可命名，于是外部写不出 `impl Sealed`，也就永远满足不了 `Greet` 的前提条件。可见性与可实现性由此分开。

代价也要一并说：下游无法把自己的类型接进这个抽象。所以它只适合「实现者集合由你提供」的场合。

</details>

**3. 设计判断。** 你要设计一个 `Connector` trait，把消息发到某个端点。同一个类型可能同时支持「测试端点」和「生产端点」两套配置，两种情况的返回类型相同。这里该用关联类型还是泛型参数？再加一问：如果端点在类型创建时就固定了（比如 `TcpConnector` 只连 TCP），又该怎么选？

<details><summary>参考答案</summary>

第一种情况用**泛型参数**（`trait Connector<E>`）：同一个类型对同一抽象有两种解释，而关联类型一个类型只能填一次。注意这与 `Add` 同构——`Meters + Meters` 与 `Meters + f64` 就是「同一类型两种解释」，`Add` 的做法正是把可变的那一侧做成泛型参数。

第二种情况用**关联类型**：端点由类型唯一决定（`TcpConnector` 只会连 TCP），调用方不需要挑，也就不该被迫在每处标注。此时还能顺带用 `T: Connector<Endpoint = Tcp>` 这样的关联类型约束把端点钉死。

还有一条辅助判据：可变的那一侧是**输入**（调用方决定发去哪里）→ 泛型参数；是由实现类型唯一决定的**输出** → 关联类型。

</details>

## 延伸阅读

- The Rust Reference，Traits（父 trait、dyn 兼容性）：<https://doc.rust-lang.org/reference/items/traits.html>
- The Rust Reference，Associated items（关联函数、类型、常量）：<https://doc.rust-lang.org/reference/items/associated-items.html>
- The Rust Reference，Implementations（孤儿规则与一致性）：<https://doc.rust-lang.org/reference/items/implementations.html>
- The Rust Reference，Type system attributes（`non_exhaustive`）：<https://doc.rust-lang.org/reference/attributes/type_system.html>
- The Rust Reference，Diagnostic attributes（`#[diagnostic::on_unimplemented]`）：<https://doc.rust-lang.org/reference/attributes/diagnostics.html>
- The Book 第 20 章「高级 trait」：<https://doc.rust-lang.org/book/ch20-02-advanced-traits.html>
- `std::iter::Iterator`：<https://doc.rust-lang.org/std/iter/trait.Iterator.html>
- `std::ops::Add`：<https://doc.rust-lang.org/std/ops/trait.Add.html>
