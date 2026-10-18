# 第 36 章 高级类型系统（nightly 标注）

## 这一章解决什么问题

第 9 章把 trait 讲成「一组方法的契约」，第 12 章讲怎么设计它，第 13 章讲 `dyn`，第 15 章讲 const 泛型。这一章不再引入新的 trait 用法，而是回答三个你一直在用、却从没有正面回答的问题。

**第一，「为什么这条 impl 编译器不许写」。** 你一定撞过两类报错：给外部类型实现外部 trait 被拒；同一个 trait 对同一个类型写了两条实现被拒。它们背后是同一条规则——**一致性（coherence）**。一致性有两个检查口：跨 crate 的**孤儿规则（orphan rule）**，和 crate 内部的**重叠（overlap）判定**。很多资料把孤儿规则简化成「要么 trait 本地，要么类型本地」，这句话漏掉了后半截：**未覆盖（uncovered）的类型参数**。而正是后半截，让 `impl<T> SomeTrait for Box<T>` 和 `impl<T> SomeTrait for Vec<T>` 有了不同的命运。

**第二，「能不能声明某个类型不实现某个 trait」。** 标准库自己在用：`Rc<T>` 不是 `Send`，裸指针不是 `Send`，`&T` 不是 `DerefMut`。但那是编译器给标准库留的门，Reference 写得很直白：「There is currently no stable way to specify additional negative implementations; they exist only in the standard library.」这一章给你 stable 上的替代品。

**第三，「通用实现 + 特例实现」与「const 参数参与算术」离稳定还有多远。** 这两件事分别挂在 specialization 与 generic_const_exprs 名下，连同 adt_const_params，到今天仍是 nightly。

读完这一章你能：

- 拿孤儿规则逐条判定一条 `impl` 能不能写，并预测它会被 E0117、E0210 还是 E0119 拦下；
- 说清 `Vec<LocalType>` 与 `Box<LocalType>` 的差别从哪来；
- 说出 negative impls（负实现）、specialization、adt_const_params、generic_const_exprs 各自的语法形状、追踪 issue 编号，以及今天在 stable 上该写什么。

> [!NOTE]
> 回指：trait 与关联类型在第 12 章（该章的「一致性（coherence）与孤儿规则的细节」一节已给出一致性基线、2×2 与 newtype 逃生门），`dyn` 兼容性在第 13 章，const 泛型与 GAT 在第 15 章；`Send`/`Sync` 在第 19 章，`Pin` 在第 22 章；`compile_fail` 与 doc test 在第 31 章。

## 为什么这样设计

### 为什么必须「全局唯一」

trait 方法调用要被静态解析：编译器看到 `x.foo()`，得在**编译期**决定调哪一份实现。如果同一对 (trait, 类型) 允许有多份实现，这个问题就没有确定答案——除非把选择推迟到链接期（冲突那时才炸），或者推迟到运行期（静态分派就没了）。Rust 的选择是把「唯一性」做成**编译期可判定的局部性质**：检查一条 `impl` 是否合法，只看它的头部（trait、类型、参数），不需要看任何方法体。这条保证叫一致性（coherence），它带来两个直接好处。

- **编译是局部的。** 判断 `T: Trait` 是否成立，只需要看三处：trait 的所有者写过哪些 impl、类型的所有者写过哪些 impl、当前 crate 写过哪些 impl——不必翻遍整张依赖图。
- **下游不会被悄悄破坏。** 孤儿规则禁止「外部 trait + 外部类型」的组合，于是每一对组合都只有一个可能的作者：要么是 trait 的所有者，要么是类型的所有者。

第二点值得多说一句，它是孤儿规则存在的真正理由。假设允许任意 crate 写 `impl Display for u32`：那么你升级任意一个依赖，都可能凭空多出一条和你代码冲突的 impl，编译突然失败。

不过「扩展权归 trait 的所有者」不等于「所有权随便用」。下游可以给自己的类型实现你的 trait，所以所有者事后补一条 blanket impl（`impl<T: Display> MyTrait for T`），仍可能撞上下游为自有类型写下的实现而报 E0119——这是本章练习 2 实测出来的语义版本陷阱。

### 为什么越界的三件事都还在 nightly

三条想越界的能力，卡住的原因并不一样。

- **negative impls（负实现）**：它给一致性规则加了一个新的事实来源——「这对组合永远不成立」。这同时是语义版本承诺（今天声明不实现，将来不能改口），又牵动 auto trait 的自动推导与 trait 求解器里负面信息的传播。
- **specialization**：它要在一部分情形下**放弃**唯一性，改成「谁更具体谁赢」。这需要一套判定「更具体」的偏序，还要保证被特化掉的方法不会反过来破坏调用方已经做过的类型检查。官方至今只把它拆出一个保守子集给标准库用，unstable book 的原话是「A minimal, sound subset of specialization intended to be used by the standard library until the soundness issues with specialization are fixed.」
- **更复杂的 const 参数**：`adt_const_params` 要回答「什么样的值可以参与类型相等判定」，`generic_const_exprs` 要回答「两个含泛型参数的常量表达式何时相等」——后者是一个通用的约束求解问题，官方主动让它保持不完整（unstable book 的警告原文是「This feature is incomplete; its design and syntax may change.」）。

