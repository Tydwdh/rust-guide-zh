# 第 34 章 手写集合与智能指针

## 这一章解决什么问题

`Vec` 从第 6 章一直用到第 33 章，`Arc` 与 `Mutex` 从第 19 章用到项目 3。这一章把它们倒过来造一遍：手写一个 mini `Vec`（`RawVec` 式增长、`Drain` 守卫、ZST 惯例），一个简化版 `Arc`（引用计数 + `Acquire`/`Release` 配对），再加一把自旋 `Mutex`（`UnsafeCell` + `AtomicBool`）。目的不是重造标准库，而是把第 23 章立下的那套口径——安全 API 是外壳、不变量自己守住、每个 `unsafe` 块配一条 `// SAFETY:`——在最典型的场景里从头走到尾。

读完这一章，你应该能回答六个问题：

- 集合类型要守住哪几条不变量？为什么写入用 `ptr::write`、取走用 `ptr::read`，而不能 `v[i] = x` 或直接解引用？
- 容量为什么要翻倍？「`Vec` 的台阶是 8、16、32」这句话对吗？
- `Drain` 为什么必须是**守卫（guard）**？`mem::forget` 掉它为什么只是泄漏，不是 UB？
- ZST 为什么要把容量设成 `usize::MAX`？指针偏移在 ZST 上为什么是空操作？
- `Arc` 的克隆用 `Relaxed`、析构用 `Release` 加一次 `Acquire` fence，为什么不对称？
- 锁为什么必须用 `UnsafeCell` 装数据？`unsafe impl Send` / `Sync` 的边界怎么划？

边界先画清：智能指针与内部可变性（`Box`/`Rc`/`Cell`/`RefCell`）在第 17 章；布局、空位（niche）与量布局在第 25 章；`unsafe`、UB 与健全性在第 23 章；provenance 与别名模型在第 24、35 章；并发与内存序入门在第 19、20 章。本章直接用这些结论。

> [!NOTE]
> 本章的完整代码、测试与演示程序都在 `snippets/ch34/`，是一个零依赖的小 crate。正文里的片段是那份代码的原文，只是把与当前话题无关的部分略去了。片段之间互相引用（例如 `grow` 用到同文件里的 `RawVec` 字段），单独编译不过，所以标了 `ignore`；整份代码由下面的 `cargo test` 真跑验证。本章其余能独立成篇的代码块都真的被 `mdbook test` 编译执行。

## 为什么这样设计

### 心智模型：把不变量写在纸上

safe Rust 帮你维持一条隐含规则：每个内存位置要么放着一个合法值，要么不会被读到——编译器靠所有权与借用做到。而集合类型要「在运行期自行调度这块内存」，这条规则就得由你来维持。办法很朴素：**把不变量写成几条，每个 `unsafe` 块负责兑现其中一条。**

`MiniVec<T>` 只有三条：

1. `buf.ptr` 指向一块按 `Layout::array::<T>(buf.cap)` 分配出来的缓冲区（`cap == 0` 时它是 `NonNull::dangling()`，不指向任何分配）；
2. 前 `len` 个元素已初始化，第 `len` 个及其后**未初始化**；
3. `len <= buf.cap`。

第 2 条是全部麻烦的来源。「未初始化」不是「值是零」，而是一片还没有类型的原始字节，任何按 `T` 去读它的行为都是 UB（第 24 章）。所以集合里只有两处动作可以合法地改变「哪些位置已初始化」：写入与取走。

简化版 `Arc` 的不变量只有一条：**计数值等于活着的 `Arc` 个数**，于是「计数从 1 减到 0」那一次，就是唯一一次删除 `ArcInner` 的机会。

自旋 `Mutex` 的不变量也是一条：**`locked == true` 时恰好有一个线程持有守卫**，而它对 `data` 的读写与那个标记的原子操作配对。

### 为什么增长要翻倍

`push` 在 `len == cap` 时必须扩容。若每次只多要一格，$n$ 次 push 会搬移 $1 + 2 + \dots + n = \Theta(n^2)$ 个元素；翻倍则把均摊代价压到 O(1)。标准库源码把这条理由写在注释里：

> This guarantees exponential growth. The doubling cannot overflow because `cap <= isize::MAX` and the type of `cap` is `usize`.

它对应的两行是：

```rust,ignore
let cap = cmp::max(self.cap.as_inner() * 2, required_cap);
let cap = cmp::max(min_non_zero_cap(elem_layout.size()), cap);
```

（摘自 `library/alloc/src/raw_vec/mod.rs`，以 Rust 1.98 的源码为准。）注意第二行：std 还要给起步容量定一个下限，它按元素大小分三档：

```rust,ignore
const fn min_non_zero_cap(size: usize) -> usize {
    if size == 1 {
        8
    } else if size <= 1024 {
        4
    } else {
        1
    }
}
```

**但增长的台阶不是接口。** std 的文档说得很直白：

> Vec does not guarantee any particular growth strategy when reallocating when full, nor when `reserve` is called. The current strategy is basic and it may prove desirable to use a non-constant growth factor. Whatever strategy is used will of course guarantee O(1) amortized push.

译文：`Vec` 不保证任何特定的增长策略——无论用哪种策略，都保证 `push` 有 O(1) 的均摊代价。所以下面这张实测表的用途是「理解当前实现」，不是「抄下来当契约」。本机实测（Windows x86-64、Rust 1.98，连续 push 40 次，只记录容量发生变化的时刻）：

| 类型 | 容量台阶 |
|---|---|
| `Vec<u8>` | 8, 16, 32, 64 |
| `Vec<u64>` | 4, 8, 16, 32, 64 |
| `Vec<[u64; 300]>` | 1, 2, 4, 8, 16, 32, 64 |
| 本章的 `MiniVec<u32>` | 1, 2, 4, 8, 16, 32, 64 |

