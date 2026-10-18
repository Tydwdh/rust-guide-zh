# 第 25 章 数据布局与 repr

## 这一章解决什么问题

第 17 章留下一组数字：那个 `List` 是 16 字节，`Box<[u64]>` 是 16 字节，`Option<Box<u64>>` 和 `Box<u64>` 一样大。第 13 章说 `&dyn Trait` 是「两个机器字」。这些数字当时都是直接给的。本章交代它们从哪来，以及哪些能信、哪些不能。

读完这一章，你应该能回答四类问题：

- 一个类型占几字节、对齐到几，字段各在什么偏移？能不能自己量出来？
- 为什么 `Option<&T>` 不比 `&T` 多一个字节，`Option<u32>` 却比 `u32` 大一倍？
- 布局在什么情况下会变？「能变」为什么不是 bug？
- 什么时候必须把布局钉死，用哪个 `repr` 钉？

边界先画清楚：`extern "C"` 的调用约定、符号名与 C 字符串是第 26 章；vtable 里有几个槽位、动态调用怎么跳是第 13 章；指针本身的合法性（provenance、别名模型）是第 24 章。本章只管一件事：字节怎么摆。

## 为什么这样设计

### 心智模型：布局是编译器手里的一张牌

在 C 里，结构体的定义写在头文件里，被很多编译单元共享。两个 `.c` 文件各自编译，最后由链接器拼起来，它们必须对「这个结构体长什么样」有完全一致的理解。**布局在 C 里是契约**，是 ABI 的一部分。

Rust 的类型默认不走这条路。一个 crate 里的类型，编译器从头到尾看得见完整定义，不需要跟任何外部约定对齐。于是它可以自由地打这张牌：重排字段少留填充，把判别式（discriminant）塞进别的字段用不到的值里。参考手册把这条自由写在最前面：

> Type layout can be changed with each compilation.

译：类型布局可能随每次编译而改变。

但它不能乱来：有一部分保证是正确性的底线，少了会直接导致未定义行为（UB）。参考手册先划出范围：

> The only data layout guarantees made by this representation are those required for soundness.

把具体条款念出来——字段偏移是该字段对齐的倍数；类型对齐不低于字段中最大的对齐；结构体的字段互不重叠——它又补了一句：

> There are no other guarantees of data layout made by this representation.

译：这种表示不再提供任何别的布局保证。

Nomicon 用一对字段完全相同的结构体把这件事挑明：

> Rust does guarantee that two instances of A have their data laid out in exactly the same way. However Rust does not currently guarantee that an instance of A has the same field ordering or padding as an instance of B.

同一个类型的两个值摆放方式必须相同，否则 `Vec<A>` 没法按固定步长索引。但**声明看起来一样的两个类型**之间，字段顺序和填充都可以不同。

于是记住这句话：**布局是编译器的优化空间，不是类型的一部分。** 你明确说「我要这个布局」，它才停手。

这也解释了一个常见误解。有人升级编译器后发现某个结构体从 24 字节变成 16 字节，就把它当 bug 上报。不是 bug，那正是参考手册第一句话说的事。反过来，如果一份依赖精确布局的数据——FFI 结构体、磁盘格式、共享内存里的记录——在升级后对不上了，那确实是 bug，但**病根在当初没有把布局钉死**，不在这次升级。

### 这张牌换来什么

- **填得更紧**：字段重排以后填充更少。
- **判别式不占地方**：枚举可以借载荷类型里的无效位模式来表示「这是哪个变体」，于是省掉一个独立字段。

两条都是纯收益，代价只有一条：你不能依赖默认布局。

## 机制与规则

### 对齐、大小与填充

三个词先钉死。

- **对齐（alignment）**：合法地址必须是它的倍数，也就是 1、2、4、8、16 这些 2 的幂。对齐为 4 的值只能放在 4 的倍数地址上。
- **大小（size）**：数组里相邻两个元素之间的字节距离，含尾部填充。**大小永远是对齐的倍数**，否则数组无法用「下标乘大小」定位。
- **填充（padding）**：为了同时满足上面两条而插入的、没有任何意义的字节。

本机实测（Windows x86-64、Rust 1.98）的基本类型：

| 类型 | `size_of` | `align_of` |
|---|---|---|
| `bool`、`u8`、`i8` | 1 | 1 |
| `u16`、`i16` | 2 | 2 |
| `u32`、`i32`、`f32`、`char` | 4 | 4 |
| `u64`、`i64`、`f64`、`usize` | 8 | 8 |
| `u128`、`i128` | 16 | 16 |

大小这一列是参考手册保证的数值；**对齐不是**。参考手册的原话是：

