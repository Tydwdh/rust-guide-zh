# 第 21 章 async 基础：Future、执行器与运行时

## 这一章解决什么问题

第 19 章的并发模型很直接：一件事一个线程，阻塞了就等着。它在「任务多、等待多」的场景里开始变贵：每个线程要占一份栈，切换要进内核，最要命的是——等待一个网络响应时，那个线程什么都干不了。想在一个进程里同时照看一万条连接，一线程一任务的账算不过来。

异步（async）把「等待」从线程的属性改成了值的一部分：一个任务可以停在某个点上，把线程让给别人，之后再从那个点继续。于是少量线程就能照看大量任务。

读完这一章你能做到：

- 说清 `Future` 的契约：它是什么、谁负责推进它、为什么标准库不提供执行器（executor）；
- 看懂 `async fn`、`async` 块与 `.await` 的关系，知道它们被编译成什么形状；
- 读懂唤醒句柄（`Waker`）的角色，明白 `poll` 返回 `Pending` 时为什么必须登记唤醒；
- 一眼认出 `!Send` 的 future——跨 `.await` 持锁是最常见的那种，并知道怎么改；
- 明白「取消（cancellation）」在 Rust 里就是 `drop`，以及这对资源清理意味着什么。

## 为什么这样设计

### 线程的账，async 的账

线程模型的调度者是操作系统：栈由内核给，换出是抢占式的——你阻塞在内核里，内核就把 CPU 分给别人。代价是每个线程的资源占用和切换成本都很实在。而 async 把调度搬进了用户代码：一个 `async` 块会被编译成一个状态机值，运行时里的执行器负责推进它；它只在 `.await` 处让出，属于协作式调度。

| | 线程（第 19 章） | async 任务（task） |
|---|---|---|
| 谁调度 | 操作系统内核 | 运行时里的执行器，普通用户代码 |
| 何时切换 | 抢占式，随时可能 | 协作式，只在 `.await` 让出 |
| 每个任务的开销 | 一份栈（通常 MB 级）＋内核结构 | 一个编译器生成的状态机（state machine），大小≈局部变量之和 |
| 某个任务阻塞了 | 只堵住自己 | 堵住同一个执行器线程上的所有任务 |
| 甜点场景 | CPU 密集、任务不多 | I/O 密集、海量小任务 |

两者不是替代关系。The Book 的 async 章有一句总结值得记住：

> The runtime—specifically, its executor—manages tasks, and tasks manage futures.
> （运行时——具体说，它的执行器——管理任务；任务管理 future。）

The Book 还给过两条经验法则：CPU 密集、能切成独立分片的活用线程；I/O 密集、来料时间不确定的活用 async。真实系统常常两者并用——多线程执行器会把任务在若干线程之间搬来搬去。

### future 是惰性的

这是 async 与「启动一个线程」最像又最不像的地方。`thread::spawn` 一调用，闭包立刻在别的线程上跑；而调用一个 `async fn` 什么都不发生，你只是拿到了一个待办事项。The Book 的原话是：

> futures in Rust are lazy: they don’t do anything until you ask them to with the await keyword.

（Rust 的 future 是惰性的：你不主动要求，它什么也不做。）

拿一个最小例子验证：

```rust
async fn greet() -> String {
    println!("async fn 的函数体开始执行");
    "hi".to_string()
}

fn main() {
    let fut = greet(); // 只是创建 future
    println!("future 已创建，但函数体还没跑");
    drop(fut);
}
```

输出（Rust 1.98 实测）：

```text
future 已创建，但函数体还没跑
```

第一行永远没打印，因为 `greet()` 的函数体根本没开始。这种惰性（laziness）有个好处：可以先把 future 攒起来（塞进 `Vec`、放进结构体字段、做成任务队列），等条件齐了再交给执行器。它和第 10 章的迭代器是同一类设计——不调用消费者就什么都不发生。

惰性也意味着忘了驱动它时，编译器会提醒你（见「报错现场」）。

### 拉，而不是推

回调式 API 是推模型：事件到了，别人调用你的闭包。Rust 的 `Future` 是拉模型：执行器主动问 future「你现在好了吗」，也就是调用 `poll`。这一问一答有个直接好处：future 不需要在任何地方保存「我是怎么被推进的」——推进的责任完全在执行器那边，future 只是被问了就答。而且 `poll` 返回一个值，不是向内递归，因此 future 的状态可以平铺在一个结构体里，不必真的占一份栈。