第一行与第三行的差别就是 `min_non_zero_cap`：元素 1 字节时下限是 8，元素大于 1 KiB 时下限是 1。**「std 的台阶是 8、16、32」是个常见误解**——它只在元素大小为 1 字节时成立。把同样的 40 次 push 画成「累计扩容次数」的台阶，差别一目了然：

<svg viewBox="0 0 760 200" width="100%" role="img" aria-label="两条台阶曲线：同样 40 次 push，MiniVec 累计扩容 7 次，std 的 Vec&lt;u8&gt; 累计扩容 4 次；两者都是翻倍增长，差别只在起步容量" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">同样 40 次 push 的累计扩容次数（本机实测，Windows x86-64、Rust 1.98）</text>
<line x1="112" y1="34" x2="140" y2="34" stroke="var(--links)" stroke-width="2" />
<text x="146" y="38" font-size="12" fill="currentColor">MiniVec&lt;u32&gt;：7 次</text>
<line x1="330" y1="34" x2="358" y2="34" stroke="currentColor" stroke-width="2" />
<text x="364" y="38" font-size="12" fill="currentColor">std Vec&lt;u8&gt;：4 次</text>
<text x="2" y="70" font-size="12" fill="currentColor" opacity="0.7">累计扩容次数</text>
<line x1="110" y1="62" x2="110" y2="170" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="102" y="174" font-size="12" fill="currentColor" text-anchor="end">0</text>
<text x="102" y="160" font-size="12" fill="currentColor" text-anchor="end">1</text>
<text x="102" y="145" font-size="12" fill="currentColor" text-anchor="end">2</text>
<text x="102" y="131" font-size="12" fill="currentColor" text-anchor="end">3</text>
<text x="102" y="117" font-size="12" fill="currentColor" text-anchor="end">4</text>
<text x="102" y="103" font-size="12" fill="currentColor" text-anchor="end">5</text>
<text x="102" y="88" font-size="12" fill="currentColor" text-anchor="end">6</text>
<text x="102" y="74" font-size="12" fill="currentColor" text-anchor="end">7</text>
<path d="M110,170 H125 V155.7 H140 V141.4 H155 V127.1 H185 V112.9 H245 V98.6 H365 V84.3 H605 V70 H710" fill="none" stroke="var(--links)" stroke-width="2" />
<path d="M110,170 H125 V155.7 H245 V141.4 H365 V127.1 H605 V112.9 H710" fill="none" stroke="currentColor" stroke-width="2" />
<line x1="110" y1="170" x2="710" y2="170" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="110" y="190" font-size="12" fill="currentColor" opacity="0.7">push 次数 0 … 40</text>
<text x="410" y="190" font-size="12" fill="currentColor" opacity="0.7">两条线都是「倍增长」；台阶更高只说明起步容量更小</text>
</svg>

### 为什么 `Drain` 必须是守卫

`drain()` 要把全部元素交出去，同时让缓冲区留着继续用——中间存在一个「元素已经不在 `Vec` 里、但还没被 drop」的窗口期。正确写法是让一个**守卫**临时持有那批元素：它的生命周期挂在 `&mut self` 上（借用检查器保证「取元素」与「再用 Vec」不会同时发生），欠下的清理由它的 `Drop` 兜底。

`mem::forget` 则一定是安全的：它只能让某个值的析构不发生。Nomicon 在 `drain` 那节把这条写成了注释：

```text
// this is a mem::forget safety thing. If Drain is forgotten, we just
// leak the whole Vec's contents.
```

「泄漏整段内容」是资源问题，不是 UB（分界在第 23 章）。反过来说：如果 `drain()` 不先把 `len` 归零，`mem::forget` 之后 `Vec` 会把同一批元素当成自己的再 drop 一遍——双重释放。**「forget 安全」不是白捡的，是设计里刻意买来的性质。**

### 为什么 `Arc` 把计数和数据放一起

引用计数必须跟着数据走：最后一个强引用离开时，要靠这个计数决定「现在删」。把计数与 `data` 放在同一块分配里，等于把「一次分配」变成「一次释放」，顺带让 `Arc` 保持一个指针大小。本机实测：

```text
size_of::<Arc<u128>>()         = 8
size_of::<Option<Arc<u128>>>() = 8
```

`Option` 没让它变大：`Arc` 内部是非空指针，空指针值正好给 `None` 用（空位优化，第 25 章）——这比「`Rc` 加一把锁」划算，因为 clone 只是一次原子加。

## 机制与规则

### 34.1 三条不变量与 `RawVec`

`Vec<T>` 其实由两半组成：`(ptr, cap)` 管缓冲区，`len` 管「缓冲区里有多少个已初始化的元素」。把前一半单独抽出来，`len` 的变化就不必同时盯容量了：

```rust,ignore
{{#include ../snippets/ch34/src/mini_vec.rs:rawvec}}
```

这段里有三处值得停一下。

**`new()` 里的 ZST 分支。** `cap` 直接给 `usize::MAX`，指针给 `NonNull::dangling()`。`ptr: NonNull<T>` 本身不携带「这块内存有多大」，容量只能由 `cap` 提供——两者必须一起改，这就是把它们放进同一个结构体的原因。

**`grow()` 里那条 `isize::MAX` 线。** `Layout::array::<T>(n)` 的文档说明是：

> On arithmetic overflow or when the total size would exceed `isize::MAX`, returns `LayoutError`.

