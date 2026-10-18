# 第 19 章 并发基础：thread、Send/Sync 与共享状态

## 这一章解决什么问题

第 17 章给了你一套跨线程可用的工具：`Arc` 管共享所有权，`Mutex`/`RwLock` 管共享可变，`OnceLock`/`LazyLock` 管一次性初始化；还给了你一张「哪些类型能跨线程」的分布表。那张表只回答**结论**，不回答结论从哪来。要真写出一个多线程程序，你还需要另外一套东西：怎么起线程、怎么把数据递过去、怎么把结果收回来、怎么在编译期证明「这块数据不该被两个线程同时碰」。

这一章把这条链补全。读完你能：

- 用 `thread::spawn` 起线程，并说清它的 `'static` 要求从哪来；
- 用 `thread::scope`（1.63 起稳定）让线程借用栈上的局部数据，并解释作用域凭什么保证安全；
- 用 `join` 拿回返回值，也处理子线程 panic 的传播；
- 准确说出 `Send`/`Sync` 的语义：它们是 auto trait，由编译器逐字段推导，可以用 `unsafe impl` 覆盖；
- 判断某个类型为什么不是 `Send`/`Sync`，以及什么时候写 `unsafe impl` 是正当的；
- 用 `std::sync::mpsc` 的通道（channel）把数据在线程之间递过去，并处理「发送端全部消失」这类正常结局；
- 用 `thread_local!` 给每个线程留一份自己的状态。

本章的边界先划清：`Ordering`、CAS 与无锁在第 20 章，`async`/`Future` 在第 21 章，`Pin` 在第 22 章；别名模型与数据竞争的形式化定义在第 35 章，`loom` 这类并发测试工具在第 31 章。第 17 章讲过的 `Mutex`/`RwLock` API 面不重讲，这里只写「跨线程组合时新出现的东西」。

## 为什么这样设计

### 并发错误的账先由谁付

一块内存被两个线程同时读写，程序的行为就不再由你的代码决定。C 与 C++ 都把它叫作数据竞争（data race），并明确列为未定义行为（UB）：编译器和 CPU 可以做任何优化而无需保证结果。这类 bug 的恶劣之处在于它**不可复现**——在你机器上跑一万次都对，换一台机器、换一个优化档位就错。

Rust 的答案是把这件事挪到编译期，代价是类型系统上多两条约束。这个交易的划算之处在于：并发错误本来就极难靠测试发现（第 31 章会讲专门为此而生的工具），而类型检查每次编译都跑。

### 两条规则覆盖三条路径

两个线程想踩到同一块内存，只有三条路：把值搬过去、把引用发过去、通过某个同步原语共享。Rust 用两个 trait 把前两条路标出来：

- `Send`：这个类型能**整个搬到**另一个线程。std 的原文是：Types that can be transferred across thread boundaries.
- `Sync`：这个类型的**共享引用**能发到另一个线程。精确定义 std 写得极短：The precise definition is: a type `T` is `Sync` if and only if `&T` is `Send`.

第三条路（锁、原子、消息传递）不用新规则：它们内部都建立在 `Send`/`Sync` 与 `unsafe` 之上，对外只暴露「已经消化掉同步」的安全接口。所以这两条规则不是「并发检查的一部分」，而是**全部**——第 17 章那张分布表里的每一格都是它们的推论。

`Send`/`Sync` 还有一处特殊：**由编译器自动推导**。std 的说明是：This trait is automatically implemented when the compiler determines it’s appropriate.——这类 trait 叫 auto trait。编译器只看你的字段：每个字段都 `Send`，你这个类型就 `Send`；反过来，**一个 `Rc` 字段就足以把整个结构体拉下水**。这就是「传染」的来源，也是你以后看并发报错的第一反应：哪个字段先不合格。

### 主线程退出就是进程退出

还有一条容易吃亏的模型事实。std 把 Rust 的线程模型一句话说清：

> An executing Rust program consists of a collection of native OS threads, each with their own stack and local state.

native OS threads 意味着起线程有真实成本，`join` 是真的等系统线程结束。而更要紧的是退出规则：

> When the main thread of a Rust program terminates, the entire program shuts down, even if other threads are still running, and any destructors on the remaining threads' stacks may not be executed.

主线程不等人。`main` 返回的那一刻，还没跑完的线程连同它们栈上的值一起被掐断，`Drop` 可能一次都不跑。所以「起完线程就不管」几乎总是错的：要么 `join`，要么用带自动 join 的 `thread::scope`，要么把通道发出去再接收（接收端等消息，本身就是一种同步）。

## 机制与规则

### `thread::spawn`：`'static` 从哪来

起线程的函数签名是这样的（下面是签名本身，不是独立程序，所以块里标了 `ignore`）：

```rust,ignore
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
```

这三个 `where` 子句里，std 专门给了理由的是两条（译自原文）：`'static` 是因为「线程可能活得比创建它的作用域更久，而我们无法知道它何时返回，只好要求参数一直有效到程序结束」；`Send` 是因为「闭包要按值从创建它的线程传给新线程，返回值也要从新线程传回 `join` 的那一侧」——这也一并解释了返回值的 `T: Send`。

```rust
use std::thread;

fn main() {
    let handle = thread::spawn(|| {
        println!("子线程里跑起来了");
        6 * 7
    });
    let value = handle.join().unwrap();
    println!("join 拿到返回值：{value}");
}
```

```text
子线程里跑起来了
join 拿到返回值：42
```

`move` 不是装饰：闭包默认按引用捕获，而引用借的是栈上的局部变量，活不过 `'static`。`move` 把捕获变成「拿走所有权」，于是闭包自己能活到程序结束，约束才成立。

### `thread::scope`：把 `join` 交给作用域

