// 卫生性：本机实测（Windows x86-64、Rust 1.98）。
// 局部变量与标签按「宏定义处」解析，其余符号（函数、类型、生命周期）按「调用处」解析。
fn main() {
    let x = 1;
    fn func() -> &'static str {
        unreachable!("定义处的 func 不该被调用")
    }

    macro_rules! check {
        () => {{
            // 取定义处的 x（1），不是调用处遮蔽出来的 x
            assert_eq!(x, 1);
            // func 取调用处的版本
            func()
        }};
    }

    {
        let x = 2;
        fn func() -> &'static str {
            "调用处的 func"
        }
        println!("check!() = {}", check!());
        println!("调用块里的 x = {x}");
    }

    // 宏里引入的绑定与调用处的同名绑定互不干扰
    macro_rules! decl_x {
        () => {
            let x = 100;
            println!("宏里的 x = {x}");
        };
    }
    decl_x!();
    println!("调用处的 x = {x}");

    // 宏里引入的标签同样不会外泄
    macro_rules! inner_label {
        () => {
            'inner: loop {
                break 'inner;
            }
        };
    }
    inner_label!();
    println!("标签卫生：展开内部的 'inner 对外不可见");

    // 宏捕获的是定义处可见的那个绑定
    macro_rules! first_x {
        () => {
            x
        };
    }
    let x = 42;
    println!("first_x!() = {}，调用处的 x = {}", first_x!(), x);

    // 类型名与生命周期不在卫生范围内：按调用处解析
    macro_rules! make {
        () => {
            Box::new(MyType)
        };
    }
    struct MyType;

    macro_rules! passthrough {
        ($v:expr) => {{
            // 展开式里直接用调用处函数签名上的 'a
            let y: &'a u8 = $v;
            y
        }};
    }
    fn caller<'a>(v: &'a u8) -> &'a u8 {
        passthrough!(v)
    }

    let made = make!();
    println!("make!() 造出调用处的类型: {}", std::any::type_name_of_val(&*made));
    let n = 7u8;
    println!("passthrough 走了一遍调用处的 'a: {}", caller(&n));
}