也就是说，把错误交给 `Layout` 判断即可，不需要自己算乘法溢出。`grow()` 里真正会触发的失败路径有两条：容量翻倍溢出 `usize`、`Layout::array` 返回 `Err`；代码里另有一条 `size() <= isize::MAX` 判断，按 `Layout::array` 的契约不可达——它是防御性检查，注释里写明了这一点。

**ZST 该不该调用分配器。** `GlobalAlloc::alloc` 的 Safety 段写着：

> `layout` must have non-zero size. Attempting to allocate for a zero-sized layout will result in undefined behavior.

所以 ZST 的路径上绝不能走到 `alloc::alloc`：`new()` 把 `cap` 设成 `usize::MAX` 让「满」不可达，`grow()` 一开始就拦住 ZST。

有了这一半，`MiniVec` 的 `push` 与 `pop` 短得惊人：

```rust,ignore
{{#include ../snippets/ch34/src/mini_vec.rs:push-pop}}
```

`push` 里不能写 `v[len] = elem`：赋值会先把旧值 drop 掉，而那块内存要么未初始化、要么属于一个已经被 `pop` 取走的旧值——两者都会触发 UB。Nomicon 的说法是：

> The correct way to do this is with `ptr::write`, which just blindly overwrites the target address with the bits of the value we provide.

译文大意：正确做法是用 `ptr::write`，它只管把值的比特盲写过去，不读旧内容。`pop` 的对称动作是 `ptr::read`：把比特读出来、那块内存逻辑上回到未初始化，因此**读完之后必须让 `len` 不再覆盖它**——这也是 `len -= 1` 写在 `ptr::read` 之前就生效的原因。

### 34.2 增长台阶不是接口

34.1 的 `grow` 用的是教科书式的「翻倍」，起步容量 1；std 按元素大小分档起步，于是同样 40 次 push 少扩容几次（mini 7 次、std 的 `Vec<u8>` 4 次）。两种口径都合法，只是**只有 std 的那条写进了文档**——文档承诺的是「均摊 O(1)」，不是具体数字。用法上记住两条：要**依赖**确切容量就用 `with_capacity` / `reserve` 并读 `capacity()`，别猜 `push` 之后是几；别把台阶写成断言——换个元素大小数字就变，上面表里三行 `Vec` 已经演示过。

### 34.3 `Drain` 守卫：借用 + 收尾

`drain()` 把元素从 `Vec` 里「借走」，再交还给调用者。它必须先斩断 `MiniVec` 一侧的所有权：

```rust,ignore
{{#include ../snippets/ch34/src/mini_vec.rs:drain}}
```

注意三件事：`len` 在构造守卫**之前**就归零（forget 安全的地基）、返回类型带 `'_`（= `&mut self` 的生命周期，所以守卫活着的时候别人借不到 `self`）、`Drain::new` 是 `unsafe fn`——裸指针没有生命周期，那块内存的归属只能由文档里的 `# Safety` 段约定，而调用点用一条 `// SAFETY:` 兑现。这正是第 23 章那套口径的完整样例。

守卫本身要做三件事：迭代、ZST 特判、`drop` 收尾：

```rust,ignore
{{#include ../snippets/ch34/src/mini_vec.rs:drain-guard}}
```

`Drop` 里那行 `for _ in &mut *self {}` 就是收尾：把没被调用者取走的元素逐个读出来丢弃。三个结局合起来看，就是一个守卫能出的所有岔路：

<svg viewBox="0 0 760 240" width="100%" role="img" aria-label="Drain 守卫的生命周期：drain 之前缓冲区里 5 个已初始化元素归属 MiniVec；drain 之后 len 归零、守卫持有 start 到 end 之间的元素；守卫 drop 时把没取走的元素逐个析构，缓冲区仍归 MiniVec" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">Drain 守卫把一个 MiniVec 劈成两半：元素归守卫，缓冲区归 MiniVec</text>
<text x="0" y="54" font-size="12" fill="currentColor">drain() 之前</text>
<text x="0" y="72" font-size="12" fill="currentColor" opacity="0.7">len = 5</text>
<rect x="150" y="40" width="48" height="28" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="174" y="59" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e0</text>
<rect x="198" y="40" width="48" height="28" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="222" y="59" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e1</text>
<rect x="246" y="40" width="48" height="28" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="270" y="59" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e2</text>
<rect x="294" y="40" width="48" height="28" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="318" y="59" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e3</text>
<rect x="342" y="40" width="48" height="28" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="366" y="59" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e4</text>
<text x="398" y="59" font-size="12" fill="currentColor" opacity="0.7">第 5 个元素之后是未初始化（cap &gt; len 时）</text>
<text x="0" y="118" font-size="12" fill="currentColor">drain() 之后</text>
<text x="0" y="136" font-size="12" fill="currentColor" opacity="0.7">len = 0，守卫持有 [start, end)</text>
<rect x="342" y="104" width="48" height="28" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="366" y="123" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e4</text>
<rect x="294" y="104" width="48" height="28" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="318" y="123" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e3</text>
<rect x="246" y="104" width="48" height="28" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="270" y="123" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e2</text>
<rect x="198" y="104" width="48" height="28" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="222" y="123" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e1</text>
<rect x="150" y="104" width="48" height="28" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="174" y="123" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">e0</text>
<text x="146" y="145" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">start</text>
<text x="394" y="145" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">end</text>
<text x="0" y="176" font-size="12" fill="currentColor">守卫 drop 的三种结局</text>
<text x="0" y="196" font-size="12" fill="currentColor" opacity="0.7">① 元素全被取走：缓冲区原样留给 MiniVec，接着 push 不用重新分配</text>
<text x="0" y="214" font-size="12" fill="currentColor" opacity="0.7">② 只取了一部分就丢掉守卫：Drop 把剩下的元素逐个读出来 drop（欠的账还清）</text>
<text x="0" y="232" font-size="12" fill="currentColor" opacity="0.7">③ mem::forget(守卫)：元素泄漏；len 早已归零，所以不会被重复 drop（不是 UB）</text>
</svg>

