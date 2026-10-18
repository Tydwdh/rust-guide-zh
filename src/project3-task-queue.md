# 项目 3：迷你任务队列

第 19–22 章把并发与异步的概念一件件拆开讲：线程与 `Send`/`Sync`、通道、锁、原子、`async`/`Future`、`Pin`。这一章反过来，用标准库做一个能用的**线程池（thread pool）**：提交闭包，固定数量的工作线程（worker）并行执行，结果拿得回来，关闭时手上的活干完再退出。代码在 `examples/task-queue/`，零依赖、不用 `unsafe`，也不用 `async`（异步版运行时留到第 38 章）。

## 这一章解决什么问题

### 要做的东西

`task-queue` 是一个库 crate（`lib`），对外只有三样东西：池 `ThreadPool`、句柄 `TaskHandle<T>`，以及等待超时时给出的 `Timeout`。

| 入口 | 作用 |
|---|---|
| `ThreadPool::new(size)` | 固定 `size` 个 worker，队列无界，提交永不阻塞 |
| `ThreadPool::bounded(size, capacity)` | 队列最多缓冲 `capacity` 个任务，满了提交就阻塞 |
| `pool.spawn(f)` | 提交任务，不要结果 |
| `pool.submit(f)` | 提交任务，返回句柄 `TaskHandle<T>` |
| `handle.join()` | 一直等到任务结束 |
| `handle.join_timeout(d)` | 最多等 `d`，等不到就返回 `Timeout` |
| `pool.shutdown()` / `drop(pool)` | 关掉队列，等 worker 把手上的活干完 |

一次提交在这几样东西之间走一圈：

<svg viewBox="0 0 600 240" width="100%" role="img" aria-label="一次提交的路径：调用方把闭包装箱成 Job 送进队列，worker 在锁里取出任务、在锁外执行，结果经每任务一条私有通道回到句柄，调用方在句柄上 join" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">一次提交的路径：提交 → 执行 → 结果回来</text>
<g fill="none" stroke="currentColor" stroke-width="1.5">
<rect x="8" y="34" width="160" height="84" rx="6" />
<rect x="196" y="34" width="160" height="84" rx="6" />
<rect x="384" y="34" width="208" height="84" rx="6" />
<rect x="384" y="150" width="208" height="66" rx="6" />
<rect x="8" y="150" width="160" height="66" rx="6" />
</g>
<g font-size="13" fill="currentColor" text-anchor="middle">
<text x="88" y="56">调用方</text>
<text x="276" y="56">队列</text>
<text x="488" y="56">worker × N</text>
<text x="488" y="172">结果端</text>
<text x="88" y="172">句柄</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle" font-family="monospace">
<text x="88" y="78">pool.submit(f)</text>
<text x="276" y="78">Sender&lt;Job&gt;</text>
<text x="488" y="78">Arc&lt;Mutex&lt;Receiver&lt;Job&gt;&gt;&gt;</text>
<text x="488" y="193">Sender&lt;thread::Result&lt;T&gt;&gt;</text>
<text x="88" y="193">TaskHandle&lt;T&gt;</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle">
<text x="88" y="99">装箱成 Job</text>
<text x="276" y="99">无界，或有界</text>
<text x="488" y="99">锁里取，锁外跑</text>
<text x="488" y="209">值，或 panic 负载</text>
<text x="88" y="209">join / join_timeout</text>
</g>
<g stroke="var(--links)" stroke-width="2" fill="none">
<line x1="168" y1="76" x2="190" y2="76" />
<line x1="356" y1="76" x2="378" y2="76" />
<line x1="488" y1="118" x2="488" y2="146" />
<line x1="384" y1="183" x2="174" y2="183" />
<line x1="88" y1="150" x2="88" y2="122" />
</g>
<g fill="var(--links)">
<polygon points="196,76 186,71 186,81" />
<polygon points="384,76 374,71 374,81" />
<polygon points="488,150 483,140 493,140" />
<polygon points="168,183 178,178 178,188" />
<polygon points="88,118 83,128 93,128" />
</g>
<g font-size="12" fill="var(--links)">
<text x="496" y="140" text-anchor="start">结果</text>
<text x="44" y="140" text-anchor="start">join</text>
</g>
</svg>

### 验收标准

做完要能满足下面四条，缺一条都算没做完：

1. `cargo run -p task-queue --example demo` 能跑起来（在 `examples/` 里；仓库根出发则加 `--manifest-path examples/Cargo.toml`）；
2. 任务 panic 只炸自己：worker 照旧活着，别的任务照旧跑，panic 负载能从句柄上拿回来 —— 绝不让整个池跟着崩；
3. 关闭是优雅的：调用 `shutdown` 或让池离开作用域之后，已经提交的任务**全部跑完**，worker 才退出；
4. `cargo test --manifest-path examples/Cargo.toml -p task-queue` 全绿，覆盖正常路径、边界（队列容量 0 与容量大于 0、等待超时、零 worker）与错误路径（任务 panic），并发用例可以重复跑而不翻车。