## 机制与规则

### 从第 12 章的基线出发

第 12 章已经给过基线和那张 2×2：一致性由两个口子把守（孤儿规则管跨 crate 的归属，重叠判定管同一对组合不被写两遍）、`Vec<T>` 里的 `T` 算被覆盖而裸 `T` 算未覆盖、`&`/`&mut`/`Box`/`Pin` 的 fundamental 特例、newtype 是逃生门，以及「同 crate 内两条 impl 撞上同一个具体类型就是错误」。这些结论本章直接沿用，不再重复。

本章只补第 12 章没有展开的两件事：**判定的扫描顺序**（`T0` 排在最前，不是排在最后），以及一份**逐条判定表**——把常见写法连同「被哪一条规则挡住、报哪个错误码」一次对齐。

### 孤儿规则的精确版本

Reference 给出的形式化版本（原话）：

> Given `impl<P1..=Pn> Trait<T1..=Tn> for T0`, an `impl` is valid only if at least one of the following is true:
> - `Trait` is a local trait
> - All of
>   - At least one of the types `T0..=Tn` must be a local type. Let `Ti` be the first such type.
>   - No uncovered type parameters `P1..=Pn` may appear in `T0..Ti` (excluding `Ti`)

用中文说，只有两点要注意，另加一条错误码分工。

- **扫描顺序**：判定对象是 `impl<..> Trait<T1, ..., Tn> for T0` 里的 `T0 → T1 → ... → Tn`，**`T0` 排在最前**。注意 `T0` 是 self 类型（写在 `for` 后面），`T1..=Tn` 才是 trait 的类型参数。先在这一串里找第一个本地类型，记作 `Ti`。
- **窗口**：`Ti` 之前那一段（`T0` 到 `Ti`，不含 `Ti`）不能出现**未覆盖**的类型参数；一个本地类型都找不到时，这个窗口就是整条 impl。
- **错误码分工**（1.98 实测）：头部存在未覆盖的类型参数 → **E0210**；没有本地类型、但也没有未覆盖的参数 → **E0117**。

下面这张图把整条判定链摆开：

<svg viewBox="0 0 760 262" width="100%" role="img" aria-label="孤儿规则判定流程：先看 trait 是否本地；否则按 T0 到 Tn 的顺序找第一个本地类型，找不到时若头部有未覆盖参数报 E0210、否则报 E0117；找到 Ti 后检查它前面有没有未覆盖参数，有则报 E0210，没有则通过">
<defs><marker id="ch36-arrow" markerUnits="userSpaceOnUse" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 Z" fill="currentColor"/></marker></defs>
<g fill="none" stroke="currentColor" stroke-width="1.6">
<rect x="14" y="14" width="250" height="46" rx="5" stroke="var(--quote-border)"/>
<rect x="306" y="14" width="180" height="46" rx="5" stroke="var(--quote-border)"/>
<rect x="536" y="14" width="210" height="46" rx="5" stroke="var(--links)"/>
<rect x="306" y="98" width="180" height="56" rx="5" stroke="var(--quote-border)"/>
<rect x="536" y="98" width="210" height="56" rx="5" stroke="var(--warning-border)"/>
<rect x="40" y="176" width="270" height="56" rx="5" stroke="var(--quote-border)"/>
<rect x="390" y="176" width="150" height="56" rx="5" stroke="var(--warning-border)"/>
<rect x="580" y="176" width="166" height="56" rx="5" stroke="var(--links)"/>
<line x1="264" y1="37" x2="306" y2="37" marker-end="url(#ch36-arrow)"/>
<line x1="486" y1="37" x2="536" y2="37" marker-end="url(#ch36-arrow)"/>
<line x1="396" y1="60" x2="396" y2="98" marker-end="url(#ch36-arrow)"/>
<line x1="486" y1="126" x2="536" y2="126" marker-end="url(#ch36-arrow)"/>
<polyline points="396,154 396,166 175,166 175,176" marker-end="url(#ch36-arrow)"/>
<line x1="310" y1="204" x2="390" y2="204" marker-end="url(#ch36-arrow)"/>
<polyline points="175,232 175,250 663,250 663,232" marker-end="url(#ch36-arrow)"/>
</g>
<g fill="currentColor" font-size="12" font-family="sans-serif">
<text x="28" y="43" font-family="monospace">impl&lt;..&gt; Trait&lt;T1..=Tn&gt; for T0</text>
<text x="396" y="43" text-anchor="middle">Trait 是本地 trait？</text>
<text x="641" y="43" text-anchor="middle">通过</text>
<text x="504" y="30" text-anchor="middle">是</text>
<text x="404" y="84">否</text>
<text x="396" y="124" text-anchor="middle">按 T0 → Tn 顺序</text>
<text x="396" y="140" text-anchor="middle">找第一个本地类型 Ti</text>
<text x="511" y="119" text-anchor="middle">找不到</text>
<text x="641" y="118" text-anchor="middle">没有本地类型时：</text>
<text x="641" y="134" text-anchor="middle">有未覆盖参数 → E0210</text>
<text x="641" y="150" text-anchor="middle">否则 → E0117</text>
<text x="285" y="161" text-anchor="middle">找到</text>
<text x="175" y="204" text-anchor="middle">Ti 之前出现</text>
<text x="175" y="220" text-anchor="middle">uncovered 参数？</text>
<text x="465" y="204" text-anchor="middle">E0210</text>
<text x="465" y="220" text-anchor="middle">未覆盖的参数</text>
<text x="663" y="208" text-anchor="middle">通过</text>
<text x="350" y="197" text-anchor="middle">是</text>
<text x="420" y="244" text-anchor="middle">否</text>
</g>
</svg>

