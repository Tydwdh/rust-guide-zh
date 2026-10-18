//! 池本身：一条共享的任务队列 + N 个 worker 线程。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, SendError, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::handle::TaskHandle;

/// 一个任务：装箱的一次性闭包，没有参数、没有返回值。
///
/// `Send` 是因为它会被搬到 worker 线程上执行；`'static` 是因为它会比
/// 提交它的那次调用活得更久 —— 队列归池所有，池比任何一次调用都长命。
type Job = Box<dyn FnOnce() + Send + 'static>;

/// 提交口：无界队列，或定长队列。
///
/// 两种发送端的 `send` 签名一样，差别只有「队列满了会不会阻塞」，所以这里
/// 用一个枚举把差别收进一个方法，池的其余部分不必再分叉。
enum Queue {
    Unbounded(Sender<Job>),
    Bounded(SyncSender<Job>),
}

impl Queue {
    fn send(&self, job: Job) -> Result<(), SendError<Job>> {
        match self {
            Queue::Unbounded(tx) => tx.send(job),
            Queue::Bounded(tx) => tx.send(job),
        }
    }
}

/// 固定大小的线程池：`size` 个 worker 从同一条队列里取任务。
///
/// ```
/// use task_queue::ThreadPool;
///
/// let pool = ThreadPool::new(2);
/// pool.spawn(|| println!("跑在 worker 上"));
/// assert_eq!(pool.worker_count(), 2);
/// pool.shutdown();
/// ```
pub struct ThreadPool {
    /// 发送端。`None` 表示队列已关（`shutdown` 或 `drop` 的第一步）。
    queue: Option<Queue>,
    /// worker 句柄，关闭时逐个 join。
    workers: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    /// 建一个池：`size` 个 worker，队列无界（提交永不阻塞）。
    ///
    /// # Panics
    ///
    /// `size` 为 0 时 panic：没有 worker 的池会收下任务却永远不跑，
    /// 提交者只会拿到一个永不返回的 `join`。
    pub fn new(size: usize) -> ThreadPool {
        ThreadPool::build(size, None)
    }

    /// 建一个池：队列最多缓冲 `capacity` 个任务，满了提交就阻塞。
    ///
    /// `capacity` 为 0 时是交会（rendezvous）队列：每次提交都要等一个
    /// 空闲 worker 来取。
    ///
    /// # Panics
    ///
    /// `size` 为 0 时 panic（与 [`ThreadPool::new`] 相同）。
    pub fn bounded(size: usize, capacity: usize) -> ThreadPool {
        ThreadPool::build(size, Some(capacity))
    }

    /// worker 线程数。
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// 提交一个不用拿结果的任务。
    ///
    /// 任务 panic 不会带走 worker，也不会影响别的任务；想拿到 panic 负载，
    /// 用 [`ThreadPool::submit`]。
    pub fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.enqueue(Box::new(move || {
            let _ = catch_unwind(AssertUnwindSafe(job));
        }));
    }

    /// 提交一个任务，返回它的句柄。
    ///
    /// ```
    /// use task_queue::ThreadPool;
    ///
    /// let pool = ThreadPool::new(1);
    /// let handle = pool.submit(|| "结果在这里");
    /// assert_eq!(handle.join().unwrap(), "结果在这里");
    /// ```
    pub fn submit<T, F>(&self, job: F) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        self.enqueue(Box::new(move || {
            // 正常结束与 panic 走同一条路：先算出 `thread::Result<T>`，再发回去。
            let _ = tx.send(catch_unwind(AssertUnwindSafe(job)));
        }));
        TaskHandle::new(rx)
    }

    /// 关掉池：先丢发送端，再等 worker 退出。
    ///
    /// 等价于把池 `drop` 掉；显式调用的好处是读代码的人知道这里是在收工。
    /// worker 会把已经排队的任务跑完再退。
    pub fn shutdown(mut self) {
        self.close();
    }

    fn build(size: usize, capacity: Option<usize>) -> ThreadPool {
        assert!(size > 0, "线程池至少需要一个 worker");
        let (queue, rx) = match capacity {
            None => {
                let (tx, rx) = mpsc::channel();
                (Queue::Unbounded(tx), rx)
            }
            Some(capacity) => {
                let (tx, rx) = mpsc::sync_channel(capacity);
                (Queue::Bounded(tx), rx)
            }
        };
        // 接收端只能有一个持有者，而 worker 有 size 个：放进锁里，谁空谁等。
        let rx = Arc::new(Mutex::new(rx));
        let workers = (0..size)
            .map(|_| {
                let rx = Arc::clone(&rx);
                thread::spawn(move || work(rx))
            })
            .collect();
        ThreadPool {
            queue: Some(queue),
            workers,
        }
    }

    fn enqueue(&self, job: Job) {
        // 池活着 ⇒ 发送端活着 ⇒ 至少有一个 worker 还在 `recv`，所以这次
        // `send` 不会失败。`expect` 是把这条不变量写下来，而不是替它编一个
        // 永远用不到的错误类型。第一条的 `Option` 只有 `close` 能清空，而
        // `close` 要 `&mut self` —— 手里有 `&self`，队列就必然还活着。
        let queue = self
            .queue
            .as_ref()
            .expect("close 需要 `&mut self`，此处池必然还活着");
        queue
            .send(job)
            .expect("worker 在池还活着的时候不会全部消失");
    }

    fn close(&mut self) {
        // 第一步：丢发送端。worker 的 `recv` 因此拿到 `Err`，循环才会结束。
        // 队列里还没被取走的任务不会丢：通道在发送端消失后照样能把存货交出来。
        drop(self.queue.take());
        // 第二步：等 worker 退出；它们会先把存货跑完。
        for worker in self.workers.drain(..) {
            // worker 不跑可能 panic 的代码，所以这里不会有 `Err`；即便有，
            // 在 `Drop` 里再抛 panic 会直接 abort，忽略才是对的做法。
            let _ = worker.join();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.close();
    }
}

/// worker 的主循环：拿到任务就放手去跑，跑完再回来等下一个。
fn work(rx: Arc<Mutex<Receiver<Job>>>) {
    loop {
        // 锁只护着「取任务」这一下：任务本身跑在锁外，worker 之间才是真并行。
        let next = match rx.lock() {
            Ok(guard) => guard.recv(),
            // 持锁时 panic 才会中毒，而这里没有会 panic 的代码；真中毒了也
            // 照样取任务（`into_inner` 把守卫拿回来），不让池跟着瘫掉。
            Err(poisoned) => poisoned.into_inner().recv(),
        };
        match next {
            Ok(job) => job(),
            // 所有发送端都没了：队列关掉，worker 退休。
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "至少需要一个 worker")]
    fn zero_workers_is_rejected() {
        let _ = ThreadPool::new(0);
    }

    #[test]
    fn both_queue_kinds_report_a_gone_receiver() {
        // 发送端自己活着，但对端没了：send 报错而不是永久阻塞。
        let (tx, rx) = mpsc::channel();
        drop(rx);
        assert!(Queue::Unbounded(tx).send(Box::new(|| {})).is_err());

        let (tx, rx) = mpsc::sync_channel(1);
        drop(rx);
        assert!(Queue::Bounded(tx).send(Box::new(|| {})).is_err());
    }
}