本次验证用稳定版 rustc 1.98.0、edition 2024。终端记录都真跑过；只省掉 cargo 自己的 `Compiling` / `Finished` / `Running` 行，以及末尾的耗时汇总行。

### 先看效果

`examples/task-queue/examples/demo.rs` 演示四件事，连跑两次的现实输出是这样（两次都是真的）：

```console
$ cargo run -q --manifest-path examples/Cargo.toml -p task-queue --example demo
完成顺序：丙 甲 乙 丁
平方和：30

thread '<unnamed>' (6720) panicked at task-queue\examples\demo.rs:34:30:
这个任务故意炸了
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
任务 panic 了：这个任务故意炸了
池还活着：是的
关闭前排队的任务都跑完了：[0, 2, 3, 4, 1]
$ cargo run -q --manifest-path examples/Cargo.toml -p task-queue --example demo
完成顺序：甲 乙 丙 丁
平方和：30

thread '<unnamed>' (26512) panicked at task-queue\examples\demo.rs:34:30:
这个任务故意炸了
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
任务 panic 了：这个任务故意炸了
池还活着：是的
关闭前排队的任务都跑完了：[0, 2, 1, 3, 4]
```

这场输出里有四件性质完全不同的事：

- **完成顺序每次都不一样**：八个任务的活摊给三个 worker，谁先跑完由调度决定。测试里不能对这类顺序下断言。
- **平方和、panic 那句话、池还活着那句话每次都一样**：每个任务的结果走自己那条私有通道，不经过共享状态。
- **`panicked at` 那几行还在**：默认 panic hook 在 panic 被抓住**之前**就把消息打到标准错误上了（「设计取舍」第 4 条展开）；括号里是系统给的线程号。
- **关闭时排队的任务都跑完了**：两次的完成顺序不同，但合起来都是 `0..5` 这五个任务 —— 关的是队列，不是任务。

## 设计取舍

动手之前先定几件影响全局的事，每条都给「为什么不选另一个」。

**1. 任务怎么表示？** 一个装箱的闭包：

```rust
{{#include ../examples/task-queue/src/pool.rs:14}}
```

三条约束各有来路，缺一条都编不过：

- `Box<dyn …>`：队列里要装很多种闭包，每个闭包的匿名类型都不同，而 `channel` 只收发一种类型 —— 必须装箱、擦成同一个 trait 对象。
- `FnOnce`：任务只跑一次，还要能**消费**捕获到的值（比如把 `String` 移进去再用掉）；换成 `Fn` 反而更严。
- `Send`：任务在**别的线程**上执行，闭包连同捕获物都要被搬过去（第 19 章）。
- `'static`：任务被塞进比调用栈活得久的队列 —— `submit` 返回后那个局部作用域就没了，任务可能还排着队，所以闭包只能自己拥有数据。想借栈上的切片就用 `thread::scope`（第 19 章）：作用域结束前它派出的线程都被 join，借用天然安全 —— 代价是池子本身也得活在作用域里，每个签名都拖着 `'scope`。这个项目选了 `'static`。

**2. 谁能从队列里取任务？** 锁 + 通道：

`std::sync::mpsc` 的接收端文档第一句就是 **The receiving half of Rust's `channel` (or `sync_channel`) type. This half can only be owned by one thread.** —— 一条通道只有一个接收端，而 worker 有 N 个，所以它必须共享。办法就是第 19 章那招 `Arc<Mutex<Receiver<Job>>>`：每个 worker 拿一份 `Arc`，取任务时 `lock`，取到就**立刻放锁**；任务体在锁外跑，所以 N 个 worker 是真正并行的。

不选「每个 worker 一条通道，提交时轮着发」：那样完全不需要锁，代价是队头阻塞 —— 第 k 个任务被派给 worker k 之后，那个 worker 手上要是有个慢任务，后面派给它的活只能干等，别的 worker 却闲着。共享接收端没有这个问题：谁空谁取。多出来的开销是每个任务一对加锁/解锁，相对一次任务调度可以忽略。

**3. 提交之后结果怎么回来？** 每个任务配一条只属于它的私有通道：

```rust,ignore
let (tx, rx) = mpsc::channel();
```

