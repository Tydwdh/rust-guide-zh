# 第 20 章 原子与内存序

## 这一章解决什么问题

第 19 章把并发问题变成了「加锁」或「传消息」两件事。锁的语义好懂（同一时刻只有一个线程进临界区），但这条保证很贵：要排他、要排队，还要处理持锁线程 panic 之后的锁中毒（poisoning）。

有些共享状态不需要这么重：一个统计请求数的计数器、一个「初始化完成」的标志、一个可以被无锁（lock-free）读到的快照指针、一个自旋锁（spinlock）。它们要的是两件比锁轻得多的事：

- **一步完成的读-改-写（read-modify-write，RMW）**：如果 `x += 1` 被拆成「读 x」「写 x + 1」两步，两个线程就会互相覆盖——结果是少加几次，而且每次运行少的次数都不一样；
- **说清可见性**：A 线程写了数据，B 线程何时、以什么顺序能看到它。

第 19 章的 `Mutex` 把这两件事一起给你，代价是排他；原子类型（atomic type）则把同样两件事拆开卖：**原子性**有固定保证，**可见性**由你选的**内存序（memory ordering）**决定。

读完这一章，你能：

- 用 `AtomicUsize`、`AtomicBool` 这类类型做无锁计数与标志，不引入锁；
- 说出 `Ordering` 五个值各自保证什么、能配哪种操作、错配会得到编译错误还是 panic；
- 解释「Release 写 + Acquire 读」为什么能建立先行发生（happens-before），以及把任意一半换成 `Relaxed` 之后哪个环节失去了保证；
- 读懂 `compare_exchange` 循环，并知道 `_weak` 版本的伪失败为什么必须放在循环里；
- 说明原子类型内部为什么需要 `UnsafeCell`，以及它凭什么可以是 `Sync` 的；
- 判断一个需求值不值得手写无锁结构。

本章只讲原子类型与内存序。`thread`、`Send`/`Sync` 的系统讲解、channel、锁中毒在第 19 章；性能测量与优化在第 29 章；用 `loom` 做并发模型检查在第 31 章；别名模型与形式化语义在第 35 章。

## 为什么这样设计

### 硬件和编译器都在动你的代码

你在源码里写的顺序，不是机器执行的顺序。三层都会重排：

1. **编译器**可以把不相干的访存挪来挪去，也可以把值长时间留在寄存器里；
2. **CPU** 在执行时乱序，写操作先进存储缓冲（store buffer）再慢慢刷进缓存，别的核可能在缓冲还没排空前就看到了旧值；
3. **缓存一致性协议**保证最终大家看到同一份数据，但「最终」是多久、以什么顺序，各架构给的答案不同。Nomicon 的总结是：*Most notably x86/64 provides strong ordering guarantees, while ARM provides weak ordering guarantees.*

所以「我在 A 线程先写数据、再写标志，B 线程看到标志后就能看到数据」这句话**不是**语言层面自动成立的，它需要在两边各写一个带特定内存序的操作，由语言和机器一起保证。

Rust 的做法不是发明新规则，而是复用已经成熟的那一套。标准库模块文档开门见山：

> Rust atomics currently follow the same rules as C++20 atomics, specifically the rules from the `intro.races` section, without the “consume” memory ordering.

也就是说，C++ 的 `std::atomic` 与 `memory_order_*` 的知识可以直接搬过来（少一个 `memory_order_consume`）。这也是本章把「内存序」讲清楚的原因：五值的名字、配对方式、什么时候必须用 `SeqCst`，都是同一套词汇。

### 为什么默认不用锁，而用「原子 + 你选序」

语言设计上有两种极端：**全给**（所有共享访问都按顺序一致性（sequential consistency）处理，写起来简单、跑起来慢）和**全不给**（你自己保证，出错就是未定义行为）。Rust 选了中间：安全的原子类型覆盖最常用的几个整数与布尔，操作本身永不撕裂、永不丢更新；至于「顺序」，交给调用点显式选择。

代价是显式：每个原子操作多一个 `Ordering` 参数。收益是你可以写出「只在需要的地方付钱」的代码——引用计数只需要 `Relaxed`，发布数据用 `Release`/`Acquire`，真要全序才上 `SeqCst`。这个参数看起来啰嗦，但它是本章唯一必须记住的东西。

### 为什么数据竞争必须是未定义行为

如果允许「不加同步地读写共享内存」有个确定结果，编译器就没法做任何优化：任何把值留在寄存器里的决定都可能改变程序语义。所以 Rust 与 C++ 一样，把它定义为未定义行为（UB, undefined behavior）。标准库的措辞是：

> The most important aspect of this model is that *data races* are undefined behavior. A data race is defined as conflicting non-synchronized accesses where at least one of the accesses is non-atomic. Here, accesses are *conflicting* if they affect overlapping regions of memory and at least one of them is a write.

三个关键词：**冲突**（区域重叠且至少一方是写）、**未同步**（没有 happens-before 关系）、**至少一方非原子**。注意最后一条：两个原子操作之间即使没有任何顺序保证，也不构成数据竞争——只会有「谁先谁后说不准」的语义问题，不会有 UB。这正是原子类型能安全地用于并发的原因，也是本章为什么先讲原子性、再讲内存序。

## 机制与规则