四个术语界定上面每一条：

| 术语 | 判定时关心的那一点 |
|---|---|
| 本地 trait（local trait） | 当前 crate 定义的 trait；与它接收哪些类型实参无关 |
| 本地类型（local type） | 当前 crate 定义的 struct / enum / union。**不受类型实参影响**：`struct Foo` 是本地类型，`Vec<Foo>` 不是；`LocalType<ForeignType>` 是本地类型 |
| 未覆盖 / 被覆盖（uncovered / covered） | 类型参数作为**另一个类型的实参**出现就是被覆盖：`Vec<T>` 里的 `T` 被覆盖，裸 `T` 未覆盖。**只有未覆盖的参数受窗口限制** |
| 基础类型构造器（fundamental type constructor） | `&`、`&mut`、`Box`、`Pin`。它的两条规定（本地性可以穿透、但自己不能覆盖参数）连同原文出处都在第 12 章，这里只用后果判定：`Box<LocalType>` 算本地类型，`Box<T>` 里的 `T` 仍算未覆盖 |

第 12 章引过这些术语的 Reference 原文，上表只按「判定时要用到哪一点」重新组织。

### 逐条判定

把 `Point` 当成本地 struct、`MyTrait` 当成本地 trait，下表每一条都在 Rust 1.98 上实测过：

| `impl` | 结果 | 依据 |
|---|---|---|
| `impl Display for Point` | 通过 | `T0` 是本地类型 |
| `impl<T> MyTrait for Vec<T>` | 通过 | trait 本地（规则第 1 条） |
| `impl Index<usize> for Box<Point>` | 通过 | `Box` 是 fundamental，`Box<Point>` 算本地类型 |
| `impl<T> From<Point> for Vec<T>` | 通过 | 第一个本地类型是 `T1 = Point`；窗口里的 `T0 = Vec<T>` 中 `T` 被 `Vec` 覆盖 |
| `impl Display for Vec<Point>` | E0117 | 没有本地类型，也没有未覆盖参数（`Point` 藏在 `Vec` 里，不算本地类型） |
| `impl<T> Display for Vec<T>` | E0117 | 同样没有本地类型；`T` 被 `Vec` 覆盖，不算未覆盖参数 |
| `impl Index<usize> for Vec<Point>` | E0117 | `Vec<Point>` 不是本地类型，`usize` 也不是；头部没有未覆盖参数 |
| `impl<T> Display for T` | E0210 | 没有本地类型，`T` 裸露：存在未覆盖参数 |
| `impl<T> Display for Box<T>` | E0210 | 没有本地类型；`Box` 是 fundamental，盖不住 `T`，于是 `T` 未被覆盖 |
| `impl<T> From<Point> for Box<T>` | E0210 | 第一个本地类型是 `T1 = Point`，但窗口里的 `T0 = Box<T>` 含**未**覆盖的 `T` |

把最后六行并排看，就是错误码的分工：**没有未覆盖参数**的外来组合（`impl Display for Vec<Point>`、`impl<T> Display for Vec<T>`、`impl Index<usize> for Vec<Point>`）报 E0117，**有未覆盖参数**的（`impl<T> Display for T`、`impl<T> Display for Box<T>`、`impl<T> From<Point> for Box<T>`）报 E0210——有没有类型参数本身并不决定用哪个码。

再把 `impl Index<usize> for Box<Point>` 与 `impl<T> From<Point> for Box<T>` 并排看，就是「fundamental 只做两件事」的全部内容：它让 `Box<本地类型>` 仍算本地类型，但它不允许 `Box<T>` 去覆盖 `T`。`&`、`&mut`、`Pin` 同理。

> [!WARNING]
> 「trait 本地」这条不要求 `Self` 类型也本地，但你依旧受重叠判定约束。给外部类型实现本地 trait 完全合法（第 12 章的扩展 trait 就是这么写的；newtype 的家在第 9 章），只要不和别人已经写下的实现撞车。

