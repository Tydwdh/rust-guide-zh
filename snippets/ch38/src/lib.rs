//! 第 38 章的极简 async 运行时：手写状态机 + 手造 `Waker` + 手写 pin 投影 + 单线程执行器。
//!
//! 只用 std，代码在 stable 1.98 上编译并通过 `cargo test`。

use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Wake, Waker};
use std::time::Duration;

// ---------------------------------------------------------------------------
// 1. 手写状态机：不用 async/await，把一个挂在 .await 上的计算写成枚举
// ---------------------------------------------------------------------------

/// 等价于 `async { let s = a + b; yield_now().await; s + c }` 的手写状态机。
///
/// 编译器给 `async` 块生成的也是这样一个带标签的枚举：每个「挂起点之后还要用」
/// 的局部变量，都变成某个变体的字段。
pub enum TwoStep {
    /// 还没开始：三个操作数都还在
    Start { a: u32, b: u32, c: u32 },
    /// 第一次 poll 之后：`a + b` 已经算出来，`c` 还得留着
    Half { sum: u32, c: u32 },
    /// 结束态：只为了让 `mem::replace` 有个占位值
    Done,
}

impl TwoStep {
    pub fn new(a: u32, b: u32, c: u32) -> Self {
        TwoStep::Start { a, b, c }
    }
}

impl Future for TwoStep {
    type Output = u32;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        // TwoStep 的字段全是 u32，因此它是 Unpin 的：拿 &mut self 没有任何风险。
        let this = self.get_mut();
        match std::mem::replace(this, TwoStep::Done) {
            TwoStep::Start { a, b, c } => {
                // 第一段算完，把中间结果写回状态机（这就是「状态」的含义）
                *this = TwoStep::Half { sum: a + b, c };
                // 登记唤醒：告诉执行器「我还能往下走，记得再来 poll 我一次」
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            TwoStep::Half { sum, c } => Poll::Ready(sum + c),
            TwoStep::Done => panic!("已完成的 future 不应再被 poll"),
        }
    }
}

// ---------------------------------------------------------------------------
// 2. 执行器：就绪队列 + 任务 + 唤醒路径
// ---------------------------------------------------------------------------

/// 队列空着、目标却还没达成：没有谁会再来唤醒我们了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stalled;

impl fmt::Display for Stalled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("执行器停滞：队列为空且无人登记唤醒")
    }
}

impl std::error::Error for Stalled {}

/// 队列空着时，执行器最多睡这么久；醒来还是空队列就判定停滞。
///
/// 真运行时用事件源（epoll、I/O 完成队列、定时器堆）决定「该睡到什么时候」，
/// 这个超时只是兜底；这里说出来是为了让「极简」的边界清清楚楚。
const STALL_TIMEOUT: Duration = Duration::from_millis(300);

/// 任务共用的那一半：就绪队列 + 让执行器睡/醒的信号。
struct Shared {
    state: Mutex<State>,
    signal: Condvar,
    /// 只影响 `Waker` 的构造方式：false = `Waker::from(Arc<W>)`，true = 手写 vtable。
    raw_waker: bool,
}

struct State {
    queue: VecDeque<Arc<Task>>,
}

/// 一个任务：堆上 pin 住的 future + 「是否已在就绪队列里」的标记。
pub struct Task {
    shared: Arc<Shared>,
    future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
    /// 与队列同锁保护：置位表示「已经在队列里」，用来合并重复唤醒。
    queued: AtomicBool,
}

impl Task {
    /// poll 一次。
    fn run(self: &Arc<Self>) {
        // 两条构造 Waker 的路：安全版（Wake + Arc）与手写 vtable 版。结果完全一样。
        let waker = if self.shared.raw_waker {
            raw_waker(self)
        } else {
            Waker::from(Arc::clone(self))
        };
        let mut cx = Context::from_waker(&waker);

        let mut slot = self.future.lock().unwrap();
        // 把 future 从槽里取出来 poll：移动的是 `Pin<Box<_>>`，堆上的值没有动。
        if let Some(mut fut) = slot.take() {
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(()) => {} // 完成：fut 在这里析构
                Poll::Pending => *slot = Some(fut),
            }
        }
    }
}

