// 递归宏：本机实测（Windows x86-64、Rust 1.98）。
// 1) 内部规则（`@` 前缀）+ TT muncher 计数
macro_rules! count {
    (@acc $n:expr;) => {
        $n
    };
    (@acc $n:expr; $head:ident $($rest:ident)*) => {
        count!(@acc $n + 1; $($rest)*)
    };
    ($($id:ident)*) => {
        count!(@acc 0; $($id)*)
    };
}

// 2) 递归展开成嵌套表达式
macro_rules! sum {
    () => {
        0
    };
    ($head:expr $(, $rest:expr)*) => {
        $head + sum!($($rest),*)
    };
}

// 3) 嵌套重复的转置（Reference 的例子）
macro_rules! transpose {
    ( $( $i:ident ),* ; $( $j:ident ),* ) => {
        ( $( ($i, $j) ),* )
    };
}

fn main() {
    // 常量上下文里也能用：展开在编译期完成
    const N: usize = count!(a b c d e);
    println!("count! 得到 {N}");
    println!("sum!(1, 2, 3, 4) = {}", sum!(1, 2, 3, 4));

    let (a, b, c, d) = (1, 2, 3, 4);
    println!("transpose!(a, b; c, d) = {:?}", transpose!(a, b; c, d));
}
