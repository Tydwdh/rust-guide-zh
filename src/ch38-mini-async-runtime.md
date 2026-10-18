# 第 38 章 自己实现极简 async 运行时

## 这一章解决什么问题

第 21 章把 async 的用法讲完了：`async fn`、`.await`、`Send` 边界、以及怎么挑一个运行时（runtime）。那一章留下了一句没展开的话——标准库只给契约，不给执行器（executor）。你一直在用别人搭好的运行时，但没有亲手搭过一个。

这一章补上这一环：不用任何 async/await 语法糖，从 `Future`、`Waker`、`Pin` 三个原语出发，搭出一个能跑的极简单线程运行时。做完之后，「`.await` 展开成什么」「返回 `Pending` 之后谁来叫我」「`Pin` 到底挡住了什么」会从「文档这么说」变成「我写过」。

读完这一章你能做到：

- 手写一个 `impl Future` 状态机，说清每个变体对应哪一段被挂起的计算；
- 用两条路造出 `Waker`：安全的 `Wake` + `Arc`，以及手写 `RawWaker` + vtable，并让引用计数严格配对；
- 手写 pin 投影，用私有方法加文档化的 `SAFETY` 注释把它封成安全 API；
- 实现单线程执行器的三件套：就绪队列、唤醒路径、`block_on`，并指出「丢唤醒」的窗口错在哪；
- 认出「违反 `poll` 契约」的运行时症状。

划界说明：`.await` 的语义与执行器的用法在第 21 章；pin 的完整语义（`Unpin`、`PhantomPinned`、三个 safe 入口）在第 22 章，本章只补上那里预告过的 **pin 投影**。异步 I/O 与定时器不在范围里——它们是运行时的另一半（事件源），第 21 章说过那是选型时最重要的判据。

## 为什么这样设计

### 执行器只有四个问题要回答

一个执行器的全部工作可以拆成四问：

1. **谁来被 poll？** 任务（task）是调度单位：一个任务 = 一个 future。调度器就是一条就绪队列（ready queue）。
2. **`Pending` 之后怎么知道该回来？** 答案写在 `poll` 的参数里：`cx.waker()`。future 把这份句柄交给事件源，事件源准备好之后 `wake()` 一下。
3. **等待期间线程干什么？** 睡下（park）。`poll` 不许阻塞，队列空时执行器必须把线程真正挂起，否则就是空转烧核。
4. **任务存在哪里？** 堆上、被 pin 住（`Pin<Box<dyn Future>>`）。future 可能自引用，所以执行器得保证它创建之后再也没搬过家。

本章的代码就是把这四条逐一落实。**没有一行是魔法**：`Waker` 是一个带 vtable 的句柄，`spawn` 是一次类型擦除的装箱，唤醒就是把任务塞回队列。

### 唤醒句柄为什么长成 trait 对象的样子

`Future::poll` 的签名里必须能接收「任意执行器的唤醒方式」，所以 `Waker` 只能是类型擦除的：一个数据指针加一张函数指针表（vtable）。`Wake` 这个 trait 的文档对它的定义只有一句话：

> The implementation of waking a task on an executor.

（在一个执行器上唤醒任务这件事的实现方式。）

`Waker` 类型的文档接着给出它的第一条硬性质，正文里唤醒路径的写法就是它的后果：

> Implements `Clone`, `Send`, and `Sync`; therefore, a waker may be invoked from any thread, including ones not in any way managed by the executor.

（它实现了 `Clone`、`Send` 与 `Sync`；因此一个 waker 可以从任何线程被调用，包括完全不受该执行器管理的线程。）

这不是客套：网络事件线程拿到 waker 就能直接叫醒任务，不必先通知执行器。所以「谁被唤醒了」必须能被多个线程同时说，且不能丢——见 38.4。

第二条是执行器必须兑现的承诺，写在 `Waker::wake` 的文档里：

> As long as the executor keeps running and the task is not finished, it is guaranteed that each invocation of `wake()` (or `wake_by_ref()`) will be followed by at least one `poll()` of the task to which this `Waker` belongs, such that the call to `wake()` (or `wake_by_ref()`) happens-before the beginning of the invocation of `poll()`.

（只要执行器还在跑、任务还没结束，那么每一次 `wake()`（或 `wake_by_ref()`）之后，都保证至少会有一次针对该 waker 所属任务的 `poll()`，并且 `wake` 的调用 happens-before 那次 `poll` 的开头。）

紧接着还有一句，看着像让步，其实是给实现留的余量：

> Note that the above implies that multiple wake-ups may be coalesced into a single `poll()` invocation by the executor.

（也就是说，上述保证意味着多次唤醒可以被执行器合并成一次 `poll()`。）

最后这句直接定了本章唤醒路径的写法：**队列里不必塞两个副本，一个 `AtomicBool` 标记就够**。合并唤醒不是偷懒，是文档允许的。

### 两条构造 `Waker` 的路，代价不同

标准库给了两个入口：

| 入口 | 你要提供 | 代价 |
|---|---|---|
| `Waker::from(Arc<W>)`，`W: Wake` | 实现一个方法（`wake`） | 唤醒数据必须放在 `Arc` 里 ⇒ 需要分配器；沿用默认的 `wake_by_ref` 时每次唤醒有一次 `Arc::clone` 原子操作（`wake` 与自带实现不克隆） |
| `Waker::new(ptr, &VTABLE)` / `Waker::from_raw(raw)` | 数据指针加四个 `unsafe fn` | 不额外分配、不加原子操作，但四个函数都要自己保证内存安全 |

`Wake` 是后来补上的安全通道，文档把它与 `RawWaker` 的关系说得很直白：