> The alignment of primitives is platform-specific. In most cases, their alignment is equal to their size, but it may be less. In particular, i128 and u128 are often aligned to 4 or 8 bytes even though their size is 16, and on many 32-bit platforms, i64, u64, and f64 are only aligned to 4 bytes, not 8.

译：基本类型的对齐是平台相关的——多数情况下等于大小，但可能更小；`i128`/`u128` 常常只对齐到 4 或 8，很多 32 位平台上 `i64`/`u64`/`f64` 只对齐到 4 而不是 8。所以别把右边那列抄进跨平台断言。

测量工具一共五个（1.98 全部稳定）：

| 工具 | 量什么 |
|---|---|
| `size_of::<T>()` | 类型大小，编译期常量 |
| `align_of::<T>()` | 类型对齐，编译期常量 |
| `offset_of!(T, field)` | 字段相对类型起点的偏移 |
| `size_of_val(&v)` | 值的大小（DST 用，见后文） |
| `align_of_val(&v)` | 值的对齐（DST 用，见后文） |

前两个要求 `T: Sized`；`offset_of!` 的要求不同——被取偏移的**字段**类型必须是 `Sized`，但字段所在的容器可以是 DST。std 文档给的就是这个形状：

```rust
use std::fmt::Debug;
use std::mem::offset_of;

#[repr(C)]
struct Header<T: ?Sized> {
    tag: u8,
    body: T,
}

fn main() {
    // 取的是 Sized 字段：容器本身是 DST 也算得出来。
    println!("{}", offset_of!(Header<dyn Debug>, tag));
}
```

```text
0
```

反过来，`body` 这种 DST 字段的偏移取不了——它的对齐可能是动态的，只能从一个真实的指针上问。`offset_of!` 支持链式字段访问（如 `offset_of!(Outer, inner.field)`），不接受数组下标。

### repr(Rust)：默认表示

不写 `repr` 就是默认表示；显式写 `#[repr(Rust)]` 与不写完全等价（参考手册保证）。它给你的只有上面那几条底线保证，别的一概不管。

下面这一段是本机实测。同一批字段写成两个类型，一个不加属性、一个加 `#[repr(C)]`：

<svg viewBox="0 0 760 200" width="100%" role="img" aria-label="同一个四字段结构体的两种表示：默认表示把字段重排为 d、b、a、c 后共 16 字节，repr(C) 按声明顺序算出 24 字节并插入更多填充" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">字段声明顺序：a: u8, b: u32, c: u8, d: u64（Windows x86-64、Rust 1.98）</text>
<text x="0" y="60" font-size="12" fill="currentColor">repr(Rust)</text>
<text x="0" y="78" font-size="12" fill="currentColor" opacity="0.7">16 字节</text>
<rect x="110" y="44" width="208" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="214" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">d: u64</text>
<rect x="318" y="44" width="104" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="370" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">b: u32</text>
<rect x="422" y="44" width="26" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="435" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">a</text>
<rect x="448" y="44" width="26" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="461" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">c</text>
<rect x="474" y="44" width="52" height="30" rx="4" fill="none" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="500" y="64" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">填充</text>
<text x="112" y="88" font-size="12" fill="currentColor" opacity="0.7">0</text>
<text x="320" y="88" font-size="12" fill="currentColor" opacity="0.7">8</text>
<text x="424" y="88" font-size="12" fill="currentColor" opacity="0.7">12</text>
<text x="450" y="88" font-size="12" fill="currentColor" opacity="0.7">13</text>
<text x="476" y="88" font-size="12" fill="currentColor" opacity="0.7">14</text>
<text x="0" y="132" font-size="12" fill="currentColor">repr(C)</text>
<text x="0" y="150" font-size="12" fill="currentColor" opacity="0.7">24 字节</text>
<rect x="110" y="116" width="26" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="123" y="136" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">a</text>
<rect x="136" y="116" width="78" height="30" rx="4" fill="none" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="175" y="136" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">填充</text>
<rect x="214" y="116" width="104" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="266" y="136" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">b: u32</text>
<rect x="318" y="116" width="26" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="331" y="136" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">c</text>
<rect x="344" y="116" width="182" height="30" rx="4" fill="none" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="435" y="136" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">填充</text>
<rect x="526" y="116" width="208" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="630" y="136" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">d: u64</text>
<text x="112" y="160" font-size="12" fill="currentColor" opacity="0.7">0</text>
<text x="138" y="160" font-size="12" fill="currentColor" opacity="0.7">1</text>
<text x="216" y="160" font-size="12" fill="currentColor" opacity="0.7">4</text>
<text x="320" y="160" font-size="12" fill="currentColor" opacity="0.7">8</text>
<text x="346" y="160" font-size="12" fill="currentColor" opacity="0.7">9</text>
<text x="528" y="160" font-size="12" fill="currentColor" opacity="0.7">16</text>
<text x="0" y="188" font-size="12" fill="currentColor" opacity="0.7">默认表示重排后只需 16 字节；repr(C) 按声明顺序算偏移，填充更多，共 24 字节。</text>
</svg>