### 有哪些原子类型

标准库给了布尔、整数和指针三类原子：

| 类型 | 备注 |
|---|---|
| `AtomicBool` | 一位；`AtomicBool::new` 可在 `static` 里用（`const fn`） |
| `AtomicU8`…`AtomicI64`、`AtomicUsize`/`AtomicIsize` | 每个位宽一组；`Usize`/`Isize` 跟着目标平台 |
| `AtomicU128`/`AtomicI128` | 平台支持时才有（`#[cfg(target_has_atomic = "128")]`） |
| `AtomicPtr<T>` | 原子地读换一个 `*mut T`，无锁栈、一次性初始化的底座 |

两点必须知道：

- **不是所有平台都有所有位宽。** 标准库用 `#[cfg(target_has_atomic = "…")]` 这类按位宽开关控制可用性（键是 `target_has_atomic`，可取 `"8"`、`"16"`、`"32"`、`"64"`、`"128"`、`"ptr"`）。模块文档点名的例外包括：32 位指针的 PowerPC 与 MIPS 没有 `AtomicU64`/`AtomicI64`；`thumbv6m-*` 与 `thumbv8m.base-*` 只有 `load`/`store`，没有任何 CAS（compare-and-swap，比较并交换）操作。模块文档给的清单是：*`AtomicUsize` and `AtomicIsize` are generally the most portable, but even then they’re not available everywhere.*
- **只有具体类型名可用。** 编译器内部把 `AtomicU32` 看成泛型 `Atomic<u32>` 的特化，但那个泛型拼写仍是不稳定特性（nightly-only 的 `generic_atomic`，跟踪 issue 130539），stable 上写 `Atomic<u32>` 会报 E0658。**用 `AtomicU32` 这样的具体名字**。

`static` 里的原子量很常见，而且初始化是常量表达式：

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static REQUESTS: AtomicU64 = AtomicU64::new(0);