`submit` 把 `tx` 塞进闭包，把 `rx` 包成 `TaskHandle` 还给调用方。任务跑完就 `tx.send(值)`；任务 panic 就把 panic 负载发回来。句柄的 `join` 就是在这条通道上 `recv`，返回类型与 `std::thread::JoinHandle::join` 一模一样（`std::thread::Result<T>`，也就是 `Result<T, Box<dyn Any + Send + 'static>>`）—— 读者在标准库里学过一次的 API 形状，这里不用重学。

不选「结果统一塞回一条公共通道」：那样要么乱序（谁先跑完谁先发，调用方得自己按 id 配对），要么在池里养一张「id → 收件人」的表再配一把锁。私有通道把配对问题消掉了 —— 句柄本来就是接收端。副作用是 `Receiver` 不是 `Sync`（第 19 章那张分布表里的一格），所以句柄不能共享、但可以**搬**：在 A 线程提交、在 B 线程 `join` 没问题，测试里专门验了这一条。

**4. 任务 panic 怎么隔离？** 在 worker 调用任务体的地方套一层 `catch_unwind`：

```rust,ignore
let _ = catch_unwind(AssertUnwindSafe(job));
```

标准库说它 **Invokes a closure, capturing the cause of an unwinding panic if one occurs.**，并要求闭包满足 `UnwindSafe`：任务 panic 到一半时，捕获的变量可能停在中间状态，而那个状态之后还可能被外面看到。我们的用法没有后半段 —— panic 之后只把负载原样转发，再也不碰捕获物，所以那句「我保证这是安全的」成立。这个包装类型的定位也就是这句：**A simple wrapper around a type to assert that it is unwind safe.**

代价和边界要写清楚：

- 默认 panic hook 会**先把消息打到标准错误**，然后才开始展开、才轮到 `catch_unwind` 抓。所以「先看效果」里那几行 `panicked at` 是正常的，不是池漏了；测试输出里同样会看到它（libtest 会把输出收起来，只在失败时展示）。
- `catch_unwind` **抓不到 abort 型的 panic**：文档的原话是 **This function only catches unwinding panics, not those that abort the process.** 把 `panic = "abort"` 打开，整个进程就没了，池再强也救不了。

**5. 队列要不要有界？** 两种都给，内部用一个枚举抹平差别：

无界队列是 `channel()` —— 文档说它 **conceptually has an infinite buffer**，发送永不阻塞：提交永远成功，代价是任务堆在内存里。有界队列是 `sync_channel(n)` —— **`send` will block if there is no space in the internal buffer**：满了就阻塞提交方，这就是**背压（backpressure）**。`n = 0` 是个特殊形态：**a “rendezvous” channel where each sender atomically hands off a message to a receiver** —— 交会，每次提交都要等到真有空闲 worker 来接。

**6. 关闭怎么表达？** 不发明信号，直接用「挂断」：

池里只有一份发送端。丢掉它，所有 worker 的 `recv` 就会拿到 `Err`，循环结束 —— **关闭信号就是队列断了**。不需要往队列里塞 N 个哨兵任务（worker 数变了还得改），也不需要一把共享的 `AtomicBool`（第 20 章的东西，这里用不上）。

策略上选「把已经提交的活干完」，而不是「立刻丢下走人」：提交者手上已经有句柄了，结果不该被静默丢掉。通道还兜了个底 —— 发送端消失之后**缓冲里的消息照样能被接收**，所以「丢发送端 → join worker」这个顺序会把队列排空，而不是把存货丢掉。

## 逐块实现

下面每块都是从 `examples/task-queue/` 里切出来的原文（多数来自 `src/`，另有一块测试），块末注明用到哪几章的概念。顺序按依赖来。

模块骨架只有两个文件 —— `lib.rs` 把内部模块收好，对外重导出 `ThreadPool`、`TaskHandle` 与 `Timeout`：

```rust,ignore
{{#include ../examples/task-queue/src/lib.rs:24:28}}
```

> [!NOTE]
> 这些块大多标了 `ignore`：它们是跨模块互相引用的片段，`mdbook test` 会把每个块当独立程序编译，单独自然编不过（例如 `TaskHandle` 用到了同文件的 `Timeout`）。片段与文件逐字一致；整份代码由下面 `cargo test` 真跑验证。

### 1. 句柄：`src/handle.rs`

先是等待失败的类型。超时不是取消，所以这个类型只需要说清「请求等了多久」（真正返回时可能略晚一点点）：

```rust
# use std::error::Error;
# use std::fmt;
# use std::time::Duration;
{{#include ../examples/task-queue/src/handle.rs:9:24}}
```

句柄本身是一个 `Receiver`，`join` 与 `join_timeout` 就是两种等法：

```rust,ignore
{{#include ../examples/task-queue/src/handle.rs:26:63}}
```

三处值得停一下：