数字本身：`Mixed` 是 16 字节，字段偏移依次是 `d` 0、`b` 8、`a` 12、`c` 13，末尾 2 字节填充；`CMixed` 是 24 字节，偏移 `a` 0、`b` 4、`c` 8、`d` 16。等一下——`d` 是 8 字节对齐，它摆在偏移 0 完全合法；把 `d` 放前、两个 `u8` 挤进尾部，填充就只剩 2 字节。

**但这是本机这一次编译的结果，不是承诺。** 编译器也可以按声明顺序摆，那就得 24 字节——想省内存就量完再调字段顺序，别对着源码猜。

> [!WARNING]
> 上面这段实测还说明另一件事：**字段偏移在默认表示下不是稳定接口。** 如果你的代码依赖某个偏移（`unsafe` 里按偏移读写、把结构体直接写进文件），就必须换成 `repr(C)`。

### repr(C)：把布局写成契约

`#[repr(C)]` 是唯一自带一套**确定布局算法**的表示，所以它有两个用途：跟 C 对接，以及让自己能安全地依赖布局做手脚（重解释、按偏移读写）。Nomicon 的原话：

> The order, size, and alignment of fields is exactly what you would expect from C or C++.

在它后面还有一句同样重要：

> Any type you expect to pass through an FFI boundary should have `repr(C)`, as C is the lingua-franca of the programming world.

`repr(C)` 结构体的大小与偏移有明确的算法，参考手册逐条给出：

1. 从偏移 0 开始。
2. 按声明顺序取每个字段：如果当前偏移不是该字段对齐的倍数，先补填充字节；此处的值就是该字段的偏移。
3. 偏移加上字段大小，继续下一个。
4. 全部走完后，把总偏移向上取整到「结构体对齐」的倍数，得到大小。

其中结构体对齐 = 字段对齐的最大值（没有字段时为 1）。用它验算上面的 `CMixed`：`a` 在 0；`b` 对齐 4，补 3 字节到 4；`c` 在 8；`d` 对齐 8，从 9 补到 16，占 16 到 24；结构体对齐 8，24 已经是倍数，大小 24。分毫不差。

`repr(C)` 也不是万能的，Nomicon 列了它的边界，其中最容易被忘掉的是：**零大小类型在 `repr(C)` 里仍然是零大小**，而 C++ 的空类型至少要占 1 字节。也就是说一个空结构体在两边的大小不一致，不能互相骗。胖指针和元组在 C 里根本没有对应物，Nomicon 直接说它们永远不是 FFI 安全的。跨语言的那部分细则留给第 26 章。

### repr(transparent)：显式承诺「我就是它」

`#[repr(transparent)]` 用于「只有一个非零大小字段」（外加任意多个零大小字段，比如 `PhantomData<T>`）的结构体或单变体枚举。参考手册给它的语义是一句话：

> Structs and enums with this representation have the same layout and ABI as the only non-size 0 non-alignment 1 field, if present, or unit otherwise.

也就是布局和 ABI 与被包住的那个字段完全一致。它存在的意义是：普通单字段结构体**通常**也长这样，但不受保证；`repr(transparent)` 把这个「通常」变成承诺，于是把它传给期待内层类型的 C 代码是被保证可行的。

本机实测：`#[repr(transparent)] struct Meters(f64)` 与 `f64` 同为 8 字节、对齐 8。

约束也硬：它不能和别的表示组合，超出一个非平凡字段就报 `error[E0690]: transparent struct needs at most one non-trivial field, but has 2`。

### repr(u*)、repr(i*) 与 repr(C) 枚举

无字段枚举（所有变体都不带数据）是最容易踩的一类。Nomicon 明确说：**没写 `repr` 的无字段枚举是纯 Rust 类型，布局与表示都不稳定**。给它加 `#[repr(u8)]`，它的布局就等同于 `u8`；`repr(i32)` 就是 `i32`。判别值必须装得下，装不下是编译错误。

带字段的枚举复杂得多。`repr(C)` 的带字段枚举在参考手册里被描述成「一个 `repr(C)` 结构体，装两个东西」：

- **tag**：把各变体的字段全去掉后得到的 `repr(C)` 无字段枚举；
- **payload**：每个带字段变体各自的 `repr(C)` 结构体，合成一个 `repr(C)` 联合体。