/// 唤醒路径：把任务放回就绪队列。这就是 `Wake` 要求实现的那件事。
impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // 标记与入队在同一把锁下完成，堵掉「丢了唤醒」的那个窗口。
        let mut state = self.shared.state.lock().unwrap();
        if !self.queued.swap(true, Ordering::AcqRel) {
            state.queue.push_back(Arc::clone(self));
            self.shared.signal.notify_one();
        }
    }
}

/// 单线程执行器：维护就绪队列，把队列里的任务一个个 poll 掉。
pub struct Executor {
    shared: Arc<Shared>,
}

impl Executor {
    pub fn new() -> Self {
        Self::build(false)
    }

    /// 同样的执行器，但用第 4 节手写的 `RawWaker` 造唤醒句柄。
    pub fn with_raw_waker() -> Self {
        Self::build(true)
    }

    fn build(raw_waker: bool) -> Self {
        Executor {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    queue: VecDeque::new(),
                }),
                signal: Condvar::new(),
                raw_waker,
            }),
        }
    }

    /// 把 future 包成任务放进就绪队列。
    pub fn spawn<F>(&self, fut: F) -> Arc<Task>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            shared: Arc::clone(&self.shared),
            future: Mutex::new(Some(Box::pin(fut))),
            queued: AtomicBool::new(false),
        });
        // 首次入队走的就是唤醒那条路
        task.wake_by_ref();
        task
    }

    fn next_task(&self) -> Option<Arc<Task>> {
        let mut state = self.shared.state.lock().unwrap();
        let task = state.queue.pop_front()?;
        // 出队与清标记同样在同一把锁下
        task.queued.store(false, Ordering::Release);
        Some(task)
    }

    /// 把当前就绪队列跑空，返回 poll 了多少次。
    ///
    /// 任务在 `poll` 里自己唤醒自己时，会重新入队，于是这一轮会继续跑下去。
    pub fn run(&self) -> usize {
        let mut polls = 0;
        while let Some(task) = self.next_task() {
            task.run();
            polls += 1;
        }
        polls
    }

    /// 跑到 `done()` 为 true 为止。
    ///
    /// 队列空了而目标没达成时，先睡一会儿等唤醒；睡满 [`STALL_TIMEOUT`] 还是空队列，
    /// 就判定没人会再唤醒我们（正常任务不会走到这一步——那是契约被违反的信号）。
    pub fn run_until<D: Fn() -> bool>(&self, done: D) -> Result<(), Stalled> {
        loop {
            while let Some(task) = self.next_task() {
                task.run();
            }
            if done() {
                return Ok(());
            }

            let state = self.shared.state.lock().unwrap();
            if !state.queue.is_empty() {
                continue; // 有人抢先入队了
            }
            let (state, timeout) = self.shared.signal.wait_timeout(state, STALL_TIMEOUT).unwrap();
            let empty = state.queue.is_empty();
            drop(state);

            if timeout.timed_out() && empty && !done() {
                return Err(Stalled);
            }
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// 把根 future 交给执行器，跑完取回输出。
pub fn block_on<F, T>(fut: F) -> T
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let out = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&out);
    let ex = Executor::new();
    ex.spawn(async move {
        let v = fut.await;
        *slot.lock().unwrap() = Some(v);
    });
    match ex.run_until(|| out.lock().unwrap().is_some()) {
        Ok(()) => out.lock().unwrap().take().expect("根任务没有产出"),
        Err(e) => panic!("{e}"),
    }
}

// ---------------------------------------------------------------------------
// 3. 手写 RawWaker：不安全点只有一处——裸指针与引用计数
// ---------------------------------------------------------------------------

