# 第 24 章 指针、provenance 与未初始化内存

## 这一章解决什么问题

第 23 章把 unsafe 的五项能力和未定义行为（UB）清单摆上了桌。这一章处理清单里最容易出事的两个角落：裸指针，以及「内存还没写进去」的那段时间。

先纠正一个很普遍的心智模型：**裸指针不只是一串地址**。它是「地址 + 来历（provenance）」——这个地址属于哪块分配、是从哪条引用派生出来的。把指针转成整数再转回来，地址可能分毫不差，但来历丢了；拿这种指针去读写就是 UB。Rust 没有禁止「指针 ↔ 整数」的转换，它要求你把「我在这里丢弃了来历」或者「我要暴露来历」明确写出来。

另一半是未初始化（uninitialized）内存。`bool` 必须是 0 或 1，`&T` 必须非空、对齐——类型系统假设值总是有效的。可现实里经常需要「先划一块地方，晚点再放东西」，比如自己实现 `Vec`、给 FFI 传 out-pointer。`MaybeUninit` 和 `ManuallyDrop` 就是为这段时间准备的工具：它们让你能安全地表达「这里还没有有效值」，代价是「到底初始化了没有」这件事由你自己维护。

读完这一章，你应该能够：

- 在裸指针和 `NonNull` 之间做有依据的取舍；
- 用严格来历（strict provenance）那一组 API 说清「我只是要地址」还是「我要保留来历」；
- 用 `&raw const`/`&raw mut` 造裸指针，而不先造出一个引用；
- 用 `MaybeUninit` 写延迟初始化，用 `ManuallyDrop` 控制析构时机，并且知道各自会漏在哪；
- 对别名模型（aliasing model）有个能用的直觉，知道 Miri 到底在替你检查什么。

字段布局与 `repr` 留给第 25 章，跨语言 ABI 留给第 26 章，别名模型的形式化留给第 35 章。unsafe 的五项能力和 Miri 的用法已经在第 23 章交代过，本章只回指，不重复。

## 为什么这样设计

### 优化器必须先知道「谁在动这块内存」

别名规则不是为了给程序员添麻烦，而是为了让优化器敢下判断。Nomicon 的 `aliasing` 一章给了最小例子（大意）：`fn compute(input: &u32, output: &mut u32)`。如果允许 `input` 和 `output` 重叠（有人写 `compute(&x, &mut x)`），那「把 `*input` 缓存进寄存器、在 `> 10` 的分支里直接把 `output` 赋成 2」这种优化就不成立；只有两个借用保证不重叠时它才保序。

于是有了「别名规则」（术语沿用 `GLOSSARY.md`）：同一时刻，共享借用可以有多个，可变借用只能有一个。裸指针之所以特殊，是因为它**不吃**这条规则的保证：`*mut T` 存在期间，编译器不能假定「不会有别人碰这块内存」。

### 为什么要给指针带上「来历」

机器上的指针就是整数。那为什么语言还非要强调「来历」？因为别名规则关心的不是「地址相等吗」，而是**派生关系**。两个地址相同、来历不同的指针，在这个模型里是不同的东西。

设想你把 `&arr[1]` 转成整数存起来，等数组释放之后，另一块大小完全不同的分配恰好落在同一个地址上。你凭那个整数取回指针——地址没错，但它声称自己能访问原先那块分配的范围。编译器有权按「这两件事无关」来优化，于是程序在语义上已经坏了，尽管它「跑起来看着正常」。

严格来历就是把这件事摆到台面上的一组 API。std 文档把它们分成两类标签：`addr`/`with_addr`/`map_addr` 标的是 "This is a Strict Provenance API."，`expose_provenance`/`with_exposed_provenance` 标的是 "This is an Exposed Provenance API."。两类都不需要 `unsafe`，区别在于前者不碰来历、后者会「暴露」来历。

一个直觉说法：

| 你想做的事 | 该用 | 代价 |
|---|---|---|
| 只想读地址（比较、打印、算哈希） | `addr()` | 不暴露来历，最保守 |
| 换地址但保留来历（加偏移、按对齐取整） | `with_addr` / `map_addr` | 无 |
| 真的要跟整数世界打交道（FFI、序列化） | `expose_provenance` + `with_exposed_provenance` | 「尽力而为」：文档说这类指针可能不被检查工具支持 |
| 只要一个「高位编号」式的地址 | `without_provenance` | 没有来历，不能解引用 |

> [!NOTE]
> 本章所有 API 与版本号都在本机 1.98 文档快照（`std/ptr`、`releases.md`）上逐条核对过。示例在 stable 1.98 上实跑；标 **nightly** 的内容在 stable 上不可用。本机只装了 stable 工具链，所以本章**不含 Miri 实测输出**：涉及别名的结论出自官方文档与模型规则，Miri 的用法见第 23 章。

## 机制与规则

### 裸指针与 `NonNull`

`*const T` 和 `*mut T` 的规则很简单：可以存在于任何地方、指向任何地址（包括空和悬垂），但**读写必须进 `unsafe`**（第 23 章）。它们既不带生命周期，也不实现 `Send`/`Sync`。