另外，`MiniVec` 自己的 `Drop` 也复用这个守卫（`let _ = self.drain();`），于是元素按从前到后的顺序析构，与 std 的 `Vec` 一致。

### 34.4 ZST：三条惯例

ZST 全靠约定撑着。Nomicon 把两条硬约束摆在开头：分配器 API 对大小 0 的申请是 UB；**指针偏移在 ZST 上是空操作**。三条惯例由此而来。

**一、容量用 `usize::MAX` 当哨兵。** std 文档的承诺是：

> For `Vec<T>` where `T` is a zero-sized type, there will be no allocation and the capacity will always be `usize::MAX`.

译文：当 `T` 是零大小类型时，不分配内存，容量恒为 `usize::MAX`。于是 `Vec<()>` 永远不会满，「满了就扩容」这条路径天然不可达；反过来，`RawVec::grow` 一旦在 ZST 上被调用，就说明真的溢出了，可以直接 panic——这条承诺也推翻了 `Vec` 里「靠内存耗尽来兜住溢出」的旧假设。本机实测 `Vec<()>::new().capacity()` 就是 `usize::MAX`：

```rust
fn main() {
    let mut v: Vec<()> = Vec::new();
    assert_eq!(v.capacity(), usize::MAX);
    v.push(());
    assert_eq!(v.len(), 1);
    assert_eq!(v.capacity(), usize::MAX);
    println!("{}", v.capacity());
}
```

```text
18446744073709551615
```

**二、`NonNull::dangling()` 一个值兼两份差事。** 它同时表示「还没分配」与「零大小分配」。判定能不能释放内存要同时看两件事：`cap != 0 && size_of::<T>() != 0`——只写 `cap != 0` 会把 ZST 的哨兵当成分配过。

**三、`ptr::read` / `ptr::write` 在 ZST 上不看指针。** Nomicon 的表述是：ZST 只有一个值，因而没有状态需要存取，`ptr::read` 与 `ptr::write` 实际上不会去碰那个地址。这正是 `MiniVec::push` 对 ZST 也照样成立的原因。但迭代器不行：`start.add(1)` 在 ZST 上等于没动，计数器永远追不上终点。所以 ZST 路径改走整数加法（`Drain::new` 算终点、`next` 推进起点）：

```rust,ignore
{{#include ../snippets/ch34/src/mini_vec.rs:zst-ptr}}
```

本机实测：往 `MiniVec<()>` 里 push 一万次，容量始终是 `usize::MAX`，一次分配都没有（`tests/mini_vec.rs` 的 `zst_capacity_is_usize_max_and_pushing_never_allocates` 就在断言这件事）。

### 34.5 简化版 `Arc`

`Arc` 的结构比 `Vec` 简单：一个非空指针加一个 `PhantomData`。

```rust,ignore
{{#include ../snippets/ch34/src/mini_arc.rs:layout}}
```

三处设计都有理由：

- **数据与计数同块分配。** `ArcInner<T>` 里 `rc` 在前、`data` 在后，一次分配、一次释放。
- **`NonNull<T>` 而不是 `*mut T`。** `*mut T` 在 `T` 上**不变（invariant）**，会让 `Arc` 过于严格（第 16 章的 variance）；drop checker 那半条由 `PhantomData<ArcInner<T>>` 补上。
- **`unsafe impl Send/Sync` 的边界。** `T: Send + Sync` 一个都不能少：共享的是 `&T`（要 `Sync`），而 `Arc` 自己要被送走、可能带着 `T`（要 `Send`）。少了任何一条，`Arc<Rc<u32>>` 就能跨线程，`Rc` 的计数立刻变成数据竞争。

克隆与析构是本章唯一需要**逐字读内存序**的地方：

```rust,ignore
{{#include ../snippets/ch34/src/mini_arc.rs:clone}}
```

```rust,ignore
{{#include ../snippets/ch34/src/mini_arc.rs:drop}}
```

为什么不都用 `SeqCst`？因为这里的两处需求本来就不对称：

- **克隆只要原子性，不要顺序。** 克隆不读 `data`，也不指望看见谁的写。`Relaxed` 提供原子读改写，代价最低。
- **析构要在「最后一次引用消失」这个瞬间建立起先后关系。** 减计数用 `Release`：它把本线程对 `data` 的写（可能来自共享的 `Mutex<T>`）发布出去；最后一个线程再用一次 `Acquire` fence 把那些写接住。std 源码里 `impl Drop for Arc` 的注释把这条链子说得最清楚：

> This fence is needed to prevent reordering of use of the data and deletion of the data. Because it is marked `Release`, the decreasing of the reference count synchronizes with this `Acquire` fence.

紧接着还有一句解释为什么「不是最后一个就不需要同步」：

> Because `fetch_sub` is already atomic, we do not need to synchronize with other threads unless we are going to delete the object.

译文大意：`fetch_sub` 本身就是原子的，所以除非这一次就是要删除对象，否则不需要与别的线程同步。这正是 `if ... != 1 { return; }` 写在 fence **之前**的原因——顺序在这里不是风格问题，是语义。