> This trait is a memory-safe and ergonomic alternative to constructing a `RawWaker`. It supports the common executor design in which the data used to wake up a task is stored in an `Arc`. Some executors (especially those for embedded systems) cannot use this API, which is why `RawWaker` exists as an alternative for those systems.

（这个 trait 是构造 `RawWaker` 的内存安全、更顺手的选择。它支持「唤醒数据放在 `Arc` 里」这种常见的执行器设计；有些执行器（尤其是嵌入式）用不了这个 API，所以 `RawWaker` 作为替代品存在。）

翻译成选型建议：**能实现 `Wake` 就实现 `Wake`**，除非你在 `no_std` 环境里，或者对每次唤醒的原子操作敏感——嵌入式运行时常常连 `Arc` 都用不上。本章两条路都写：执行器默认走 `Wake`，另给一个 `with_raw_waker()` 开关走手写 vtable，用同一个工作负载验证两者结果一致。

### pin 投影为什么必须由你自己保证

第 22 章讲过，`Pin<&mut T>` 的保证是「这个 `T` 从此不移动」。可一个 future 常常持有**另一个** future（`.await` 一个子任务就是），执行器 poll 外层时，必须把同一份保证传给内层。问题是：从 `Pin<&mut Self>` 里拿不出 `&mut self.inner`，那等于把 pin 保证丢在半路。`Pin::map_unchecked_mut` 就是为这一步准备的，文档把责任写得没有余地：

> This function is unsafe. You must guarantee that the data you return will not move so long as the argument value does not move (for example, because it is one of the fields of that value), and also that you do not move out of the argument you receive to the interior function.

（这个函数是 unsafe 的。你必须保证：返回的数据在参数值不移动的前提下也不会移动（例如它是那个值的字段之一），并且你不会把值移出传给内部函数的那个参数。）

std 的 pin 模块把这类「把 pin 顺着字段传下去」的写法叫**结构化 pinning（structural pinning）**，并附了一条本章必须同时守住的约束：

> A struct can be `Unpin` only if all of its structurally-pinned fields are, too. This is `Unpin`'s behavior by default. However, as a library author, it is your responsibility not to write something like `impl<T> Unpin for Struct<T> {}` and then offer a method that provides structural pinning to an inner field of `T`, which may not be `Unpin`!

（只有当一个结构体所有结构化 pin 的字段都是 `Unpin` 时，它才能是 `Unpin`；这是 `Unpin` 的默认行为。但作为库作者，你有责任不要一边写 `impl<T> Unpin for Struct<T> {}`、一边提供一个对 `T` 的内部字段做结构化 pinning 的方法——那个 `T` 可能不是 `Unpin`。）

这就是 38.5 那个 `Budgeted` 的判据：它只有 `inner` 是结构化 pin 的，所以**不要写任何 `Unpin` 实现**，让它跟着 `F` 自动推导（第 22 章讲过 auto trait 的推导规则）。手写投影再配上手写的 `Unpin`，就是 UB。

## 机制与规则

### 38.1 手写一个 `impl Future` 状态机

从最朴素的开始：不用 `async`，把「算出 `a + b`，让位一次，再加上 `c`」写成一个枚举。三个变体分别对应「还没开始 / 停在挂起点 / 结束」，跨过挂起点还要用的变量就住进变体里：

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

/// 等价于 `async { let s = a + b; yield_now().await; s + c }` 的手写版。
enum TwoStep {
    Start { a: u32, b: u32, c: u32 },
    Half { sum: u32, c: u32 },
    Done,
}