`NonNull<T>` 是「非空裸指针」，它比 `*mut T` 多了三件事：

1. **永不为空**——即使从不解引用。这样枚举就能拿这个被禁用的值当判别式，于是 `Option<NonNull<T>>` 与 `*mut T` 同宽。
2. **对 `T` 协变（covariant）**。
3. 有 `from_ref`/`from_mut` 这类安全构造器，不必先落到裸指针再 `unsafe` 包回来。

第 1 条是实打实的内存收益，跑一下就知道：

```text
Option<NonNull<Node>> = 8 字节
Option<*mut Node>     = 16 字节
```

（`Node` 是一个普通的结构体；上面的数字来自本章 `snippets/ch24/nonnull_list.rs` 的真实输出，编译目标是 64 位。）

取舍规则可以照抄文档的原话：官方文档在 `NonNull` 页面上直接写 "If you’re not sure if you should use `NonNull<T>`, just use `*mut T`!"。它同时提醒：`NonNull` "is ultimately more dangerous to use because of its additional properties"。多出来的「危险」指的是上面第 2 条的协变：如果你通过 `NonNull<T>` 暴露了修改 `T` 的途径，而 `T` 里可能有更短生命周期的引用，协变就会带来不健全——这时要人为加一个不变字段（文档建议 `PhantomData<Cell<T>>` 或 `PhantomData<&'a mut T>`）。

两点容易忽略的事实：

- 两者都不实现 `Send`/`Sync`。文档给 `NonNull` 的理由是 "NonNull pointers are not `Send` because the data they reference may be aliased."。要跨线程传，得自己在 `unsafe impl` 里承诺独占（第 19 章的 `Send`/`Sync` 那一套）。
- 类型不同，编译器不会替你转。把 `*mut u8` 递给要 `NonNull<u8>` 的函数，上来就是一记类型错误：

```rust,compile_fail
use std::ptr::NonNull;

fn take(p: NonNull<u8>) -> u8 {
    unsafe { *p.as_ptr() }
}

fn main() {
    let mut x = 1u8;
    let raw: *mut u8 = &raw mut x;
    println!("{}", take(raw));
}
```

```text
error[E0308]: mismatched types
  --> src/main.rs:10:25
   |
10 |     println!("{}", take(raw));
   |                    ---- ^^^ expected `NonNull<u8>`, found `*mut u8`
   |                    |
   |                    arguments to this function are incorrect
   |
   = note:   expected struct `NonNull<u8>`
           found raw pointer `*mut u8`
note: function defined here
  --> src/main.rs:3:4
   |
 3 | fn take(p: NonNull<u8>) -> u8 {
   |    ^^^^ --------------
```

修法就是让「非空」这件事在类型层面成立：`NonNull::new(raw)` 返回 `Option`，你自己决定是 `unwrap` 还是走别的分支；如果调用点已经知道非空，`NonNull::from_mut(&mut x)` 更干净。

### 指针上的 provenance API

这是这一章变化最快的一块，所以下面按「稳定的」和「还是 nightly 的」分开列。版本号来自本机 1.98 文档与 `releases.md`。

先把四种「指针形态」摆在一起：地址能不能对上是一回事，能不能拿来用是另一回事。

<svg viewBox="0 0 720 252" width="100%" role="img" aria-label="四种指针形态的地址与来历对照：*mut T 与 NonNull&lt;T&gt; 既有地址又有来历（NonNull 保证非空），usize 与 without_provenance 只有地址、没有来历，因此不能解引用——地址对不等于可用" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">四种「指针形态」：地址、来历、能不能解引用</text>
<text x="300" y="46" font-size="13" fill="currentColor" text-anchor="middle">有地址？</text>
<text x="468" y="46" font-size="13" fill="currentColor" text-anchor="middle">有来历？</text>
<text x="636" y="46" font-size="13" fill="currentColor" text-anchor="middle">能解引用？</text>
<g stroke="var(--quote-border)" stroke-width="1.5">
<line x1="216" y1="54" x2="216" y2="190" />
<line x1="384" y1="54" x2="384" y2="190" />
<line x1="552" y1="54" x2="552" y2="190" />
<line x1="0" y1="88" x2="720" y2="88" />
<line x1="0" y1="122" x2="720" y2="122" />
<line x1="0" y1="156" x2="720" y2="156" />
</g>
<line x1="0" y1="54" x2="720" y2="54" stroke="currentColor" stroke-width="1.5" />
<line x1="0" y1="190" x2="720" y2="190" stroke="currentColor" stroke-width="1.5" />
<text x="8" y="75" font-size="12" font-family="monospace" fill="currentColor">*mut T</text>
<text x="8" y="109" font-size="12" font-family="monospace" fill="currentColor">NonNull&lt;T&gt;</text>
<text x="8" y="143" font-size="12" font-family="monospace" fill="currentColor">usize</text>
<text x="8" y="177" font-size="12" font-family="monospace" fill="currentColor">without_provenance(addr)</text>
<text x="300" y="75" font-size="12" fill="currentColor" text-anchor="middle">有（可空、可悬垂）</text>
<text x="300" y="109" font-size="12" fill="currentColor" text-anchor="middle">有（保证非空）</text>
<text x="300" y="143" font-size="12" fill="currentColor" text-anchor="middle">有（就是地址本身）</text>
<text x="300" y="177" font-size="12" fill="currentColor" text-anchor="middle">有（你给的数值）</text>
<text x="468" y="75" font-size="12" fill="var(--links)" text-anchor="middle">有（派生链决定）</text>
<text x="468" y="109" font-size="12" fill="var(--links)" text-anchor="middle">有</text>
<text x="468" y="143" font-size="12" fill="var(--warning-border)" text-anchor="middle">无</text>
<text x="468" y="177" font-size="12" fill="var(--warning-border)" text-anchor="middle">无</text>
<text x="636" y="75" font-size="12" fill="currentColor" text-anchor="middle">能（unsafe）</text>
<text x="636" y="109" font-size="12" fill="currentColor" text-anchor="middle">能（unsafe）</text>
<text x="636" y="143" font-size="12" fill="var(--warning-border)" text-anchor="middle">不能（不是指针）</text>
<text x="636" y="177" font-size="12" fill="var(--warning-border)" text-anchor="middle">不能（非零大小 UB）</text>
<text x="0" y="216" font-size="12" fill="currentColor">地址都能对上，不等于能用：解引用要有来历，且这次访问得落在来历覆盖的范围内。</text>
<text x="0" y="236" font-size="12" fill="currentColor" opacity="0.7">usize 转回指针要显式认领来历（with_exposed_provenance）；without_provenance 不认领，只许对齐的零大小访问。</text>
</svg>