> [!WARNING]
> 下面四个小节的特性全部**只在 nightly 工具链上可用**，语法与行为随时可能改，别写进生产依赖：`negative_impls`（追踪 #68318）、`specialization` / `min_specialization`（#31844）、`adt_const_params`（#95174）、`generic_const_exprs`（#76560）。标 `ignore` 的示例只作形状说明，本机（只装 stable 1.98）**没有**编译过；每个小节里标 `compile_fail` 的块则是**在 stable 上**踩到 feature gate 的真实报错。想自己动手验证得先装 nightly（`rustup toolchain install nightly`，临时用 `cargo +nightly build`），工具链的分层见第 1 章。

### nightly：negative impls（追踪 #68318）

语法形状（unstable book 的示例）：

```rust,ignore
#![feature(negative_impls)]

trait DerefMut {}
impl<T: ?Sized> !DerefMut for &T {}
```

negative impl 有三条规则（unstable book 逐条列出，这里照译）：

- 里面不写任何条目（没有方法，也没有关联类型）；
- 它和正向 impl 一样必须遵守孤儿规则；
- 它不能与任何正向 impl「重叠」。

还有两条容易漏掉的后果：

- **它是语义版本承诺。** 删掉一条 negative impl 属于破坏性变更，unstable book 的原话是「It is a breaking change to remove a negative impl.」
- **对 auto trait，它同时关掉自动推导。** 写 `impl !SomeAutoTrait for T` 有两重作用：声明 `T` 永不实现它，并禁止编译器按字段自动推导出这条实现。

> [!NOTE]
> unstable book 还记着一个表达能力上的缺口：目前没有办法表达「这个类型**暂时**不实现某个 auto trait，但将来可能实现」。对普通类型不写 impl 就够了，对 auto trait 却不行——自动推导总会发生。官方给的变通就是嵌一个 marker 字段，代价是将来想让它实现时得动类型定义。

标准库今天在大量使用它（下面这些就在 1.98 自带的 std 源码里，路径以 `library/` 开头）：

```rust,ignore
// library/core/src/marker.rs
impl<T: PointeeSized> !Send for *const T {}
impl<T: PointeeSized> !Send for *mut T {}

// library/alloc/src/rc.rs
impl<T: ?Sized, A: Allocator> !Send for Rc<T, A> {}

// library/core/src/ops/deref.rs —— 为修 Pin 的健全性而加，见 #66544
impl<T: ?Sized> !DerefMut for &T {}
```

stable 上写它会得到（以 Rust 1.98 为例）：

```rust,compile_fail
struct Handle;

impl !Send for Handle {}

fn main() {}
```

```text
error[E0658]: negative impls are experimental
  |
  | impl !Send for Handle {}
  |      ^^^^^
  |
  = note: see issue #68318 <https://github.com/rust-lang/rust/issues/68318> for more information
  = help: use marker types for now
```

编译器给的 `help` 就是官方替代方案：**marker 类型**。想让类型不实现某个 auto trait，只要塞一个本身不实现它的零尺寸字段：

```rust,compile_fail
use std::marker::PhantomData;
use std::rc::Rc;

struct Handle {
    _not_send: PhantomData<Rc<()>>,
}

fn assert_send<T: Send>(t: T) -> T {
    t
}

fn main() {
    let h = Handle { _not_send: PhantomData };
    let h = assert_send(h);
    let _ = h;
}
```

```text
error[E0277]: `Rc<()>` cannot be sent between threads safely
  |
  |     let h = assert_send(h);
  |             ----------- ^ `Rc<()>` cannot be sent between threads safely
  |             |
  |             required by a bound introduced by this call
  |
  = help: within `Handle`, the trait `Send` is not implemented for `Rc<()>`
```

`Rc<()>` 既不是 `Send` 也不是 `Sync`，所以 `PhantomData<Rc<()>>` 一次挡住两个；要挡 `!Unpin` 就用 `PhantomPinned`（第 22 章）。`PhantomData<*const ()>` 也常用，作用相同。

> [!NOTE]
> marker 类型只影响 **auto trait**（`Send`/`Sync`/`Unpin`/`UnwindSafe` 这一族）。想声明「这个类型永远不实现 `DerefMut`」这类普通 trait，stable 上做不到，只能靠不写实现加文档说明。

### nightly：specialization 与 min_specialization（追踪 #31844）

「有 blanket impl 兜底，再给具体类型一份更快的实现」——这在 stable 上会直接撞 E0119（见「报错现场」最后一例）。specialization 要做的就是允许这种重叠，前提是能判定谁「更具体」。RFC 1210 里 `Extend` 的例子正是这个形态：

```rust,ignore
// RFC 1210 的 Extend 例子（节选；`...` 是原文里的省略）
pub trait Extend<A, T: IntoIterator<Item = A>> {
    fn extend(&mut self, iterable: T);
}

// 通用实现：`default` 限定符表示它可以被更具体的实现覆盖
impl<A, T> Extend<A, T> for Vec<A>
where
    T: IntoIterator<Item = A>,
{
    default fn extend(&mut self, iterable: T) {
        // ... 逐个 push
    }
}

// 更具体的实现：切片走批量拷贝的快路径
impl<'a, A> Extend<A, &'a [A]> for Vec<A> {
    fn extend(&mut self, iterable: &'a [A]) {
        // ... 快路径
    }
}
```