换句话说，C 侧看到的是一个 tagged union，可以按这个结构去读它。反过来，C 的 enum 其实只是「typedef 加几个命名常量」，对象里可以存任何整数值——Rust 的枚举不行，构造一个不匹配任何变体的值是 UB。参考手册专门警告：拿无字段枚举去建模 C 的 enum 常常是错的。

### niche 优化：判别式可以不占地方

先看一个类型「有没有空位」。`&T` 永远非空，于是「指针全为零」这个位模式在它里面永远不会出现——这种「该类型不可能出现的位模式」叫**空位（niche）**。枚举需要一个判别式告诉程序「现在是哪个变体」，如果某个变体的载荷自带空位，判别式就可以借用它，不必另占字节。

本机实测（Windows x86-64、Rust 1.98）：

| 类型 | `size_of` | 说明 |
|---|---|---|
| `&u64` | 8 | 一个机器字 |
| `Option<&u64>` | 8 | 空指针表示 `None` |
| `Box<u64>` | 8 | 同上 |
| `Option<Box<u64>>` | 8 | 同上 |
| `NonZeroU32` | 4 | 0 不可用，是空位 |
| `Option<NonZeroU32>` | 4 | 0 表示 `None` |
| `u32` | 4 | 所有位模式都合法 |
| `Option<u32>` | 8 | 没空位可用，只能加判别式 |
| `bool` | 1 | 只有 0/1 有效 |
| `Option<bool>` | 1 | 2 表示 `None` |
| `Option<Option<bool>>` | 1 | 外层 `None` 用 `0xff` |
| `fn()` | 8 | 函数指针非空 |
| `Option<fn()>` | 8 | 空指针表示 `None` |
| `*const u8` | 8 | **裸指针可以是 null，没有空位** |
| `Option<*const u8>` | 16 | 只能另加判别式 |

这张表解释了两件容易误解的事。第一，`Option<T>` 不一定比 `T` 大——有空位可借就是零开销。第二，`Option<*const u8>` **不比** `*const u8` 小：裸指针允许为空，没有空位（`&T` 与 `Box<T>` 才有）；要可空又不加字节就用 `NonNull<T>`，实测 `Option<NonNull<u32>>` 是 8 字节。

表里「哪个取值表示 `None`」只是当前实现（本机实测：`Option<bool>` 用 2、`Option<Option<bool>>` 的外层用 `0xff`、`Option<NonZeroU32>` 用 0），不是接口，别背。

两个必须说清的限定：

- **niche 优化不保证。** 它是编译器当前的选择，换个版本、换个目标平台可能就没了。代码的正确性不能依赖「`Option<&T>` 恰好是 8 字节」，但性能调优和内存预算可以拿它做参考。
- **显式 `repr` 会关掉它，但只限带字段的枚举。** Nomicon 原话：`Adding an explicit repr(u*), repr(i*), or repr(C) to an enum with fields suppresses the null-pointer optimization`。给带字段的枚举加 `repr`，等于告诉编译器「布局我自己说了算」，它就不再借用空位；无字段枚举不受影响——上表的 `Tagged8` 带着 `repr(u8)`，`Option<Tagged8>` 仍是 1 字节。

### 同一个枚举，载荷一换，策略就换

下面这张图是本章最值得停下来看的一处。两个枚举只差载荷类型，`List` 的载荷是 `i32`、`BigList` 的载荷是 `u64`，都是「非空指针变体 + 单元变体」的形状：