**`Weak` 不阻断释放。** 本章的 `MiniArc` 没有 `Weak`（那要第二个计数器），但行为可以用 std 的 `Arc` 看清楚：`Weak` 让那块分配（两个计数）活到最后一个弱引用消失，`data` 却该析构就析构——强引用归零那一刻值就没了，`upgrade()` 从此给 `None`。另一个容易误读的数字：std 文档写明没有强引用时 `weak_count` 返回 0（大意如此）——演示程序的输出里两点连在一起。

> [!WARNING]
> 本章没跑 Miri，也没跑 loom。Miri 只在 nightly 提供（本机只有 stable 1.98），所以只给文档化的命令、不给输出：
>
> ```console
> $ rustup +nightly component add miri
> $ cargo +nightly miri test --manifest-path snippets/ch34/Cargo.toml
> ```
>
> 更系统的做法是 `loom`（把线程交错穷举出来）——本章同样没跑，只用真线程压测（4 × 20 000 次累加）加人工审读内存序。**写完 unsafe 并发代码而没跑过这两样里任何一样，就等于没验证过内存序**；`cargo test` 全绿只说明「在这个平台上这一次没翻车」。

### 34.6 自旋 `Mutex`

锁要解决两件事：让 `&self` 的持有者能改数据，且同一时刻只有一个。前半句要 `UnsafeCell`，后半句要原子标记：

```rust,ignore
{{#include ../snippets/ch34/src/mini_mutex.rs:guard}}
```

`UnsafeCell` 的文档把它的地位说得很明白：

> `UnsafeCell<T>` opts-out of the immutability guarantee for `&T`: a shared reference `&UnsafeCell<T>` may point to data that is being mutated.

同一段文档紧接着提醒：

> `UnsafeCell` does nothing to avoid data races; they are still undefined behavior.

译文：`UnsafeCell` 不负责避免数据竞争——竞争仍然是 UB。**它只解开「共享引用指向不可变数据」这一条假设，不提供任何同步。** 同步是你在这之上用原子操作搭的，而且两半各有分工：抢锁用 `Acquire`，它与上一个解锁者的 `Release` 配对，保证对方写进 `data` 的值对刚拿到锁的我可见；解锁用 `Release`，保证我对 `data` 的写排在「把标记写回 `false`」之前——下一个抢到锁的人因此看得见。两者缺一，就是第 20 章说的那种「看不到别人的写」的数据竞争。

几个边界要说清：

- **`Mutex<T>` 的 `Sync` 边界是 `T: Send`，不是 `T: Sync`。** 共享 `&Mutex<T>` 是为了让别的线程**独占**地拿到 `&mut T`，所以只要 `T` 能被送到别的线程（`Send`）就够。
- **`MutexGuard<T>` 的 `Sync` 边界是 `T: Sync`，而且必须显式写下来**（下面代码块最后那行 `unsafe impl`）。守卫的 `Deref` 会发出 `&T`，所以「`&MutexGuard` 跨线程共享」只有在 `T: Sync` 时成立。不写它，守卫的 `Sync` 会从字段 `&'a Mutex<T>` 自动推导、边界退化成 `T: Send`：`MutexGuard<Cell<u32>>` 于是成了 `Sync`，两个线程各拿一个 `&Cell<u32>` 去写——本机实测（补这一行之前）各写 200 万次，最终只到 2355873。补上之后，那种共享写法立刻变成 `E0277`（note 点名 `MutexGuard<'_, Cell<u32>>` 不实现 `Sync`）。**自动推导只看字段能不能共享，看不见 `Deref` 里那条路——这正是必须手工收窄边界的场合。**
- **锁守卫不必是 `!Send`。** std 的 `MutexGuard` 显式标了 `!Send`（文档原话：`A MutexGuard is not Send to maximize platform portability.`）；我们的自旋版没这个限制，因为解锁只是一个原子写，谁执行都行。
- **没有锁中毒（poisoning）。** 持锁线程 panic 后 std 的 `lock` 返回 `Err`，我们不会——有意的简化（第 19 章讲过锁中毒）。自旋也只适合极短临界区，高竞争场景请用 std 的实现。

## 动手看看

`snippets/ch34/` 是零依赖小 crate，三个模块对应上面三块内容。第一个演示程序把增长台阶、ZST、布局与 `drain` 一起打出来：

```rust,ignore
{{#include ../snippets/ch34/examples/vec_steps.rs}}
```

```console
$ cargo run --quiet --manifest-path snippets/ch34/Cargo.toml --example vec_steps
```

```text
== std::vec::Vec 的容量台阶（40 次 push，去重后）==
Vec<u8>       : [8, 16, 32, 64]
Vec<u64>      : [4, 8, 16, 32, 64]
Vec<[u64;300]>: [1, 2, 4, 8, 16, 32, 64]

== MiniVec 的容量台阶（同一次数）==
MiniVec<u32>  : [1, 2, 4, 8, 16, 32, 64]

== ZST ==
Vec::<()>::new().capacity() = 18446744073709551615
std    push 10 次: cap = 18446744073709551615, len = 10
MiniVec push 10 次: cap = 18446744073709551615, len = 10

== 布局（第 25 章的量法）==
size_of::<Vec<u8>>()                     = 24
size_of::<Option<Vec<u8>>>()             = 24
size_of::<MiniVec<u8>>()                 = 24
size_of::<Option<MiniVec<u8>>>()         = 24
size_of::<MiniVec<()>>()                 = 24
size_of::<Arc<u128>>()                   = 8
size_of::<Option<Arc<u128>>>()           = 8
size_of::<MiniArc<u128>>()               = 8
size_of::<Option<MiniArc<u128>>>()       = 8

== 字节转储：MiniVec<u8> 的 3 个机器字（第 25 章的读法）==
API 读数    : cap = 4, len = 3
三个机器字  : word0 = 首元素地址吗 -> true
              word1 = 4, word2 = 3
缓冲区字节  : [AA, BB, CC]（只读 len 个：容量是 4，第 4 个字节还没初始化）

== drain 只借出元素，不还缓冲区 ==
drain 之前: len = 5, capacity = 8
drain 出来: [0, 1, 2, 3, 4]
drain 之后: len = 0, capacity = 8（缓冲区照旧）
缓冲区还能用: [99]
```