`'static` 的要求很硬，可绝大多数并发的用途恰恰是「把手上这批数据分给几个线程算完，再拿回结果」——数据就在栈上。为了不逼你为每次并行都套一层 `Arc`，1.63 加了 `thread::scope`。文档把契约写得直白：

> Unlike non-scoped threads, scoped threads can borrow non-`'static` data, as the scope guarantees all threads will be joined at the end of the scope.

> All threads spawned within the scope that haven't been manually joined will be automatically joined before this function returns.

关键在于「自动 join」：作用域结束前，所有它派出的线程都已经结束，所以它们借的东西在整个借用期内都还活着。实测一下它能借到什么程度：

```rust
use std::thread;

fn main() {
    let words = vec!["alpha", "beta", "gamma", "delta"];
    let mut total_len = 0;

    thread::scope(|s| {
        let first = &words[0];
        s.spawn(move || {
            println!("第一个词是 {first}（借来的，不需要 'static）");
        });
        s.spawn(|| {
            println!("子线程看到 {} 个词", words.len());
        });
        s.spawn(|| {
            total_len += words.iter().map(|w| w.len()).sum::<usize>();
        });
        println!("scope 内部：主线程也在跑");
    });

    println!("scope 结束后 total_len = {total_len}");
}
```

```text
scope 内部：主线程也在跑
第一个词是 alpha（借来的，不需要 'static）
子线程看到 4 个词
scope 结束后 total_len = 19
```

三个线程的打印彼此没有顺序保证，你跑出来的行序可能不同；`total_len` 的汇总值一定是 19。

三点值得记住：

- 后两个闭包都借了 `words`，其中还不止一个——**共享借用可以并存**，所以 `&words` 能同时进多个线程；`total_len` 被 `&mut` 借走，只有一个线程拿它，别名规则照样成立。`scope` 没有放宽借用检查，它只是把「借用」和「线程存活」这两件事绑定在了一起。
- `Scope::spawn` 的约束是 `F: Send + 'scope`，不是 `'static`（`'scope` 是作用域自身的生命周期，由 `thread::scope` 内部提供）。
- 自动 join 也意味着**panic 会传染**：某个分片线程 panic，`thread::scope` 本身会 panic，信息是 `a scoped thread panicked`。想自己处理就显式 `join` 那个句柄。

`scope` 不豁免 `Send`。下面这段编译不过，而且原因与生命周期无关——作用域解决了「借来的数据活多久」，`Send` 这一关照样要过：

```rust,compile_fail
use std::cell::Cell;
use std::thread;

fn main() {
    let shared = Cell::new(0);
    thread::scope(|s| {
        s.spawn(|| shared.set(shared.get() + 1));
    });
    println!("{}", shared.get());
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0277]: `Cell<i32>` cannot be shared between threads safely
   --> src/main.rs:7:17
    |
  7 |         s.spawn(|| shared.set(shared.get() + 1));
    |           ----- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `Cell<i32>` cannot be shared between threads safely
    |           |
    |           required by a bound introduced by this call
    |
    = help: the trait `Sync` is not implemented for `Cell<i32>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicI32` instead
    = note: required for `&Cell<i32>` to implement `Send`
…
```

闭包借的是 `&shared`，而 `&Cell<i32>` 要跨线程就得 `Cell<i32>: Sync`——它不是。（换件清单见第 17 章那张分布表。）

### `join`：拿返回值，也拿 panic

`JoinHandle<T>::join` 返回 `std::thread::Result<T>`，展开就是 `Result<T, Box<dyn Any + Send + 'static>>`：成功是线程的返回值，失败是子线程 panic 时抛出的负载。负载可以 `downcast` 回原来的类型，所以「子线程为什么炸的」这件事并没有丢：

```rust
use std::thread;

fn main() {
    let h = thread::spawn(|| panic!("子线程炸了"));
    match h.join() {
        Ok(()) => println!("正常结束"),
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&'static str>()
                .copied()
                .unwrap_or("<非字符串负载>");
            println!("join -> Err，负载是：{msg}");
        }
    }
}
```

```text
thread '<unnamed>' (26560) panicked at src/main.rs:4:30:
子线程炸了
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
join -> Err，负载是：子线程炸了
```

前面那两行是子线程在 panic 现场自己打给 stderr 的（线程号每次不同），最后一行才是 `join` 返回给主线程的事实。**`join` 不会替你打印、也不会替你中止进程**——默认的 panic hook 打印一次，`join` 把控制权交回你手上，接下来是 `.unwrap()` 继续传播，还是记日志后继续跑，由你决定。

`JoinHandle<T>` 本身是 `Send + Sync` 的（与 `T` 无关，1.98 实测），所以「起一批线程、把句柄收进 `Vec`、在另一个线程里统一 join」这种写法是合法的。

### `Send` 与 `Sync`：判据、推导与断言

把判据和推论并排放：

| 说法 | 含义 | 等价条件 |
|---|---|---|
| `T: Send` | 值可以整个搬到另一个线程 | — |
| `T: Sync` | `&T` 可以发到另一个线程 | `&T: Send` |
| `&T: Send` | 把共享引用发过去 | `T: Sync` |
| `&mut T: Send` | 把可变引用发过去（移过去之后没人再持有它） | `T: Send` |

auto trait 的推导是逐字段的，所以「一个类型能不能跨线程」在你定义类型的那一刻就定了：

```rust
use std::cell::Cell;

struct NotThreadSafe {
    hits: Cell<u32>,
}

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

fn main() {
    // 结构体本身没问题：字段都是 Send，它自动就是 Send……
    assert_send::<NotThreadSafe>();
    // ……但不是 Sync。把下面这行放开，编译立刻失败
    // assert_sync::<NotThreadSafe>();
    println!("NotThreadSafe: Send 有了，Sync 没有");
}
```

```text
NotThreadSafe: Send 有了，Sync 没有
```