/// 用一个 `Arc<Task>` 裸指针造 `Waker`，走的是 `Waker::from_raw` + 自备 vtable。
///
/// 这是 `impl Wake` + `Waker::from(Arc<W>)` 在库内部做的事，写一遍就知道
/// unsafe 的责任落在哪儿：data 指针的引用计数必须与 Waker 的克隆/析构严格配对。
pub fn raw_waker(task: &Arc<Task>) -> Waker {
    // `Arc::into_raw` 把一份强引用交给裸指针；它由 vtable 的 drop 归还。
    let data = Arc::into_raw(Arc::clone(task)).cast::<()>();
    // SAFETY: data 指向一个 Task（线程安全类型），且我们为民用指针
    // 备好了完整的 vtable；后面四个函数各自维护好那一份强引用。
    unsafe { Waker::from_raw(RawWaker::new(data, &VTABLE)) }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_raw, wake_raw, wake_by_ref_raw, drop_raw);

unsafe fn clone_raw(data: *const ()) -> RawWaker {
    // 把裸指针临时还原成 Arc（不动计数），克隆一份，再把两份都交还出去
    let task = unsafe { Arc::<Task>::from_raw(data.cast::<Task>()) };
    let cloned = Arc::clone(&task);
    let _ = Arc::into_raw(task); // 原来那份还给调用者
    RawWaker::new(Arc::into_raw(cloned).cast::<()>(), &VTABLE)
}

unsafe fn wake_raw(data: *const ()) {
    // 这份强引用被消费掉
    let task = unsafe { Arc::<Task>::from_raw(data.cast::<Task>()) };
    task.wake();
}

unsafe fn wake_by_ref_raw(data: *const ()) {
    let task = unsafe { Arc::<Task>::from_raw(data.cast::<Task>()) };
    task.wake_by_ref();
    // 只是借用：把引用交还回去
    let _ = Arc::into_raw(task);
}

unsafe fn drop_raw(data: *const ()) {
    drop(unsafe { Arc::<Task>::from_raw(data.cast::<Task>()) });
}

// ---------------------------------------------------------------------------
// 4. 手写 pin 投影，封成安全 API
// ---------------------------------------------------------------------------

/// 给内层 future 一个「最多被 poll 几次」的预算：用尽就放弃，返回 `None`。
///
/// 它是本章 pin 投影的样本：内层 future 可能是 `!Unpin` 的，要按 pinned 的方式
/// 递出去，只能手写投影；而投影被包在私有方法里，外部 API 全是安全的。
pub struct Budgeted<F> {
    inner: F,
    budget: u32,
}

impl<F> Budgeted<F> {
    pub fn new(inner: F, budget: u32) -> Self {
        Self { inner, budget }
    }
}

impl<F: Future> Budgeted<F> {
    /// 把 `Pin<&mut Self>` 投影成内层 future 的 pin。
    fn inner(self: Pin<&mut Self>) -> Pin<&mut F> {
        // SAFETY:
        // ① 本类型没有 Drop 实现，不会在析构里移动字段；
        // ② 我们只把 `Pin<&mut F>` 交出去，从不通 `&mut Self` 移动 `inner`；
        // ③ 因此「Pin<&mut Self> 成立 ⇒ inner 地址固定」这条链是完整的。
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }
    }

    /// `budget` 不参与 pin：它只是个计数器，挪动它没有任何后果。
    fn budget_mut(self: Pin<&mut Self>) -> &mut u32 {
        // SAFETY: 只就地读写 u32，不会把值移出 pin。
        unsafe { &mut self.get_unchecked_mut().budget }
    }
}