这个 RFC 还把这条特性记为「Rust Issue: rust-lang/rust#31844」——正是今天的追踪 issue 号。stable 上写 `default fn` 会得到（以 Rust 1.98 为例）：

```rust,compile_fail
trait Describe {
    fn describe(&self) -> String;
}

impl<T> Describe for T {
    default fn describe(&self) -> String {
        String::from("something")
    }
}

fn main() {}
```

```text
error[E0658]: specialization is experimental
  |
  | /     default fn describe(&self) -> String {
  | |         String::from("something")
  | |     }
  | |_____^
  |
  = note: see issue #31844 <https://github.com/rust-lang/rust/issues/31844> for more information
```

`min_specialization` 是同一个 issue 下的保守子集。标准库确实在用：1.98 的 `library/core/src/lib.rs` 与 `library/alloc/src/lib.rs` 里都开着这条 feature，`library/core/src/array/mod.rs` 之类的文件里能读到 `default fn`。

stable 上的替代方案按「谁决定走哪条路」分两类：

1. **类型决定不了，就搬到运行期。** 用 `std::any::Any` 做 downcast：代价是一次运行期类型检查，而且要求 `T: 'static`。「动手看看」里有一份可跑的版本。
2. **把「特例」从类型上报到调用点。** 显式传入策略参数、包一层 newtype，或用 marker trait 拆成两条 impl，各自只覆盖自己的类型集合。

### nightly：adt_const_params（追踪 #95174）

第 15 章讲过，stable 的 const 参数只收整数、`bool`、`char`。`adt_const_params` 放开自定义类型，前提是实现 `ConstParamTy`（由 derive 生成）：

```rust,ignore
// unstable book 的示例
#![feature(adt_const_params)]
#![allow(incomplete_features)]

use std::marker::ConstParamTy;

#[derive(ConstParamTy, PartialEq, Eq)]
enum Foo {
    A,
    B,
    C,
}

#[derive(ConstParamTy, PartialEq, Eq)]
struct Bar {
    flag: bool,
}

fn is_foo_a_and_bar_true<const F: Foo, const B: Bar>() -> bool {
    match (F, B.flag) {
        (Foo::A, true) => true,
        _ => false,
    }
}
```

注意 `ConstParamTy` 本身也不稳定，同样受这条 feature 控制。stable 上写枚举 const 参数会得到（以 Rust 1.98 为例）：

```rust,compile_fail
#[derive(PartialEq, Eq)]
enum Mode {
    Fast,
    Safe,
}

fn describe<const M: Mode>() -> &'static str {
    "?"
}

fn main() {}
```

```text
error: `Mode` is forbidden as the type of a const generic parameter
  |
  | fn describe<const M: Mode>() -> &'static str {
  |                      ^^^^
  |
  = note: the only supported types are integers, `bool`, and `char`
```

stable 替代有两条路，都很常用：把模式**编码成整数**再做编译期断言；或者把模式**提升成类型参数**，用 marker 类型加关联常量表达。两份都可跑的代码在下一节。

### nightly：generic_const_exprs（追踪 #76560）

unstable book 的一句话定义是：「Allows non-trivial generic constants which have to be shown to successfully evaluate to a value by being part of an item signature.」它给的示例里那个 `where` 从句是关键（下面是节选）：

```rust,ignore
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

// 让 `N + 1` 参与到类型里
struct Foo<const N: usize>
where
    [(); N + 1]:,
{
    array: [usize; N + 1],
}
```

`where [(); N + 1]:` 不是惯用法，而是「请编译器去证明这个表达式能求值」的开关。stable 上直接写算术则是（以 Rust 1.98 为例）：

```rust,compile_fail
fn push<T, const N: usize>(a: [T; N], v: T) -> [T; N + 1] {
    let _ = (a, v);
    todo!()
}

fn main() {}
```

```text
error: generic parameters may not be used in const operations
  |
  | fn push<T, const N: usize>(a: [T; N], v: T) -> [T; N + 1] {
  |                                                        ^ cannot perform const operation using `N`
  |
  = help: const parameters may only be used as standalone arguments here, i.e. `N`
```

stable 替代是老办法：**用宏把每个需要的长度展开成一组具体 impl**。`arrayvec`、`heapless` 这类 crate 长期就这么写。「动手看看」里有一份不用任何外部依赖的版本。

## 动手看看

第 1 组例子演示孤儿规则放行的两种写法。

```rust
{{#rustdoc_include ../snippets/ch36/orphan_ok.rs}}
```

```console
$ cd snippets/ch36
$ rustc --edition 2024 orphan_ok.rs -o orphan_ok.exe
$ .\orphan_ok.exe
(1, 2)
空向量长度 = 0
```

