//! 迷你任务队列：一个只用标准库的固定大小线程池。
//!
//! 一次提交走完整条路是这样的：
//!
//! 1. 调用方把闭包交给 [`ThreadPool::spawn`]（不关心结果）或 [`ThreadPool::submit`]（要结果）；
//! 2. 闭包被装箱、送进队列，`submit` 顺手把一条只有自己知道的返回通道塞进闭包；
//! 3. 空闲 worker 从队列里取走任务，在锁外执行；
//! 4. 任务正常结束就把返回值发回那条通道，panic 就把 panic 负载发回去；
//! 5. 调用方在 [`TaskHandle`] 上等结果 —— 一直等，或者最多等一段时间。
//!
//! ```
//! use task_queue::ThreadPool;
//!
//! let pool = ThreadPool::new(2);
//! let handle = pool.submit(|| 6 * 7);
//! assert_eq!(handle.join().unwrap(), 42);
//! pool.shutdown();
//! ```
//!
//! 设计上只做了三件实事：任务是 `Box<dyn FnOnce() + Send + 'static>`（见
//! `pool.rs` 的 `Job`）、队列用 `std::sync::mpsc` 加上一把只护着「取任务」
//! 的锁、关闭靠「发送端全部消失」。`unsafe`、外部依赖、`async` 一律没有。

mod handle;
mod pool;

pub use handle::{TaskHandle, Timeout};
pub use pool::ThreadPool;