### 标准库里只有契约，没有心脏

打开 `std::future` 的文档，你看到的只有 `Future`、`IntoFuture` 与 `pending`/`ready`/`poll_fn` 这类现成的小积木（`Poll` 在 `std::task`）——**没有执行器**：没有 `block_on`，没有任务队列，没有定时器，没有 I/O 就绪事件的监听。标准库提供契约，把「谁来驱动」留给了生态系统。

这不是疏漏，而是分层：执行器要决定的东西太多（单线程还是多线程、怎么和 OS 的 I/O 多路复用对接、定时器精度、`no_std` 怎么办），把它们塞进标准库会把所有人都框死。代价是：写异步程序时，你几乎总要先选一个运行时（runtime）。本章末尾专门讲怎么选。

### 常见的三个误解

第一，**async 不等于并行**。一个任务里的两段 `.await` 是顺序执行的：前一个完成才会走到后一个。想并发，得让多个任务同时存在于执行器里（`join!`、`spawn`、任务队列），而并发也不等于并行——单线程执行器上它们是交替推进的，只有多个线程同时跑任务才有真正的并行。

第二，**async 不会自动帮你用满多核**。要并行，要么用多线程执行器把任务分到多个线程；要么对 CPU 密集的活直接开线程（第 19 章）。把一段纯计算的循环塞进 `async` 块，只会让同一个执行器线程被占住。

第三，**async 不省钱，只是换了一种花法**。省下的是「每个任务一份栈」和「阻塞时空转的线程」；换来的是每个 future 一个状态机，以及一层运行时调度。任务很少、几乎没有等待时，线程版往往更简单也更快。

## 机制与规则

### `Future` 的契约

`Future` 只有一个方法，但每条规则都有理由。以下是 std 文档里的定义：

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

（`Future` 从 edition 2024 起就在 prelude 里，不必手动 `use`。）

`Poll` 的定义来自 `std::task`，一并抄在这里：

```rust
pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

`poll` 的返回值只有两种：`Ready(T)` 表示算完了，结果是 `T`；`Pending` 表示**这次没算完**。围绕它有三条约定：

1. **`poll` 不许阻塞。** 文档原话：「This method does not block if the value is not ready. Instead, the current task is scheduled to be woken up when it’s possible to make further progress by polling again.」`poll` 应该尽快返回。真有耗时计算，应该丢给线程池或别的任务，而不是占着 `poll`。
2. **返回 `Pending` 就必须登记唤醒。** `Poll` 的文档把责任写得很清楚：「When a function returns `Pending`, the function must also ensure that the current task is scheduled to be awoken when progress can be made.」没有这一步，执行器不知道该在什么时候回来问你，任务就永远停在那里了。
3. **`Ready` 之后不要再 `poll`。** 文档说「Once a future has finished, clients should not poll it again.」；`Panics` 一节更直白：再 `poll` 一个已完成的 future「may panic, block forever, or cause other kinds of problems」。

还有一处签名细节：`poll` 收的是 `Pin<&mut Self>` 而不是 `&mut self`。原因是编译器生成的 future 状态机在跨 `.await` 时可能自引用，一旦被搬走，内部指针就悬了。`Pin` 就是「保证不被移动」的那层包装——它是第 22 章的主题，这里先记一句：**`.await` 需要 `Pin`，但 `Pin` 只与「不移动」有关，与「异步」本身无关。**

### `async` 与 `.await` 展开成什么

Rust Reference 对 `async fn` 的说明一句话点破：

> Async functions do no work when called: instead, they capture their arguments into a future. When polled, that future will execute the function’s body.

（`async fn` 被调用时不做任何工作：它把实参捕获进一个 future；被 `poll` 时，那个 future 才执行函数体。）

`async` 块同理。被编译出来的是一个枚举状的状态机，每个 `.await` 是一个可能的切点，切点两侧的局部变量被搬进状态机的字段。`.await` 做的事，Reference 的说法是「a syntactic construct for suspending a computation provided by an implementation of `std::future::IntoFuture` until the given future is ready to produce a value」，并给了一段大致展开：

```rust,ignore
// Reference 的「近似展开」，不是可编译代码：yield 是伪代码
match operand.into_future() {
    mut pinned => loop {
        let mut pin = unsafe { Pin::new_unchecked(&mut pinned) };
        match Pin::future::poll(Pin::borrow(&mut pin), &mut current_context) {
            Poll::Ready(r) => break r,
            Poll::Pending => yield Poll::Pending,
        }
    }
}
```

要看懂两件事：`.await` 其实是「循环 poll 一个 future，直到它 Ready」，而中途 `yield Poll::Pending` 就是把控制权还给执行器。换句话说，**`.await` 不是「阻塞等待」，而是「登记一下，我先让位」**。

顺带一个容易忽略的点：`.await` 接受的其实是 `IntoFuture` 而不是 `Future`——前者是后者的超集，`IntoFuture` 为所有 `T: Future` 实现了。所以 `.await` 能作用在普通 future 上，也能作用在实现了 `IntoFuture` 的 builder 类型上。细节不重要，记住 `.await` 判据是「能变成 future」即可。

同一个家族的还有 `async` 块与 async 闭包（1.85 起稳定，写作 `async |x| x + 1`），它们产出的同样是 future，同样惰性。本章后面统一说「future」，指的就是这些写法产出的东西。

`async fn` 的返回值是不透明类型（第 14 章）：你无法写出它的具体名字，只保证 `impl Future`。这也解释了为什么 future 一旦变复杂，就得装箱成 `Pin<Box<dyn Future<Output = T> + Send + '_>>`——把类型擦除掉。