fn main() {
    REQUESTS.fetch_add(1, Ordering::Relaxed);
    println!("处理过 {} 个请求", REQUESTS.load(Ordering::Relaxed));
}
```

### 操作家族

| 操作 | 语义 | 返回 |
|---|---|---|
| `load` / `store` | 原子读 / 原子写单个值 | 旧值 / `()` |
| `swap` | 换出新值 | 旧值 |
| `compare_exchange` / `_weak` | 当前值等于预期才写入 | `Result<旧值, 实际值>` |
| `fetch_add` / `fetch_sub` | 加减 | 旧值 |
| `fetch_and` / `fetch_or` / `fetch_xor` / `fetch_nand` | 位运算 | 旧值 |
| `fetch_max` / `fetch_min` | 按大小取极值 | 旧值 |
| `update` / `try_update`（1.95 起） | 用闭包算新值，失败可放弃 | `旧值` / `Result<旧值, 实际值>` |
| `as_ptr` / `get_mut` / `into_inner` | 取裸指针 / 有 `&mut` 时直取内层 / 拆掉外壳 | —— |
| `from_mut` / `from_mut_slice` / `get_mut_slice`（1.98 起） | 在 `&mut [u32]` 与 `&mut [AtomicU32]` 之间来回转 | —— |

两点说明。第一，`fetch_*` 家族返回的是**旧值**，`x.fetch_add(1, ...)` 的表达式值是加之前的值——想用「加之后的数」要自己加一。第二，`update`/`try_update` 是 1.95 起的新名字，旧名字 `fetch_update` 在 1.98 上还能用（不告警），但标准库源码里已经给它挂上了 1.99 的弃用标记（`` renamed to `try_update` for consistency ``），新代码直接用新名字。

### Ordering 五值

`Ordering` 只有五个变体。下面这张表是本章的核心，每一列都值得逐个理解（语义列译自标准库 `Ordering` 的文档注释）：

| 值 | 保证（译自 std） | 能配的操作 | 常见用途 |
|---|---|---|---|
| `Relaxed` | 只有原子性，无顺序约束 | 全部 | 引用计数、统计计数、只关心最终值 |
| `Release` | 与后续的 Acquire 读配对：本操作之前的所有写，对配对者之后的操作可见 | 会写内存的：`store`、RMW、`swap` | 发布数据（放旗子） |
| `Acquire` | 与之前的 Release 写配对：那个写之前的所有写，对本操作之后的所有读可见 | 会读内存的：`load`、RMW、`swap` | 订阅数据（读旗子） |
| `AcqRel` | 读用 Acquire、写用 Release；比 `Relaxed` 强，比 `SeqCst` 弱 | 同时读写：`compare_exchange`、`fetch_*`、`swap` | CAS 循环 |
| `SeqCst` | 在上面基础上再加一条：所有线程看到的**全体** SeqCst 操作顺序一致 | 全部 | 多个原子量之间的握手 |

顺序记忆法：`Relaxed` < `Acquire` ≈ `Release` < `AcqRel` < `SeqCst`。但「强度」不是一条直线——`Acquire` 和 `Release` 管的是不同方向，无法互相替代。

三条硬规则要知道，配错了要么编译不过，要么运行时 panic：

1. **`load` 不能用 `Release`/`AcqRel`**——读没有东西要「发布」。1.98 里这是编译期错误（lint `invalid_atomic_ordering`，默认 deny），见「报错现场」。
2. **`store` 不能用 `Acquire`/`AcqRel`**——写没有东西要「订阅」。这就是为什么 `Release` 文档里有一句提醒：*Notice that using this ordering for an operation that combines loads and stores leads to a `Relaxed` load operation!* 用 `AcqRel` 去 `store` 只相当于 `Release`。
3. **失败序不能是 `Release` 或 `AcqRel`**：`compare_exchange` 失败时只有一次读，没有「发布的写」。失败序只能是 `SeqCst`、`Acquire`、`Relaxed`。

### happens-before：配对了才算数

内存序的全部作用就是往一个偏序关系里加边：**先行发生（happens-before）**。有了它，「A 的写」才一定在「B 的读」之前，且对 B 可见。

单向的 `Release`、`Acquire` 都不会单独生效，它们必须**配对**：一个 `Release` 写，加上一个「读到了这个写的结果」的 `Acquire` 读。配对成功后，写者 Release 之前的一切写在读者 Acquire 之后都可见。下图把这条链路摊开：

<svg viewBox="0 0 720 210" width="100%" role="img" aria-label="线程 A 用 Release 写发布数据，线程 B 用 Acquire 读配对成功，于是 A 在 Release 之前的写对 B 在 Acquire 之后的读可见">
  <text x="8" y="30" font-size="14" fill="currentColor">线程 A（发布者）</text>
  <rect x="150" y="42" width="230" height="30" rx="5" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <text x="265" y="62" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">data.store(42, Relaxed)</text>
  <line x1="382" y1="57" x2="416" y2="57" stroke="currentColor" stroke-width="1.5"/>
  <polygon points="416,53 424,57 416,61" fill="currentColor"/>
  <rect x="426" y="42" width="230" height="30" rx="5" fill="none" stroke="var(--links)" stroke-width="2"/>
  <text x="541" y="62" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">flag.store(true, Release)</text>
  <line x1="541" y1="74" x2="541" y2="146" stroke="var(--links)" stroke-width="1.5" stroke-dasharray="6 4"/>
  <polygon points="537,146 541,154 545,146" fill="var(--links)"/>
  <text x="551" y="115" font-size="12" fill="var(--links)">配对成功</text>
  <text x="8" y="168" font-size="14" fill="currentColor">线程 B（订阅者）</text>
  <rect x="426" y="140" width="230" height="30" rx="5" fill="none" stroke="var(--links)" stroke-width="2"/>
  <text x="541" y="160" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">flag.load(Acquire) 为 true</text>
  <line x1="426" y1="155" x2="392" y2="155" stroke="currentColor" stroke-width="1.5"/>
  <polygon points="392,151 384,155 392,159" fill="currentColor"/>
  <rect x="152" y="140" width="230" height="30" rx="5" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <text x="267" y="160" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">data.load(Relaxed) 读到 42</text>
  <text x="8" y="198" font-size="12" fill="currentColor">左边两个框之间是 A 内部的「先后发生」；虚线是 Release/Acquire 配对；配对之后 A 在 Release 之前的写，才保证被 B 读到。</text>
</svg>

把任意一半换掉会怎样？

- **写侧用 `Relaxed`**：`store` 与之前的 `data.store` 之间不再有顺序约束。读者可能看到 `flag` 是 `true`，`data` 却还是旧值；
- **读侧用 `Relaxed`**：读与随后的 `data.load` 之间不再有顺序约束，同一个问题从另一头出现；
- **两边都 `Relaxed`**：只有一个孤立的原子写和一个孤立的原子读，什么都没连上。

两个极端提醒：如果只用它保护它自己那一个值，`Relaxed` 就够了（计数器不会丢更新，只是顺序不定）；而 `SeqCst` 的额外保证只对**跨多个原子变量**的握手有意义，用在单个计数器上属于白付成本。

### compare_exchange 与 CAS 循环

`compare_exchange`（以及允许伪失败的 `compare_exchange_weak`）是用来表达「条件写」的操作：当前值等于 `current` 就写入 `new`，返回 `Ok(旧值)`；否则一个字节都不改，返回 `Err(实际值)`。它有两个内存序参数，语义不同：

- `success` 是「写入成功」那一次读-改-写的序（写成 `Acquire` 时，写的一半退化成 `Relaxed`；写成 `Release` 时，读的一半退化成 `Relaxed`）；
- `failure` 是「比较失败」那一次读的序，只能是 `SeqCst`、`Acquire`、`Relaxed`。写成别的有两种下场：序值是**常量**（直接写 `Ordering::Release`）——同一个 `invalid_atomic_ordering` lint 在编译期就拦下；序值**非常量**（从参数、变量里传进来）——编译期看不见，执行到那一步才在 `core` 里 panic：`panic!("there is no such thing as a release failure ordering")`。两种形态都见「报错现场」第三例。

失败时返回的 `Err` **不是垃圾值**，它带着此刻的真实值——这正是 CAS 循环能推进的关键。标准写法：

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

fn update_max(target: &AtomicUsize, v: usize) {
    let mut cur = target.load(Ordering::Relaxed);
    while v > cur {
        match target.compare_exchange_weak(cur, v, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return,
            // 失败说明别人改了值：用最新的值重算，而不是直接重试
            Err(now) => cur = now,
        }
    }
}
```