impl Future for TwoStep {
    type Output = u32;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        // 字段全是 u32，所以 TwoStep 是 Unpin 的：拿 &mut self 没有任何风险
        let this = self.get_mut();
        match std::mem::replace(this, TwoStep::Done) {
            TwoStep::Start { a, b, c } => {
                // 第一段算完，把中间结果写回状态机——这就是「状态」
                *this = TwoStep::Half { sum: a + b, c };
                // 登记唤醒：告诉执行器「我还能往下走，记得再来 poll 一次」
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            TwoStep::Half { sum, c } => Poll::Ready(sum + c),
            TwoStep::Done => panic!("已完成的 future 不应再被 poll"),
        }
    }
}
```

三件事值得对着第 21 章那张状态机图看：变体就是状态；`Half` 里那两个字段就是「跨挂起点还活着的局部变量」；`Done` 只是让 `mem::replace` 有个落脚点。而 `Poll::Pending` 之前那句 `cx.waker().wake_by_ref()` 是**本章的枢纽**——`poll` 只是「问一次」，它不会自动再来；要再来，必须在这里登记。下图把三个变体与两次 poll 摊开：

<svg viewBox="0 0 700 218" width="100%" role="img" aria-label="手写状态机 TwoStep 的三个变体：Start 在第一次 poll 时算出 a+b 并写回 Half，同时登记唤醒、返回 Pending；第二次 poll 由 Half 返回 Ready(sum+c)；若没有那次登记，执行器就没有理由再 poll 它" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">手写状态机 TwoStep：变体是状态，登记唤醒是枢纽</text>
<rect x="24" y="40" width="168" height="64" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="108" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Start { a, b, c }</text>
<text x="108" y="88" font-size="12" fill="currentColor" opacity="0.75" text-anchor="middle">还没开始</text>
<rect x="266" y="40" width="168" height="64" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="350" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Half { sum, c }</text>
<text x="350" y="88" font-size="12" fill="currentColor" opacity="0.75" text-anchor="middle">停在挂起点</text>
<rect x="508" y="40" width="168" height="64" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="592" y="64" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Done</text>
<text x="592" y="88" font-size="12" fill="currentColor" opacity="0.75" text-anchor="middle">已返回 Ready</text>
<line x1="192" y1="72" x2="256" y2="72" stroke="var(--links)" stroke-width="1.5" />
<polygon points="264,72 254,67 254,77" fill="var(--links)" />
<text x="198" y="64" font-size="12" fill="currentColor">poll #1</text>
<line x1="434" y1="72" x2="498" y2="72" stroke="var(--links)" stroke-width="1.5" />
<polygon points="506,72 496,67 496,77" fill="var(--links)" />
<text x="440" y="64" font-size="12" fill="currentColor">poll #2</text>
<text x="0" y="138" font-size="12" fill="currentColor">poll #1：算出 a+b 写回状态机，调用 cx.waker().wake_by_ref() 登记唤醒，返回 Pending</text>
<text x="0" y="164" font-size="12" fill="currentColor">poll #2：从 Half{sum, c} 取出 sum 与 c，返回 Ready(sum + c)，此后不再 poll</text>
<text x="0" y="196" font-size="12" fill="var(--warning-border)">没有那次登记，执行器就没有理由回来：队列空 + 无人唤醒，任务永远停在这里</text>
</svg>

> [!NOTE]
> `TwoStep` 的字段全是 `u32`，所以它是 `Unpin` 的，可以放心 `get_mut()`（第 22 章）。需要手写投影的是「持有别的 future」的情形，见 38.5。

### 38.2 唤醒句柄：`Wake` 这条路

`Wake` 只有一个必需方法 `wake(self: Arc<Self>)`；默认的 `wake_by_ref` 会克隆 `Arc` 再调 `wake`。对一个「数唤醒次数」的句柄，两条都实现最省事。下面这段完整程序里没有执行器、没有队列，但唤醒的契约已经跑了一遍：

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

/// 唤醒句柄：只记下自己被唤醒过几次。
#[derive(Default)]
struct CountWakes(AtomicUsize);

impl Wake for CountWakes {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// 让位一次的 future：返回 Pending 之前登记唤醒。
struct YieldOnce(bool);

impl Future for YieldOnce {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        let this = self.get_mut(); // bool 字段，Unpin
        if this.0 {
            Poll::Ready("第二次 poll 才完成")
        } else {
            this.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn main() {
    let counter = Arc::new(CountWakes::default());
    let waker = Waker::from(Arc::clone(&counter)); // 安全构造入口
    let mut cx = Context::from_waker(&waker);

    let mut fut = std::pin::pin!(YieldOnce(false));
    assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(counter.0.load(Ordering::Relaxed), 1); // Pending 时登记了
    assert_eq!(fut.as_mut().poll(&mut cx), Poll::Ready("第二次 poll 才完成"));
    assert_eq!(counter.0.load(Ordering::Relaxed), 1); // Ready 后不再唤醒
}
```

`Waker::from` 这个转换本身不分配：`Arc` 早就分配好了。文档对此只有一句：

> No heap allocations or atomic operations are used for this conversion.

（这次转换不涉及堆分配与原子操作。）`Wake` 文档里说的「需要分配」，指的是它要求你把唤醒数据放进 `Arc`——也就是要求一个分配器。

### 38.3 唤醒句柄：`RawWaker` 这条路

`Waker::from_raw` 的文档先把 unsafe 的边界划清：

> The behavior of the returned `Waker` is undefined if the contract defined in `RawWaker`'s and `RawWakerVTable`'s documentation is not upheld.

（如果没有遵守 `RawWaker` 与 `RawWakerVTable` 文档里的约定，返回的 `Waker` 的行为就是未定义的。）

一句话概括责任：**data 指针指向什么、四个函数各自怎么维护那份引用，全是你的事**。文档给的关键约束有两条。第一条是「数据指针必须指向线程安全的类型，比如 `Arc`」（因为 `Waker` 是 `Send + Sync`）。第二条是：

> If the `RawWaker` will be used to construct a `Waker` then these functions must all be thread-safe (even though `RawWaker` is `!Send + !Sync`).

（如果这个 `RawWaker` 会被用来构造 `Waker`，这些函数就必须全部线程安全——哪怕 `RawWaker` 自己是 `!Send + !Sync`。）

注意括号里的反差：`RawWaker` 自己不可跨线程，由它造出的 `Waker` 却会满世界跑，所以那四个函数必须线程安全。`Waker` 是个 `#[repr(transparent)]` 的薄包装——源码里就是 `pub struct Waker { waker: RawWaker }`，那两个字段其实在 `RawWaker` 上。下图把它与 vtable 里四件事的对应关系摊开：