状态机还有一个副作用要知道：**跨 `.await` 还活着的值会被搬进 future**，所以 future 的类型里可能带着对外的借用。这带来两条实践规则。其一，`async` 块默认借用捕获，需要把所有权搬进去时写 `async move`；而 `async fn` 的实参本来就是按值捕获进 future 的。其二，如果 future 借用了别处的数据，那个 future 就不能比借来的数据活得久——这在把 future 存进结构体或任务队列时最容易撞上，编译器会用生命周期错误指出来。

「把 future 当成一个普通的、可能很大的值来看待」——这一章后面所有规则都会顺很多：它会被搬动（所以需要 `Pin`）、也会被丢掉（所以取消就是 `drop`）、它是什么类型由里面存了哪些局部变量决定（所以 `Send` 与否要看跨 `.await` 的值）。

把写法和编译器生成的东西并排放，就是下图：

<svg viewBox="0 0 660 256" width="100%" role="img" aria-label="async fn 与它编译出的状态机：左边源码里两个 .await 各是一个挂起点，右边编译器生成的枚举中，每个变体的字段正是跨挂起点还活着的局部变量 a 和 b，跑完后返回 a + b + c" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">async fn 的写法与编译器生成的状态机</text>
<text x="0" y="42" font-size="12" fill="currentColor">你写的</text>
<g font-family="monospace" font-size="12" fill="currentColor" xml:space="preserve" style="white-space:pre">
<text x="0" y="66">async fn f() -&gt; u32 {</text>
<text x="0" y="88">    let a = read_a();</text>
<text x="0" y="110">    let b = wait_b().await;  // 切点 1</text>
<text x="0" y="132">    let c = wait_c().await;  // 切点 2</text>
<text x="0" y="154">    a + b + c</text>
<text x="0" y="176">}</text>
</g>
<line x1="306" y1="140" x2="344" y2="140" stroke="var(--links)" stroke-width="1.5" />
<polygon points="356,140 344,134 344,146" fill="var(--links)" />
<text x="308" y="132" font-size="12" fill="currentColor">编译</text>
<text x="364" y="42" font-size="12" fill="currentColor">编译器生成的（一个带标签的枚举）</text>
<rect x="364" y="56" width="290" height="34" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="376" y="78" font-size="12" fill="currentColor">变体 0</text>
<text x="444" y="78" font-size="12" font-family="monospace" fill="var(--links)">{ }</text>
<text x="520" y="78" font-size="12" fill="currentColor" opacity="0.75">还没推进</text>
<rect x="364" y="100" width="290" height="34" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="376" y="122" font-size="12" fill="currentColor">变体 1</text>
<text x="444" y="122" font-size="12" font-family="monospace" fill="var(--links)">{ a }</text>
<text x="520" y="122" font-size="12" fill="currentColor" opacity="0.75">停在切点 1</text>
<rect x="364" y="144" width="290" height="34" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="376" y="166" font-size="12" fill="currentColor">变体 2</text>
<text x="444" y="166" font-size="12" font-family="monospace" fill="var(--links)">{ a, b }</text>
<text x="520" y="166" font-size="12" fill="currentColor" opacity="0.75">停在切点 2</text>
<rect x="364" y="188" width="290" height="34" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="376" y="210" font-size="12" fill="currentColor">变体 3</text>
<text x="444" y="210" font-size="12" font-family="monospace" fill="var(--links)">a + b + c</text>
<text x="520" y="210" font-size="12" fill="currentColor" opacity="0.75">完成，返回值</text>
<text x="364" y="244" font-size="12" fill="currentColor" opacity="0.75">每个挂起变体还存着正在等待的内层 future（图中省略）</text>
</svg>