三个细节：

1. **循环里用 `_weak`。** 它允许「比较相等却报失败」的**伪失败（spurious failure）**，换来更好的指令序列，循环里代价只是一次重试；只想试一次（如抢锁失败就走别的路）才用强版本。
2. **必须消费 `Err` 里的值。** 忽略它就会拿过期的 `cur` 反复 CAS：轻则空转，重则逻辑错。
3. **当心 ABA 问题（ABA problem）**。std 文档专门提醒：比较的是**相等**，不是**同一**。值从 A 变成 B 又变回 A 时 CAS 照样成功，而这期间世界可能已经变了。需要「同一性」时不能用裸 CAS。

### 原子类型为什么是 Sync：UnsafeCell 与手写的 unsafe impl

第 17 章讲过内部可变性：要在只有 `&self` 的地方改内容，必须有 `UnsafeCell`。原子类型要提供 `fn store(&self, ...)`，所以它内部必然是一个 `UnsafeCell`。标准库的骨架长得就是这样（`Atomic<T>` 的泛型外壳，注释为本书所加）：

> [!WARNING]
> 下面这段是 nightly-only：`Atomic<T>` 这种泛型拼写要等 `generic_atomic`（tracking issue #130539）稳定；stable 上请用 `AtomicU32`、`AtomicUsize` 这类具体名字（它们是 `Atomic<T>` 的类型别名）。

```rust,ignore
pub struct Atomic<T: AtomicPrimitive> {
    v: UnsafeCell<T::Storage>,
}

unsafe impl<T: AtomicPrimitive> Send for Atomic<T> {}
unsafe impl<T: AtomicPrimitive> Sync for Atomic<T> {}
```

`UnsafeCell` 本身是 `!Sync`——这正是它的职责：编译器看到它，就不再假设「`&T` 期间内容不变」。原子类型要变回 `Sync`，只能由标准库作者**手写 `unsafe impl Sync`**，并为「每次访问都是原子的」这句话负责。

于是有了一个好用的分工：**原子类型是 unsafe 内核 + 安全外壳**，你用它时一行 `unsafe` 都不用写。这也是判断「要不要自己写无锁结构」的第一条线索：一旦你需要 `UnsafeCell` + 裸指针 + 内存回收，你就在重演别人已经验过的 unsafe 代码。

模块文档还有一句承诺值得记住（译自 std）：

> 本模块所有原子类型只要可用，就保证是无锁的；这意味着它们内部不会去抢一把全局互斥锁。原子类型与操作不保证无等待（wait-free）：像 `fetch_or` 这样的操作可能是用 CAS 循环实现的。

「无锁」承诺的是「不阻塞别人」，不是「我这步一定成功」：CAS 循环会重试，重试就会空转。

### 什么时候不该手写无锁

原子类型是工具，不是目标。满足下面任何一条，就该停下来换方案：

1. **需求能用现成的做。** 计数器 + 互斥 → `Mutex<T>` 或 `RwLock<T>`；「只初始化一次」→ `OnceLock`/`LazyLock`；线程间传递 → channel；共享大盘 → `Arc<Mutex<_>>`。这些都是第 17、19 章的东西，写起来快、审起来容易。
2. **正确性说不清。** 无锁算法的正确性依赖很难在脑子里推演的内存序论证。写之前先问：我能不能用 `loom` 之类的工具把交错穷举一遍（第 31 章）？
3. **性能没测过。** 无锁不一定更快：CAS 争抢时会空转、缓存行来回弹（第 29 章谈测量）。只有剖析数据说「锁是瓶颈」时，才值得换。
4. **涉及内存回收。** 无锁数据结构一旦要释放节点，就要回答「别的线程手里那个指针怎么办」（hazard pointer、epoch 之类的方案），这部分远超出本章范围。生产代码用成熟实现，别自己发明。

反过来说，值得用原子量的场景有个共同点：**状态小、单个字、语义能用一次 CAS 或 fetch 写完**——计数器、标志位、序号、指针快照。超过这个规模，就是锁的活。

## 动手看看

下面五个程序都在本机（Windows x86-64、rustc 1.98.0、edition 2024）真跑过，输出照抄。并发的输出天生带随机性，凡是会变的数字都标注了。

### 一、为什么必须一步完成：`load` + `store` 对 `fetch_add`

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

const THREADS: usize = 4;
const PER_THREAD: usize = 100_000;