`NotThreadSafe` 的名字只是一种提醒，编译器不会因为名字而区别对待：它自动成了 `Send`（字段 `Cell<u32>` 是 `Send`）而非 `Sync`（`Cell<u32>` 不是）。**你一个标注都不写，编译器也算得出来**。

这套 `assert_send`/`assert_sync` 值得放进工具箱：想确认任何类型（或泛型参数）的分布，写一行断言让编译器回答就行。下面一次把本章要用的都断言掉：

```rust
use std::sync::mpsc;
use std::sync::MutexGuard;
use std::thread::JoinHandle;

fn assert_send<T: Send>() {}
fn assert_sync<T: Sync>() {}

fn main() {
    assert_send::<mpsc::Sender<i32>>();
    assert_sync::<mpsc::Sender<i32>>();
    assert_send::<mpsc::Receiver<i32>>();
    assert_send::<JoinHandle<i32>>();
    assert_sync::<JoinHandle<i32>>();
    assert_sync::<MutexGuard<'static, i32>>();
    println!("这些断言全部通过");
}
```

```text
这些断言全部通过
```

把 `assert_sync` 这一行单独拿出来放进 `src/main.rs`，就会得到（1.98）：

```rust,compile_fail
use std::sync::mpsc;

fn assert_sync<T: Sync>() {}

fn main() {
    assert_sync::<mpsc::Receiver<i32>>();
}
```

```text
error[E0277]: `std::sync::mpsc::Receiver<i32>` cannot be shared between threads safely
 --> src/main.rs:6:19
  |
6 |     assert_sync::<mpsc::Receiver<i32>>();
  |                   ^^^^^^^^^^^^^^^^^^^ `std::sync::mpsc::Receiver<i32>` cannot be shared between threads safely
  |
  = help: the trait `Sync` is not implemented for `std::sync::mpsc::Receiver<i32>`
note: required by a bound in `assert_sync`
 --> src/main.rs:3:19
  |
3 | fn assert_sync<T: Sync>() {}
  |                   ^^^^ required by a bound in `assert_sync`
```

这张「断言表」也是本章几条结论的证据来源：`mpsc::Sender<i32>` 既是 `Send` 也是 `Sync`（`send` 的接收者是 `&self`，多个线程共享一个发送端是安全的）；`Receiver` 只 `Send` 不 `Sync`（接收端天然是单消费者）；`JoinHandle` 两个都满足。

> [!TIP]
> `MutexGuard` 的分布是最反直觉的一格：它**不是** `Send`，却在 `T: Sync` 时**是** `Sync`。理由见下面「`Arc<Mutex<T>>`」一节。

### `unsafe impl`：把不变量写进代码

自动推导覆盖不了的场合只有一类：类型**内部有编译器看不见的东西**。典型是裸指针——`*const T` 与 `*mut T` 在 core 里被显式实现成 `!Send` 兼 `!Sync`，因为编译器无从判断它指向的内存在别的线程面前是否安全。这时候你只能自己承担：

```rust
use std::thread;

static VALUE: i32 = 1234;

/// 内部是裸指针，所以编译器默认认为它既不能发送、也不能共享。
struct RawHandle(*const i32);

impl RawHandle {
    fn get(&self) -> i32 {
        // 安全前提由 RawHandle 的约定保证：指针在程序整个运行期都有效
        unsafe { *self.0 }
    }
}

// 这两行把「不变量」的责任从编译器转到我们身上
unsafe impl Send for RawHandle {}
unsafe impl Sync for RawHandle {}

fn main() {
    let handle = RawHandle(&VALUE as *const i32);

    let mut threads = Vec::new();
    for i in 0..3 {
        let h = RawHandle(handle.0);
        threads.push(thread::spawn(move || (i, h.get())));
    }
    for t in threads {
        let (i, v) = t.join().unwrap();
        println!("线程 {i} 读到 {v}");
    }
}
```

```text
线程 0 读到 1234
线程 1 读到 1234
线程 2 读到 1234
```

这里的不变量是「指针指向的 `static` 在程序整个运行期都有效，且所有线程只读它」。写 `unsafe impl` 等于对着编译器签一份声明：我的类型确实满足 `Send`/`Sync` 的语义要求。Nomicon 的说法是 `Send` 与 `Sync` 是 unsafe trait，所以「实现它们」这件事本身就是 unsafe 操作。

> [!WARNING]
> `unsafe impl` 不是「消掉报错」的开关。给 `Rc<T>` 包一层再 `unsafe impl Send`，编译会过，但另一个线程 clone 同一份时那个非原子计数就会被两个线程同时改——那是真正的未定义行为。**只有当你的不变量能让「跨线程」成立时才写它**（第 23 章展开 unsafe 的判定方式）。

> [!TIP]
> 有个坑会让 `unsafe impl` 看起来「没生效」：edition 2021 起，闭包**按实际用到的字段**捕获（捕获精度）。同一段代码在 edition 2024 下报 `*const i32 cannot be sent between threads safely`（闭包直接把裸指针字段捕获走了），在 edition 2018 下却能编译（闭包捕获整个 `RawHandle`，于是走 `unsafe impl`）。修法不是再加一个 `unsafe impl`，而是让闭包用到整个值——比如像上面那样调一个方法。报错里 `required because it's used within this closure` 说的就是这件事。

### `Arc<Mutex<T>>`：跨线程组合

`Arc` + `Mutex` 的 API 在第 17 章讲过，这里只补跨线程才出现的四件事。

**一、`try_lock` 是非阻塞的。** 拿不到锁它立刻返回 `Err`，不会排队等着。`TryLockError` 分两种：`WouldBlock`（别人正持锁）与 `Poisoned`（锁中毒了）。