### 谁在推进：执行器与 `Waker`

执行器做的事，抽象看只有一个循环：poll 任务，收到 `Pending` 就去睡，等有人叫醒再回来 poll。为了让「唤醒（wake）」这件事有载体，`poll` 会收到一个 `Context`，里面包着一个 `Waker`。`Waker` 的文档定义是：

> A Waker is a handle for waking up a task by notifying its executor that it is ready to be run.

（`Waker` 是一个句柄：通过通知执行器「这个任务可以跑了」，把任务唤醒。）

执行器与 future 之间的往复摊开来看，就是下图：

<svg viewBox="0 0 660 244" width="100%" role="img" aria-label="执行器与 future 的往复：执行器 poll 任务，future 在 .await 处把 Context 里的 Waker 交给事件源并返回 Pending，执行器随即挂起；事件源就绪后调用 waker.wake() 叫醒执行器，执行器再 poll 同一个任务，这次拿到 Ready" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">执行器与 future 的往复：poll → Pending → 登记 Waker → wake → 再 poll</text>
<rect x="20" y="30" width="170" height="28" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="105" y="49" font-size="12" fill="currentColor" text-anchor="middle">执行器</text>
<rect x="250" y="30" width="170" height="28" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="335" y="49" font-size="12" fill="currentColor" text-anchor="middle">future（任务）</text>
<rect x="470" y="30" width="170" height="28" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="555" y="49" font-size="12" fill="currentColor" text-anchor="middle">I/O 或定时器线程</text>
<line x1="105" y1="58" x2="105" y2="232" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<line x1="335" y1="58" x2="335" y2="232" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<line x1="555" y1="58" x2="555" y2="232" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<line x1="105" y1="84" x2="327" y2="84" stroke="var(--links)" stroke-width="1.5" />
<polygon points="335,84 326,79 326,89" fill="var(--links)" />
<text x="120" y="77" font-size="12" fill="currentColor">① poll(cx)</text>
<line x1="335" y1="112" x2="547" y2="112" stroke="var(--links)" stroke-width="1.5" />
<polygon points="555,112 546,107 546,117" fill="var(--links)" />
<text x="352" y="105" font-size="12" fill="currentColor">② 把 cx.waker 登记到事件源</text>
<line x1="335" y1="140" x2="117" y2="140" stroke="var(--links)" stroke-width="1.5" />
<polygon points="105,140 114,135 114,145" fill="var(--links)" />
<text x="143" y="133" font-size="12" fill="currentColor">③ 返回 Poll::Pending</text>
<text x="20" y="166" font-size="12" fill="currentColor" opacity="0.75">④ 挂起：不再 poll（线程去干别的或睡下）</text>
<line x1="555" y1="192" x2="117" y2="192" stroke="var(--links)" stroke-width="1.5" />
<polygon points="105,192 114,187 114,197" fill="var(--links)" />
<text x="180" y="185" font-size="12" fill="currentColor">⑤ waker.wake()</text>
<line x1="105" y1="218" x2="327" y2="218" stroke="var(--links)" stroke-width="1.5" />
<polygon points="335,218 326,213 326,223" fill="var(--links)" />
<text x="120" y="211" font-size="12" fill="currentColor">⑥ 再 poll(cx)：这次 Ready</text>
</svg>

`Waker` 是 `Clone + Send + Sync` 的，文档特意点出它的含义：「a waker may be invoked from any thread, including ones not in any way managed by the executor.」——也就是说，一个网络事件线程可以直接拿到 waker，在没有运行时的线程上把任务叫醒。这就是「谁登记唤醒」里的那个「谁」。

下面这个最小例子先展示「只驱动一次」的样子。它用 `Waker::noop()` 造一个空唤醒器，适合立刻完成的 future：