<svg viewBox="0 0 760 220" width="100%" role="img" aria-label="两个形状相同的枚举：List 的 i32 载荷版把判别式与填充放在开头 4 字节、Box 在第 8 字节；BigList 的 u64 载荷版把 Box 放在偏移 0、用空指针表示 Nil，两行都占 16 字节但 Option 大小不同" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">同一个形状，只换了载荷类型，判别式就换了地方（Windows x86-64、Rust 1.98）</text>
<text x="0" y="60" font-size="12" fill="currentColor">List（i32 载荷）</text>
<text x="0" y="78" font-size="12" fill="currentColor" opacity="0.7">16 字节</text>
<rect x="110" y="44" width="104" height="30" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="162" y="64" font-size="12" fill="currentColor" text-anchor="middle">判别式 + 填充</text>
<rect x="214" y="44" width="104" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="266" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">i32</text>
<rect x="318" y="44" width="208" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="422" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Box&lt;List&gt;</text>
<text x="112" y="88" font-size="12" fill="currentColor" opacity="0.7">0</text>
<text x="216" y="88" font-size="12" fill="currentColor" opacity="0.7">4</text>
<text x="320" y="88" font-size="12" fill="currentColor" opacity="0.7">8</text>
<text x="545" y="64" font-size="12" fill="currentColor">Option&lt;List&gt; 也是 16 字节</text>
<text x="545" y="82" font-size="12" fill="currentColor" opacity="0.7">判别式还留着没用的取值</text>
<text x="0" y="132" font-size="12" fill="currentColor">BigList（u64 载荷）</text>
<text x="0" y="150" font-size="12" fill="currentColor" opacity="0.7">16 字节</text>
<rect x="110" y="116" width="208" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="214" y="136" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Box&lt;BigList&gt;</text>
<rect x="318" y="116" width="208" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="422" y="136" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">u64</text>
<text x="112" y="160" font-size="12" fill="currentColor" opacity="0.7">0</text>
<text x="320" y="160" font-size="12" fill="currentColor" opacity="0.7">8</text>
<text x="545" y="136" font-size="12" fill="currentColor">Option&lt;BigList&gt; 是 24 字节</text>
<text x="545" y="154" font-size="12" fill="currentColor" opacity="0.7">空指针已被 Nil 用掉</text>
<text x="0" y="200" font-size="12" fill="currentColor" opacity="0.7">两行都占 16 字节，但判别式一处在字节里、一处借了空指针——选哪种是编译器的自由。</text>
</svg>

读数：`List` 的开头 4 字节放着判别式与填充（这样 `i32` 正好落在 4 字节边界上），`i32` 在偏移 4、`Box<List>` 在偏移 8；`BigList` 干脆把 `Box<BigList>` 提到偏移 0，用它的空指针值直接表示 `Nil`，`u64` 落在偏移 8，一个判别字节都不花。

两个类型都只占 16 字节，但性质不同：`List` 的判别式还留着大量没用的取值，所以 `Option<List>` 能塞进同样的 16 字节；`BigList` 的空位已经被 `Nil` 用掉了，`Option<BigList>` 只能另找地方，变成 24 字节。**同一个源码形状，换个载荷类型，内存开销就差 8 字节**——这不是 bug，也不是需要背的规则，而是「布局无保证」这句话的具体样子。

### DST 与胖指针

`size_of::<T>()` 要求 `T: Sized`，因为它在编译期就要算出结果。有一类类型天生算不出来，它们叫**动态大小类型（DST，dynamically sized type）**：没有编译期已知的大小与对齐。语言暴露给我们的只有两类（外加用户自定义的）：

- **切片**：`[T]`、`str`；
- **trait 对象**：`dyn Trait`。

Nomicon 的说法是：既然连大小都不知道，这类类型只能存在于指针之后。此时指针要额外携带补全这个类型的信息，于是变成**胖指针（fat pointer）**：两个机器字。

| 胖指针 | 第一个字 | 第二个字 | 本机大小 |
|---|---|---|---|
| `&[u64]` | 首元素地址 | 元素个数 | 16 |
| `&str` / `Box<str>` | 首字节地址 | 字节长度 | 16 |
| `Box<[u64]>` | 首元素地址 | 元素个数 | 16 |
| `&dyn Trait` / `Box<dyn Trait>` | 数据地址 | vtable 指针 | 16 |

注意 `&str` 的第二个字是**字节数**，不是字符数：`"héllo"` 是 5 个字符、6 个字节，实测 `size_of_val` 给 6、`align_of_val` 给 1（第 6 章讲过 UTF-8 与 `chars()` 的区别）。还要注意表的最后一行跟前面几行不是一回事：切片的第二个字是长度，`dyn` 的第二个字是表指针——`size_of_val` 能对上 trait 对象报出大小，靠的就是从 vtable 头部读元数据（第 13 章拆过 vtable）。

想拿被指向对象的大小与对齐，用 `size_of_val` / `align_of_val`——它们只接收**引用**，签名是 `fn size_of_val<T: ?Sized>(val: &T) -> usize`，从胖指针的元数据里取答案。传裸指针会直接报 `error[E0308]: mismatched types`（`expected &_, found *const u64`）；对应裸指针的 `size_of_val_raw` / `align_of_val_raw` 到 1.98 还是 nightly 专属（`error[E0658]: use of unstable library feature 'layout_for_ptr'`，tracking issue #69835）。实测：`&[u64; 3]` 转成 `&[u64]` 后 `size_of_val` 是 24（3 × 8）、`align_of_val` 是 8；同一个 `&dyn Debug`，指向 `Circle` 时报 8/8，指向零大小的 `Dot` 时报 0/1。**trait 对象的大小只能在运行时问**，这正是动态分发的组成部分。

最后一句提醒来自参考手册，原文是：

> Though you should not rely on this, all pointers to DSTs are currently twice the size of the size of usize and have the same alignment.