**稳定面（1.98 上可用，无需 `#![feature]`）**

| API | 语义 | 稳定版本 |
|---|---|---|
| `p.addr()` | 取地址，**丢弃且不暴露**来历 | 1.84 |
| `p.with_addr(a)` | 换地址、保留来历（等价于 `wrapping_offset`） | 1.84 |
| `p.map_addr(f)` | `with_addr` 的闭包版，方便做位运算 | 1.84 |
| `p.expose_provenance()` | 暴露来历，返回地址 | 1.84 |
| `ptr::with_exposed_provenance(addr)` / `_mut` | 凭地址取回一个带「已暴露」来历的指针 | 1.84（`const` 化于 1.91） |
| `ptr::without_provenance(addr)` / `_mut` | 造一个**没有**来历的指针 | 1.84（`const` 化于 1.84） |
| `NonNull::addr` / `with_addr` / `map_addr` | 同上，参数与返回值是 `NonZero<usize>` | 1.84 |
| `NonNull::expose_provenance` / `with_exposed_provenance` / `without_provenance` | 同上 | 1.89 |
| `NonNull::from_ref` / `from_mut` | `&T`/`&mut T` → `NonNull<T>` | 1.89 |

三条容易踩的细节：

- **名字在稳定前改过。** nightly 时代这套 API 叫 `expose_addr`/`from_exposed_addr`，稳定时改成了现在这套 `*_provenance` 名字。看到旧文章里的老名字，别照抄（1.98 文档里已经搜不到它们）。
- **const 面各不相同。** `ptr::without_provenance`/`_mut`（1.84 起）、`ptr::with_exposed_provenance`/`_mut`（1.91 起）、`NonNull::without_provenance`（1.89 起）是 `const fn`；而 `addr`/`with_addr`/`map_addr`/`expose_provenance` 与 `NonNull::with_exposed_provenance` 到 1.98 还不是（最后这个的徽标就直接写成 `1.89.0 (const: unstable)`）。想在 `const fn` 里用它们，会拿到 `E0015` 或 "is not yet stable as a const fn"：

```rust,compile_fail
const fn addr_of_null() -> usize {
    core::ptr::null::<u8>().addr()
}

fn main() {
    println!("{}", addr_of_null());
}
```

```text
error[E0015]: cannot call non-const method `std::ptr::const_ptr::<impl *const u8>::addr` in constant functions
 --> src/main.rs:2:29
  |
2 |     core::ptr::null::<u8>().addr()
  |                             ^^^^^^
  |
  = note: calls in constant functions are limited to constant functions, tuple structs and tuple variants
```

- **`without_provenance` 造出来的指针真的什么都不是。** 文档的原话是 "No-provenance pointers are little more than a `usize` address in disguise."，并明确：允许做**零大小**访问（对齐时），做非零大小的访问是 UB。而 `with_exposed_provenance` 是另一个东西——它去认领「之前被暴露过的」来历，但 "The exact provenance that gets picked is not specified."。文档还补了一句更硬的："Due to its inherent ambiguity, `with_exposed_provenance` may not be supported by tools that help you to stay conformant with the Rust memory model. It is recommended to use Strict Provenance APIs such as `with_addr` wherever possible, in which case `addr` should be used instead of `expose_provenance`."——也就是说，Miri 这类检查工具可能不支持它。所以它只留给 FFI、序列化这类「必须跟整数世界握手」的场合。

> [!WARNING]
> 下表这些 API 在 stable 1.98 上不可用：未稳定的库 API 会报 `E0658`，const 形式的未稳定项报 "is not yet stable as a const fn"。要用得先切到 nightly 并加对应的 feature 门。