- `thread::Result<T>` 就是 `std::thread::Result`（`Result<T, Box<dyn Any + Send + 'static>>`）。跟着标准库走，读者不用重学「任务结果」的形状，`downcast_ref` 这类处理 panic 负载的写法也直接可用。
- `recv_timeout` 的三种结局正好对应三种语义：拿到了、等满了、对面没了。最后一种**到不了** —— 结果端只存在于任务里，任务在跑之前不会被丢弃（池会把排队的活跑完），跑完又一定恰好发过一次结果。类型系统要求处理这个分支，我们就用 `lost_result()` 把它标成「不变量被破坏」，不编一个假默认值糊过去。
- 这个文件里没有 `unsafe`，也没有锁：句柄的全部状态就是一条通道的接收端。

**这块用到第 7 章的 `Result` 与自定义错误、第 9 章的 trait 实现（`Display`/`Error`）、第 19 章的 `thread::Result` 与通道。**

### 2. 任务队列：`src/pool.rs`

先是任务和提交口这两个内部类型：

```rust
# use std::sync::mpsc::{SendError, Sender, SyncSender};
{{#include ../examples/task-queue/src/pool.rs:10:32}}
```

`Queue` 这个枚举是「两种队列只差在阻塞行为」这个事实的落点：`Sender::send` 与 `SyncSender::send` 返回同一个 `Result<(), SendError<Job>>`，所以池的其余部分只认 `Queue::send` 一个方法，不必到处 `match`。

池本体就是「一份发送端 + 一串 worker 句柄」：

```rust,ignore
{{#include ../examples/task-queue/src/pool.rs:44:49}}
```

`queue` 声明成 `Option`，是因为关闭时要**先把发送端丢掉**（丢不掉就没法只关一半）。构造与提交长这样：

```rust,ignore
{{#include ../examples/task-queue/src/pool.rs:52:90}}
```

`size` 为 0 直接 panic，而不是「建一个空池凑合用」：没有 worker 的池会心安理得地收下任务，然后让每个 `join` 永远等下去 —— 一个静默的死锁，比立刻崩掉难查得多。

`spawn` 只做一件事：把任务包一层 panic 守卫再交给 `enqueue`。`submit` 多一步 —— 先把结果通道造好，再把发送端塞进闭包：

```rust,ignore
{{#include ../examples/task-queue/src/pool.rs:92:112}}
```

`T: Send` 这条约束来自结果通道：`T` 会被 `tx.send` 送走，而句柄可能在任何线程上 `recv` —— 值要跨线程，就得是 `Send`。

关掉池分两步，顺序不能反：

```rust,ignore
{{#include ../examples/task-queue/src/pool.rs:114:120}}
```

真正的活在建池、入队与关闭这三个私有方法里：

```rust,ignore
{{#include ../examples/task-queue/src/pool.rs:122:146}}
```

三件设计在这里落地：无界/有界的选择只出现在这个 `match` 里；接收端被 `Arc<Mutex<…>>` 包好，每个 worker 拿一份 `Arc::clone`；`thread::spawn` 的 `move` 闭包只捕获 `rx` 这一份克隆，其余状态全留在池里。

```rust,ignore
{{#include ../examples/task-queue/src/pool.rs:148:172}}
```

`enqueue` 里两个 `expect` 说的是**不变量**，不是错误处理：第一条的 `Option` 只有 `close` 能清空，而 `close` 要 `&mut self`，所以手里有 `&self` 时队列必然还活着；第二条是「至少有一个 worker 还在 `recv`」，这次 `send` 失败不了。与其为一条到不了的路径编一个 `PoolClosed` 错误类型、让每个调用方都 `?` 一下，不如把事实写进 `expect` 的字符串 —— 真到了那一步，消息也够定位。`close` 则是第 6 条取舍的代码形态：先丢发送端（断掉队列 = 通知所有 worker），再逐个 `join`（等它们把存货跑完）。`join` 的返回值忽略掉：worker 不跑可能 panic 的代码，真 `Err` 了也只是少一个工人；而且在 `Drop` 里抛 panic 会直接 abort，忽略才是对的做法。

最后是 worker 的主循环与 `Drop`：

```rust,ignore
{{#include ../examples/task-queue/src/pool.rs:175:197}}
```

`work` 是本项目唯一用到锁的地方：`rx.lock()` 拿到的守卫只活到 `recv()` 返回（这一整句 `let next = …;` 结束就释放），随后的 `job()` 在锁外执行 —— 这就是「N 个 worker 并行」的全部秘密。取到任务就放手跑，取到 `Err`（发送端全没了）就退出。锁中毒那条分支在类型上必须处理：真中毒了用 `into_inner` 把守卫拿回来接着干，不让一处意外带走整个池 —— 跟任务 panic 的隔离是同一种态度。

