//! 析构顺序实验：局部变量按定义逆序析构，字段与元组元素按声明顺序析构。
//!
//! 运行：
//!   rustc --edition 2024 drop_order.rs -o drop_order
//!   ./drop_order

use std::cell::RefCell;

thread_local! {
    // 记录析构顺序，供最后断言
    static LOG: RefCell<Vec<&'static str>> = RefCell::new(Vec::new());
}

struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        LOG.with_borrow_mut(|log| log.push(self.0));
        println!("drop {}", self.0);
    }
}

struct Pair {
    first: D,
    second: D,
}

fn run() {
    let pair = Pair {
        first: D("结构体字段 first"),
        second: D("结构体字段 second"),
    };
    let tup = (D("元组 .0"), D("元组 .1"));
    let (x, y) = (D("模式绑定 x"), D("模式绑定 y")); // 书写顺序：x 先于 y
    let z = D("变量 z");

    {
        let _inner = D("内层作用域变量");
    } // 内层作用域结束：先析构

    // 读一遍各字段，避免 dead_code 警告
    let _ = (pair.first.0, pair.second.0, tup.0.0, x.0, y.0, z.0);
} // 这里按逆序析构 run 的局部变量

fn main() {
    run();
    let order = LOG.with_borrow(|log| log.join(" -> "));
    println!("总顺序：{order}");
    assert_eq!(
        order,
        "内层作用域变量 -> 变量 z -> 模式绑定 y -> 模式绑定 x -> 元组 .0 -> 元组 .1 \
         -> 结构体字段 first -> 结构体字段 second"
    );
}