<svg viewBox="0 0 700 268" width="100%" role="img" aria-label="Waker 里包着 RawWaker：RawWaker 的 data 指针指向 Arc&lt;Task&gt; 的一份强引用，vtable 指向四个函数；clone 增一份、wake 消费一份、wake_by_ref 借完归还、drop 释放一份，四个函数必须严格配对" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">Waker 包着 RawWaker：data 指向任务 Arc&lt;Task&gt;，vtable 指向四个函数</text>
<rect x="20" y="34" width="300" height="76" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="34" y="56" font-size="12" font-family="monospace" fill="currentColor">Waker { waker: RawWaker }</text>
<text x="34" y="76" font-size="12" font-family="monospace" fill="currentColor">RawWaker { data: *const (),</text>
<text x="34" y="96" font-size="12" font-family="monospace" fill="currentColor">           vtable: &amp;'static RawWakerVTable }</text>
<rect x="380" y="34" width="300" height="76" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="530" y="78" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">四个 unsafe fn</text>
<line x1="170" y1="110" x2="170" y2="132" stroke="currentColor" stroke-width="1.5" />
<polygon points="170,140 165,130 175,130" fill="currentColor" />
<text x="178" y="126" font-size="12" fill="currentColor">data</text>
<line x1="530" y1="110" x2="530" y2="132" stroke="currentColor" stroke-width="1.5" />
<polygon points="530,140 525,130 535,130" fill="currentColor" />
<text x="538" y="126" font-size="12" fill="currentColor">vtable</text>
<rect x="20" y="146" width="300" height="72" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="34" y="170" font-size="12" fill="currentColor">每个 Waker 恰好持有</text>
<text x="34" y="192" font-size="12" fill="currentColor">Arc&lt;Task&gt; 的一份强引用</text>
<text x="34" y="212" font-size="12" fill="currentColor" opacity="0.75">Waker::from(Arc&lt;Task&gt;) 走同一条路</text>
<text x="380" y="160" font-size="12" fill="currentColor">clone → Arc::clone（+1）</text>
<text x="380" y="184" font-size="12" fill="currentColor">wake → Arc::from_raw（-1，消费）</text>
<text x="380" y="208" font-size="12" fill="currentColor">wake_by_ref → from_raw + into_raw（±0）</text>
<text x="380" y="232" font-size="12" fill="currentColor">drop → Arc::from_raw 后析构（-1）</text>
<text x="0" y="256" font-size="12" fill="var(--warning-border)">少配对一次就是泄漏，多一次就是 use-after-free</text>
</svg>

用 `Arc` 当数据指针时，四个函数对应四件很固定的事：`clone` 增一份引用，`wake` 消费一份，`wake_by_ref` 借了还，`drop` 归还一份。配平它们，就既不泄漏也不 double free：

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{RawWaker, RawWakerVTable, Waker};

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop_it);

unsafe fn clone(data: *const ()) -> RawWaker {
    // 把裸指针临时还原成 Arc（不动计数），克隆一份，再把两份都交还出去
    let counter = unsafe { Arc::<AtomicUsize>::from_raw(data.cast::<AtomicUsize>()) };
    let cloned = Arc::clone(&counter);
    let _ = Arc::into_raw(counter);
    RawWaker::new(Arc::into_raw(cloned).cast::<()>(), &VTABLE)
}

unsafe fn wake(data: *const ()) {
    // 这一份引用被消费：fetch_add 之后 Arc 在此析构
    let counter = unsafe { Arc::<AtomicUsize>::from_raw(data.cast::<AtomicUsize>()) };
    counter.fetch_add(1, Ordering::Relaxed);
}

unsafe fn wake_by_ref(data: *const ()) {
    let counter = unsafe { Arc::<AtomicUsize>::from_raw(data.cast::<AtomicUsize>()) };
    counter.fetch_add(1, Ordering::Relaxed);
    let _ = Arc::into_raw(counter); // 只是借用，交还回去
}

unsafe fn drop_it(data: *const ()) {
    drop(unsafe { Arc::<AtomicUsize>::from_raw(data.cast::<AtomicUsize>()) });
}

/// 用 RawWaker 造一个 Waker：data 指针就是 Arc 的裸地址。
fn make(counter: &Arc<AtomicUsize>) -> Waker {
    let data = Arc::into_raw(Arc::clone(counter)).cast::<()>();
    // SAFETY: data 指向线程安全的 AtomicUsize，且四个函数各自维护好那一份强引用
    unsafe { Waker::from_raw(RawWaker::new(data, &VTABLE)) }
}