| API | feature 门 |
|---|---|
| `ptr::from_raw_parts`、`<ptr>::to_raw_parts`、`NonNull::from_raw_parts` | `ptr_metadata` |
| `NonNull::cast_slice`、`cast_array`、`cast_init`、`cast_uninit` | `ptr_cast_slice`、`ptr_cast_array`、`cast_maybe_uninit` |
| `MaybeUninit::array_assume_init`、`MaybeUninit::<[T; N]>::transpose` | `maybe_uninit_array_assume_init`、`maybe_uninit_uninit_array_transpose` |
| `ManuallyDrop::take`/`drop`、`MaybeUninit::assume_init_drop` 与 `NonNull::with_exposed_provenance` 的 **const** 用法 | 对应库特性 |

最后一条值得单独看一眼，因为它的报错措辞很好认：

```rust,compile_fail
use std::mem::ManuallyDrop;

const fn take_it(m: &mut ManuallyDrop<String>) -> String {
    // `take` 本身 1.42 就稳定了，但「在 const fn 里调用」还没稳定。
    unsafe { ManuallyDrop::take(m) }
}

fn main() {
    let mut slot = ManuallyDrop::new(String::from("hi"));
    println!("{}", take_it(&mut slot));
}
```

```text
error: `ManuallyDrop::<T>::take` is not yet stable as a const fn
 --> src/main.rs:5:14
  |
5 |     unsafe { ManuallyDrop::take(m) }
  |              ^^^^^^^^^^^^^^^^^^^^^
```

### `&raw const` 与 `&raw mut`

这两个叫**裸借用运算符（raw borrow operators）**，1.82 稳定（RFC 2582）。它们的操作数按 place 求值，直接产出 `*const T`/`*mut T`，**中间不经过引用**。

Reference 给了必须用它们的三类场合，原文是："The raw borrow operators must be used instead of a borrow operator whenever the place expression could evaluate to a place that is not properly aligned or does not store a valid value as determined by its type, or whenever creating a reference would introduce incorrect aliasing assumptions."——即：一，place 可能未对齐；二，place 里可能没有有效值；三，造引用会引入错误的别名假设。文档举的两个例子分别对应前两条：

- `#[repr(packed)]` 结构体的字段：`&packed.f2` 会造出未对齐引用，`&raw const packed.f2` 不会；
- `MaybeUninit<Demo>` 里还没写进去的字段：文档的注释说 `&uninit.as_mut().field` 会造出指向未初始化 `bool` 的引用，它给的替代写法是 `&raw mut (*uninit.as_mut_ptr()).field` 配 `write`。

顺带一句历史：`addr_of!(x)`/`addr_of_mut!(x)` 这两个宏 1.51 起就有，语义与 `&raw const x`/`&raw mut x` 完全一致。新代码用运算符就行，宏留给还要和旧编译器打交道的地方。

### `MaybeUninit`：把「还没写」写进类型

`MaybeUninit<T>` 的文档定义很短："A wrapper type to construct uninitialized instances of `T`." 它的作用是提供一个**内容可以无效**的容器，从而让「还没有有效值的 T」这件事能存在于类型系统里。

它和 `Option<T>` 的区别不是措辞问题：

- `MaybeUninit<T>` 本身不携带「初始化了没有」的信息，编译器也不知道；
- 它被析构时**不会**析构里面的 `T`；
- 拿出来的唯一入口全是 `unsafe`（或在 `const` 里受限）。

核心安全条件只有一句，逐字如下："It is up to the caller to guarantee that the `MaybeUninit<T>` really is in an initialized state, i.e., a state that is considered “valid” for type `T`. Calling this when the content is not yet fully initialized causes immediate undefined behavior."——注意 "immediate"：不是等你读到才发现，是调用那一刻程序就已经坏了。所以「部分初始化」也不行。

四个出口，按用途分：

| 出口 | 消耗容器？ | 用途 | 稳定版本 |
|---|---|---|---|
| `assume_init()` | 是 | 拿到 `T`，之后走正常析构 | 1.36（const 1.59） |
| `assume_init_read()` | 否（`&self`） | 原地读一份走，原处还在 | 1.60（const 1.75） |
| `assume_init_ref()` / `assume_init_mut()` | 否 | 借用里面的值 | 1.55（const 1.59 / 1.84） |
| `assume_init_drop()` | 否（`&mut self`） | 原地析构 | 1.60（const 仍 nightly） |

`assume_init_read` 是这四种里最容易出错的：它不消耗容器，所以你可以读第二次——两次读出来的 `T` 指向同一份数据，然后两次析构，就是一次重复释放。真要在 `Drop` 里处理「可能还没初始化」的字段，配一个 `bool` 标志（本章 `snippets/ch24/checked_uninit.rs` 就是这么写的），这就是 Nomicon「checked uninit」的套路：初始化状态是运行时事实，类型系统不知道，只能你自己记账。