第二组对照的是 fundamental：同一个外部 trait（`Index`），`Vec<Point>` 写不了，`Box<Point>` 可以。

```rust
{{#rustdoc_include ../snippets/ch36/fundamental_box.rs}}
```

```console
$ rustc --edition 2024 fundamental_box.rs -o fundamental_box.exe
$ .\fundamental_box.exe
p[0] = 7
```

第三组是 generic_const_exprs 的 stable 替代：宏把 `1 => 2`、`2 => 3`、`3 => 4` 逐条展开。想要更多长度，就在宏调用里继续加。

```rust
{{#rustdoc_include ../snippets/ch36/macro_push.rs}}
```

```console
$ rustc --edition 2024 macro_push.rs -o macro_push.exe
$ .\macro_push.exe
[1, 2, 3]
```

第四组是 specialization 的运行期替代，注意 `T: 'static` 这个额外要求。

```rust
{{#rustdoc_include ../snippets/ch36/any_downcast.rs}}
```

```console
$ rustc --edition 2024 any_downcast.rs -o any_downcast.exe
$ .\any_downcast.exe
其他类型
String，3 字节
```

最后两份是 adt_const_params 的 stable 替代。先看整数编码加编译期断言：`describe::<2>()` 会在编译期直接失败。

```rust
{{#rustdoc_include ../snippets/ch36/mode_u8.rs}}
```

```console
$ rustc --edition 2024 mode_u8.rs -o mode_u8.exe
$ .\mode_u8.exe
fast
safe
```

再看把模式提升成类型参数：marker 类型没有任何运行时开销，代价是模式必须在编译期写死。

```rust
{{#rustdoc_include ../snippets/ch36/mode_marker.rs}}
```

```console
$ rustc --edition 2024 mode_marker.rs -o mode_marker.exe
$ .\mode_marker.exe
fast
safe
```

## 报错现场

下面三组报错都在 Rust 1.98 上实测，只保留关键行，去掉文件路径与行号。

### E0117：本地类型藏在非 fundamental 的外来类型里

```rust,compile_fail
struct Point;

impl std::fmt::Display for Vec<Point> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} 个点", self.len())
    }
}

fn main() {}
```

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
  |
  | impl std::fmt::Display for Vec<Point> {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^----------
  |                            |
  |                            `Vec` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
  = note: for more information see https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
  = note: define and implement a trait or new type instead
```

**定位路径**：`Point` 是本地类型没错，但 `T0 = Vec<Point>` 整个不满足「本地类型」——判断对象是 `T0` 本身，不是它内部的实参。`Vec` 也不是 fundamental 构造器，所以没有豁免。

**修法**：给 `Point` 包一层本地类型（newtype），或把 impl 写在自己拥有的类型上。

```rust
struct Point;

// 本地类型拥有全部行为，Display 实现写在它身上
struct Points(Vec<Point>);

impl std::fmt::Display for Points {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} 个点", self.0.len())
    }
}

fn main() {
    println!("{}", Points(vec![Point]));
}
```

### E0210：类型参数没有被本地类型覆盖

第一种形态——整个 impl 里根本没有本地类型，只有裸类型参数：

```rust,compile_fail
impl<T> std::fmt::Display for T {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "?")
    }
}