**二、锁中毒（poisoning）只是建议性的。** 持锁线程 panic 之后，锁被标记为「中毒」，之后 `lock()` 返回 `Err(PoisonError)`；`PoisonError::into_inner()` 能把 guard 取回来。文档原话：

> Poisoning is only advisory: the `PoisonError` type has an `into_inner` method which will return the guard that would have otherwise been returned on a successful lock.

实测一遍 `try_lock` 与中毒：

```rust
use std::sync::{Arc, Mutex, TryLockError};
use std::thread;
use std::time::Duration;

fn main() {
    let shared = Arc::new(Mutex::new(10));
    println!("第一次加锁读到 {}", *shared.lock().unwrap());

    let held = Arc::clone(&shared);
    let t = thread::spawn(move || {
        let mut guard = held.lock().unwrap();
        *guard += 1;
        thread::sleep(Duration::from_millis(300));
    });
    thread::sleep(Duration::from_millis(50));
    match shared.try_lock() {
        Ok(_) => println!("奇怪：居然拿到了"),
        Err(TryLockError::WouldBlock) => println!("try_lock -> WouldBlock（有人在锁里）"),
        Err(TryLockError::Poisoned(_)) => println!("try_lock -> Poisoned"),
    }
    t.join().unwrap();

    let poisoned = Arc::new(Mutex::new(String::from("初始")));
    let p = Arc::clone(&poisoned);
    let _ = thread::spawn(move || {
        let mut guard = p.lock().unwrap();
        guard.push_str("被改了一半");
        panic!("在临界区里崩了");
    })
    .join();

    match poisoned.lock() {
        Ok(_) => println!("没有中毒"),
        Err(e) => {
            println!("lock -> Err，中毒了：{e}");
            println!("内部值仍在：{}", e.into_inner());
        }
    }
}
```

```text
第一次加锁读到 10
try_lock -> WouldBlock（有人在锁里）

thread '<unnamed>' (28972) panicked at src/main.rs:28:9:
在临界区里崩了
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
lock -> Err，中毒了：poisoned lock: another task failed inside
内部值仍在：初始被改了一半
```

最后两行是重点：中毒**不阻止**你读到那半截被改坏的字符串，它只是把「这里可能不符合不变量」的消息挂在 `Err` 上，让 `.unwrap()` 的默认写法把 panic 传播出去。所以「取回值继续用」是明确的选择，不是绕过检查。

**三、同一线程重复 `lock()` 是未指定行为。** 文档原话：

> The exact behavior on locking a mutex in the thread which already holds the lock is left unspecified. However, this function will not return on the second call (it might panic or deadlock, for example).

也就是说，第二次调用**不会正常返回**——可能 panic，可能永远卡住。跨线程的死锁（A 等 B 的锁、B 等 A 的锁）同理：`std` 不做死锁检测，也不会超时救你。锁的获取顺序也不在文档承诺范围内：不要依赖「谁先等谁先拿」，需要顺序就自己用队列表达。

**四、`MutexGuard` 不能离开它的线程。** std 给的理由很实际：

> A `MutexGuard` is not `Send` to maximize platform portability.

> On platforms that use POSIX threads (commonly referred to as pthreads) there is a requirement to release mutex locks on the same thread they were acquired.

而它偏偏是 `Sync` 的（当 `T: Sync`），因为拿 `&guard` 只能读出 `&T`，多个线程同时读没问题：

> `T` must be `Sync` for a `MutexGuard<T>` to be `Sync` because it is possible to get a `&T` from `&MutexGuard` (via `Deref`).

这条规则在 `thread::scope` 里最容易撞上：把 guard `move` 进另一个线程，编译器立刻拦下（现场四）。

### 通道：把数据递过去，而不是共享

共享内存要用锁，前提是「两个线程都要看到同一块数据」。很多时候你并不需要这个前提——算完把结果**递过去**就行。`std::sync::mpsc` 的自我定位是一句话：Multi-producer, single-consumer FIFO queue communication primitives.（多生产者、单消费者）。两类发送端对应两种缓冲策略，文档的分法很清楚：

- `channel()` 给出 `(Sender, Receiver)`，发送是**异步**的、永不阻塞，概念上是无限缓冲；
- `sync_channel(n)` 给出 `(SyncSender, Receiver)`，缓冲固定为 `n` 条，缓冲满时 `send` 阻塞等待；`n = 0` 时退化成「交会（rendezvous）」通道——每次发送都要等到接收端来取。

```rust
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<&str>(0);
    let t = thread::spawn(move || {
        println!("生产者：准备发送");
        tx.send("ping").unwrap();
        println!("生产者：交接完成");
    });
    thread::sleep(Duration::from_millis(200));
    println!("消费者：收到 {}", rx.recv().unwrap());
    t.join().unwrap();

    let (tx, rx) = mpsc::sync_channel::<i32>(2);
    tx.send(10).unwrap();
    tx.send(20).unwrap();
    println!("容量 2：两次 send 都没阻塞");
    println!("依次取出 {} 和 {}", rx.recv().unwrap(), rx.recv().unwrap());
}
```

```text
生产者：准备发送
消费者：收到 ping
生产者：交接完成
容量 2：两次 send 都没阻塞
依次取出 10 和 20
```

前两行与第三行的顺序不是巧合：容量 0 的通道里，`send` 必须等接收端接住才返回，所以「交接完成」只能出现在「收到 ping」之后。容量 2 的通道里两次 `send` 都直接进了缓冲。

**发送端的移动语义。** `Sender` 与 `SyncSender` 都不是 `Copy`：`send(&self, ...)` 只要共享引用，但要把发送能力交给一个线程，你必须先把 `Sender` 弄到那边去——要么 `clone()` 一份，要么把原件 `move` 进去。`clone` 出来的每一份都是「同一个通道的另一个生产者」，正是 mpsc 里那个 m（多生产者）的来源：

