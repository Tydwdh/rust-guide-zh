//! 句柄：[`TaskHandle`] 与它的等待结果。

use std::error::Error;
use std::fmt;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// `join_timeout` 等满了给定时长，任务还没结束。
///
/// 超时**不是取消**：任务照旧跑完，只是结果没人接了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeout {
    /// 本次等待的请求时长（真正返回时可能略晚一点）。
    pub waited: Duration,
}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "（至多）等了 {:?}，任务还没结束", self.waited)
    }
}

impl Error for Timeout {}

/// 一次 [`ThreadPool::submit`](crate::ThreadPool::submit) 的句柄。
///
/// `join` 的返回类型与 `std::thread::JoinHandle::join` 一致：`Ok` 是任务的
/// 返回值，`Err` 是任务 panic 时的负载（可以 `downcast` 回 panic 时给出的
/// 类型）。句柄本身可以搬到别的线程再等 —— 结果走的是它自己的通道，与
/// 哪个线程在等无关。
pub struct TaskHandle<T> {
    rx: Receiver<thread::Result<T>>,
}

impl<T> TaskHandle<T> {
    pub(crate) fn new(rx: Receiver<thread::Result<T>>) -> Self {
        TaskHandle { rx }
    }

    /// 一直等到任务结束。
    pub fn join(self) -> thread::Result<T> {
        match self.rx.recv() {
            Ok(outcome) => outcome,
            Err(_) => lost_result(),
        }
    }

    /// 最多等 `timeout`；等不到就返回 [`Timeout`]，任务继续跑。
    pub fn join_timeout(self, timeout: Duration) -> Result<thread::Result<T>, Timeout> {
        match self.rx.recv_timeout(timeout) {
            Ok(outcome) => Ok(outcome),
            Err(RecvTimeoutError::Timeout) => Err(Timeout { waited: timeout }),
            Err(RecvTimeoutError::Disconnected) => lost_result(),
        }
    }
}

/// 结果端不可能先于结果消失：任务在跑之前不会被丢弃（池会把排队的活跑完），
/// 而跑完的任务一定恰好发过一次结果。真走到这里说明池的不变量被破坏了。
fn lost_result() -> ! {
    unreachable!("任务的结果端在发结果之前就消失了（池的不变量被破坏）")
}