读数对照第 25 章的结论：

- `MiniVec<u8>` 与 `Vec<u8>` 都是 24 字节 = 3 个机器字（指针 + 容量 + 长度），`Option<...>` 也是 24 字节——空指针那个空位被借用成了 `None`；
- 字节转储印证了不变量：`word0` 是首元素地址、`word1 = 4` 是容量、`word2 = 3` 是长度（**顺序由 `repr(Rust)` 决定，是本机这一次的结果**）；缓冲区只有前 `len` 字节可读，第 4 个字节虽在容量范围内但还没初始化；
- `Arc<u128>` 只有 8 字节，说明计数确实和 `data` 挤在同一块分配里。

第二个演示程序把引用计数与互斥锁串起来：

```rust,ignore
{{#include ../snippets/ch34/examples/arc_mutex.rs}}
```

```console
$ cargo run --quiet --manifest-path snippets/ch34/Cargo.toml --example arc_mutex
```

```text
== 简化版 Arc：计数与释放时机 ==
new       : strong_count = 1
clone     : strong_count = 2
drop 一份 : strong_count = 1
    [drop] mini Arc 的数据
强引用清零后，上面那行 [drop] 就是析构

== MiniArc<Mutex<T>>：多线程累加 ==
4 个线程 × 50000 次 = 200000
线程退出后 strong_count = 1

== 对照：std 的 Weak 不阻止释放 ==
建好 Weak  : strong_count = 1, weak_count = 1
再 clone 一份: strong_count = 2
    [drop] std Arc 的数据
强引用清零 : upgrade() 拿得到吗 -> false
Weak 还在  : weak_count = 0
```

三条读数：

- 计数到 0 之前，`[drop]` 一行都不打——释放发生在**最后一个**强引用离开时，而不是「第一个」；
- 4 个线程各加 5 万次，结果正好 20 万——锁的配对在真实竞争下成立（但这只证明这一次跑对了，严格验证见上面的警告框）；
- `Weak` 那一组：强引用归零后 `upgrade()` 给 `None`（数据已经没了），`weak_count` 显示 0——那是文档约定的读法（没有强引用时为 0），并不表示 `Weak` 已被释放。`Weak` 的作用是「不阻止释放，但能感知释放」，第 17 章的父子目录结构就是这个用法。

三个模块自己的测试也在同一个 crate 里（开头 `Finished …` 一行，以及 `Running unittests` 与 `Doc-tests` 那两段 `0 tests` 已略去）：

```console
$ cargo test --manifest-path snippets/ch34/Cargo.toml
```

```text
     Running tests\mini_arc_mutex.rs (snippets\ch34\target\debug\deps\mini_arc_mutex-9097e61abfd4dfd5.exe)

running 6 tests
test clone_bumps_the_count_and_drop_lowers_it ... ok
test lock_is_released_when_the_guard_drops ... ok
test data_is_dropped_exactly_once_when_the_last_arc_goes ... ok
test mut_borrow_through_guard_writes_are_visible_after_a_sync_point ... ok
test threads_share_one_counter_through_arc_mutex ... ok
test guard_is_exclusive_while_held ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests\mini_vec.rs (snippets\ch34\target\debug\deps\mini_vec-72deb703ab0e5e92.exe)

running 11 tests
test capacity_grows_by_doubling ... ok
test drain_drop_cleans_up_what_was_left ... ok
test insert_and_remove_keep_order ... ok
test drop_drops_every_element_exactly_once_in_order ... ok
test drain_moves_elements_out_and_keeps_the_buffer ... ok
test layout_matches_the_same_three_words_as_std ... ok
test zst_capacity_is_usize_max_and_pushing_never_allocates ... ok
test push_pop_is_lifo ... ok
test zst_reads_through_deref_work ... ok
test forgetting_the_drain_leaks_instead_of_double_dropping ... ok
test moves_across_threads_and_is_shareable_behind_an_arc ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

（`test … ok` 的打印顺序与 `finished in` 的耗时会随运行变化；`Running …` 行里的哈希由 crate 内容与工具链决定，同一次提交重跑逐字一致。两份二进制合计 17 个测试，全绿。）

其中 `forgetting_the_drain_leaks_instead_of_double_dropping` 是本章最值得看的一条：它用 `DropCounter` 记账，先确认「被 forget 掉的守卫不 drop 任何元素」，再确认「`MiniVec` 自己也不 drop 它们」——泄漏，但没有第二次析构。

## 报错现场

### 现场一：`Drop` 类型里不能把字段搬出来（E0509）

写「把 `Vec` 拆成两半」的转换函数时，第一反应通常是直接搬字段：

```rust,compile_fail
pub struct MiniVec<T> {
    buf: Vec<T>,
    len: usize,
}

impl<T> Drop for MiniVec<T> {
    fn drop(&mut self) {}
}

impl<T> MiniVec<T> {
    fn into_parts(self) -> (Vec<T>, usize) {
        (self.buf, self.len)
    }
}

fn main() {}
```

```text
error[E0509]: cannot move out of type `MiniVec<T>`, which implements the `Drop` trait
  --> src/main.rs:12:10
   |