```rust
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

fn poll_once<F: Future>(fut: F) -> Option<F::Output> {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(v) => Some(v),
        Poll::Pending => None,
    }
}

fn main() {
    let a = poll_once(async { 1 + 1 });
    let b = poll_once(async {
        std::future::pending::<u32>().await;
        0
    });
    println!("{a:?} {b:?}");
}
```

```text
Some(2) None
```

`Waker::noop()` 的文档说它在「discarding the notification of when the future should be polled again」——丢掉唤醒通知。所以它只能用于「我确定不需要再被叫醒」的场合（比如测试）。要让它真的能等，就得给 `poll` 一个真会醒的 waker。粗略版本是这样：

```rust
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// 只够跑「立刻就好」的 future：没有真正的唤醒机制。
/// 一遇到 Pending 就会一直空转，永远出不来。
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

async fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    println!("{}", block_on(double(21)));
    println!("{}", block_on(async { 1 + 1 }));
}
```

```text
42
2
```

它能跑，但它不是执行器：`Pending` 时它在空转，且永远不会被唤醒。真版本要有一个能被别的线程触发的 waker，还要在等待时把线程真正挂起。「动手看看」里那一份就是这样写的。

> [!WARNING]
> 不要在 `poll` 里空转。`poll` 返回 `Pending` 后，执行器线程应该去干别的（或睡下）。写一个 `loop { if pending { continue } }` 会烧掉一个核，还会饿死同一执行器上的其它任务。

### 组合子：`join!` 与 `select!`

有了「多个 future + 一个执行器」，最常见的两种组合需求是：等全部完成（并发，不是并行），以及等第一个完成。

```rust,ignore
// 需要外部 crate（futures），故不进本书的编译测试
use futures::{join, select};

// 等全部完成，拿到各分支的结果
let (a, b) = join!(fetch("a"), fetch("b"));

// 谁先完成就用谁
select! {
    r = fetch("a") => println!("a 先到：{r:?}"),
    r = fetch("b") => println!("b 先到：{r:?}"),
}
```

这两个宏在 stable 上都用不上：`std::future::join!` 至今是 nightly-only 实验 API（`future_join`），`select!` 则只在 `futures` 这类 crate 里。**语义上要记住两点**：

- `join!` 是「都跑完再回来」，返回各分支结果的元组；它并不会把任务分到多核上，只是在一个任务里轮流推进多个 future。
- `select!` 是「谁先回来算谁」，其余分支在那一刻被**取消**——取消的具体形式就是 `drop`。

### 取消：`drop` 掉一个 future

Rust 没有单独的「取消」API，因为不需要：future 就是一个普通值，取消就是 `drop`。它停在哪、状态机里存着什么，都会被正常析构。这有两面：

- **好的一面**：取消点上的资源清理是你已经会的 `Drop`，不需要额外的协议。
- **要小心的一面**：一个 `async fn` 不保证跑完。跨 `.await` 借来的资源、启动了一半的协议状态，都可能在下一毫秒被丢掉。写 `select!` 或设置超时时，得按「随时可能被砍」来设计。

这也解释了 `select!` 里那个「谁先完成算谁」的代价：另几个分支不是暂停，是彻底没了。

### `!Send` 的 future

执行器常把任务丢到别的线程上跑（多线程调度），这就要求任务本身是 `Send`。而一个 future 是不是 `Send`，取决于它的状态机里有没有存 `!Send` 的值——也就是，**有没有 `!Send` 的值跨过了 `.await`**。

跨 `.await` 持锁是最常见的写法，也是最常见的坑：

```rust,compile_fail
use std::sync::Mutex;

fn assert_send<T: Send>(_: T) {}

fn main() {
    let m = Mutex::new(0u32);
    assert_send(async {
        let guard = m.lock().unwrap();
        std::future::ready(()).await; // 这行 .await 落在 guard 的存活区间里
        println!("{}", *guard);
    });
}
```

以 Rust 1.98 为例，关键几行：

```text
error: future cannot be sent between threads safely
   = help: within `{async block@src/main.rs:7:17: 7:22}`, the trait `Send` is not implemented for `std::sync::MutexGuard<'_, u32>`
note: future is not `Send` as this value is used across an await
  --> src/main.rs:9:32
   |
 9 |         std::future::ready(()).await; // 这行 .await 落在 guard 的存活区间里
   |                                ^^^^^ await occurs here, with `guard` maybe used later