`Drop` 与 `shutdown` 共用 `close`，所以两种写法完全等价：显式调用是给读代码的人看的，`drop(pool)` 也不会漏掉任何一步。

**这块用到第 9 章的泛型与 trait 约束、第 10 章的闭包与迭代器（`map`/`drain`）、第 13 章的 `dyn` 与 trait 对象、第 17 章的 `Box`/`Arc`/`Drop`、第 19 章的 `Send`/`'static`/`Mutex`/`mpsc`/`JoinHandle`/锁中毒。**

### 3. 概念落点对照

| 章 | 概念 | 在本项目里的落点 |
|---|---|---|
| 4 | 所有权与借用 | 闭包捕获到的值怎么进任务（`move`）、为什么不能借出来 |
| 5 | 结构体、枚举、模式匹配 | `Queue` 的两个变体、`ThreadPool` 的私有字段、`match` 分支 |
| 9 | 泛型与 trait 约束 | `submit<T, F>` 的 `where`、`Timeout` 的 `Display`/`Error` |
| 10 | 迭代器与闭包 | 建池的 `map` + `collect`、`close` 的 `drain` |
| 11 | 测试与调试 | `src` 里的单元测试、`tests/pool.rs` 的集成测试 |
| 13 | `dyn` 与 trait 对象 | `Box<dyn FnOnce() + Send + 'static>` 把闭包的匿名类型擦成同一个类型 |
| 16 | 生命周期 | `'static` 边界为什么逼着任务拥有自己的数据 |
| 17 | 智能指针与 `Drop` | `Arc`、`Drop::drop` 里跑关闭流程 |
| 19 | 并发基础 | `thread::spawn`、`Send`、`Mutex`、`mpsc`、`JoinHandle`、`thread::Result` |
| 20–22 | 原子 / `async` / `Pin` | 都不用：没有无锁结构，一把 `Mutex` 就够；任务跑在固定的 N 个 worker 上，`join` 就是阻塞当前线程等通道 |

## 报错现场

### 报错一：任务借了局部变量（E0373）

把 `submit` 的约束单独拿出来，就是这段最小复现：

```rust,compile_fail
fn submit<T, F>(job: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    job()
}

fn main() {
    let text = String::from("借来的");
    let len = submit(|| text.len());
    println!("{len}");
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0373]: closure may outlive the current function, but it borrows `text`, which is owned by the current function
  --> main.rs:11:22
   |
11 |     let len = submit(|| text.len());
   |                      ^^ ---- `text` is borrowed here
   |                      |
   |                      may outlive borrowed value `text`
   |
note: function requires argument type to outlive `'static`
  --> main.rs:11:15
   |
11 |     let len = submit(|| text.len());
   |               ^^^^^^^^^^^^^^^^^^^^^
help: to force the closure to take ownership of `text` (and any other referenced variables), use the `move` keyword
   |
11 |     let len = submit(move || text.len());
   |                      ++++
```

定位路径：`submit` 要求 `F: 'static`，而闭包捕获的是 `&text` —— 活不到 `'static` 的借用。`note` 那行点破了约束来源，`help` 给的是同一个方向：让闭包**拥有**数据。真项目里就是 `pool.submit(move || text.len())`（`move` 之后 `text` 进了闭包，外面就再用不了了），或者干脆别借 —— 用 `Arc` 共享只读数据。想「借完还回来」，那是 `thread::scope` 的活。

### 报错二：捕获了 `Rc`（E0277）

`Send` 这一半同样会挡人：

```rust,compile_fail
use std::rc::Rc;

fn submit<T, F>(job: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    job()
}

fn main() {
    let shared = Rc::new(String::from("只属于当前线程"));
    let len = submit(move || shared.len());
    println!("{len}");
}
```

以 Rust 1.98 为例：

```text
error[E0277]: `Rc<String>` cannot be sent between threads safely
  --> main.rs:13:22
   |
13 |     let len = submit(move || shared.len());
   |               ------ -------^^^^^^^^^^^^^
   |               |      |
   |               |      `Rc<String>` cannot be sent between threads safely
   |               |      within this `{closure@main.rs:13:22: 13:29}`
   |               required by a bound introduced by this call
   |
   = help: within `{closure@main.rs:13:22: 13:29}`, the trait `Send` is not implemented for `Rc<String>`
…
note: required by a bound in `submit`
  --> main.rs:6:24
   |
 3 | fn submit<T, F>(job: F) -> T
   |    ------ required by a bound in this function
 6 |     F: FnOnce() -> T + Send + 'static,
   |                        ^^^^ required by this bound in `submit`