写值用 `write`：它覆盖旧内容且**不析构**旧的（`1.55` 稳定，`const` 化于 `1.85`）。这也意味着「先 `write` 再 `assume_init`」不会重复析构，但「对已经 `assume_init_read` 过的格子再 `write`」就会泄漏——账还是你的。

`zeroed()` 的坑要单独说。文档写得很直白："It depends on `T` whether that already makes for proper initialization. For example, `MaybeUninit<usize>::zeroed()` is initialized, but `MaybeUninit<&'static i32>::zeroed()` is not because references must not be null."

```rust,ignore
use std::mem::MaybeUninit;

// 下面这些是 UB，写在这里只为说明「全零 ≠ 有效值」。
let ok: usize = unsafe { MaybeUninit::<usize>::zeroed().assume_init() };
let bad: &'static i32 = unsafe { MaybeUninit::<&'static i32>::zeroed().assume_init() };
let worse: bool = unsafe { MaybeUninit::<bool>::zeroed().assume_init() }; // false，恰好合法
let oops: bool = unsafe { MaybeUninit::<bool>::uninit().assume_init() };  // 未初始化 → UB
```

判据是「`T` 的全零位模式是不是一个合法值」：整数、`[u8; N]`、以及所有字段都满足这一条的复合类型可以；引用、`NonZero*`、有 niche 的枚举不行。拿不准就别用 `zeroed`，用 `uninit` 加随后显式 `write`。

还有三个值得记住的点：

- **不要用 `mem::uninitialized`。** 它 1.39 起就是 deprecated，文档直接写着 "use `mem::MaybeUninit` instead"。
- **未初始化内存没有固定值。** 文档专门提醒：同一个未初始化字节读两次可能得到不同结果，所以哪怕类型是 `i32`，把未初始化的数据当成值来用也是 UB。
- **1.93 起有切片的配套方法**：`<[MaybeUninit<T>]>::assume_init_ref`/`assume_init_mut`/`assume_init_drop`，以及 `MaybeUninit<[T; N]>` 与 `[MaybeUninit<T>; N]` 之间的 `From`/`AsRef`/`AsMut`（1.95）。

### `ManuallyDrop`：控制「什么时候析构」

`ManuallyDrop<T>` 的文档定义："A wrapper to inhibit the compiler from automatically calling `T`’s destructor. This wrapper is 0-cost."，以及 "`ManuallyDrop<T>` is guaranteed to have the same layout and bit validity as `T`"。也就是说它是纯粹的「析构控制」：布局、位有效性、优化都不变，只是编译器不再自动帮你调 `T` 的析构函数。

出口分两种：

- **安全**：`ManuallyDrop::new(v)`（1.20，const 1.32）和 `ManuallyDrop::into_inner(slot)`（1.20，const 1.32）。后者把所有权还给你，正常场景优先用它。
- **`unsafe`**：`ManuallyDrop::take(&mut slot)`（1.42）和 `ManuallyDrop::drop(&mut slot)`（1.20）。两者的契约都是「恰好一次」：`take` 拿走值但不改槽位内容，`drop` 原地析构但不清空槽位。调两次就是重复释放。（这两个函数的 const 形式在 1.98 仍是 nightly，见上一小节的报错。）

它的三个经典用途：

1. 类型实现了 `Drop`，但你还需要一个「把某个字段搬出去」的方法——直接搬会被编译器拦住，用 `ManuallyDrop` 包住那个字段即可；
2. 某个字段的析构必须晚于其他清理动作（先关文件再关句柄之类的顺序）；
3. 故意泄漏：`ManuallyDrop` 什么都不做，所以忘了 `take`/`drop` 的后果是泄漏，而不是 UB。

> [!WARNING]
> 把 `ManuallyDrop` 放进结构体或枚举会改变析构语义。`ManuallyDrop` 自己的文档有专门一节讲这件事（"Safety hazards when storing `ManuallyDrop` in a struct or an enum."）。它不是「换个类型」那么轻，写的时候要像维护不变式一样维护它。

### 别名模型的直觉

既然 `&mut T` 承诺独占，那「什么时候算独占」就必须有个可判定的说法。Rust Reference 现行的措辞是：

> Breaking the pointer aliasing rules. The exact aliasing rules are not determined yet, but here is an outline of the general principles:
>
> `&T` must point to memory that is not mutated while they are live (except for data inside an `UnsafeCell<U>`), and `&mut T` must point to memory that is not read or written by any pointer not derived from the reference and that no other reference points to while they are live. `Box<T>` is treated similar to `&'static mut T` for the purpose of these rules.

注意第一句：**规则的确切形式还没定案**。Reference 只给了原则和「存活时长」的几条上界（比如：每次解引用或重借都算活跃，传给函数/从函数返回都算活跃，引用传给函数时至少活跃到该调用结束）。

工程上能用的直觉就三条：

1. `&T` 活跃期间，那块内存不能被改（`UnsafeCell` 除外）；
2. `&mut T` 活跃期间，不能有任何**不是从它派生**的指针去读或写那块内存；
3. 派生关系是链式的：从 `&mut` 拿到的裸指针「记得」它来自谁；一旦中间又出现了别的引用/重借，早先那条链可能就作废了。

