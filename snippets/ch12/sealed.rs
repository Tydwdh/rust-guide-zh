mod lib {
    // 不是 pub：lib 模块之外看不见它
    mod private {
        pub trait Sealed {}
    }

    // 公开的 trait：外面可以拿它当边界，但不能自己实现
    pub trait Greet: private::Sealed {
        fn greet(&self) -> String;
    }

    pub struct English;

    impl private::Sealed for English {}

    impl Greet for English {
        fn greet(&self) -> String {
            String::from("hello")
        }
    }
}

// 方法来自 trait，用之前要把 trait 带进作用域
use lib::Greet;

fn main() {
    let e = lib::English;
    println!("{}", e.greet());
}