```rust
use std::sync::mpsc::{self, RecvError};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<String>();

    let mut handles = Vec::new();
    for i in 0..2 {
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            for k in 0..2 {
                tx.send(format!("来自线程{i}的第{k}条")).unwrap();
            }
        }));
    }
    drop(tx); // 关键：原件必须显式丢掉

    for handle in handles {
        handle.join().unwrap();
    }

    let mut received: Vec<String> = rx.iter().collect();
    received.sort();
    println!("收到 {} 条：{received:?}", received.len());

    // `_` 模式不绑定：发送端在这一行结束时就没了
    let (_, rx2) = mpsc::channel::<i32>();
    match rx2.recv() {
        Ok(v) => println!("收到 {v}"),
        Err(RecvError) => println!("recv -> Err(RecvError)：所有发送端都消失了"),
    }
}
```

```text
收到 4 条：["来自线程0的第0条", "来自线程0的第1条", "来自线程1的第0条", "来自线程1的第1条"]
recv -> Err(RecvError)：所有发送端都消失了
```

**「发送端消失」是正常结局，不是错误。** 文档原话：

> An unsuccessful operation is normally indicative of the other half of a channel having “hung up” by being dropped in its corresponding thread.

这正是 `rx.iter()` 能当作迭代器用的原因：所有发送端 drop 之后，迭代自然结束。反过来说，**只要还有一个发送端活着，接收端就永远等不到「结束」**——忘了 `drop(tx)` 就会卡死（练习 2）。要临时看一眼而不阻塞，用 `try_recv()`；要带超时用 `recv_timeout()`。

### `thread_local!`：每个线程一份

有些状态天然是「每线程一份」：随机数种子、日志缓冲区、复用的临时数组、递归或调用深度计数。这些值既不该跨线程共享（共享就得加锁），也不该在每次调用时重新分配。`thread_local!` 提供的正是这个：一个全局名字，每个线程看到自己的那份。

```rust
use std::cell::Cell;
use std::thread;

thread_local! {
    static HITS: Cell<u32> = const { Cell::new(0) };
}

fn hit() {
    HITS.with(|c| c.set(c.get() + 1));
}

fn main() {
    hit();
    hit();
    HITS.with(|c| println!("主线程 hits = {}", c.get()));

    let t = thread::spawn(|| {
        println!("新线程一开始 hits = {}", HITS.with(|c| c.get()));
        hit();
        println!("新线程自己加了 1 次：{}", HITS.with(|c| c.get()));
    });
    t.join().unwrap();

    HITS.with(|c| println!("子线程结束后，主线程仍是 {}", c.get()));
}
```

```text
主线程 hits = 2
新线程一开始 hits = 0
新线程自己加了 1 次：1
子线程结束后，主线程仍是 2
```

规则与坑：

- 宏产生的是一把 `LocalKey<T>`，主入口是 `with`。文档对它的限制是：The `with` method yields a reference to the contained value which cannot outlive the current thread or escape the given closure.——引用出不去，因为别的线程随时可能结束它。真要把值拿出来，得复制或 `clone`、或者用 `Cell::take` 这种搬值方法（第 17 章）。
- 初始化是**首次访问时按线程执行**的：Initialization is dynamically performed on the first call to a setter (e.g. `with`) within a thread, and values that implement `Drop` get destructed when a thread exits.——即「惰性初始化 + 线程退出时析构」。
- `LocalKey<Cell<T>>` 有一组捷径方法（`get`/`set`/`take`/`replace`，1.73 起）省掉闭包；注意 `set` 有个副作用——它**不会**跑惰性初始化函数，而是直接用给定值初始化（这在需要「先设再读」的场合可以省一次分配）。

> [!NOTE]
> `thread_local!` 解决的是「每线程一份」，不是「全局一份」。要全局共享一个只初始化一次的值，用第 17 章的 `OnceLock`/`LazyLock`。

### 本章这些类型的分布

把本章出现的类型并进第 17 章那张表（1.98 实测，`T` 指类型参数）：

| 类型 | `Send` | `Sync` | 说明 |
|---|---|---|---|
| `JoinHandle<T>` | 是 | 是 | 与 `T` 无关；句柄本身能进 `Vec`、能跨线程 join |
| `mpsc::Sender<T>` | `T: Send` | `T: Send` | `send` 只要 `&self`，多线程共享一个发送端是安全的 |
| `mpsc::SyncSender<T>` | `T: Send` | `T: Send` | 同上，只是会阻塞 |
| `mpsc::Receiver<T>` | `T: Send` | **否** | 单消费者 |
| `MutexGuard<'_, T>` | **否** | `T: Sync` | 不能移动走，但共享引用可以发过去 |
| `LocalKey<T>` | 是 | 是 | 句柄本身是 auto trait，可随线程移动；但 `with` 取到的永远是**当前线程**那一份（1.98 实测，与 `T` 无关） |

## 动手看看

下面这个程序把本章的几件东西拼在一起：`thread::scope` 借本地切片、多个生产者用 `mpsc` 汇报、主线程归并。

```rust
{{#rustdoc_include ../snippets/ch19/wordcount.rs}}
```

在仓库根编译运行（Windows 写法；其他平台编译成 `wordcount`、运行 `./wordcount`）：

```console
$ rustc --edition 2024 snippets/ch19/wordcount.rs -o wordcount.exe
$ .\wordcount.exe
```

```text
the    3
fox    2
brown  1
dog    1
jumps  1
lazy   1
over   1
quick  1
```

设计上有三处和本章内容直接对应：