违反这三条就是 UB，但**借用检查器管不到裸指针**：`rustc` 只拦得下最露骨的一类——从 `&T` 强转出 `*mut T` 再写，撞的是默认 deny 的 `invalid_reference_casting`——绕一下它就只能放行，因为别名规则是**运行时语义**层面的要求。这正是 Miri 的意义：把候选规则实现成检查器，逐步执行你的程序，遇到违规就报错。（用法见第 23 章；Cargo Book 的 `cargo miri` 手册页原话是 "This command is only available on the nightly channel."。）

下面这段就落在「rustc 放行」那一侧：先取裸指针，再插一个共享借用打断派生链，最后在共享借用还活着时通过裸指针写回去——直接违反第 1 条。它在本机 stable 1.98 上照常编译、照常跑完：

```rust
let mut v = 1u32;
let raw = &raw mut v;
let shared = &v;      // 共享借用插进来，raw 那条链被打断
unsafe { *raw = 2 };  // UB：shared 还活着，却写了它
println!("{shared}"); // 打印 2：看着没错，语义上已经越线
```

（本机只有 stable，未用 Miri 复跑；判定依据是 Reference 的别名原则与两套候选模型的规则。）

现在官方在用的候选模型有两个：**Stacked Borrows** 与 **Tree Borrows**（都在 Miri 里实现；树模型还给了 `-Zmiri-tree-borrows*` 系列选项，Unstable Book 的 `llvm-writable` 一节就提到过其中一个）。它们对上面第三点的处理方式不同，因此有些边界写法会在一者下通过、在另一者下报错。它们是研究模型，不是已定案的规范——形式化、对照与取舍放在第 35 章。

## 动手看看

五个片段都在 `snippets/ch24/`，都是完整可跑的程序，本章输出是真跑真粘的。编译方式统一是：

```console
$ rustc --edition 2024 snippets/ch24/nonnull_list.rs -o nonnull_list.exe
$ .\nonnull_list.exe
```

**一、`NonNull` 支撑的单链表**（`nonnull_list.rs`）。它把「非空」写进了类型：`next` 是 `Option<NonNull<Node>>`，于是空列表与「链尾」用同一个 `None` 表达，而 `Option` 不额外占空间。

```rust
{{#rustdoc_include ../snippets/ch24/nonnull_list.rs}}
```

```text
len = 3
values = [1, 2, 3]
Option<NonNull<Node>> = 8 字节
Option<*mut Node>     = 16 字节
```

代码里有三处值得停留：`NonNull::from_mut` 取指针与 `Box::into_raw` 交所有权**必须按这个顺序**（反了就是悬垂）；`Drop` 里用 `Box::from_raw` 逐个回收；迭代器那段是 `unsafe { ptr.as_ref() }` 的典型用法——只要列表还活着，节点的生命周期就有保证。

**二、provenance 的四种玩法**（`provenance.rs`）：

```rust
{{#rustdoc_include ../snippets/ch24/provenance.rs}}
```

```text
base.addr() = 0x64c5eff8a0
自动解引用第二个元素 = 20
标记前 0x64c5eff8a0，标记后 0x64c5eff8a1
往返之后读到 = 10
nowhere.addr() = 0x1000，is_null = false
```

（地址每次运行都不同，这行按本机实况粘。）关键是第二行：`base.with_addr(base.addr() + size_of::<u32>())` 得到的指针**照样能解引用**，因为来历还在。如果写成 `(base.addr() + size_of::<u32>()) as *const u32` 就不同了：地址一样，但 `addr()` 不暴露来历，后面的 `as` 只会去捡「已经被暴露过的」那份（std 把这类操作归在 Exposed Provenance 家族，结果「尽力而为」），所以它什么也捡不到——地址对，不能解引用。这就是 `as` 与 `with_addr` 的分水岭。

**三、用 `MaybeUninit` 做「最多初始化一次」的格子**（`checked_uninit.rs`）。它配了一个 `bool` 记账：

```rust
{{#rustdoc_include ../snippets/ch24/checked_uninit.rs}}
```

```text
还没放东西：None
放了之后能借到：true
取走之后：None
drop Noisy(first)
再放一次，析构 Once 时应当只 drop second
drop Noisy(second)
```

数一下 `drop Noisy`：恰好两次，对应两次 `set`——一次来自显式 `drop(taken)`，一次来自 `Once::drop` 里的 `assume_init_drop`。同一个值只被析构一次，这就是 `take` 里那行 `self.init = false;` 买来的东西。

把那一行删掉，行为立刻变形（本机探针实跑，路径与线程号已略去）：

```text
还没放东西：None
放了之后能借到：true
取走之后：Some(())          // 值早被 take 走了，格子却仍报「有值」
drop Noisy(first)           // 显式 drop(taken)
thread 'main' panicked at checked_broken.rs:26:9:
只能设置一次                 // 第二次 set 撞上断言
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
drop Noisy(second)          // panic 展开时析构掉 set 的实参
drop Noisy(first)           // 同一份数据被析构第二次：重复释放
```

退出码是 101。要害不是「多打一行」，而是 `drop Noisy(first)` 出现两次：`Once::drop` 对已被取走的数据又调了一次 `assume_init_drop`。那行记账就是「只析构一次」与「同一份数据被两处拥有」的分界。