fn main() {
    // 一、load + store：读-改-写拆成了两步
    let bad = Arc::new(AtomicUsize::new(0));
    let mut hs = Vec::new();
    for _ in 0..THREADS {
        let c = Arc::clone(&bad);
        hs.push(thread::spawn(move || {
            for _ in 0..PER_THREAD {
                let v = c.load(Ordering::Relaxed);
                c.store(v + 1, Ordering::Relaxed);
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    println!("load + store：期望 {}，实际 {}", THREADS * PER_THREAD, bad.load(Ordering::Relaxed));

    // 二、fetch_add：读-改-写是一步
    let good = Arc::new(AtomicUsize::new(0));
    let mut hs = Vec::new();
    for _ in 0..THREADS {
        let c = Arc::clone(&good);
        hs.push(thread::spawn(move || {
            for _ in 0..PER_THREAD {
                c.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    println!("fetch_add：   期望 {}，实际 {}", THREADS * PER_THREAD, good.load(Ordering::Relaxed));
}
```

把上面这段存成 `counter.rs`，然后：

```console
$ rustc --edition 2024 counter.rs -o counter.exe
$ .\counter.exe
```

```text
load + store：期望 400000，实际 110023
fetch_add：   期望 400000，实际 400000
```

`load` 那一段**一行 `unsafe` 也没有、也不会 UB**（每次访问都是原子的），只是逻辑上把「加一」拆成了两步，于是大量更新被覆盖。注意实际值每次都不同（本机几次运行是 110023、141000、114928、175575、124777），`fetch_add` 则一次不差。

顺手看一个陷阱：这段加 `-O` 重编译，本机跑 5 次都打印 `400000`（更早一次同类运行出现过 `391785`）。反汇编里那个循环体是 5 条**不带 `lock` 的 `incq`**——优化器把这对 `Relaxed` 的 load/store 合并成了一条普通自增。丢不丢更新，取决于两个核是否恰好同时持有那行缓存。这正是并发 bug 的典型形态：**代码对不对，不取决于你在本机看到什么**。

### 二、Release/Acquire 的正确姿势：发布与订阅

```rust
use std::hint::spin_loop;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

fn main() {
    let data = Arc::new(AtomicUsize::new(0));
    let ready = Arc::new(AtomicBool::new(false));

    let (d2, r2) = (Arc::clone(&data), Arc::clone(&ready));
    let producer = thread::spawn(move || {
        d2.store(42, Ordering::Relaxed); // 写数据
        r2.store(true, Ordering::Release); // 放旗子：之前的写对配对者可见
    });

    while !ready.load(Ordering::Acquire) {
        spin_loop(); // 订阅：只有 Acquire 才能保证读到旗子之前的写
    }
    println!("消费者读到数据：{}", data.load(Ordering::Relaxed));
    producer.join().unwrap();
}
```

把上面这段存成 `msg.rs`，然后：

```console
$ rustc --edition 2024 msg.rs -o msg.exe
$ .\msg.exe
```

```text
消费者读到数据：42
```

这个程序的输出永远是 42，因为 `Release`/`Acquire` 的配对给了 `data.store` 到 `data.load` 之间的 happens-before。把两处换成 `Relaxed` 后，同一个程序在 x86-64 上**多半还是会打印 42**——这正是它危险的地方：少了一次配对，正确性就只剩下「这台机器这次没暴露」。（`spin_loop()` 是给 CPU 的自旋提示，不改变内存语义。）

### 三、RMW 家族巡礼：CAS 循环、位运算、swap

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

const THREADS: usize = 4;
const PER_THREAD: usize = 20_000;

fn update_max(target: &AtomicUsize, v: usize) {
    let mut cur = target.load(Ordering::Relaxed);
    while v > cur {
        match target.compare_exchange_weak(cur, v, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return,
            Err(now) => cur = now,
        }
    }
}

fn main() {
    // 一、手写 CAS 循环
    let max = Arc::new(AtomicUsize::new(0));
    let mut hs = Vec::new();
    for t in 0..THREADS {
        let m = Arc::clone(&max);
        hs.push(thread::spawn(move || {
            for i in 0..PER_THREAD {
                update_max(&m, t * PER_THREAD + i);
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    println!("CAS 循环求最大值：{}（期望 {}）", max.load(Ordering::Relaxed), THREADS * PER_THREAD - 1);

    // 二、标准库的 update 就是同一个循环
    let max2 = Arc::new(AtomicUsize::new(0));
    let old = max2.update(Ordering::AcqRel, Ordering::Acquire, |cur| cur.max(100));
    println!("update 返回旧值 {old}，现在是 {}", max2.load(Ordering::Relaxed));

    // 三、CAS 失败时 Err 里带着「现在的值」
    let x = AtomicUsize::new(7);
    match x.compare_exchange(10, 20, Ordering::AcqRel, Ordering::Acquire) {
        Ok(v) => println!("成功，旧值 {v}"),
        Err(v) => println!("失败，当前值其实是 {v}"),
    }
    println!("x 仍然是 {}", x.load(Ordering::Relaxed));

    // 四、位运算家族与 swap
    let flags = AtomicUsize::new(0b0000);
    let old = flags.fetch_or(0b0101, Ordering::Relaxed);
    println!("fetch_or 返回旧值 {old:#06b}，现在 {:#06b}", flags.load(Ordering::Relaxed));
    let taken = flags.swap(0b1000, Ordering::Relaxed);
    println!("swap 返回 {taken:#06b}，现在 {:#06b}", flags.load(Ordering::Relaxed));
    let cleared = flags.fetch_and(0b0111, Ordering::Relaxed);
    println!("fetch_and 返回 {cleared:#06b}，现在 {:#06b}", flags.load(Ordering::Relaxed));
}
```

把上面这段存成 `cas.rs`，然后：

```console
$ rustc --edition 2024 -O cas.rs -o cas.exe
$ .\cas.exe
```

```text
CAS 循环求最大值：79999（期望 79999）
update 返回旧值 0，现在是 100
失败，当前值其实是 7
x 仍然是 7
fetch_or 返回旧值 0b0000，现在 0b0101
swap 返回 0b0101，现在 0b1000
fetch_and 返回 0b1000，现在 0b0000
```

四段分别说明：CAS 循环能算出正确极值；`update` 的返回值是**旧值**（旧值是 `0`，新值 `100` 已经写进去了）；失败的 CAS 一个字节都没改，`Err` 里是真实当前值；`fetch_*` 与 `swap` 返回的都是旧值。

### 四、两种内存序的对照：store buffering

「`Relaxed` 少了保证」在 x86-64 上并不容易看见（这台机器的内存模型偏强）。能看见差别的最小实验是 **store buffering（SB）**：两个线程各写一个变量、再读另一个，看会不会出现「两边都读到 0」。

```rust,no_run
{{#rustdoc_include ../snippets/ch20/litmus.rs}}
```

这段程序的完整源码在 `snippets/ch20/litmus.rs`，在仓库根目录编译运行：

```console
$ rustc --edition 2024 -O snippets/ch20/litmus.rs -o litmus.exe
$ .\litmus.exe
```

```text
Relaxed          两者都读到 0 的轮数：20 / 2000000
Release/Acquire  两者都读到 0 的轮数：77 / 2000000
SeqCst           两者都读到 0 的轮数：0 / 2000000
```

再跑一次（同一份二进制）：

```text
Relaxed          两者都读到 0 的轮数：51 / 2000000
Release/Acquire  两者都读到 0 的轮数：313 / 2000000
SeqCst           两者都读到 0 的轮数：0 / 2000000
```

三行读法：

- `Relaxed` 与 `Release`/`Acquire` 都是**非零**：`Release` 写、`Acquire` 读都不约束本线程里的写读顺序，挡不住「各自的写还在缓冲里、先读了别人的变量」的交错。两次运行里 `Release/Acquire` 反而更大——它们在这台机器上编译成**一模一样的指令**（下一节看汇编），差别只是调度噪声。
- `SeqCst` 两次都是 **0**：额外的那条全序保证把这种结果排除掉了。
- 单向的 `Release`/`Acquire` 只在**互相配对**时生效，而 SB 里两个线程读的是对方的变量，谁都没读到对方写的那个值，配不上。

这个实验是**概率性**的：只有极少数轮次会撞上那个窗口，也可能跑几次全是 0。要确定性证据得靠 `loom`/Miri 那类工具（第 31 章）；本节的价值只在于：**同一段程序换个 `Ordering`，可观察结果真的会变**。Nomicon 对「本机测不出来」有一句提醒：*Asking for guarantees that are too weak on strongly-ordered hardware is more likely to happen to work, even though your program is strictly incorrect. If possible, concurrent algorithms should be tested on weakly-ordered hardware.*

### 五、序在 x86-64 上到底编译成了什么

把下面这段存成 `orderings.rs`，用 `--emit asm` 看汇编（`-O`）：

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

#[unsafe(no_mangle)]
pub fn store_relaxed(a: &AtomicUsize, v: usize) {
    a.store(v, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub fn store_release(a: &AtomicUsize, v: usize) {
    a.store(v, Ordering::Release);
}

#[unsafe(no_mangle)]
pub fn store_seqcst(a: &AtomicUsize, v: usize) {
    a.store(v, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub fn load_relaxed(a: &AtomicUsize) -> usize {
    a.load(Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub fn load_acquire(a: &AtomicUsize) -> usize {
    a.load(Ordering::Acquire)
}

#[unsafe(no_mangle)]
pub fn load_seqcst(a: &AtomicUsize) -> usize {
    a.load(Ordering::SeqCst)
}

#[unsafe(no_mangle)]
pub fn rmw_relaxed(a: &AtomicUsize, v: usize) {
    a.fetch_add(v, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub fn rmw_seqcst(a: &AtomicUsize, v: usize) {
    a.fetch_add(v, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub fn cas_acqrel(a: &AtomicUsize, cur: usize, new: usize) -> bool {
    a.compare_exchange(cur, new, Ordering::AcqRel, Ordering::Acquire).is_ok()
}

fn main() {}
```

```console
$ rustc --edition 2024 -O --emit asm orderings.rs -o orderings.s
```

只摘函数体（`-O`，rustc 1.98，x86-64）：

```text
store_relaxed:  movq %rdx, (%rcx)
store_release:  movq %rdx, (%rcx)
store_seqcst:   xchgq %rdx, (%rcx)          ; 隐式带 lock，等于一次全屏障
load_relaxed:   movq (%rcx), %rax
load_acquire:   movq (%rcx), %rax
load_seqcst:    movq (%rcx), %rax
rmw_relaxed:    lock addq %rdx, (%rcx)
rmw_seqcst:     lock addq %rdx, (%rcx)
cas_acqrel:     lock cmpxchgq %r8, (%rcx)
```

三件事一目了然：

1. **读全都一样**：x86-64 上 `load` 无论什么序都是一条 `mov`——它的内存模型不允许「读越过读」，Acquire 在这里不需要额外指令。
2. **写里只有 `SeqCst` 特殊**：`Release` 与 `Relaxed` 都是普通 `mov`；`SeqCst` 变成 `xchg`，它隐式带 `lock`，效果等于一条完整屏障（memory barrier）。上面的 SB 实验能挡住「都读到 0」的正是这一条。
3. **RMW 一律带 `lock`**：`fetch_add` 无论什么序都是 `lock addq`，`CAS` 是 `lock cmpxchgq`，因为「读-改-写」本身必须原子。

结论有两层。第一，**`Relaxed` 不是「更快」的同义词**。Nomicon 的判断是：*On strongly-ordered platforms most accesses have release or acquire semantics, making release and acquire often totally free. This is not the case on weakly-ordered platforms.* 也就是说，在 x86-64 上写 `Relaxed` 省不到什么，收益要到弱序平台上才可能出现——而且得测（第 29 章）。第二，**别用 x86-64 上的汇编当正确性证据**：这些指令能对上，只说明这台机器「顺带」满足了你没要求的序；换到弱序机器，同一份代码会多出屏障指令，语义才真正由你写的 `Ordering` 决定。跨平台差异与形式化在第 35 章。

## 报错现场

### 一、给 `load` 传了 `Release`

最小复现：

```rust,compile_fail
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let x = AtomicUsize::new(1);
    println!("{}", x.load(Ordering::Release));
}
```

以 Rust 1.98 为例，关键几行：

```text
error: atomic loads cannot have `Release` or `AcqRel` ordering
 --> src/main.rs:5:27
  |
5 |     println!("{}", x.load(Ordering::Release));
  |                           ^^^^^^^^^^^^^^^^^
  |
  = help: consider using ordering modes `Acquire`, `SeqCst` or `Relaxed`
  = note: `#[deny(invalid_atomic_ordering)]` on by default
```

定位路径：`load` 只读取，没有「之前的写要发布给别人」这回事，所以 `Release` 对它没有意义。这是编译期检查（lint `invalid_atomic_ordering` 默认 deny），`store` 配 `Acquire`/`AcqRel` 也报同样的错。修法看 `help` 一行：`load` 用 `Acquire`（要订阅别人的发布）或 `Relaxed`（只要值），`store` 用 `Release` 或 `Relaxed`。

### 二、拿 `Cell` 当全局计数器

最小复现：

```rust,compile_fail
use std::cell::Cell;

static COUNTER: Cell<u64> = Cell::new(0);

fn main() {
    COUNTER.set(COUNTER.get() + 1);
    println!("{}", COUNTER.get());
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0277]: `Cell<u64>` cannot be shared between threads safely
 --> src/main.rs:3:17
  |
3 | static COUNTER: Cell<u64> = Cell::new(0);
  |                 ^^^^^^^^^ `Cell<u64>` cannot be shared between threads safely
  |
  = help: the trait `Sync` is not implemented for `Cell<u64>`
  = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU64` instead
  = note: shared static variables must have a type that implements `Sync`
```

定位路径：`static` 的值一进程一份、谁都能借到 `&`，所以编译器要求它的类型是 `Sync`。第 17 章已经确认 `Cell<T>` 是「送得出去、共享不出去」的典型（`!Sync`）。这里的两条修法，编译器在 note 里都给了：要么回到 `RwLock`/`Mutex`（第 19 章），要么换成 `AtomicU64`——后者在 `&self` 上就能改，且是 `Sync`。上一节的操作家族里那些 `fetch_*` 就是替代 `get`/`set` 的写法。

### 三、CAS 的失败序：常量被编译期拦下，非常量才轮到运行期

最小复现（序值是常量）：

```rust,compile_fail
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let x = AtomicUsize::new(1);
    let _ = x.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Release);
}
```

以 Rust 1.98 为例，关键几行：

```text
error: `compare_exchange`'s failure ordering may not be `Release` or `AcqRel`, since a failed `compare_exchange` does not result in a write
 --> src/main.rs:5:56
  |
5 |     let _ = x.compare_exchange(1, 2, Ordering::AcqRel, Ordering::Release);
  |                                                        ^^^^^^^^^^^^^^^^^ invalid failure ordering
  |
  = help: consider using `Acquire` or `Relaxed` failure ordering instead
  = note: `#[deny(invalid_atomic_ordering)]` on by default
```

同样的错误序值，如果是从参数传进来的，编译期就看不见了：

```rust,should_panic
use std::sync::atomic::{AtomicUsize, Ordering};

fn cas_with(x: &AtomicUsize, failure: Ordering) -> bool {
    x.compare_exchange(1, 2, Ordering::AcqRel, failure).is_ok()
}

fn main() {
    let x = AtomicUsize::new(1);
    println!("{}", cas_with(&x, Ordering::Release));
}
```

这一段能编译，跑到 `compare_exchange` 里面才炸（开头那截工具链路径已略）：

```text
thread 'main' (11296) panicked at ...\library\core\src\sync\atomic.rs:4011:29:
there is no such thing as a release failure ordering
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

定位路径：`Release`/`AcqRel` 作为**失败**序没有意义——失败的那一次 CAS 不构成写。模块文档括号里那句就是判据：

> (A `compare_exchange` or `compare_exchange_weak` that does not succeed is not considered a write.)

所以这条规则在编译期就能判，也就有了 `invalid_atomic_ordering` 这个 lint；只有当序值不是常量时编译器无从判断，检查才落到 `core` 里的运行期 `panic!`。修法看 `help` 一行：失败序写 `Acquire` 或 `Relaxed`。

## 小结

1. **原子类型解决两件事**：一步完成的读-改-写，以及可指定强度的可见性。计数器、标志位、指针快照是它最合适的形状。
2. **`Ordering` 是成对语义**：`Release` 写与 `Acquire` 读配对才产生 happens-before；`Relaxed` 只有原子性，什么都不连；`AcqRel` 用于 RMW；`SeqCst` 才给全体 SeqCst 操作一个共同顺序。
3. **`load` 不能用 `Release`/`AcqRel`，`store` 不能用 `Acquire`/`AcqRel`，CAS 的失败序不能是 `Release`/`AcqRel`**；三条都由同一个 lint `invalid_atomic_ordering` 在编译期拦下（默认 deny）。区别在于失败序那条只拦得住**常量**写法；序值非常量时要等运行期 `panic!` 才发现。
4. **CAS 循环三件套**：循环里用 `_weak`（允许伪失败）、必须消费 `Err` 里的最新值、注意 ABA（相等 ≠ 同一）。
5. **原子类型是 `UnsafeCell` + 手写 `unsafe impl Sync`** 的安全外壳；读侧承诺 lock-free（内部不抢全局锁），不承诺 wait-free（可能空转重试）。
6. **测不出问题不等于没问题**：x86-64 上 `Relaxed` 与 `Release`/`Acquire` 编译结果相同，把 `Relaxed` 用错地方，本机多半照样「对」。要确定性结论，用 `loom`/Miri，并优先质疑「这里真的需要自己写吗」。

## 练习

**练习 1（预测编译结果）** 下面这段能编译吗？不能的话，报什么错？

```rust,compile_fail
use std::sync::atomic::{AtomicUsize, Ordering};

fn publish(x: &AtomicUsize) {
    x.store(1, Ordering::Acquire);
}
```

**练习 2（找错）** 下面这个「只在更大时更新」的函数有个致命问题，指出来并修正：

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

fn update_max(target: &AtomicUsize, v: usize) {
    let mut cur = target.load(Ordering::Relaxed);
    while v > cur {
        match target.compare_exchange(cur, v, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return,
            Err(_) => {}
        }
    }
}
```

**练习 3（选序）** 给下面四个场景各挑一个最弱够用的序，并说明理由：

1. `Arc` 内部的强引用计数；
2. 统计「处理过多少请求」的全局计数器；
3. 线程 A 算好一份配置，用一个 `AtomicBool` 通知线程 B 来取；
4. 两个线程各写一个原子标志，之后都要读对方那个，用来判断「谁先到」。

<details><summary>参考答案</summary>

**练习 1** 不能编译。`store` 只写内存，没有东西可订阅，`Acquire` 对它没有意义：1.98 报 `` error: atomic stores cannot have `Acquire` or `AcqRel` ordering ``，`help` 一行建议改用 `Release`、`SeqCst` 或 `Relaxed`（与正文里 `load` 那条同属 `invalid_atomic_ordering` 检查，默认 deny）。这里应该换成 `Release`（要把这次写之前的写发布出去）或 `Relaxed`（只关心这个值本身）。

**练习 2** 两个问题，第一个是致命伤：`Err(_)` 把真实当前值丢掉了，`cur` 永远停在最初读到的那个值上。只要别的线程把值改到超过 `v`（或者改成一个 `cur` 之外的值），这个 CAS 就会一直失败、一直空转——变成死循环。修法是 `Err(now) => cur = now`。第二个是小问题：既然在循环里重试，用 `compare_exchange_weak` 更合适（允许伪失败，换来更好的指令）。

**练习 3**

1. 增量用 `Relaxed`（`Arc::clone` 里正是 `fetch_add(1, Relaxed)`）：计数只关心「加一/减一不出错」，不携带要发布的数据。顺序需求落在**减到 0**：`Arc` 用 `fetch_sub(1, Release)`，最后一个引用再补 Acquire 屏障才析构，好让前面对数据的写对析构者可见。
2. `Relaxed`。计数之间不需要顺序关系，`fetch_add` 的原子性就够。
3. `Release`（写 `AtomicBool` 的那一侧）+ `Acquire`（读的那一侧）。这正是本章的发布/订阅模式：配置的写必须 happens-before 到使用方。
4. `SeqCst`。两边各写一个变量、再读对方那个，需要全体 SeqCst 操作有共同顺序才能断定「谁先到」；本章的 SB 实验里也只有它把「都读到 0」压到 0。

</details>

## 延伸阅读

- `std::sync::atomic` 模块文档（内存模型一节是本章多数规则的出处）：<https://doc.rust-lang.org/std/sync/atomic/>
- `Ordering` 五个变体的逐条说明：<https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html>
- `AtomicUsize` 的完整操作清单（`load`/`store`/`compare_exchange`/`fetch_*`）：<https://doc.rust-lang.org/std/sync/atomic/type.AtomicUsize.html>
- `core::cell::UnsafeCell`：<https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html>
- The Nomicon《Atomics》一章：<https://doc.rust-lang.org/nomicon/atomics.html>
- The Nomicon《Data Races and Race Conditions》：<https://doc.rust-lang.org/nomicon/races.html>