- **`thread::scope` 让分片借用 `words`。** 切片来自栈上的 `Vec<&str>`，`words.chunks(..)` 产出的每片都是借来的，直接 `move` 进线程——不需要 `Arc`、不需要 `clone`，因为作用域保证 join。
- **`tx.clone()` 是 m（多生产者）的实现方式。** 每片线程拿一份发送端，算完把自己那份 `HashMap` 发出去；发送端随线程结束被 drop。
- **`drop(tx)` 是终止条件。** 主线程手里的原件必须丢掉，`rx.iter()` 才会结束——`words` 里所有分片都发完之后，接收端靠「全部挂断」知道工作完成。

还有一处值得留意：**合并只在主线程做**。分片线程各自计自己的局部表，冲突面被压到最小；如果把 `Arc<Mutex<HashMap>>` 传进去、每数一个词加一次锁，锁竞争会吃掉并行的全部收益。这是通道式设计与锁式设计的分水岭：**通道把同步点收敛到「交接消息」这一处**。

`worker` 数量这里写死 3。真实程序可以用 `std::thread::available_parallelism()` 拿本机建议的并行度，再做上下限取舍。

## 报错现场

下面四条按报错来源排列：生命周期 → 类型约束 → 所有权 → 跨线程能力。行号按「最小复现放进 `src/main.rs`」给出；编译器 1.98，无关输出已删。

### 现场一：闭包借了活不到 `'static` 的东西（E0373）

```rust,compile_fail
use std::thread;

fn main() {
    let words = vec!["alpha", "beta"];
    let handle = thread::spawn(|| println!("{} 个词", words.len()));
    handle.join().unwrap();
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `words`, which is owned by the current function
 --> src/main.rs:5:32
  |
5 |     let handle = thread::spawn(|| println!("{} 个词", words.len()));
  |                                ^^                     ----- `words` is borrowed here
  |                                |
  |                                may outlive borrowed value `words`
  |
note: function requires argument type to outlive `'static`
 --> src/main.rs:5:18
  |
5 |     let handle = thread::spawn(|| println!("{} 个词", words.len()));
  |                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: to force the closure to take ownership of `words` (and any other referenced variables), use the `move` keyword
  |
5 |     let handle = thread::spawn(move || println!("{} 个词", words.len()));
  |                                ++++
```

读法：主 span 指出「闭包可能活得比当前函数久，但它借了 `words`」，`note` 点破约束来源（参数要活到 `'static`），`help` 给出第一步修法。但要留心：加 `move` 只解决「借用太长」，如果后面还要用 `words`，主线就会换成「值已被移动」。真正想「借完了还回来」就该用 `thread::scope`：

```rust
use std::thread;

fn main() {
    let words = vec!["alpha", "beta"];
    thread::scope(|s| {
        s.spawn(|| println!("{} 个词", words.len()));
    });
    println!("借完还在：{words:?}");
}
```

```text
2 个词
借完还在：["alpha", "beta"]
```

### 现场二：`Arc<RefCell<T>>` 也过不了 `Sync`（E0277）

从第 17 章的「四格表」，`RefCell<T>` 该换成 `Mutex`/`RwLock`，外层 `Rc` 换成 `Arc`。但把 `Arc` 套在 `RefCell` 外面照样不行：

```rust,compile_fail
use std::sync::Arc;
use std::cell::RefCell;
use std::thread;

fn main() {
    let shared = Arc::new(RefCell::new(0));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let s = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            *s.borrow_mut() += 1;
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    println!("{}", shared.borrow());
}
```

```text
error[E0277]: `RefCell<i32>` cannot be shared between threads safely
   --> src/main.rs:10:36
    |
 10 |           handles.push(thread::spawn(move || {
    |  ______________________-------------_^
    | |                      |
    | |                      required by a bound introduced by this call
 11 | |             *s.borrow_mut() += 1;
 12 | |         }));
    | |_________^ `RefCell<i32>` cannot be shared between threads safely
    |
    = help: the trait `Sync` is not implemented for `RefCell<i32>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` instead
    = note: required for `Arc<RefCell<i32>>` to implement `Send`
note: required because it's used within this closure
…
```

定位路径只有一步：主 span 落在这个闭包上，说明**闭包整体不是 `Send`**；`help` 指出缺的是 `RefCell<i32>: Sync`；最后一条 `note` 把链条接完——`Arc<T>` 要 `Send` 就得 `T: Send + Sync`（第 17 章那张表里最容易记错的一格），所以内层不合格时外层救不了。换件：`Arc<Mutex<i32>>`；读多写少就用 `RwLock`。

### 现场三：`Sender` 不是 `Copy`（E0382）

```rust,compile_fail
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || tx.send(1).unwrap()).join().unwrap();
    tx.send(2).unwrap();
    println!("{:?}", rx.recv());
}
```

```text
error[E0382]: borrow of moved value: `tx`
 --> src/main.rs:7:5
  |
5 |     let (tx, rx) = mpsc::channel();
  |          -- move occurs because `tx` has type `std::sync::mpsc::Sender<i32>`, which does not implement the `Copy` trait
6 |     thread::spawn(move || tx.send(1).unwrap()).join().unwrap();
  |                   ------- -- variable moved due to use in closure
  |                   |
  |                   value moved into closure here
7 |     tx.send(2).unwrap();
  |     ^^ value borrowed here after move
  |
help: consider cloning the value before moving it into the closure
  |
6 ~     let value = tx.clone();
7 ~     thread::spawn(move || value.send(1).unwrap()).join().unwrap();
  |
```

`move` 关键字一写，`tx` 的**所有权**就进了闭包，之后的 `tx.send(2)` 自然无效。修法就是 `help` 说的那条——`clone()` 一份生产者身份再移进去。这也解释了 mpsc 的用法模板：**原件留在主线程（或者干脆 `drop`），每个工作线程分一个 `clone`**。

### 现场四：`MutexGuard` 出不了线程（E0277）

```rust,compile_fail
use std::sync::Mutex;
use std::thread;

fn main() {
    let m = Mutex::new(0);
    thread::scope(|s| {
        let guard = m.lock().unwrap();
        s.spawn(move || {
            println!("{}", *guard);
        });
    });
}
```

