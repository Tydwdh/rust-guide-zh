//! 极简单线程执行器：够说明「谁推进 future」和「Waker 干什么」。
//! 真运行时要做的事远不止这些：定时器、I/O 就绪事件、任务抢占、多线程调度……（第 38 章）
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

/// 唤醒句柄：记录「任务已被唤醒」，并把阻塞中的执行器线程叫起来。
struct Parker {
    woken: Mutex<bool>,
    condvar: Condvar,
}

impl Parker {
    fn new() -> Arc<Parker> {
        Arc::new(Parker {
            woken: Mutex::new(false),
            condvar: Condvar::new(),
        })
    }

    /// 睡到有人 wake 为止；已经被唤醒过就不睡（否则会丢唤醒）。
    fn park(&self) {
        let mut woken = self.woken.lock().unwrap();
        while !*woken {
            woken = self.condvar.wait(woken).unwrap();
        }
        *woken = false;
    }
}

// 实现 Wake（而不是手搓 RawWaker）是安全写法：Waker::from 会把 Arc 包成 Waker。
impl Wake for Parker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        *self.woken.lock().unwrap() = true;
        self.condvar.notify_one();
    }
}

/// 单任务执行器：一直 poll，直到 future 返回 Ready。
fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let parker = Parker::new();
    let waker = Waker::from(parker.clone());
    let mut cx = Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => parker.park(),
        }
    }
}

/// 多任务执行器：把一组 future 当任务队列，每次唤醒后把没完成的都推进一遍。
fn block_on_all<F: Future>(futures: Vec<F>) -> Vec<F::Output> {
    let mut tasks: Vec<(Pin<Box<F>>, Option<F::Output>)> =
        futures.into_iter().map(|f| (Box::pin(f), None)).collect();
    let parker = Parker::new();
    let waker = Waker::from(parker.clone());
    let mut cx = Context::from_waker(&waker);
    loop {
        for (fut, out) in tasks.iter_mut() {
            if out.is_none() {
                if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                    *out = Some(v);
                }
            }
        }
        if tasks.iter().all(|(_, out)| out.is_some()) {
            return tasks.into_iter().map(|(_, out)| out.unwrap()).collect();
        }
        parker.park();
    }
}

/// 睡够指定毫秒才完成的 future：真正的等待交给后台线程。
struct Delay {
    ms: u64,
    started: bool,
    done: Arc<AtomicBool>,
}

impl Delay {
    fn new(ms: u64) -> Delay {
        Delay {
            ms,
            started: false,
            done: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done.load(Ordering::Acquire) {
            return Poll::Ready(());
        }
        if !self.started {
            self.started = true;
            let done = self.done.clone();
            let ms = self.ms;
            // 关键一步：把 waker 复制一份交给别的线程，睡够之后再叫醒任务。
            let waker = cx.waker().clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(ms));
                done.store(true, Ordering::Release);
                waker.wake();
            });
        }
        Poll::Pending
    }
}

async fn label(ms: u64, text: &'static str) -> &'static str {
    Delay::new(ms).await;
    text
}

fn main() {
    let t = Instant::now();
    block_on(async {
        Delay::new(60).await;
        Delay::new(90).await;
    });
    println!("顺序 await 两个 Delay：{} ms", t.elapsed().as_millis());

    let t = Instant::now();
    let labels = block_on_all(vec![label(60, "A(60ms)"), label(90, "B(90ms)")]);
    println!("并发跑两个 Delay：{} ms -> {labels:?}", t.elapsed().as_millis());
}