译：虽然你不该依赖这一点，但目前所有指向 DST 的指针都是 `usize` 的两倍大小，对齐与 `usize` 相同。本机 `usize` 是 8 字节，胖指针 16 字节，符合这个描述；换个目标平台照抄就不一定了。

## 动手看看

四个程序都在本机跑过（Windows x86-64、Rust 1.98）。第一个量结构体：

```rust
{{#rustdoc_include ../snippets/ch25/structs.rs}}
```

```text
Mixed    size=16 align=8
  偏移 a=12 b=8 c=13 d=0
CMixed   size=24 align=8
  偏移 a=0 b=4 c=8 d=16
CHeader  size=8 align=4 偏移 tag=0 len=4
Aligned16 size=16 align=16
Meters   size=8 align=8（f64 是 8 / 8）
```

`offset_of!(Mixed, d)` 对默认表示照样能用；它报出的偏移不是接口，只是这次编译的结果。`CHeader` 是典型的 FFI 头：C 侧写 `struct { uint8_t tag; uint32_t len; }`，两边都得出 8 字节、`len` 在偏移 4。`Aligned16` 是把对齐抬到 16 的写法（`align` 只抬不降），一个 `u64` 因此占了 16 字节。

第二个量枚举与 niche：

```rust
{{#rustdoc_include ../snippets/ch25/enums.rs}}
```

```text
Plain3   size=1 align=1   Option<Plain3>  size=1
Tagged8  size=1 align=1   Option<Tagged8> size=1
Tagged32 size=4 align=4
TwoU64   size=16 align=8   Option<TwoU64>  size=16
CMsg     size=16 align=8
--- niche 优化 ---
&u64                  8
Option<&u64>          8
Box<u64>              8
Option<Box<u64>>      8
u32                   4
Option<u32>           8
NonZeroU32            4
Option<NonZeroU32>    4
bool                  1
Option<bool>          1
Option<Option<bool>>  1
*const u8             8
Option<*const u8>     16
Option<fn()>          8
```

`Plain3` 没写 `repr`，三个变体也只占 1 字节——无字段枚举的判别值只有 0、1、2，一个字节够用，编译器不会浪费。`TwoU64` 的两个变体载荷都是 `u64`，没有任何空位可借，判别式只能实打实占地方，于是 16 字节。但 `Option<TwoU64>` 也一样是 16 字节：那个独立判别式只用了两个取值，还剩大量空位，`Option<TwoU64>` 的判别式正好塞进去（对照下面的 `BigList`：它的空位被 `Nil` 用完，`Option` 就要 24 字节）。

第三个程序回到第 17 章那个 `List`，并且加了一个只改载荷类型的对照组：

```rust
{{#rustdoc_include ../snippets/ch25/list.rs}}
```

```text
List     size=16 align=8   i32 偏移=4  Box 偏移=8
Option<List>     size=16
BigList  size=16 align=8   u64 偏移=8  Box 偏移=0
Option<BigList>  size=24
```

第 17 章数过 `List` 是 16 字节、判别字段就在开头（那种 `u64` 载荷的形状才会折进空指针）；这里把偏移与 `Option` 大小补齐：`List` 的开头 4 字节是判别式与填充，`i32` 在偏移 4、`Box<List>` 在偏移 8，判别式还留着没用的取值，所以 `Option<List>` 仍是 16 字节；`BigList` 把 `Box` 提到偏移 0、用空指针表示 `Nil`，空位被占满，`Option<BigList>` 就是 24 字节。两种策略都合法——差别只在于编译器算了哪笔账。

> [!NOTE]
> 上面这段没用 `offset_of!` 量枚举内部，因为枚举变体字段的偏移在 1.98 还没稳定（`error[E0658]: using enums in offset_of is experimental`）。量枚举内部偏移暂时只能把字段地址相减。

第四个量 DST：

```rust
{{#rustdoc_include ../snippets/ch25/dst.rs}}
```

```text
&[u64; 3]      8
&[u64]        16
&str          16
Box<[u64; 3]> 8
Box<[u64]>    16
Box<str>      16
&dyn Debug    16
Box<dyn Debug> 16
size_of_val(&[u64; 3] 转 &[u64]) = 24 align_of_val = 8
str 的字节数=6 字符数=5 size_of_val=6 align_of_val=1
同一个 &dyn Debug 指向不同值：size_of_val = 8 / 0，align_of_val = 8 / 1
Box<dyn Debug> 里的值：size_of_val = 8
[u16; 0]: size=0 align=2
```