fn main() {}
```

```text
error[E0210]: type parameter `T` must be used as an argument to some local type (e.g., `MyStruct<T>`)
  |
  | impl<T> std::fmt::Display for T {
  |      ^ uncovered type parameter
  |
  = note: implementing a foreign trait is only possible if at least one of the types for which it is implemented is local
  = note: only traits defined in the current crate can be implemented for a type parameter
```

第二种形态——本地类型有，但它排在未覆盖参数**后面**：

```rust,compile_fail
struct Point;

impl<T> From<Point> for Box<T> {
    fn from(_: Point) -> Self {
        unimplemented!()
    }
}

fn main() {}
```

```text
error[E0210]: type parameter `T` must be covered by another type when it appears before the first local type (`Point`)
  |
  | impl<T> From<Point> for Box<T> {
  |      ^ uncovered type parameter
  |
  = note: implementing a foreign trait is only possible if at least one of the types for which it is implemented is local,
          and no uncovered type parameters appear before that first local type
  = note: in this case, 'before' refers to the following order: `impl<..> ForeignTrait<T1, ..., Tn> for T0`,
          where `T0` is the first and `Tn` is the last
```

**定位路径**：两段 `note` 已经把顺序讲明了——扫描从 `T0` 开始。第二条例子里的 `T1 = Point` 确实是本地类型，但 `T0 = Box<T>` 排得更前，而 `Box` 是 fundamental，**不能**覆盖 `T`。把 `Box` 换成 `Vec`，`T` 被 `Vec<T>` 覆盖，规则就满足了（这正是「动手看看」里 `impl<T> From<Crate> for Vec<T>` 能编译的原因）。

顺带说清 E0210 与 E0117 的分工：头部存在**未覆盖**的类型参数时报 E0210，没有本地类型、却也没有未覆盖参数时报 E0117。所以 `impl<T> From<Point> for Box<T>`（`Box` 盖不住 `T`）与 `impl<T> Display for T` 都是 E0210，而 `impl Display for Vec<Point>` 与 `impl<T> Display for Vec<T>`（`T` 被 `Vec` 覆盖）都是 E0117。

**修法**：让本地类型出现在 `T0` 位置，或让参数被非 fundamental 的构造器覆盖。想让 `Box<T>` 这种写法还能用，就只能等 specialization 稳定——或者把行为挪到本地 trait 上（本地 trait 不受第 2、3 条约束）。

### E0119：两条 impl 重叠，哪怕 trait 是你的

```rust,compile_fail
trait Show {
    fn show(&self) -> String;
}

impl<T> Show for T {
    fn show(&self) -> String {
        String::from("其它")
    }
}

impl<T> Show for Vec<T> {
    fn show(&self) -> String {
        format!("{} 个元素", self.len())
    }
}

fn main() {}
```

```text
error[E0119]: conflicting implementations of trait `Show` for type `Vec<_>`
  |
  | impl<T> Show for T {
  | ------------------ first implementation here
  |
  | impl<T> Show for Vec<T> {
  | ^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Vec<_>`
```

**定位路径**：孤儿规则这一关过了（`Show` 本地），但重叠判定不通过——`Vec<T>` 是 `T` 的一个具体实例，两条 impl 的实例集合交集非空。另一类常见来源是外部 crate：错误信息里会写「conflicting implementation in crate `core`」，那说明你和标准库的某条 impl 撞上了，比如自己写 `impl<T> From<T> for MyType` 会与 core 的 `impl<T> From<T> for T` 冲突。

**修法**：三条路。一是去掉 blanket impl，只保留具体类型的实现；二是收窄 blanket impl 的适用范围，让它与具体实现**不相交**（只把边界从 `Debug` 换成别的更窄的 bound 往往还不够，通常要引入一个只有目标类型实现的 marker trait）；三是等 specialization 稳定后用 `default fn` 表达「默认实现 + 特例实现」。

## 小结

- **一致性（coherence）由两个检查口组成**：跨 crate 的孤儿规则，和同一对组合的重叠判定。两者都以 `impl` **头部**为判断对象，与方法体无关。
- **孤儿规则的形式化版本只有三条**：trait 本地 → 通过；否则按 `T0 → T1 → ... → Tn` 顺序找第一个本地类型 `Ti`，再检查它前面那一段窗口。错误码分工：头部存在**未覆盖**的类型参数 → E0210；没有本地类型、却也没有未覆盖参数 → E0117。
- **本地类型的判定不看实参**：`Vec<Local>` 不是本地类型，`Local<Foreign>` 是。**覆盖的判定相反**：`Vec<T>` 里的 `T` 被覆盖，裸 `T` 未覆盖。
- **fundamental 构造器（`&`、`&mut`、`Box`、`Pin`）只做两件事**：让 `Box<本地类型>` 仍算本地类型；禁止自己覆盖内部参数。
- **negative impls 是 nightly**（#68318），stable 上的替代是 marker 类型（`PhantomData<Rc<()>>`、`PhantomPinned` 等），但它只能影响 auto trait。
- **specialization / min_specialization**（#31844）、**adt_const_params**（#95174）、**generic_const_exprs**（#76560）都还在 nightly。stable 上分别用运行期 downcast、整数编码或 marker 类型、宏展开来绕过。

## 练习

1. 判断下面四条 `impl` 在 stable 1.98 上的结果（通过 / 哪种错误码），并说明理由。`Point` 是本地类型，`MyTrait` 是本地 trait，`Index`、`From` 来自标准库。
   a. `impl<T> MyTrait for Vec<T>`
   b. `impl<T> From<T> for Point`
   c. `impl<T> From<Vec<T>> for Point`
   d. `impl Index<usize> for &Point`
2. 库作者给自己的 trait 事后补一条 blanket impl（如 `impl<T: Display> MyTrait for T`）时，下游可能因为什么突然编译不过？下游在什么情况下不受影响？反过来，下游能对标准库的 trait 写 blanket impl（如 `impl<T> std::fmt::Display for T`）吗？
3. 在 stable 上实现一个 `describe`：对 `String` 返回「String，N 字节」，对其它类型返回「其他类型」。要求不用 nightly。

<details><summary>参考答案</summary>

**第 1 题**

- **a. 通过。** `MyTrait` 是本地 trait，命中孤儿规则第 1 条，类型参数怎么写都无所谓。
- **b. E0119。** 孤儿规则本身通过了（`T0 = Point` 是本地类型，它前面没有任何类型），但 `core` 里已经有 `impl<T> From<T> for T`，当 `T = Point` 时两条 impl 命中同一对组合。错误信息里会写「conflicting implementation in crate `core`」。
- **c. 通过。** `T0 = Point` 是本地类型，排在第一位，前面没有未覆盖参数；`T` 被 `Vec` 覆盖，不构成未覆盖参数。
- **d. 通过。** `&` 是 fundamental 构造器，所以 `&Point` 算本地类型，`T0` 这一项直接满足。

**第 2 题**

先看方向性。下游能不能写 blanket impl，取决于 `Self` 与 trait 的类型参数是否落在自己的 crate 里。对标准库的 trait 写 `impl<T> Display for T`：trait 是外部的，`T` 又是裸类型参数，两边都不是你的，规则直接拒绝（E0210，本章实测）。trait 的所有者则不同——trait 是它的，写 blanket impl 天然通过孤儿规则。

但**通过孤儿规则不等于兼容**。下游有权为自己的类型实现你的 trait（类型是它的），于是：

- 下游类型**满足** blanket impl 的 bound 时会被撞上。实测（两个 crate）：`dep` 起初只导出 `pub trait MyTrait {}`，下游写 `struct Local; impl dep::MyTrait for Local {}` 编译通过；`dep` 后来加上 `impl<T: Display> MyTrait for T {}`（`Local` 实现了 `Display`），下游立刻报错：

```text
error[E0119]: conflicting implementations of trait `MyTrait` for type `Local`
  |
  | impl dep::MyTrait for Local {}
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: conflicting implementation in crate `dep`:
          - impl<T> MyTrait for T
            where T: std::fmt::Display;
```

- 下游类型**不满足** bound 时不受影响：两条 impl 的实例集合不相交（实测把 `Local` 换成没有 `Display` 的类型，编译通过）。

所以「加 blanket impl」属于**破坏性变更**：要么走大版本，要么另起一个 trait。孤儿规则给的是「谁能写」的许可，不是「不会破坏兼容」的保证。

**第 3 题**（思路，两种写法都可接受）

运行期 downcast 版（完整可跑的版本在「动手看看」里）：

```rust
use std::any::Any;

trait Describe {
    fn describe(&self) -> String;
}

impl<T: Any> Describe for T {
    fn describe(&self) -> String {
        match (self as &dyn Any).downcast_ref::<String>() {
            Some(s) => format!("String，{} 字节", s.len()),
            None => String::from("其他类型"),
        }
    }
}

fn main() {
    println!("{}", 42.describe());
    println!("{}", String::from("hey").describe());
}
```

另一种做法是不用 blanket impl：给需要快路径的类型定义一个空 marker trait（`trait FastDescribe {}` 配 `impl FastDescribe for String {}`），写 `impl<T: FastDescribe> Describe for T` 走快路径，其余类型逐个显式实现（必要时用宏展开）。它没有运行期判断的成本，但覆盖面是「你点过名的类型」，而不是「所有类型」。

</details>

## 延伸阅读

- The Rust Reference：[Trait implementation coherence](https://doc.rust-lang.org/reference/items/implementations.html#trait-implementation-coherence)（孤儿规则、重叠、fundamental 的正文位置）
- The Rust Reference：[Glossary](https://doc.rust-lang.org/reference/glossary.html#fundamental-type-constructors)（local type / uncovered type / fundamental type constructor 的准确定义）
- The Rust Reference：[Auto traits](https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits)（negative impl 与自动推导的交互）
- RFC 2451 [Re-rebalancing Coherence](https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html)、RFC 1023 [Rebalancing Coherence](https://github.com/rust-lang/rfcs/blob/master/text/1023-rebalancing-coherence.md)（现行孤儿规则与 fundamental 的设计来源）
- RFC 1210 [Impl Specialization](https://rust-lang.github.io/rfcs/1210-impl-specialization.html)
- The Unstable Book：[negative_impls](https://doc.rust-lang.org/unstable-book/language-features/negative-impls.html)、[specialization](https://doc.rust-lang.org/unstable-book/language-features/specialization.html)、[min_specialization](https://doc.rust-lang.org/unstable-book/language-features/min-specialization.html)、[adt_const_params](https://doc.rust-lang.org/unstable-book/language-features/adt-const-params.html)、[generic_const_exprs](https://doc.rust-lang.org/unstable-book/language-features/generic-const-exprs.html)
- 追踪 issue：[#31844](https://github.com/rust-lang/rust/issues/31844)（specialization / min_specialization）、[#68318](https://github.com/rust-lang/rust/issues/68318)（negative impls）、[#95174](https://github.com/rust-lang/rust/issues/95174)（adt_const_params）、[#76560](https://github.com/rust-lang/rust/issues/76560)（generic_const_exprs）
- 错误码正文：[E0117](https://doc.rust-lang.org/error_codes/E0117.html)、[E0210](https://doc.rust-lang.org/error_codes/E0210.html)、[E0119](https://doc.rust-lang.org/error_codes/E0119.html)
- 标准库文档：[`std::any::Any`](https://doc.rust-lang.org/std/any/trait.Any.html)、[`std::marker::PhantomData`](https://doc.rust-lang.org/std/marker/struct.PhantomData.html)、[`std::marker::PhantomPinned`](https://doc.rust-lang.org/std/marker/struct.PhantomPinned.html)