12 |         (self.buf, self.len)
   |          ^^^^^^^^
   |          |
   |          cannot move out of here
   |          move occurs because `self.buf` has type `Vec<T>`, which does not implement the `Copy` trait
```

（以 Rust 1.98 为例；`error: aborting…` 与 `For more information…` 两行按本书约定略去。）原因是析构函数会在返回值之后继续跑，如果字段已经被搬走，`drop` 就得面对一个残缺的 `self`。这也是 Nomicon 在实现 `IntoIter` 时用 `ptr::read(&self.buf)` 加 `mem::forget(self)` 的原因：**要么让类型不长尾巴（用 `ManuallyDrop`），要么用裸指针读走字段、再把原值 forget 掉。**

### 现场二：`drain()` 与借用冲突（E0502）

守卫的价值在报错里看得最清楚——`drain()` 借走的是 `&mut self`，所以「先拿一个元素的引用、再 drain」必然被拦下：

```rust,compile_fail
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    let mut d = v.drain(..);
    println!("{first} {:?}", d.next());
}
```

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:17
  |
3 |     let first = &v[0];
  |                  - immutable borrow occurs here
4 |     let mut d = v.drain(..);
  |                 ^^^^^^^^^^^ mutable borrow occurs here
5 |     println!("{first} {:?}", d.next());
  |                ----- immutable borrow later used here
```

（以 Rust 1.98 为例；末尾两行略去。）这正是守卫存在的意义：`d.next()` 拿到的元素与 `v` 的重用不可能同时发生——**守卫首先是给借用检查器看的**，`Drop` 收尾才是运行时那一半。修法也在这里：别先存 `&v[0]`，要值就先拷出来（`let first = v[0];`），或者干脆用 `drain()` 的迭代器边取边用——`for x in v.drain(..)` 里借用只覆盖迭代期间。

### 现场三：容量算不出来（`LayoutError` 与 `capacity overflow`）

`grow()` 里那条 `isize::MAX` 的线是运行时检查，`Layout::array` 先替我们算：

```rust
use std::alloc::Layout;

fn main() {
    println!("{:?}", Layout::array::<u64>(usize::MAX));
    println!("{:?}", Layout::array::<()>(usize::MAX).map(|l| (l.size(), l.align())));
}
```

```text
Err(LayoutError)
Ok((0, 1))
```

第一条是「元素个数乘元素大小超出 `isize::MAX`」；第二条是 ZST 特例：数组永远算得出来——但**算得出来不等于能拿去 `alloc`**（大小 0 的分配是 UB，见 34.1）。std 的 `Vec` 把这条线变成用户可见的 panic：

```console
$ rustc --edition 2024 src/main.rs -o overflow.exe
$ .\overflow.exe
```

```text
thread 'main' panicked at library\alloc\src\raw_vec\mod.rs:28:5:
capacity overflow
```

（节选：panic 行里的线程号与 `/rustc/<hash>/` 前缀、以及末尾那行 ``note: run with `RUST_BACKTRACE=1` …`` 都已略去；`Vec::with_capacity(usize::MAX)`，以 Rust 1.98 为例。）我们的 `grow()` 里那句 `panic!("capacity overflow")` 就是同一个原因。

## 小结

- **安全外壳 + 不变量。** `MiniVec` 三条（缓冲区按 `cap` 分配、前 `len` 个已初始化、`len <= cap`）、`Arc` 一条（计数 = 活着的引用数）、`Mutex` 一条（`locked` 为真时恰有一个守卫）。每个 `unsafe` 块都该能指着其中一条说「我兑现的是它」。
- **未初始化不是零。** 写入用 `ptr::write`、读走用 `ptr::read`，`ptr::copy` 用于重叠区间；`=` 与解引用都会先按 `T` 求值，那是 UB 的入口。
- **增长是策略，不是接口。** `push` 均摊 O(1)，台阶由实现决定：std 按元素大小分档起步，本章的 mini 版从 1 翻倍。要确切容量就 `with_capacity`，别猜。
- **守卫把「借用」和「收尾」绑在一起。** `drain()` 先归零 `len`（forget 安全），`Drain::drop` 兜住没取走的元素；`mem::forget` 的代价是泄漏，不是 UB。
- **ZST 靠三条惯例。** 容量 `usize::MAX`、`NonNull::dangling()` 兼两职、读写不看指针；迭代器必须走整数计数，因为指针偏移在 ZST 上是空操作。
- **`Arc` 与 `Mutex` 都是「安全外壳包住原子操作」。** `Arc` 的克隆用 `Relaxed`（只要原子性）、析构用 `Release` 加 `Acquire` fence（要在最后一个引用消失时建立先后关系），`T: Send + Sync` 一条都不能省；`Mutex` = `UnsafeCell` + 原子标记，抢锁 `Acquire`、解锁 `Release`，`Sync` 的边界是 `T: Send`。**守卫另算：`MutexGuard` 的 `Sync` 边界是 `T: Sync`（它要发 `&T`），必须手工写明。** 这类代码的严格验证要靠 Miri/loom（本章只跑了真线程压测）。

## 练习

### 练习 1：为什么不能写 `*ptr = elem`

`MiniVec::push` 里如果把这行

```rust,ignore
ptr::write(self.buf.ptr.as_ptr().add(self.len), elem);
```

换成

```rust,ignore
*self.buf.ptr.as_ptr().add(self.len) = elem;
```

第一次 push 时会出什么问题？在「已经 push 过两个元素、第三个位置是未初始化」的情况下又会怎样？

<details><summary>参考答案</summary>

两个时刻都错，但性质可以分开说。

`*ptr = elem` 是一次「给已初始化的位置赋值」：赋值本身会先把该位置上的旧值 drop 掉。