第一行和第四行值得对照：`&[u64; 3]` 与 `Box<[u64; 3]>` 都是瘦指针（8 字节），因为定长数组是 `Sized`；一旦发生**不定长转换（unsizing coercion）**——`&[u64; 3]` 变成 `&[u64]`——指针就胖了一倍。最后一行则提醒：零大小类型也有对齐，只是大小是 0。

## 报错现场

第一个报错来自把 DST 塞进 `size_of`：

```rust,compile_fail
fn main() {
    let n = std::mem::size_of::<str>();
    println!("{n}");
}
```

以 Rust 1.98 为例，关键几行（本机路径已省略）：

```text
error[E0277]: the size for values of type `str` cannot be known at compilation time
   --> src/main.rs:2:33
    |
  2 |     let n = std::mem::size_of::<str>();
    |                                 ^^^ doesn't have a size known at compile-time
    |
    = help: the trait `Sized` is not implemented for `str`
note: required by an implicit `Sized` bound in `std::mem::size_of`
```

定位路径很清楚：`size_of` 的签名是 `pub const fn size_of<T>() -> usize`，`T` 上带着隐式 `Sized` 约束；`str` 不满足，于是报「`Sized` 没有为 `str` 实现」。这不是报错在挑刺：一个 `str` 可以长可以短，编译期本来就无从知道。

修法分两种。要类型大小，换成 `Sized` 的类型；要**值**的大小，用 `size_of_val` 并给它一个引用：

```rust
fn main() {
    let s: &str = "hello";
    println!("{}", std::mem::size_of_val(s));
    println!("{}", std::mem::size_of::<&str>());
}
```

```text
5
16
```

第二个报错来自 `repr(packed)`，它把对齐降到 1、消掉全部填充。这在解析外来格式时很有用，但会引出一个陷阱：

```rust,compile_fail
#[repr(C, packed)]
struct Packed {
    a: u8,
    b: u32,
}

fn main() {
    let p = Packed { a: 1, b: 2 };
    let r = &p.b;
    println!("{}", *r);
}
```

以 Rust 1.98 为例（同样的代码，行号与上面一致）：

```text
error[E0793]: reference to field of packed struct is unaligned
 --> src/main.rs:9:13
  |
9 |     let r = &p.b;
  |             ^^^^
  |
  = note: this struct is 1-byte aligned, but the type of this field may require higher alignment
  = note: creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)
  = help: copy the field contents to a local variable, or replace the reference with a raw pointer and use `read_unaligned`/`write_unaligned` (loads and stores via `*p` must be properly aligned even when using raw pointers)
```

`Packed` 的对齐是 1，`b` 落在偏移 1，于是 `&p.b` 会造出一个对齐 4 的引用指向 1 的对齐地址。**构造这种引用本身就是 UB，哪怕你从不读它**，所以编译器在这里直接拦住。修法写在 help 里：按值取一份副本（`let b = p.b;`），或者改用裸指针加 `read_unaligned`。本机实测 `Packed` 是 5 字节对齐 1，而同样的 `Unpacked { a: u8, b: u32 }`（默认表示）和 `#[repr(C)]` 版本都是 8 字节对齐 4——省下的 3 字节就是对齐的价钱。

## 小结

- 布局 = 大小 + 对齐 + 字段偏移 + 判别式怎么放；默认表示 `repr(Rust)` 只保证正确性必需的几条——字段偏移是字段对齐的倍数、类型对齐不低于字段最大对齐、字段不重叠——参考手册明说「没有别的保证」。这些都能用 `size_of`、`align_of`、`offset_of!`、`size_of_val` 量出来，而且要在**本机**量。
- `repr(C)` 把布局写成契约（字段按声明顺序、大小按固定算法算），是 FFI 和「靠自己按偏移读写」的前提；`repr(transparent)` 承诺「我就是那个唯一非零字段」。
- 无字段枚举不写 `repr` 就没有稳定布局；带字段枚举写 `repr` 会换来确定的 tag + payload 结构，代价是失去 niche 优化。
- niche 优化让判别式借用载荷里的无效位模式，`Option<&T>`、`Option<Box<T>>`、`Option<NonZeroU32>` 才能零开销。裸指针可以是 null，没有空位，所以 `Option<*const T>` 比 `*const T` 大——需要可空指针时用 `NonNull<T>`。
- 同一形状的枚举换个载荷类型，编译器就可能换一套布局策略；`Option<X>` 的大小随之改变。这不是需要背的规则，而是「别依赖默认布局」的实例。
- DST（`str`、`[T]`、`dyn Trait`）没有编译期已知大小，只能待在胖指针后面：数据地址 + 元数据（长度或 vtable 指针），本机各 16 字节。大小与对齐要问 `size_of_val`。

## 练习