```

定位路径：`move` 解决了生命周期，却暴露出「捕获物不 `Send`」—— `Rc` 的计数是普通字段，两个线程同时改就完蛋（第 19 章讲过它为什么被排除在 `Send` 之外）。修法是把共享方式换成原子的 `Arc<String>`（`T: Send + Sync` 时整个类型 `Send`），或者干脆把 `String` 移进去独占使用。最后那个 `note` 用的是复现里 `submit` 的约束；真项目里它会指向 `ThreadPool::submit` 的 `F: Send` —— 约束放在泛型参数上，报错就会自动把用户指到正确的位置。

### 报错三：`catch_unwind` 要 `UnwindSafe`（E0277）

自己写 panic 守卫时会撞上这个 —— 拿掉 `AssertUnwindSafe`：

```rust,compile_fail
use std::panic::catch_unwind;

fn guarded<F: FnOnce()>(job: F) {
    let _ = catch_unwind(job);
}

fn main() {
    guarded(|| println!("任务跑起来了"));
}
```

以 Rust 1.98 为例：

```text
error[E0277]: the type `F` may not be safely transferred across an unwind boundary
   --> main.rs:4:26
    |
  4 |     let _ = catch_unwind(job);
    |             ------------ ^^^ `F` may not be safely transferred across an unwind boundary
    |             |
    |             required by a bound introduced by this call
    |
note: required by a bound in `std::panic::catch_unwind`
   --> …\library\std\src\panic.rs:358:40
    |