fn main() {
    let counter = Arc::new(AtomicUsize::new(0));
    let waker = make(&counter);
    assert_eq!(Arc::strong_count(&counter), 2); // 我们 + Waker 各一份

    waker.wake_by_ref(); // vtable 的 wake_by_ref
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    assert_eq!(Arc::strong_count(&counter), 2); // 借用不消费

    let clone = waker.clone(); // vtable 的 clone
    assert_eq!(Arc::strong_count(&counter), 3);
    drop(clone); // vtable 的 drop
    assert_eq!(Arc::strong_count(&counter), 2);

    waker.wake(); // vtable 的 wake：引用被消费
    assert_eq!(counter.load(Ordering::Relaxed), 2);
    assert_eq!(Arc::strong_count(&counter), 1);
}
```

它和 38.2 那段是同一份语义，区别只在「谁来实现这几个函数」：那边是 `std` 按 `Wake` 生成的，这边是你手写的。`Waker::new` 从 Rust 1.83 起可用；`Waker::from_raw` 更早。真跑起来的话，四次断言全过——没有泄漏、没有 double free。

### 38.4 唤醒路径：就绪队列与一个不能丢唤醒的窗口

现在把唤醒接到真实任务上。执行器为每个任务分配一个 `Task`：里面是堆上 pin 住的 future、一个「是否已在队列里」的标记、以及共享的队列。**`Waker` 的 data 指针指向这个 `Task` 的 `Arc`**，于是 `wake` 要做的唯一一件事就是把任务放回队列：

```rust
# use std::collections::VecDeque;
# use std::sync::atomic::{AtomicBool, Ordering};
# use std::sync::{Arc, Mutex};
# use std::task::Wake;
# #[allow(dead_code)]
# struct Task { shared: Arc<Shared>, queued: AtomicBool }
# struct Shared { state: Mutex<State> }
# struct State { queue: VecDeque<Arc<Task>> }
impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // 标记与入队在同一把锁下完成：既不丢唤醒，也不重复排队
        let mut state = self.shared.state.lock().unwrap();
        if !self.queued.swap(true, Ordering::AcqRel) {
            state.queue.push_back(Arc::clone(self));
        }
    }
}
```

这段里有个容易写错的地方，正好是第 21 章讲执行器那节提到的那个窗口。假设标记与队列**不在同一把锁**下：执行器先出队、再清标记，中间那一小段时间里任务「既不在队列里、标记又是 true」，此时别的线程来 `wake` 会被标记挡掉——**这次唤醒就永远丢了**（任务卡死到这个执行器结束）。反过来，`wake` 那边先判断标记、后拿锁入队，同样有窗口。本章的做法是把标记与队列放进**同一把锁**：`wake_by_ref` 在锁内做「测试并置位 + 入队」，执行器在同一个锁内做「出队 + 清位」。这样重复唤醒会被合并成一次入队（文档允许），丢唤醒则不可能发生。

顺带分清两个方法：`wake(self)` 消费手上这份 `Waker`，`wake_by_ref(&self)` 只借用（本章的 `wake` 实现就是转调 `wake_by_ref`）。手上只有 `&Waker` 时用后者，不必克隆。

> [!WARNING]
> `LocalWaker` / `LocalWake` 在 1.98 上仍是 nightly-only（feature `local_waker`，issue #118959）：写 `use std::task::LocalWaker;` 会得到 E0658。它的定位是「本地版 `Waker`」，文档说：「The benefits of using `LocalWaker` over `Waker` are that it allows the local waker to hold data that does not implement `Send` and `Sync`. Additionally, it saves calls to `Arc::clone`, which requires atomic synchronization.」要用它得等它稳定。别想着在 stable 上用 `RawWaker` 自己拼一个「本地版」——`Waker` 本身就是 `Send + Sync` 的，那样做不健全。

### 38.5 pin 投影：把 unsafe 关进一个私有方法

`Budgeted<F>` 是本章那个「给内层 future 一个 poll 预算」的组合子：它只有两个字段，`inner: F`（要结构化 pin）和 `budget: u32`（普通计数）。投影写成两个私有方法，各配一条 `SAFETY`，说明保证什么、为什么不越界：

```rust
# use std::future::Future;
# use std::pin::Pin;
# #[allow(dead_code)]
# struct Budgeted<F> { inner: F, budget: u32 }
impl<F: Future> Budgeted<F> {
    /// 把 `Pin<&mut Self>` 投影成内层 future 的 pin。
    fn inner(self: Pin<&mut Self>) -> Pin<&mut F> {
        // SAFETY:
        // ① 本类型没有 Drop 实现，不会在析构里移动字段；
        // ② 只交出 `Pin<&mut F>`，从不通 `&mut Self` 移动 inner；
        // ③ 于是「父值不动 ⇒ 字段不动」这条链完整。
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// `budget` 不参与 pin：它只是计数器。
    fn budget_mut(self: Pin<&mut Self>) -> &mut u32 {
        // SAFETY: 只就地读写 u32，不会把值移出 pin。
        unsafe { &mut self.get_unchecked_mut().budget }
    }
}
```

三条要点：`inner` 用 `map_unchecked_mut` 交出 `Pin<&mut F>`，前提是「父值不动 ⇒ 字段不动」；`budget` 不参与 pin，`get_unchecked_mut` 原地读写即可；**整个类型不写 `Unpin` 实现**，让它跟着 `F` 走（`F` 不是 `Unpin` 时它也不是）。两边一合，里面是两处 `unsafe`，外面是纯安全 API（`Countdown` 与 `block_on` 都来自本章的 crate）：

```rust,ignore
let v = block_on(Budgeted::new(Countdown::new("C", 100), 3));
assert_eq!(v, None); // 预算用尽，内层 future 被放弃
```

第 22 章把投影列为「`unsafe` 的入口之一，不是随手写的 getter」，这里补上了另一半：**投影本身可以写成安全的对外 API**，前提是把它关进私有方法、并守住结构化 pinning 的 `Unpin` 约束。生态里那类自动生成投影的宏（`pin-project` 一族）做的就是这件事。

### 38.6 执行器：任务、`block_on` 与停滞诊断

任务被 poll 的那一步长这样：进去时用**任务自己的 `Arc`** 造唤醒句柄，回来时按结果决定 future 是留下还是析构：

```rust
# use std::future::Future;
# use std::pin::Pin;
# use std::sync::{Arc, Mutex};
# use std::task::{Context, Poll, Wake, Waker};
# #[allow(dead_code)]
# struct Task { future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>> }
# impl Wake for Task {
#     fn wake(self: Arc<Self>) {}
# }
impl Task {
    fn run(self: &Arc<Self>) {
        // 唤醒句柄的载荷就是任务自己：wake 一次就等于「把我放回队列」
        let waker = Waker::from(Arc::clone(self));
        let mut cx = Context::from_waker(&waker);

        // 先取到 guard，再把它保护的那个 Option 取出来 poll；guard 持有到 poll 结束
        let mut slot = self.future.lock().unwrap();
        if let Some(mut fut) = slot.take() {
            // 移动的是 `Pin<Box<_>>`，堆上的 future 没有动
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(()) => {} // 完成：future 在这里析构
                Poll::Pending => *slot = Some(fut),
            }
        }
    }
}
```

（crate 里的 `run` 与这段逐行对应，只多一个 `with_raw_waker()` 分支——38.3 说过，两条路只差构造唤醒句柄那一句。）

外环是 `Executor::run_until(done)`：队列非空就一个个 poll；队列空而目标达成，返回 `Ok`；队列空、目标没达成，就 `Condvar::wait_timeout` 睡下——`wake` 那边入队后会 `notify_one`，把它叫起来。睡满一个超时（本章设 300 ms）还是空队列，说明没有任何事件源或任务会再唤醒它，于是返回 `Err(Stalled)`。

`block_on(fut)` 是 `run_until` 的一个用例：把根 future 包成任务，输出写进共享槽位，`done` 就是「槽位里有值」。

> [!NOTE]
> 这个超时不是调度策略，而是**契约违反的探测器**：正常任务只要返回 `Pending`，就登记了唤醒，队列迟早会再有它。真运行时不需要靠超时发现这一点——它睡在事件源（epoll、I/O 完成队列、定时器）上，由事件源决定何时醒，超时只是兜底。

## 动手看看

`snippets/ch38/` 是一个真 crate（`ch38-runtime`）：`src/lib.rs` 是上面所有零件的完整版加 9 个测试，`src/main.rs` 是四个演示。

```console
$ cd snippets/ch38
$ cargo run -q
TwoStep(1, 2, 3) => 6
队列跑空：7 次 poll，顺序 A#3 B#2 A#2 B#1 A#1
Budgeted 用尽预算 => None
RawWaker 版：3 次 poll，顺序 D#2 D#1
```

四行分别对应：手写状态机被 `block_on` 驱动到完成（1+2+3）；两个任务在同一台执行器上交替推进——**注意交错顺序是 A、B、A、B、A**，它们轮流被 poll，这正是第 21 章说的「并发不等于并行」在最小尺度上的样子；pin 投影包出的预算组合子放弃了内层 future；换上手写 vtable 的唤醒句柄，同一个任务给出同样的结果。

演示程序本身不长，就是这四次调用。它整块标 `ignore` 的原因是：`mdbook test` 把每个代码块当**独立的 crate** 编译，块里的 `use ch38_runtime::…` 拿不到本 crate，编不过；`ignore` 只是让它不进测试闸门，include 照常展开、正文照常显示：

```rust,ignore
{{#rustdoc_include ../snippets/ch38/src/main.rs}}
```

再看测试：

```console
$ cargo test
   Compiling ch38-runtime v0.1.0 (D:\Rust\snippets\ch38)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.45s
     Running unittests src\lib.rs (target\debug\deps\ch38_runtime-257e93ca44616e3a.exe)

running 9 tests
test tests::handwritten_state_machine_registers_wake_on_pending ... ok
test tests::raw_waker_keeps_refcounts_balanced ... ok
test tests::raw_waker_and_safe_waker_agree ... ok
test tests::projection_keeps_inner_future_in_place ... ok
test tests::budgeted_stops_after_budget_is_used_up ... ok
test tests::wake_is_coalesced_into_one_queue_slot ... ok
test tests::tasks_advance_in_queue_order ... ok
test tests::waker_can_wake_from_another_thread ... ok
test tests::work_stealing_is_out_of_scope_but_stall_is_detected ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s

     Running unittests src\main.rs (target\debug\deps\ch38_runtime-7cff073792d74cac.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ch38_runtime

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

（libtest 并行调度，各机器顺序可能不同。）

挑几条说说它们各守什么：`handwritten_state_machine_registers_wake_on_pending` 验证「`Pending` 时必须登记、`Ready` 后不该再唤醒」；`wake_is_coalesced_into_one_queue_slot` 连唤三次只入队一次；`raw_waker_keeps_refcounts_balanced` 把 `clone`/`wake`/`wake_by_ref`/`drop` 四个 vtable 入口依次走一遍，逐步检查引用计数；`projection_keeps_inner_future_in_place` 用「每次 poll 都记下自己地址」的 future 验证内层没搬家；`waker_can_wake_from_another_thread` 在另一个线程睡 20 ms 后唤醒，`block_on` 照样拿到结果。

## 报错现场

### 现场一：`Waker` 的载荷必须 `Send + Sync`（E0277）

想让唤醒句柄顺手带一个 `Rc`（在单线程执行器里这个念头很自然）：

```rust,compile_fail
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Wake, Waker};

struct LocalWaker(Rc<()>);

impl Wake for LocalWaker {
    fn wake(self: Arc<Self>) {
        let _ = &self.0;
    }
}

fn main() {
    let w: Waker = Waker::from(Arc::new(LocalWaker(Rc::new(()))));
    let _ = w;
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0277]: `Rc<()>` cannot be sent between threads safely
  --> src\main.rs:14:20
   |
14 |     let w: Waker = Waker::from(Arc::new(LocalWaker(Rc::new(()))));
   |                    ^^^^^ `Rc<()>` cannot be sent between threads safely
   |
   = help: within `LocalWaker`, the trait `Send` is not implemented for `Rc<()>`
note: required because it appears within the type `LocalWaker`
  --> src\main.rs:5:8
   |
 5 | struct LocalWaker(Rc<()>);
   |        ^^^^^^^^^^
   = note: required for `Waker` to implement `From<Arc<LocalWaker>>`

error[E0277]: `Rc<()>` cannot be shared between threads safely
   ...
```

两个错误连在一起出现（`Send` 与 `Sync`）不是巧合：`Waker` 可以从任意线程调用，装在它里面的东西两样都得满足。定位路径：看到 `required for 'Waker' to implement 'From<Arc<LocalWaker>>'` 就知道问题不在 `Waker`，在你的载荷。修法是把载荷换成线程安全类型（原子类型、`Arc<Mutex<…>>`）；确实不需要跨线程的话，只能等 `LocalWaker` 稳定（见 38.4 的提示）。38.3 那份手写 vtable 版能过，是因为它的 data 指针指向 `Arc<Task>`。

### 现场二：任务的 future 必须 `Send`（E0277）

本章的执行器收的是 `Future<Output = ()> + Send + 'static`（这条约束为什么躲不掉，见本段末）。下面用一个同签名的 `spawn` 复现它，于是跨 `.await` 的 `Rc` 会被拦下：

```rust,compile_fail
use std::future::Future;
use std::rc::Rc;

fn spawn<F: Future<Output = ()> + Send + 'static>(_f: F) {}

fn main() {
    let rc = Rc::new(1u32);
    spawn(async move {
        let n = rc.clone();
        std::future::pending::<()>().await;
        println!("{n}");
    });
}
```

以 Rust 1.98 为例，关键几行：

```text
error: future cannot be sent between threads safely
  --> src\main.rs:8:5
   |
 8 | /     spawn(async move {
 9 | |         let n = rc.clone();
10 | |         std::future::pending::<()>().await;
11 | |         println!("{n}");
12 | |     });
   | |______^ future created by async block is not `Send`
   |
   = help: within `{async block@src\main.rs:8:11: 8:21}`, the trait `Send` is not implemented for `Rc<u32>`
note: future is not `Send` as this value is used across an await
  --> src\main.rs:10:38
   |
 9 |         let n = rc.clone();
   |             - has type `Rc<u32>` which is not `Send`
10 |         std::future::pending::<()>().await;
   |                                      ^^^^^ await occurs here, with `n` maybe used later
note: required by a bound in `spawn`
  --> src\main.rs:4:35
   |
 4 | fn spawn<F: Future<Output = ()> + Send + 'static>(_f: F) {}
   |                                   ^^^^ required by this bound in `spawn`
```

规则和第 21 章的 `MutexGuard` 那例完全一样（`future is not Send as this value is used across an await`），只是这次的约束来自执行器的 `spawn`。修法也一样：收窄那个值的存活区间，或者换成 `Arc`。

这里要顶掉一个直觉错误：**单线程 ≠ 不要求 `Send`**。本章的执行器只在一个线程里跑任务，但它的 `Waker` 必须是 `Send + Sync`，而 `Waker` 的载荷就是任务本身（`Arc<Task>`）——载荷线程安全，就得任务里的 future 也是 `Send`。想彻底摆脱这条约束，得让 waker 也变成「本地」的，也就是 `LocalWaker`（还没稳定）。

### 现场三：投影之后 `Pin::new` 不再放行（E0277）

`Budgeted` 持有 `!Unpin` 的内层 future 时，它自己也 `!Unpin`。于是「先造值、再 pin」的写法会撞墙：

```rust,compile_fail
use std::pin::Pin;

struct Budgeted<F> {
    inner: F,
    budget: u32,
}

async fn inner() -> u32 {
    1
}

fn main() {
    let mut b = Budgeted { inner: inner(), budget: 3 };
    let _pinned = Pin::new(&mut b);
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0277]: `{async fn body of inner()}` cannot be unpinned
    --> src\main.rs:14:28
     |
  14 |     let _pinned = Pin::new(&mut b);
     |                   -------- ^^^^^^ within `Budgeted<impl Future<Output = u32>>`, the trait `Unpin` is not implemented for `{async fn body of inner()}`
     |                   |
     |                   required by a bound introduced by this call
     |
     = note: consider using the `pin!` macro
             consider using `Box::pin` if you need to access the pinned value outside of the current scope
note: required because it appears within the type `impl Future<Output = u32>`
    --> src\main.rs:8:21
     |
   8 | async fn inner() -> u32 {
     |                     ^^^
note: required because it appears within the type `Budgeted<impl Future<Output = u32>>`
    --> src\main.rs:3:8
     |
   3 | struct Budgeted<F> {
     |        ^^^^^^^^
...
```

这正是结构化 pinning 想要的传播：内层不是 `Unpin`，外层就不该是。修法不是给 `Budgeted` 补一个 `impl Unpin`（那会让 38.5 的投影立刻变成 UB），而是换一个真会 pin 的构造入口——`pin!` 就地 pin、`Box::pin` 装箱（第 22 章）。

### 现场四：忘了登记唤醒 → 执行器报停滞

返回 `Pending` 却不登记唤醒，编译期看不出来。用本章的运行时报一次（探针调用 `block_on(async { std::future::pending::<u32>().await })`；线程号随运行变化，路径已省略）：

```text
thread 'main' panicked at snippets\ch38\src\lib.rs:255:19:
执行器停滞：队列为空且无人登记唤醒
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

定位路径：这是第 21 章第 2 条约定被违反的症状——`poll` 返回 `Pending` 时没有「确保当前任务在能继续推进时被唤醒」。本章把它做成了可诊断的：队列空 + 目标未达成 + 超时 = `Stalled`。`std::future::pending()` 正是一个「永远 `Pending`、从不登记」的 future，很适合拿来做这个实验。在真实运行时里，同一个 bug 的症状通常是「程序就这么卡住了」，什么也不打印。

## 小结

- 执行器只做四件事：维护就绪队列、调 `poll`、队列空时把线程挂起、被唤醒后再 poll；`Waker` 是它和 future 之间唯一的通道——类型擦除的句柄（data 指针 + vtable），`Send + Sync`，代价是载荷必须线程安全。
- `wake` 的契约里有两句最有用：`wake` happens-before 下一次 `poll`；多次唤醒可以合并成一次 `poll`——后者让「一个 `AtomicBool` 标记」成为正确实现。
- 造 `Waker` 有两条路：`Wake` + `Waker::from(Arc<W>)`（安全，要分配器）与 `RawWaker` + vtable（四个 `unsafe fn` 自己配平引用计数）。
- 唤醒路径的正确性全在锁的粒度上：标记与队列同一把锁，「测试并置位 + 入队」和「出队 + 清位」都在锁内，才不会丢唤醒。
- pin 投影必须手写、必须 `unsafe`，因为「父值不动 ⇒ 字段不动」这条推理编译器不知道；投影之外还要守住结构化 pinning 的 `Unpin` 约束（本章的做法是干脆不写 `Unpin` 实现）。
- 忘了登记唤醒是编译期看不出的错误，本章把它做成了运行时诊断（`Stalled`）。

## 练习

**1.** 下面这个 future 用手写状态机实现了「被 poll 三次后返回」。它能通过编译与这一次运行，但放进任何真实执行器都会出问题。问题在哪？

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct ThreeYields(u8);

impl Future for ThreeYields {
    type Output = u8;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u8> {
        let this = self.get_mut();
        this.0 += 1;
        if this.0 >= 3 {
            Poll::Ready(this.0)
        } else {
            Poll::Pending
        }
    }
}

fn main() {
    let mut fut = std::pin::pin!(ThreeYields(0));
    let mut cx = Context::from_waker(std::task::Waker::noop());
    // 手动 poll 三次，这里看起来「刚好能跑」
    let _ = fut.as_mut().poll(&mut cx);
    let _ = fut.as_mut().poll(&mut cx);
    let _ = fut.as_mut().poll(&mut cx);
}
```

**2.** 本章的执行器是单线程的，`spawn` 却要求 `Future + Send`。这条约束能不能去掉？如果去掉，会在哪一步卡住？

**3.** `Budgeted` 只有 `inner` 一个字段是结构化 pin 的。下面这个「省掉 unsafe」的改动为什么会让 38.5 的投影变成 UB？

```rust,ignore
impl<F> Unpin for Budgeted<F> {}
```

<details><summary>参考答案</summary>

**1.** 它在返回 `Pending` 之前没有登记唤醒——`_cx` 被丢在一边，`cx.waker()` 从未被调用。本章的 `poll` 契约（第 21 章第 2 条）要求：返回 `Pending` 就必须保证任务的唤醒已经安排好。上面这段之所以「能跑」，是因为 `main` 手动调了三次 `poll`，把「谁来再 poll 我」这件事从 future 转移到了调用者身上；一旦交给本章的执行器，第一次 `poll` 返回 `Pending` 之后没有任何人唤醒它，任务就永久停住，`block_on` 会报 `Stalled`。

修法是在 `Pending` 分支前加一行：

```rust,ignore
this.0 += 1;
if this.0 >= 3 {
    Poll::Ready(this.0)
} else {
    cx.waker().wake_by_ref(); // 登记
    Poll::Pending
}
```

（把 `_cx` 改回 `cx`。）这也是为什么第 21 章强调 `poll` 的参数不能忽略——它是 future 与执行器之间唯一的沟通渠道。

**2.** 不能去掉，卡在 `Waker` 上。极简执行器的唤醒句柄是 `Waker::from(Arc<Task>)`，而 `Wake` 要求 `W: Send + Sync`（第 19 章讲过这两个 trait 的含义）：唤醒可能被任何线程触发，所以承载唤醒的 `Task` 必须能跨线程共享；`Task` 里装着 future，所以那个 future 必须 `Send`。这就是 `spawn` 约束的真正来源，也是第 21 章「跨 `.await` 持锁的 future 不是 `Send`」在本章的对应物。

要真正放宽，得换一套唤醒句柄：用只在一个线程内有效的「本地 waker」（`LocalWaker` / `LocalWake`），它允许载荷里放 `Rc` 这类 `!Send` 类型——但 1.98 上它还不可用（见 38.4 的提示）。在此之前，单线程执行器碰到 `!Send` 的值只有三条路：把它换成 `Arc` 等线程安全类型；把它留在 future **外面**（future 只捕获必要的那部分，值在等待点旁边按需借还）；或者自己 `unsafe impl Send`，并承担「不会有别的线程碰到它」的证明责任。

**3.** `Unpin` 是给 unsafe 代码看的声明：「这个类型的值被 pin 住之后，我仍然可以随便移动它」。`impl<F> Unpin for Budgeted<F>` 是一个无条件实现，它宣称 `Budgeted<任何 F>` 都随便移动——包括 `F` 是 `!Unpin`（比如自引用的 async 块）的时候。而 38.5 的 `inner()` 已经把 `Pin<&mut Self>` 投影成了 `Pin<&mut F>`，等于对外承诺「外层不动 ⇒ 内层不动」。两句话一撞：任何拿到 `Pin<&mut Budgeted<F>>` 的代码都能用 `get_mut()` 拿到 `&mut Budgeted<F>`（`Unpin` 放行的），再用 `mem::swap` 把整个结构体换掉；此时内层 future 的空间被复用，而它自己内部指向自己的指针还留在原处——`Pin<&mut F>` 的保证被撕了。

正确的做法正是本章的选择：**一个 `Unpin` 实现都不要写**，让 auto trait 推导规则去管——`F: Unpin` 时 `Budgeted<F>: Unpin`，否则不是。pin 模块文档把这条叫「Structural Unpin」，并明确点名了不要写 `impl<T> Unpin for Struct<T> {}` 再对外提供结构化投射。

</details>

## 延伸阅读

- `std::task`（`Waker`、`Context`、`Poll`、`Wake`、`RawWaker`）：<https://doc.rust-lang.org/std/task/index.html>
- `Wake` trait（安全构造 `Waker` 的入口）：<https://doc.rust-lang.org/std/task/trait.Wake.html>
- `Waker`（含 `from_raw`、`new`、`noop` 与 `wake` 的内存序保证）：<https://doc.rust-lang.org/std/task/struct.Waker.html>
- `RawWaker`: <https://doc.rust-lang.org/std/task/struct.RawWaker.html>
- `RawWakerVTable`（数据指针与四个函数的契约）：<https://doc.rust-lang.org/std/task/struct.RawWakerVTable.html>
- `std::future`：<https://doc.rust-lang.org/std/future/index.html>
- `std::pin` 模块文档（Drop guarantee、Projections and Structural Pinning）：<https://doc.rust-lang.org/std/pin/index.html>
- `Pin::map_unchecked_mut`：<https://doc.rust-lang.org/std/pin/struct.Pin.html#method.map_unchecked_mut>
- The Book 的异步章（`Future`、`Pin`、执行器的入门视角）：<https://doc.rust-lang.org/book/ch17-00-async-await.html>
