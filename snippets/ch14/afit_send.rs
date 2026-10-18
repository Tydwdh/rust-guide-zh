// 编译期断言：T 必须是 Send，否则这里编译失败
fn assert_send<T: Send>(_: T) {}

trait Job {
    // 与 `async fn run(&self) -> u32` 等价，但把 future 的 Send 保证写进了契约
    fn run(&self) -> impl Future<Output = u32> + Send;
}

struct Add(u32, u32);

impl Job for Add {
    fn run(&self) -> impl Future<Output = u32> + Send {
        let (a, b) = (self.0, self.1);
        async move { a + b }
    }
}

// trait 上写了 Send，泛型代码就能放心依赖它
async fn describe<J: Job>(job: J) -> String {
    format!("结果：{}", job.run().await)
}

fn main() {
    assert_send(Add(1, 2).run());
    let _pending = describe(Add(2, 3)); // 泛型函数照常可用（这里不驱动它）
    println!("Add::run 的 future 满足 Send");
}