impl<F: Future> Future for Budgeted<F> {
    type Output = Option<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self;
        match this.as_mut().inner().poll(cx) {
            Poll::Ready(v) => Poll::Ready(Some(v)),
            Poll::Pending => {
                let budget = this.budget_mut();
                if *budget == 0 {
                    Poll::Ready(None)
                } else {
                    *budget -= 1;
                    Poll::Pending
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 5. 演示与测试用的零件
// ---------------------------------------------------------------------------

/// 每被 poll 一次就让位一次，把每次 poll 记进共享的日志（顺带看交错顺序）。
pub struct Countdown {
    name: &'static str,
    left: u32,
    log: Arc<Mutex<Vec<String>>>,
}

impl Countdown {
    pub fn new(name: &'static str, left: u32) -> Self {
        Self {
            name,
            left,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 让若干个任务写进同一份日志，用来看它们在队列里的交错顺序。
    pub fn sharing(name: &'static str, left: u32, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self { name, left, log }
    }

    /// 与这个任务共享日志的句柄，测试里用它检查推进顺序。
    pub fn log(&self) -> Arc<Mutex<Vec<String>>> {
        Arc::clone(&self.log)
    }
}

impl Future for Countdown {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut(); // 字段都是 Unpin 的
        if this.left == 0 {
            return Poll::Ready(());
        }
        this.log
            .lock()
            .unwrap()
            .push(format!("{}#{}", this.name, this.left));
        this.left -= 1;
        // 还能往下走，直接叫醒自己（等价于 `yield_now().await`）
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// 记录自己每次被 poll 时的地址，用来验证 pin 投影没有让内层 future 搬家。
    struct AddrProbe {
        seen: Arc<Mutex<Vec<usize>>>,
        left: u32,
    }

    impl Future for AddrProbe {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            let addr = self.as_ref().get_ref() as *const Self as usize;
            self.seen.lock().unwrap().push(addr);
            let this = self.get_mut();
            if this.left == 0 {
                return Poll::Ready(());
            }
            this.left -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

    /// 只数唤醒次数的 Waker：用来在没有执行器的情况下手动 poll。
    #[derive(Default)]
    struct CountWakes(AtomicUsize);

    impl Wake for CountWakes {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref()
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn handwritten_state_machine_registers_wake_on_pending() {
        let counter = Arc::new(CountWakes::default());
        let waker = Waker::from(Arc::clone(&counter));
        let mut cx = Context::from_waker(&waker);

        let mut fut = std::pin::pin!(TwoStep::new(1, 2, 3));
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(counter.0.load(Ordering::Relaxed), 1, "Pending 必须登记唤醒");
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Ready(6));
        assert_eq!(counter.0.load(Ordering::Relaxed), 1, "Ready 之后不再唤醒");
    }

    #[test]
    fn budgeted_stops_after_budget_is_used_up() {
        let mut fut = std::pin::pin!(Budgeted::new(Countdown::new("X", 100), 2));
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending); // 用完第 2 格
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending); // 用完第 1 格
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Ready(None)); // 预算见底，放弃
    }

    #[test]
    fn projection_keeps_inner_future_in_place() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut fut = std::pin::pin!(Budgeted::new(
            AddrProbe {
                seen: Arc::clone(&seen),
                left: 2,
            },
            10,
        ));
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(fut.as_mut().poll(&mut cx), Poll::Ready(Some(())));

        let addrs = seen.lock().unwrap();
        assert_eq!(addrs.len(), 3);
        assert!(
            addrs.iter().all(|a| *a == addrs[0]),
            "内层 future 在投影之后不应该搬家：{addrs:?}"
        );
    }

    #[test]
    fn tasks_advance_in_queue_order() {
        // 两个任务共用一个日志：看出来的是它们在就绪队列里的真实交错顺序
        let log = Arc::new(Mutex::new(Vec::new()));
        let ex = Executor::new();
        ex.spawn(Countdown::sharing("A", 3, Arc::clone(&log)));
        ex.spawn(Countdown::sharing("B", 2, Arc::clone(&log)));
        assert_eq!(ex.run(), 7);
        assert_eq!(*log.lock().unwrap(), ["A#3", "B#2", "A#2", "B#1", "A#1"]);
    }

    #[test]
    fn raw_waker_and_safe_waker_agree() {
        let (safe_a, safe_b) = (Countdown::new("A", 3), Countdown::new("B", 2));
        let (raw_a, raw_b) = (Countdown::new("A", 3), Countdown::new("B", 2));
        let logs = [
            safe_a.log(),
            safe_b.log(),
            raw_a.log(),
            raw_b.log(),
        ];

        let safe = Executor::new();
        safe.spawn(safe_a);
        safe.spawn(safe_b);
        let safe_polls = safe.run();

        let raw = Executor::with_raw_waker();
        raw.spawn(raw_a);
        raw.spawn(raw_b);
        let raw_polls = raw.run();

        assert_eq!(safe_polls, raw_polls);
        assert_eq!(*logs[0].lock().unwrap(), *logs[2].lock().unwrap());
        assert_eq!(*logs[1].lock().unwrap(), *logs[3].lock().unwrap());
    }

    #[test]
    fn waker_can_wake_from_another_thread() {
        // 第一次 poll 时起一个线程，睡一小会儿后从别的线程唤醒；
        // 唤醒到达后执行器再 poll 一次，这次才拿到结果。
        struct FromThread {
            rx: std::sync::mpsc::Receiver<u32>,
            tx: Option<std::sync::mpsc::Sender<u32>>,
        }
        impl Future for FromThread {
            type Output = u32;
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
                let this = self.get_mut();
                if let Some(tx) = this.tx.take() {
                    let waker = cx.waker().clone(); // Waker 是 Send + Sync，可以带走
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(20));
                        let _ = tx.send(7);
                        waker.wake();
                    });
                }
                match this.rx.try_recv() {
                    Ok(v) => Poll::Ready(v),
                    Err(_) => Poll::Pending,
                }
            }
        }