```text
error[E0277]: `std::sync::MutexGuard<'_, i32>` cannot be sent between threads safely
   --> src/main.rs:8:17
    |
 8 |           s.spawn(move || {
    |             ----- ^------
    |             |     |
    |  ___________|_____within this `{closure@src/main.rs:8:17: 8:24}`
    | |           |
    | |           required by a bound introduced by this call
 9 | |             println!("{}", *guard);
 10 | |         });
    | |_________^ `std::sync::MutexGuard<'_, i32>` cannot be sent between threads safely
    |
    = help: within `{closure@src/main.rs:8:17: 8:24}`, the trait `Send` is not implemented for `std::sync::MutexGuard<'_, i32>`
note: required because it's used within this closure
…
note: required by a bound in `Scope::<'scope, 'env>::spawn`
…
    |
203 |         F: FnOnce() -> T + Send + 'scope,
    |                            ^^^^ required by this bound in `Scope::<'scope, 'env>::spawn`
```

这不是借用检查器在拦，而是 `MutexGuard: !Send` 在拦——解锁必须发生在加锁的线程上。正确的做法是把**要共享的数据**拿出来，而不是把锁的持有权搬走：在临界区里算完，把结果 `clone`/移动出来，再让 guard 在锁所在的作用域结束。

## 小结

- C/C++ 里，两个线程无同步地读写同一块内存就是数据竞争（data race），属于未定义行为，且极难复现；Rust 把它提前到编译期，代价是类型上的两条约束。
- `thread::spawn` 要求 `F: Send + 'static`：`'static` 来自「线程可能比创建它的作用域活得久」，`Send` 来自「闭包按值跨过线程边界」。要借栈上数据，用 `thread::scope`（1.63 起）：作用域结束前自动 join，所以借用安全；panic 会传染给 `scope` 本身。
- `join` 返回 `std::thread::Result<T>`（即 `Result<T, Box<dyn Any + Send + 'static>>`）：拿返回值，也拿 panic 负载（可 `downcast`）。
- `Send` = 值可以整个搬到另一个线程；`Sync` = `&T` 可以发过去（`&T: Send` 当且仅当 `T: Sync`）。它们是 auto trait：编译器逐字段推导，一个 `Rc` 字段就能毁掉整个类型。
- `unsafe impl` 是给「编译器看不见内部不变量」的类型准备的（裸指针、`UnsafeCell`）；写它等于签一份声明，不变量守不住就是 UB。闭包捕获精度（edition 2021+）会让它看起来没生效。
- `Arc<Mutex<T>>` 的跨线程增量：`try_lock` 非阻塞、中毒只是建议性的、同一线程重复 `lock()` 未指定、`MutexGuard` 不是 `Send`（但 `T: Sync` 时是 `Sync`）。
- `mpsc` 的通道分无限缓冲（`Sender`）与定长缓冲（`SyncSender`）；发送端全部消失时接收操作返回 `Err`，`rx.iter()` 因此能作为迭代器结束——**忘了 `drop` 原件就会卡死**。
- `thread_local!` 给每个线程一份独立状态，引用不能逃出 `with` 的闭包；`LocalKey<Cell<T>>` 有 `get`/`set` 捷径（1.73 起）。

## 练习

**1. 预测编译结果。** 下面两个类型都常被误当成「可以跨线程」。它们各自是不是 `Send`？用「断言函数」的手法验证，并解释差在哪。

```rust,compile_fail
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn assert_send<T: Send>() {}

fn main() {
    assert_send::<Rc<Mutex<i32>>>();
    assert_send::<Arc<Cell<i32>>>();
}
```

<details><summary>参考答案</summary>

两个都**不是** `Send`，但理由不同层。1.98 的关键输出：

```text
error[E0277]: `Rc<std::sync::Mutex<i32>>` cannot be sent between threads safely
 --> src/main.rs:8:19
  |
8 |     assert_send::<Rc<Mutex<i32>>>();
  |                   ^^^^^^^^^^^^^^ `Rc<std::sync::Mutex<i32>>` cannot be sent between threads safely
  |
  = help: the trait `Send` is not implemented for `Rc<std::sync::Mutex<i32>>`
…

error[E0277]: `Cell<i32>` cannot be shared between threads safely
 --> src/main.rs:9:19
  |
9 |     assert_send::<Arc<Cell<i32>>>();
  |                   ^^^^^^^^^^^^^^ `Cell<i32>` cannot be shared between threads safely
  |
  = help: the trait `Sync` is not implemented for `Cell<i32>`
  = note: required for `Arc<Cell<i32>>` to implement `Send`
…
```

- `Rc<Mutex<i32>>`：外层 `Rc` 就不过关（非原子计数），`Send` 的需求在它这里就断了，内层装什么无关紧要——这正是 auto trait 逐字段推导的直接后果。**内层修好外层没用**。
- `Arc<Cell<i32>>`：外层 `Arc` 对 `Send` 的要求是 `T: Send + Sync`。`Cell<i32>: Send` 没问题，但 `Cell<i32>: !Sync`，于是 `Arc<Cell<i32>>` 不是 `Send`。修法要么换掉 `Cell`（用第 20 章的 `AtomicI32`），要么换掉外层语义（`Mutex`/`RwLock`）。

这两条合起来说明换件顺序：**先看最外层是不是 `Send`，再看 `Arc` 的内层是不是 `Send + Sync`**。

</details>

**2. 找错并修。** 下面这段程序编译通过，但运行时会停不下来。指出卡在哪一行、为什么，并给出最小修法。

```rust,no_run
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<i32>();
    for i in 0..3 {
        let tx = tx.clone();
        thread::spawn(move || tx.send(i).unwrap());
    }

    let mut total = 0;
    for v in rx.iter() {
        total += v;
    }
    println!("{total}");
}
```