**四、`ManuallyDrop` 控制析构时机**（`manually_drop.rs`）：

```rust
{{#rustdoc_include ../snippets/ch24/manually_drop.rs}}
```

```text
drop Log(build)
close 之后 Session 还活着
s2 一直没 close，将在作用域结束时析构
drop Log(test)
drop Log(plain)
```

`drop Log(build)` 出现在 `close` 那一刻，而不是 `Session` 离开作用域时——这正是「手动控制析构」的效果。`closed` 标志保证无论走 `close` 还是走 `Drop`，`Log` 都只被析构一次。

**五、`&raw` 与 packed 字段**（`raw_borrow.rs`）：

```rust
{{#rustdoc_include ../snippets/ch24/raw_borrow.rs}}
```

```text
未对齐读 = 0xdeadbeef
写回后 tag = 2
标签地址 = 0x1000ff987
n = 7
```

（`0xdeadbeef` 是因为代码里写的就是 `0xDEAD_BEEF`；`标签地址` 每次运行都不同。）

第一行是 `&raw const p.value` + `read_unaligned` 的结果。把 `&raw const p.value` 改成 `&p.value`，程序连编译都过不去——见下一节。

## 报错现场

### 一、对 packed 字段取引用

最小复现：

```rust,compile_fail
#[repr(packed)]
struct Packed {
    tag: u8,
    value: u32,
}

fn main() {
    let p = Packed { tag: 1, value: 2 };
    let v = &p.value;
    println!("{}", v);
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0793]: reference to field of packed struct is unaligned
 --> src/main.rs:9:13
  |
9 |     let v = &p.value;
  |             ^^^^^^^^
  |
  = note: this struct is 1-byte aligned, but the type of this field may require higher alignment
  = note: creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)
  = help: copy the field contents to a local variable, or replace the reference with a raw pointer and use `read_unaligned`/`write_unaligned` (loads and stores via `*p` must be properly aligned even when using raw pointers)
```

定位路径：`E0793` → 错误索引 → 「对 packed 结构体字段取引用」。这条诊断把因果链说全了：`#[repr(packed)]` 让结构体 1 字节对齐，而 `u32` 要求 4 字节对齐，于是 `&p.value` 造出未对齐引用——**即使从不解引用**，造出来那一刻就是 UB。报错里 `even if that reference is never dereferenced` 值得记住，它和第 23 章那条「引用必须始终有效」是同一条规则的两次露面。

修法二选一：把值拷出来（`let v = p.value;`，但注意这也只在值可 `Copy` 时省事），或者换成 `&raw const p.value` 再用 `read_unaligned`。后者就是 `snippets/ch24/raw_borrow.rs` 里的写法。

### 二、解引用空指针

最小复现：

```rust,compile_fail
fn main() {
    let v = unsafe { *std::ptr::null::<i32>() };
    println!("{v}");
}
```

以 Rust 1.98 为例：

```text
error: dereferencing a null pointer
 --> src/main.rs:2:22
  |
2 |     let v = unsafe { *std::ptr::null::<i32>() };
  |                      ^^^^^^^^^^^^^^^^^^^^^^^^ this code causes undefined behavior when executed
  |
  = note: `#[deny(deref_nullptr)]` on by default
```

这条不是类型错误，而是**默认 deny 的 lint** 在起作用：`deref_nullptr` 抓的是「直接解引用编译器能确定为空字面量的指针」。它的价值在于点破一个常见误解——进了 `unsafe` 不等于编译器闭嘴：`unsafe` 只是把「保证不出 UB」的责任转移给你，lint 该报还是会报。

两个变体值得知道：

- 写成 `let p: *const i32 = std::ptr::null(); unsafe { *p }` 就不再触发这个 lint（编译器不追这条数据流）。也就是说：**lint 过了不代表代码对了**，这里依然是 UB。
- 真正需要「可能为空」的指针时，用 `Option<NonNull<T>>` 或者老老实实判 `is_null()`，而不是靠 `unsafe` 硬上。

## 小结

- 裸指针是「地址 + 来历（provenance）」。`addr` 只取地址、不暴露来历，`with_addr`/`map_addr` 换地址保来历，`expose_provenance` + `with_exposed_provenance` 把来历交给整数世界，`without_provenance` 直接没有来历（只能做零大小访问）。这套严格来历 API 1.84 稳定（`NonNull` 上的 `*_provenance` 是 1.89）；`addr` 一族与 `NonNull::with_exposed_provenance` 到 1.98 还不是 `const fn`；旧名 `from_exposed_addr` 已经不用了。
- `NonNull<T>` = 非空 + 协变 + 有安全构造器，代价是比裸指针更容易被误用；拿不准时用 `*mut T`。两者都不实现 `Send`/`Sync`。
- 造裸指针优先用 `&raw const`/`&raw mut`（1.82）：place 未对齐或没有有效值时，`&` 一造出来就是 UB。
- `MaybeUninit<T>` 把「可能还没初始化」写进类型，但初始化状态要你自己记账；`assume_init` 要求完全初始化，`assume_init_read` 容易读两次，`zeroed` 只对「全零位模式合法」的 `T` 成立。
- `ManuallyDrop<T>` 只控制析构（布局不变，零成本）；`take`/`drop` 的契约是「恰好一次」，忘了的后果是泄漏而不是 UB。
- 别名规则的确切形式官方还没定案，Reference 只给原则；Stacked/Tree Borrows 是 Miri 在用的两个候选模型，形式化见第 35 章。

## 练习

**1. 预测编译结果。** 下面两段哪一段过不了编译？为什么？

```rust
let p: *const u8 = std::ptr::null();
let a = p.addr();
```

```rust,compile_fail
#[repr(packed)]
struct Packed {
    tag: u8,
    value: u32,
}