```

`MutexGuard` 是 `!Send`（它必须由取锁的线程释放），而它又跨了 `.await`，于是整个状态机不是 `Send`。修法不是换锁，而是**收窄 guard 的存活区间**，让它在 `.await` 之前就析构：

```rust
use std::sync::Mutex;

fn assert_send<T: Send>(_: T) {}

fn read(m: &Mutex<u32>) -> u32 {
    *m.lock().unwrap()
}

fn main() {
    let m = Mutex::new(0u32);

    // 修法一：把 guard 关进一个块里，块结束就析构
    assert_send(async {
        {
            let guard = m.lock().unwrap();
            println!("{}", *guard);
        }
        std::future::ready(()).await;
    });

    // 修法二：把「取锁读数」收进普通函数，async 块里只剩一个 u32
    assert_send(async {
        let v = read(&m);
        std::future::ready(()).await;
        println!("{v}");
    });
}
```

两种都编译通过（实测）。修法二更值得推荐：`read` 是个同步函数，「临界区里不放 `.await`」这条规矩就固化在它的签名里了。

同样的道理适用于任何 `!Send` 值：`Rc`、`RefCell` 的借用 guard、`!Send` 的 I/O 句柄。诊断时先看报错里的 `future is not Send as this value is used across an await` 那几行——它会把肇事的值指出来。

> [!NOTE]
> 第 14 章讲过 `async fn in trait`（AFIT）的同一处痛点：trait 里的 `async fn` 只承诺返回「某个 future」，没说它 `Send`，所以泛型调用方不能跨线程执行它；要么改写签名加上 `Send`，要么装箱成 `Pin<Box<dyn Future<Output = T> + Send + '_>>`。同一节还提到另一条限制：含 `async fn` 的 trait 不能做 trait 对象。

### 运行时怎么选

因为标准库不带执行器，选运行时是必答题。判据按重要性排：

1. **除了执行器还提供什么。** 大多数程序真正要的是 I/O 与定时器——「等这个 socket 可读」「200 毫秒后叫我」。这些是运行时提供的，不是 `Future` 自带的。只用某个运行时的执行器、却想用另一个运行时的 I/O future，通常行不通：那个 future 需要它自家的驱动器来登记唤醒。
2. **生态是否统一。** 你依赖的库如果写死了某一族的 trait（比如只接受某运行时的 `AsyncRead`），你就被绑定了。选之前先看依赖树。
3. **调度模型。** 单线程执行器（任务不要求 `Send`、可与 `Rc` 共存）还是多线程执行器（任务必须 `Send`、第 19 章的规则直接生效）。有些运行时两种都提供。
4. **目标平台。** `no_std`、无操作系统的嵌入式环境里，能用的运行时是另一个集合。
5. **配套工具。** 能否在测试里控制时间（暂停虚拟时钟）、有没有可观测性支持（tracing）、任务能否指定到某个线程。

本章不推荐具体运行时：这三年的生态还在动，选型要看你当下依赖的那棵树。The Book 的 async 章整章使用某一个运行时来举例，读它时可当作一个参照。

## 动手看看

下面这个程序里有一份能真正「等」的执行器。它把前面几节的零件装起来：

- `Parker`：实现 `Wake` 的唤醒句柄，内部是「一个 bool + 一个 `Condvar`」；
- `block_on`：单任务，poll 到 `Pending` 就 `park`；
- `block_on_all`：把一组 future 当任务队列，每次唤醒后把没完成的都推进一遍；
- `Delay`：一个真正的异步等待——第一次 `poll` 时把 `waker` 复制给后台线程，睡够后由那个线程 `wake()`。

```rust
{{#rustdoc_include ../snippets/ch21/executor.rs}}
```

在仓库根编译并运行（下面按 Windows 写；其他平台编译成 `executor`、运行 `./executor`）：

```console
$ rustc --edition 2024 snippets/ch21/executor.rs -o executor.exe
$ .\executor.exe
```

```text
顺序 await 两个 Delay：150 ms
并发跑两个 Delay：90 ms -> ["A(60ms)", "B(90ms)"]
```

（毫秒数随机器浮动；要紧的是两条：顺序版本是两段之和，并发版本约等于较慢的那一段。）

三个地方值得停下来看：

**一、`Waker` 真的被用到了。** `Delay::poll` 收到的 `cx` 里带着执行器给的 waker；它 `clone` 一份交给后台线程，线程睡完后 `waker.wake()`。`wake` 触发 `Parker::wake_by_ref`：写标记、`notify_one`。挂起在 `Condvar::wait` 上的执行器线程于是醒来，重新 poll——这一次 `done` 是 `true`，返回 `Ready`。整条链上没有任何忙等。