<details><summary>参考答案</summary>

卡在 `for v in rx.iter()`：三个克隆出来的发送端在各自线程结束时都被 drop 了，但**主线程手里的原始 `tx` 还活着**。`rx.iter()` 只在「所有发送端都消失且缓冲为空」时才结束，所以收完 3 条之后它继续阻塞等待第 4 条，永远等不到。

用带超时的接收可以看出这个状态（下面是产生它的最小程序与实测输出）：

```rust
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let (tx, rx) = mpsc::channel::<i32>();
    for i in 0..3 {
        let tx = tx.clone();
        thread::spawn(move || tx.send(i).unwrap());
    }

    // 主线程手里的 tx 还活着：三条消息收完后，通道并没有「结束」
    let mut total = 0;
    for _ in 0..3 {
        total += rx.recv().unwrap();
    }
    println!("收到 3 条，合计 {total}");

    match rx.recv_timeout(Duration::from_millis(300)) {
        Ok(v) => println!("第 4 条 = {v}"),
        Err(e) => println!("第 4 次 recv：{e}"),
    }

    drop(tx);
    match rx.recv_timeout(Duration::from_millis(100)) {
        Ok(v) => println!("{v}"),
        Err(e) => println!("drop(tx) 之后：{e}"),
    }
}
```

```text
收到 3 条，合计 3
第 4 次 recv：timed out waiting on channel
drop(tx) 之后：channel is empty and sending half is closed
```

第一段 `timeout` 说明「没有消息，但通道还活着」；`drop(tx)` 之后立刻变成 `Disconnected`（显示为 channel is empty and sending half is closed）。最小修法就是在起完生产者之后、进入接收循环之前把原件丢掉：

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<i32>();
    for i in 0..3 {
        let tx = tx.clone();
        thread::spawn(move || tx.send(i).unwrap());
    }
    drop(tx); // 主线程不再发送

    let mut total = 0;
    for v in rx.iter() {
        total += v;
    }
    println!("{total}");
}
```

```text
3
```

另一条路是别用 `iter()`，改用 `recv_timeout` 或先约定「收到的条数」再 `recv()` 固定次数。**只要发送端数量不是静态可知的，就该用「全部挂断」这一条**——但要保证真的一起挂断。

</details>

**3. 解释为什么。** `MutexGuard<'_, T>` **不是** `Send`，却在 `T: Sync` 时**是** `Sync`。这两条为什么必须如此？如果 `MutexGuard` 是 `Send` 会出什么问题？

<details><summary>参考答案</summary>

两条都由「解锁这个动作本身」决定，理由 std 写在 `MutexGuard` 的类型文档里。

`!Send`：文档原话是 A `MutexGuard` is not `Send` to maximize platform portability，紧接着解释：在 POSIX threads（pthreads）平台上，**互斥锁必须在加锁的同一个线程里解锁**。如果 guard 能移走，就可能出现「A 线程加锁、B 线程解锁」——在那类平台上直接违反底层 API 的约束，实现无法同时保持可移植与健全。

`Sync`（当 `T: Sync`）：guard 实现了 `Deref<Target = T>`，所以 `&guard` 能拿到 `&T`。能共享 `&guard` 就等于「多个线程同时读里面的值」，这件事在 `T: Sync` 时本来就安全（std：`T` must be `Sync` for a `MutexGuard<T>` to be `Sync` because it is possible to get a `&T` from `&MutexGuard`）。注意这**没有**让两个线程同时持有独占访问——`Sync` 管的是共享引用，拿到 `&T` 只能读。

于是「`!Send` 但 `Sync`」并不矛盾：`Send` 问「能不能把整块东西搬走」（解锁责任会跟着移动，不行），`Sync` 问「能不能把这个东西的共享引用发过去」（只读，可以）。

实践上的后果：持锁期间要把值交给别的线程，**交值，不要交 guard**——在临界区里 `clone`/计算/取出结果，让 guard 在锁所在的作用域自然 drop。

</details>

## 延伸阅读

- The Book 第 16 章《Fearless Concurrency》（`thread`/channel/`Mutex`/`Arc` 的入门路径）：<https://doc.rust-lang.org/book/ch16-00-concurrency.html>
- `std::thread` 模块文档（线程模型、`spawn` 的约束、`scope`、`Builder`、`available_parallelism`）：<https://doc.rust-lang.org/std/thread/index.html>
- `std::marker::Send` 与 `std::marker::Sync`（精确定义与引用类型的对应关系）：<https://doc.rust-lang.org/std/marker/trait.Send.html>、<https://doc.rust-lang.org/std/marker/trait.Sync.html>
- `std::sync::mpsc`（`channel`/`sync_channel`/`try_recv`/`recv_timeout`）：<https://doc.rust-lang.org/std/sync/mpsc/index.html>
- `std::sync::Mutex`（中毒、`try_lock`、重入行为）与 `std::sync::PoisonError`：<https://doc.rust-lang.org/std/sync/struct.Mutex.html>、<https://doc.rust-lang.org/std/sync/struct.PoisonError.html>
- `std::thread_local!` 与 `std::thread::LocalKey`（每线程存储、`with` 的边界）：<https://doc.rust-lang.org/std/macro.thread_local.html>、<https://doc.rust-lang.org/std/thread/struct.LocalKey.html>
- The Nomicon：`Send` 与 `Sync`（含 `unsafe impl` 的语义与 `Rc` 为什么不合格）：<https://doc.rust-lang.org/nomicon/send-and-sync.html>、锁中毒：<https://doc.rust-lang.org/nomicon/poisoning.html>
- The Rust Reference：行为视为未定义的情形（数据竞争在其中）：<https://doc.rust-lang.org/reference/behavior-considered-undefined.html>