358 | pub fn catch_unwind<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Result<R> {
    |                                        ^^^^^^^^^^ required by this bound in `catch_unwind`
help: use parentheses to call this type parameter
    |
  4 |     let _ = catch_unwind(job());
    |                             ++
help: consider further restricting type parameter `F` with trait `UnwindSafe`
    |
  3 | fn guarded<F: FnOnce() + std::panic::UnwindSafe>(job: F) {
   |                        ++++++++++++++++++++++++
```

定位路径：`catch_unwind` 的签名里写着 `F: UnwindSafe`，而泛型参数 `F` 上什么都没有。`help` 给的是加约束那条（把 `UnwindSafe` 传染给所有调用者；前一条 `use parentheses` 是这一族报错的误判建议，不是这里的问题）；另一条 —— 在调用点包一层 `AssertUnwindSafe` —— 来自 `std` 文档，编译器不知道我们的用法在 panic 之后就不再碰捕获物。池子选后者：语义上说得清，也不会把 `UnwindSafe` 传染给每个提交任务的人。

## 测试

测试分两层：碰内部细节的用例留在 `src/pool.rs` 的 `#[cfg(test)] mod tests`（那里能碰到私有的 `Queue`），其余都在 `examples/task-queue/tests/pool.rs`，只走公开 API。跑一遍：

```console
$ cargo test --manifest-path examples/Cargo.toml -p task-queue
running 2 tests
test pool::tests::both_queue_kinds_report_a_gone_receiver ... ok
test pool::tests::zero_workers_is_rejected - should panic ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 13 tests
test dropping_the_pool_finishes_queued_tasks ... ok
test a_panicking_task_does_not_take_the_pool_with_it ... ok
test every_submitted_task_returns_its_value ... ok
test one_worker_runs_tasks_in_submission_order ... ok
test every_worker_can_run_a_task_at_the_same_time ... ok
test a_handle_can_be_joined_from_another_thread ... ok
test results_arrive_even_after_the_pool_is_dropped ... ok
test a_bounded_queue_finishes_tasks_that_were_already_submitted ... ok
test several_threads_can_submit_to_a_small_bounded_queue ... ok
test tasks_can_be_submitted_from_several_threads ... ok
test join_timeout_gives_up_without_cancelling_the_task ... ok
test a_full_bounded_queue_blocks_the_submitter_until_a_worker_is_free ... ok
test a_rendezvous_queue_blocks_the_submitter_until_a_worker_is_free ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

   Doc-tests task_queue

running 3 tests
test task-queue\src\lib.rs - (line 11) ... ok
test task-queue\src\pool.rs - pool::ThreadPool (line 36) ... ok
test task-queue\src\pool.rs - pool::ThreadPool::submit (line 94) ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

（取自一次真跑；省掉 cargo 的 `Compiling` / `Finished` / `Running` 行，以及末尾的 `all doctests ran …`、`Wall time: …` 两行；libtest 并行跑用例，顺序每次可能不同。合计：单元 2、集成 13、文档 3。）三类路径都有覆盖：

- **正常路径**：50 个任务的值全部对上（`every_submitted_task_returns_its_value`）、四个线程一起提交、单 worker 下按提交顺序执行、句柄搬到别的线程再 `join`；
- **边界**：三个 worker 同时开跑（`every_worker_can_run_a_task_at_the_same_time`）、`join_timeout` 等满就返回且不取消任务、有界队列把提交方挡住（容量 1、容量 0 各一例）、容量 4 的队列里排进去的活在 `drop` 之后照样跑完、池 `drop` 之后结果照样收得到；
- **错误路径**：任务 panic 之后池照旧可用、负载能 `downcast` 回原始类型、`ThreadPool::new(0)` 立刻 panic、两端失配时 `send` 报错。

### 并发用例怎么写得能重复跑

并发测试最容易变成「十次里绿九次」。这份测试靠三条规则站住：

**一、不猜时间，把「先后」写成同步事实。** 想证明「三个 worker 同时在跑」，不用 `sleep` 估摸，而是让每个任务先报到、再等一条只有自己知道的放行通道；主线程收齐三次报到才放行。池要是串行执行，第二个报到就永远等不到 —— 用例会**失败并打印**，而不是挂死（`recv_timeout` 给的是 5 秒上界）。交会队列那条同理：容量 0 让「提交会阻塞」变成协议保证。

```rust,ignore
{{#include ../examples/task-queue/tests/pool.rs:53:78}}
```

**二、肯定断言给宽上界，否定断言靠协议。** 一切「应当发生」的等待都配 `LONG`（5 秒）这样的上界，超了就是真失败；一切「不该发生」的等待只等 `SHORT`（200 毫秒），而它之所以能成立是因为上一步的协议已经保证对方动不了 —— 时间在这里只决定「多快判负」，不决定结论。

**三、要顺序就只有一个执行者。** 「按提交顺序执行」只在单 worker 下断言：一个 worker + 一条 FIFO 通道，顺序是数据结构给的保证，不是运气。多 worker 的用例只断言值与集合。

这三条之外还有一条最朴素的验证：**重复跑**。同一份测试二进制连跑 20 次、外加开线程数为 1 和 `--release` 各跑一遍，全绿；真跑过的命令与结果记在本票的 `## Comments` 里。

调试时两个顺手手段：`cargo test -- --nocapture` 能看到 libtest 收起来的 panic 输出；打出 `pool.worker_count()` 能立刻确认池的大小。

## 收尾：局限与下一步

这个池能用，但它是**教学尺寸**的，先知道它没做什么：

- **没有取消。** 任务一旦提交就会跑完（`join_timeout` 只是不等了）。标准库没有取消机制：要么自己在任务里查一个 `AtomicBool`，要么换成 `async` —— 那里「取消」就是 drop 一个 future（第 21、38 章）。
- **没有优先级、抢占、动态扩缩。** 队列是 FIFO，worker 数建好就固定。
- **返回值必须 `Send + 'static`。** 想返回一个借来的切片做不到 —— 换 `thread::scope`。
- **每个任务一次加锁、一次通道往返。** 任务极短、提交量极大时队列本身会成为瓶颈；那种场景要么批量提交，要么上无锁结构（第 20 章的 CAS 循环是入门）。

想接着练的话，几个由易到难的方向：

1. **给 worker 起名**：用 `thread::Builder::new().name(...)` 起名，panic 消息里就会写清是哪个 worker。
2. **默认并发度**：加一个 `ThreadPool::with_available_parallelism()`，内部用 `std::thread::available_parallelism()`（返回 `Result<NonZeroUsize>`）决定 worker 数。
3. **结果批量收**：加一个 `join_all`，或者让 `submit` 直接返回 `impl Future`（第 14 章）—— 那就要有执行器了。
4. **背压策略**：给有界队列配一个 `try_submit`，满了返回 `Err(Full)` 而不是阻塞；`SyncSender::try_send` 已经准备好了。
5. **换成 `async`**：把 worker 换成执行器、把 `join` 换成 `.await`，就是第 38 章要展开的那条路 —— 好消息是这一章的接口形状（提交 → 句柄 → 等结果）几乎不用改。

完整代码在 `examples/task-queue/`，跑法只有一句：`cargo test --manifest-path examples/Cargo.toml -p task-queue`。

## 小结

- 线程池的全部结构就是三样东西：一条队列、N 个从队列取任务的 worker、每任务一条私有的结果通道。
- `Send + 'static` 是入场券：任务要在别的线程跑（`Send`），还要比提交它的那次调用活得久（`'static`）。想借栈上的数据，答案不是放宽约束，而是换 `thread::scope`。
- `mpsc` 的接收端只有一个持有者，所以多 worker 必须靠 `Arc<Mutex<Receiver<_>>>` 共享；锁只护着「取任务」，任务体跑在锁外，并行才成立。
- `catch_unwind` 把任务 panic 挡在 worker 之外，`AssertUnwindSafe` 声明「panic 之后不再看捕获物」；默认 panic hook 会先打消息，abort 型 panic 抓不到。
- 关闭不需要额外信号：丢掉发送端，worker 的 `recv` 就结束 —— 通道的「挂断」本身就是通知，存货照样能被取走，所以「排队先跑完」是默认行为。
- 并发测试不猜时间：同步事实定先后、上界只负责判负、顺序断言只在单执行者下做，再重复跑若干次。

## 练习

1. `Job` 为什么非得是 `Box<dyn FnOnce() + Send + 'static>`？把 `Box` 去掉（变成泛型参数）会怎样？把 `FnOnce` 换成 `Fn` 又会怎样？
2. `ThreadPool::new(0)` 我们让它立刻 panic，而不是「建一个没有 worker 的池照常用」。为什么这样更好？如果真让它建出来，第一个会出问题的地方在哪里？
3. 如果改成「每个 worker 一条通道、提交时轮着发」，`ThreadPool` 的字段与方法会变成什么样？换来了什么、又丢掉了什么？

<details><summary>参考答案</summary>

**1.** `Box` 是为了**把许多种不同的闭包擦成同一个类型**：每个闭包类型的名字都是编译器生成的匿名类型，而 `channel` 只能收发一种类型。不装箱就得让 `ThreadPool` 带上一个写不出来的泛型参数。`FnOnce` 是因为任务只跑一次、且允许消费捕获到的值 —— `submit(move || drop(text))` 这种写法在 `Fn` 下直接编不过。这四处各自对应一个真实要求：`Box` 擦类型、`Send` 搬到别的线程、`'static` 活得够久、`FnOnce` 只跑一次且可以消费捕获物。

**2.** 没有 worker 的池会收下任务、把结果通道留在内存里，然后**永远不跑**：提交方拿到句柄，`join()` 一直阻塞 —— 没有报错、没有日志、只有卡住。这种静默死锁比 panic 难查得多，而且从类型上完全看不出来。`size = 0` 是调用方的编程错误（不是运行时数据问题），用 `assert!` 在第一现场崩掉、把原因写进消息，比事后追一个卡死的 `join` 便宜得多。真想支持「暂时没有任务」，那也该是「别建池」。

**3.** 字段从「一份发送端 + 一把锁」变成「`Vec<Sender<Job>>` + 一个轮询下标」，`Arc<Mutex<Receiver<Job>>>` 整个消失，worker 循环里也不再需要 `lock` —— 每个 worker 直接 `recv` 自己那条通道。丢掉的是负载均衡：任务被预先派到某个 worker 名下，那个 worker 忙的时候，派给它的任务只能等（队头阻塞），别的 worker 闲着也不会来帮忙。换来的是零锁、零共享可变状态，提交路径也更短。所以它适合「任务粒度均匀、数量多」的场景，不适合长任务与短任务混在一起的场景 —— 我们的池要兼顾后者，所以选了共享接收端。

</details>

## 延伸阅读

- `std::thread` 模块文档（线程模型、`spawn` 的约束、`scope`、`Builder`、`available_parallelism`）：<https://doc.rust-lang.org/std/thread/index.html>
- `std::thread::JoinHandle::join` 与 `std::thread::Result`（本项目的句柄沿用了这个形状）：<https://doc.rust-lang.org/std/thread/struct.JoinHandle.html>
- `std::sync::mpsc`（`channel`/`sync_channel`/`recv_timeout`/`Sender`/`Receiver` 的完整语义）：<https://doc.rust-lang.org/std/sync/mpsc/index.html>
- `std::sync::Mutex` 与 `PoisonError`（`lock`/`try_lock`/中毒与 `into_inner`）：<https://doc.rust-lang.org/std/sync/struct.Mutex.html>、<https://doc.rust-lang.org/std/sync/struct.PoisonError.html>
- `std::sync::Arc`（跨线程共享所有权）：<https://doc.rust-lang.org/std/sync/struct.Arc.html>
- `std::panic::catch_unwind` 与 `std::panic::AssertUnwindSafe`（能抓什么、抓不到什么）：<https://doc.rust-lang.org/std/panic/fn.catch_unwind.html>、<https://doc.rust-lang.org/std/panic/struct.AssertUnwindSafe.html>
- The Book 第 16 章《Fearless Concurrency》：<https://doc.rust-lang.org/book/ch16-00-concurrency.html>
- The Nomicon：`Send` 与 `Sync`：<https://doc.rust-lang.org/nomicon/send-and-sync.html>
- Cargo Book《Workspaces》（`examples/Cargo.toml` 这种多 crate 布局）：<https://doc.rust-lang.org/cargo/reference/workspaces.html>