**二、就绪标记与唤醒标记的先后不能反。** `Delay` 那边先置 `done`，再 `wake()`；执行器这边 `park()` 则是先看 `Parker::woken` 再阻塞——这正是为了堵一个经典的丢唤醒窗口：如果 `wake()` 恰好在「poll 返回 `Pending`」与「线程真正睡下」之间发生，只看 `Condvar` 就会睡过头。生产级执行器都要处理这类唤醒与阻塞之间的时序窗口（第 38 章会把它讲完整）。

**三、多任务才是 async 的意义。** `block_on_all` 里，两个 `Delay` 各自返回 `Pending` 后，执行器先睡下；60 ms 时 A 的线程唤醒它，推进一轮后再次睡下，90 ms 时 B 的线程再唤醒它——两次睡眠、两次唤醒，总耗时由最慢的一个决定，而不是相加。这就是「一个线程照看许多任务」在最小尺度上的样子。

再看取消：

```rust
{{#rustdoc_include ../snippets/ch21/cancel.rs}}
```

```console
$ rustc --edition 2024 snippets/ch21/cancel.rs -o cancel.exe
$ .\cancel.exe
```

```text
任务 A 开始，停在等待点
drop future
任务 A 的局部变量 被 drop
main 结束
```

这个程序先 `poll` 一次，把 future 推进到 `pending().await` 处，然后 `drop` 它。可以看到析构是「随着状态机一起发生的」：停在等待点上的那个局部变量 `_loud` 被正常 drop，没有泄漏，也没有谁需要显式通知。换成真实场景，这里的 `Loud` 可能是一笔数据库事务、一份半写完的文件——`drop` 会照常把它交还给析构逻辑。

## 报错现场

### 1. `.await` 用在非 async 上下文（E0728）

最小复现：

```rust,compile_fail
fn main() {
    let n = std::future::ready(1u32).await;
    println!("{n}");
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0728]: `await` is only allowed inside `async` functions and blocks
 --> src/main.rs:2:38
  |
1 | fn main() {
  | --------- this is not `async`
2 |     let n = std::future::ready(1u32).await;
  |                                      ^^^^^ only allowed inside `async` functions and blocks
```

定位路径：`.await` 需要一个「任务上下文」——也就是某个执行器正在 poll 的那个 async 上下文。`main` 是普通函数，没人 poll 它，所以没有上下文可传。

修法：把逻辑包进 async 块，再交给执行器。可运行的例子是 `block_on(async { ... })`；生产代码里则用运行时的属性宏把 `async fn main` 包起来——展开结果仍然是一个普通 `fn main`，内部调用运行时的 `block_on`。

### 2. 忘了 `.await`

最小复现：

```rust
async fn greet() -> String {
    "hi".to_string()
}

fn main() {
    greet();
}
```

以 Rust 1.98 为例，关键几行：

```text
warning: unused implementer of `Future` that must be used
 --> src/main.rs:6:5
  |
6 |     greet();
  |     ^^^^^^^
  |
  = note: futures do nothing unless you `.await` or poll them
  = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
```

看到 `unused implementer of Future` 就是惰性在提醒你：创建了却没驱动。注意这只是 warning——如果函数返回的 future 有意交给别人驱动，用 `let fut = greet();` 绑定一下就能消掉它。

### 3. 跨 `.await` 持锁 → future 不是 `Send`

复现与关键行见「`!Send` 的 future」一节。定位路径：找报错里的 `future is not Send as this value is used across an await`，它会指名那个 `!Send` 的值。修法是收窄它的存活区间，而不是去换数据类型。

## 小结

- `Future` 只有 `poll` 一个方法，约定三条：不阻塞；返回 `Pending` 必须登记唤醒；返回 `Ready` 后别再 poll。
- `async fn` 与 `async` 块都是惰性的：调用只产生一个 future，被 poll 才执行函数体。`.await` 的语义是「循环 poll 到 Ready，中途让位」。
- 标准库只有契约，没有执行器；`Waker` 是任务与执行器之间的唤醒通道，可以从别的线程触发。
- 执行器 = 「poll → Pending 就睡 → 被唤醒再 poll」的循环；`.await` 需要 `Pin`，因为状态机可能自引用（第 22 章）。
- 取消就是 `drop`：等待点上的局部变量照常析构，但 `async fn` 不保证跑完，跨 `.await` 的资源要按「随时可能被砍」来设计。
- future 的 `Send` 由「状态机里有没有 `!Send` 值跨过 `.await`」决定；跨 `.await` 持锁是最常见的反例。