        let (tx, rx) = std::sync::mpsc::channel();
        assert_eq!(block_on(FromThread { rx, tx: Some(tx) }), 7);
    }

    #[test]
    fn work_stealing_is_out_of_scope_but_stall_is_detected() {
        // 返回 Pending 却不登记唤醒：违反 poll 的契约，执行器只能报停滞
        struct NeverWakes;
        impl Future for NeverWakes {
            type Output = u32;
            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
                Poll::Pending
            }
        }
        let ex = Executor::new();
        ex.spawn(async {
            let _ = NeverWakes.await;
        });
        assert_eq!(ex.run_until(|| false), Err(Stalled));
    }

    #[test]
    fn wake_is_coalesced_into_one_queue_slot() {
        let ex = Executor::new();
        let task = ex.spawn(Countdown::new("A", 0));
        // spawn 已经入队一次；再唤醒两次不应该再排队
        task.wake_by_ref();
        task.wake_by_ref();
        assert_eq!(ex.shared.state.lock().unwrap().queue.len(), 1);
        assert_eq!(ex.run(), 1);
    }

    #[test]
    fn raw_waker_keeps_refcounts_balanced() {
        let ex = Executor::new();
        let task = ex.spawn(Countdown::new("A", 0));
        let before = Arc::strong_count(&task);

        let waker = raw_waker(&task); // vtable 的 data 从一份强引用开始
        assert_eq!(Arc::strong_count(&task), before + 1);

        waker.wake_by_ref(); // vtable 的 wake_by_ref：借了还，不消费
        assert_eq!(Arc::strong_count(&task), before + 1);

        let cloned = waker.clone(); // vtable 的 clone：+1
        assert_eq!(Arc::strong_count(&task), before + 2);

        drop(waker); // vtable 的 drop：-1
        assert_eq!(Arc::strong_count(&task), before + 1, "drop 必须归还那一份");

        cloned.wake(); // vtable 的 wake：消费这份引用
        assert_eq!(Arc::strong_count(&task), before);
    }
}
