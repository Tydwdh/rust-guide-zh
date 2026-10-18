//! 取消一个 future = 把它 drop 掉：等待点上的局部变量会随状态机一起析构。
use std::future::pending;
use std::task::{Context, Waker};

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("{} 被 drop", self.0);
    }
}

fn main() {
    let mut cx = Context::from_waker(Waker::noop());

    let mut fut = Box::pin(async {
        let _loud = Loud("任务 A 的局部变量");
        println!("任务 A 开始，停在等待点");
        pending::<()>().await;
        println!("永远不会打印");
    });

    // 驱动一次：跑到 .await，返回 Pending
    assert!(fut.as_mut().poll(&mut cx).is_pending());

    // 取消：丢掉这个停在等待点上的 future
    println!("drop future");
    drop(fut);
    println!("main 结束");
}