fn main() {
    let p = Packed { tag: 1, value: 2 };
    let r = &p.value;
    println!("{}", r);
}
```

**2. 解释为什么。** 为什么 `Option<NonNull<T>>` 与 `*mut T` 同宽，而 `Option<*mut T>` 要占两个机器字？

**3. 找错。** 下面这段用 `MaybeUninit` 的代码有什么问题？会在哪一步出事？

```rust,ignore
let mut slot = MaybeUninit::<String>::uninit();
slot.write(String::from("hello"));
let s1 = unsafe { slot.assume_init_read() };
let s2 = unsafe { slot.assume_init_read() };
drop(s1);
drop(s2);
```

<details><summary>参考答案</summary>

**1.** 第一段能过，第二段过不了（`error[E0793]: reference to field of packed struct is unaligned`）。

第一段里 `p.addr()` 只是取地址：它不暴露来历、也不解引用，所以安全且合法；`null()` 造出来的空指针取地址同样没问题。

第二段的问题不在「有没有读写」，而在「有没有造出未对齐引用」：`#[repr(packed)]` 把结构体对齐压到 1 字节，字段 `value: u32` 自己要求 4 字节对齐，`&p.value` 于是会产出一个未对齐引用——按规则，**造出来**就是 UB，哪怕一次都没被解引用（诊断里那行 `even if that reference is never dereferenced` 说的就是这个）。改成 `&raw const p.value` 拿裸指针、再用 `read_unaligned` 就读得出来。

**2.** 因为枚举要靠判别式区分变体，而这个判别式不能和任何合法值冲突。`NonNull<T>` 的契约是「永不为空」，于是空值是一个**被类型系统保证不会出现**的位模式——编译器正好拿它当 `None` 的判别式，`Option<NonNull<T>>` 就退化成单个指针大小。`*mut T` 没有这个保证，空指针是一个完全合法的值，判别式无处安放，只能另加一个字。实测（本章 `nonnull_list.rs` 的输出）：8 字节 vs 16 字节。

反过来说：这个省下来的字是 `NonNull` 为保证非空**付出代价换来的**——你不能再拿空指针当哨兵值用，也不能凭空造一个「空但未使用」的 `NonNull`。（`NonNull::dangling()` 是合法做法：它对齐、但不指向有效内存，不能解引用；文档还专门提醒别拿它当「尚未初始化」的哨兵值，因为那个地址可能碰巧就是某个有效指针。）

**3.** 问题在后面两次 `assume_init_read`：第一次读出一个 `String`（`s1`），槽位里的字节没有因此失效，也没有被标记；第二次再读一次，得到 `s2`，它和 `s1` 是同一份数据的两份所有权。于是 `drop(s1)` 释放了那块堆内存、`drop(s2)` 又释放一次——重复释放。

修法取决于意图：只想拿一份就用 `assume_init()`（消耗容器）或 `assume_init_ref()`（借出来用）；确实要在 `&mut self` 上搬走值，那就得记账，像 `checked_uninit.rs` 的 `Once::take` 那样先清标志位再读——把「这份数据已经被搬走了」这件事记下来，第二次调用就不会再读。注意 `write` 也不能救场：它覆盖内容但不动你的记账，`assume_init_read` 的语义本身就不带「取走」的含义。

</details>

## 延伸阅读

- 别名与优化动机（Nomicon《Aliasing》）：<https://doc.rust-lang.org/nomicon/aliasing.html>
- 延迟初始化两节（Nomicon《Checked Uninitialized Memory》《Unchecked Uninitialized Memory》）：<https://doc.rust-lang.org/nomicon/checked-uninit.html>、<https://doc.rust-lang.org/nomicon/unchecked-uninit.html>
- 裸借用运算符（Reference 的 `&raw` 一节）：<https://doc.rust-lang.org/reference/expressions/operator-expr.html>
- UB 清单与有效性要求（Reference）：<https://doc.rust-lang.org/reference/behavior-considered-undefined.html>
- `std::ptr` 模块文档（provenance API 全表）：<https://doc.rust-lang.org/std/ptr/index.html>
- `MaybeUninit`：<https://doc.rust-lang.org/std/mem/union.MaybeUninit.html>
- `ManuallyDrop`：<https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html>
- 别名模型的形式化现状（Unsafe Code Guidelines）：<https://rust-lang.github.io/unsafe-code-guidelines/>