## 练习

**1.** 下面这个函数不打印任何东西，为什么？改成能打印的版本。

```rust
async fn work() {
    println!("干活");
}

fn main() {
    work();
}
```

**2.** 判断下面代码能否编译。如果不能，给出最小修改让它编译（假设确实要让它编译）。

```rust,compile_fail
use std::sync::Mutex;

fn main() {
    let m = Mutex::new(0u32);
    let fut = async {
        let guard = m.lock().unwrap();
        std::future::ready(()).await;
        println!("{}", *guard);
    };
    needs_send(fut);
}

fn needs_send<T: Send>(_: T) {}
```

**3.** `select!` 的两个分支里，一个刚被 `poll` 一次、尚未完成；另一个立刻完成。前一个分支的 future 会发生什么？如果它内部持有一个用来写日志的文件句柄，会怎样？

<details><summary>参考答案</summary>

**1.** 因为 `async fn` 是惰性的：`work()` 只创建一个 future，没有任何东西 poll 它，函数体不会执行，同时编译器会给一条 `unused implementer of Future that must be used` 的 warning。修改方式是用执行器驱动它，最小版本是本章的 `block_on`：

```rust
# use std::future::Future;
# use std::pin::pin;
# use std::task::{Context, Poll, Waker};
# fn block_on<F: Future>(fut: F) -> F::Output {
#     let mut fut = pin!(fut);
#     let mut cx = Context::from_waker(Waker::noop());
#     loop {
#         if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
#             return v;
#         }
#     }
# }
async fn work() {
    println!("干活");
}

fn main() {
    block_on(work());
}
```

（上面用 `#` 隐藏了 `block_on` 的定义——就是本章前面那个版本。）

或者把它 `.await` 进一个更大的 async 块里，交给运行时。

**2.** 不能编译。报错是 `future cannot be sent between threads safely`，并附 ``future is not `Send` as this value is used across an await``：`MutexGuard` 是 `!Send`，而它跨过了 `.await`。最小修改是把 guard 关进一个块，让它在 `.await` 之前析构：

```rust
use std::sync::Mutex;

fn main() {
    let m = Mutex::new(0u32);
    let fut = async {
        {
            let guard = m.lock().unwrap();
            println!("{}", *guard);
        }
        std::future::ready(()).await;
    };
    needs_send(fut);
}

fn needs_send<T: Send>(_: T) {}
```

如果要求「先取锁、等一会儿、再用锁里的值」，那得重新设计：要么改成先 await、后取锁；要么把要用的值复制出来，让 guard 不跨等待点。

**3.** `select!` 会在另一个分支完成的瞬间返回，把未完成的分支 drop 掉。那个 future 的状态机随之析构，它内部存活的局部变量（包括已经轮询过一次的异步操作）会被正常 `drop`。所以那个文件句柄会随着 future 析构而被关闭——只要它被 future 拥有（比如 `File`），就不会泄漏；但如果句柄是借来的或存在别处，取消时不会有人替你完成「写完整条日志」这类业务动作。这正是「取消 = drop」要按「随时可能被砍」设计的原因。

</details>

## 延伸阅读

- The Book 的异步章：<https://doc.rust-lang.org/book/ch17-00-async-await.html>（含 `Future`、`Pin`、`Stream` 与 futures/tasks/threads 的收尾对照）
- The Book 第 17.6 节「futures、tasks 与 threads」：<https://doc.rust-lang.org/book/ch17-06-futures-tasks-threads.html>
- `std::future`：<https://doc.rust-lang.org/std/future/index.html>
- `Future` trait：<https://doc.rust-lang.org/std/future/trait.Future.html>
- `std::task`（`Waker`、`Context`、`Poll`）：<https://doc.rust-lang.org/std/task/index.html>
- `IntoFuture`：<https://doc.rust-lang.org/std/future/trait.IntoFuture.html>
- Rust Reference，await 表达式：<https://doc.rust-lang.org/reference/expressions/await-expr.html>
- Rust Reference，异步函数：<https://doc.rust-lang.org/reference/items/functions.html#async-functions>