1. 第一次 push（`len == 0`）时，那个位置是分配器刚给回来的原始字节；push 过两个之后，第三个位置同样还没初始化。把它当成 `T` 来 drop，就是读未初始化内存 + 对一个不存在的值调用析构——UB。
2. 还有一种「看起来没问题」的情况：位置被 `pop` 读过、逻辑上未初始化，但残留着旧值的比特。此时 `*ptr = elem` 会把残留比特当成合法值 drop 一次——**同一个值被 drop 两次**。

`ptr::write` 避开的正是这件事：不读旧值、不调用析构。反过来，`ptr::write` 用在「旧值还需要析构」的位置上就是另一种错——旧值被直接覆盖（泄漏，不是 UB），要先 `drop_in_place`。**动作要和内存状态对上：未初始化写用 `write`，换掉已初始化的值用 `drop_in_place` 加 `write`。**

</details>

### 练习 2：找出这个 `drain()` 的 bug

下面这个 `drain` 只差一行就正确。缺的是哪一行？缺了它，什么情况下会出 UB？

```rust,ignore
pub fn drain(&mut self) -> Drain<'_, T> {
    let len = self.len;
    // 缺的那一行在这里
    // SAFETY: buf.ptr 指向 cap >= len 个元素的缓冲区，且前 len 个已初始化；
    // len 刚被设成 0，MiniVec 自己不会再碰这些元素。
    unsafe { Drain::new(self.buf.ptr.as_ptr(), len) }
}
```

<details><summary>参考答案</summary>

缺的是 `self.len = 0;`。

少了它，`MiniVec` 仍然认为前 `len` 个元素归自己所有，于是同一批元素会被算两次：

- **守卫正常 drop（最常见）**：守卫把元素交给调用者之后，`MiniVec::drop` 又按残留的 `len` 把它们读出来 drop 一遍——同一个值析构两次（对持有堆数据的类型就是双重释放），UB。
- **只取了一部分就 `mem::forget` 守卫**：已经交出去的那几个会在 `MiniVec::drop` 里再被 drop 一次，同样是 UB。
- **一个都没取就 `mem::forget`**：这批元素只被 `MiniVec` drop 一次，这次看不出问题——但那是碰巧，`len` 已经不再描述这块缓冲区的内容了。

所以那一行不是优化，而是守卫设计的基石：**先把所有权从 `Vec` 一侧斩断，再构造守卫**。反过来（先构造守卫、再归零）在单线程下通常也对，但中间一旦 panic 就说不清了——让不变量在每个中间点都成立才是安全的写法。

</details>

### 练习 3：预测编译结果

下面的 `spawn` 能不能编译？为什么？（`MiniArc` 就是本章的实现。）

```rust,ignore
use std::cell::Cell;
use ch34_snippets::mini_arc::MiniArc;

let shared = MiniArc::new(Cell::new(0u32));
std::thread::spawn(move || {
    shared.set(1);
});
```

<details><summary>参考答案</summary>

**不能编译**，报 `E0277`，关键几行是（本机实测节选，以 Rust 1.98 为例）：

```text
error[E0277]: `Cell<u32>` cannot be shared between threads safely
    = help: the trait `Sync` is not implemented for `Cell<u32>`
    = note: required for `MiniArc<Cell<u32>>` to implement `Send`
```

理由在 `Arc` 的 `Send` 实现上：`unsafe impl<T: Send + Sync> Send for MiniArc<T>`。`Cell<u32>` 是 `Send`（能送走），但**不是 `Sync`**——它允许通过 `&Cell` 修改内容，两个线程共享同一个 `Cell` 会直接构成数据竞争。`MiniArc` 跨线程时必须保证「共享出来的 `&T` 是安全的」，所以要求 `T: Sync`；`Cell` 过不了这道门，`MiniArc<Cell<u32>>` 也就不是 `Send`。

两条延伸：

- 想要「多线程共享可变状态」，把 `Cell` 换成 `Mutex<u32>`（本章的 `Mutex<T>` 在 `T: Send` 时 `Sync`），或者换成 `AtomicU32`。前者靠锁排除并发访问，后者靠原子操作，两者的代价不一样。
- 这也是 std 的 `Arc` 与 `Rc` 在 `Send`/`Sync` 上给出不同边界的原因：`Rc` 的计数不是原子的。手写 `unsafe impl` 时把边界划松，就是把安全保证漏出去。

</details>

## 延伸阅读

- Nomicon 手写 `Vec` 全程：<https://doc.rust-lang.org/nomicon/vec/vec.html>
- Nomicon 手写 `Arc`：<https://doc.rust-lang.org/nomicon/arc-mutex/arc-and-mutex.html>
- `std::vec::Vec`（「Capacity and reallocation」一节）：<https://doc.rust-lang.org/std/vec/struct.Vec.html>
- `std::alloc::Layout`：<https://doc.rust-lang.org/std/alloc/struct.Layout.html>
- `std::alloc::GlobalAlloc`（分配器的 Safety 契约）：<https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html>
- `std::cell::UnsafeCell`（含别名规则一节）：<https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html>
- `std::sync::atomic`（原子类型与 `Ordering`）：<https://doc.rust-lang.org/std/sync/atomic/index.html>
- `std::sync::Arc`（`Weak`、`strong_count`、`weak_count`）：<https://doc.rust-lang.org/std/sync/struct.Arc.html>
- `std::ptr`（`read`/`write`/`copy`/`NonNull`）：<https://doc.rust-lang.org/std/ptr/index.html>
- Reference：行为被视作未定义的情形：<https://doc.rust-lang.org/reference/behavior-considered-undefined.html>