**一、预测大小与对齐。** `struct Two { a: u8, b: u64 }` 与 `#[repr(C)] struct CTwo { a: u8, b: u64 }`，在本机（x86-64）各是多少字节、对齐多少？字段偏移各是多少？为什么默认表示在这件事上占不到便宜？

**二、找错。** 有人写了这个类型，想把 `u32` 包装成一个「不透明 ID」，并要求它能跟 C 头文件里的 `uint32_t` 互换：

```rust
#[repr(C)]
struct Id(u32);
```

（a）它的布局满足要求吗？（b）既然满足，为什么还要存在 `#[repr(transparent)]`？

**三、解释。** 下表三项都在本机实测过。为什么 `Option<TwoU64>` 跟 `TwoU64` 一样大，而 `Option<BigList>` 比 `BigList` 大 8 字节？

| 类型 | 大小 |
|---|---|
| `TwoU64`（两个变体载荷都是 `u64`） | 16 |
| `Option<TwoU64>` | 16 |
| `BigList`（`Cons(u64, Box<BigList>)` + `Nil`） | 16 |
| `Option<BigList>` | 24 |

<details><summary>参考答案</summary>

**一、** 两者在本机都是 16 字节。

- `Two`（默认表示）：实测偏移 `b` 0、`a` 8。编译器把 8 字节对齐的 `b` 放到前面，`a` 跟在后面，末尾补 7 字节，共 16 字节，对齐 8。
- `CTwo`（`repr(C)`）：实测偏移 `a` 0、`b` 8。算法按声明顺序走：`a` 在 0，`b` 对齐 8，从 1 补 7 字节到 8，占 8 到 16；结构体对齐 8，大小 16。

两者恰好同大小，但**这是巧合**（填充总量一样），不是保证：`a` 换到 `b` 后面就能省下那 7 字节的情况（比如上一节的 `Mixed`），默认表示才有优势。反过来说，任何「默认表示一定更小」或「默认表示一定按声明顺序」的想法都不成立——要确认只能量。

**二、**（a）满足。单字段结构体加 `#[repr(C)]` 时，那个 `u32` 就是偏移 0、对齐 4，整个结构体也是 4 字节对齐 4，跟 C 的 `uint32_t` 一致。（b）因为 `repr(C)` 给的是「C 的结构体布局规则」，恰好在这个形状下和单个 `u32` 重合；而 `repr(transparent)` 给的是**语义承诺**——布局与 ABI 都等同于那个字段，因此可以来回 `transmute`、可以跨 FFI 直接替换内层类型。用 `repr(C)` 是靠巧合，用 `repr(transparent)` 是靠保证；后者还能用在单变体枚举上，`repr(C)` 做不到同样的承诺。

**三、** 判别式占不占地方，取决于有没有空位可借。

`TwoU64` 的两个变体载荷都可能是任意 `u64`，一个无效位模式都没有，判别式只能实打实占字节：8 字节载荷 + 判别式与填充 = 16 字节。但这个独立判别式还留着大量没用的取值（两个变体只用了两个），`Option<TwoU64>` 的判别式正好塞进这些空位，于是还是 16 字节。

`BigList` 的 `Nil` 折进了 `Box<BigList>` 的空指针值，一个判别字节都不花（16 字节：`Box` 在偏移 0、`u64` 在偏移 8）。代价是那个空位被 `Nil` 独占，`Option<BigList>` 再没有空位可借，只能加自己的判别式，于是变成 24 字节。

结论不是「哪个更好」，而是「枚举布局是编译器算计的结果」：想看自己手上的枚举花了几个字节，量 `Option` 的大小是最快的探测手段之一。

</details>

## 延伸阅读

- 参考手册的布局总章，本章所有「保证」的出处：[Type layout](https://doc.rust-lang.org/reference/type-layout.html)
- Nomicon 讲默认表示的自由度与它换来的东西：[repr(Rust)](https://doc.rust-lang.org/nomicon/repr-rust.html)
- Nomicon 给出 `repr(C)`、`repr(transparent)`、`repr(u*)`、`repr(packed)` 的语义与坑：[Alternative representations](https://doc.rust-lang.org/nomicon/other-reprs.html)
- Nomicon 讲 DST、胖指针、零大小与无值类型：[Exotically Sized Types](https://doc.rust-lang.org/nomicon/exotic-sizes.html)
- 本章用到的工具都在一个模块里，文档有各条保证与限制：[std::mem](https://doc.rust-lang.org/std/mem/index.html)
- 两个配套入口：[std::mem::offset_of](https://doc.rust-lang.org/std/mem/macro.offset_of.html)、[std::alloc::Layout](https://doc.rust-lang.org/std/alloc/struct.Layout.html)（自己算或校验内存布局时用）
