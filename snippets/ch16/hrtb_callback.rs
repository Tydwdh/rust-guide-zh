//! HRTB 实战：把「对任意生命周期的 `&str` 都可用」的回调存进结构体。
//!
//! 运行：
//!   rustc --edition 2024 hrtb_callback.rs -o hrtb_callback
//!   ./hrtb_callback

/// 给闭包补上「对任意生命周期都成立」的签名。
/// 没有这一层，闭包的签名会被推断成某个具体的生命周期。
fn annotate<F>(f: F) -> F
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    f
}

struct FirstWord<F> {
    f: F,
}

impl<F> FirstWord<F>
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    fn run<'s>(&self, s: &'s str) -> &'s str {
        (self.f)(s) // 返回值的生命周期跟着入参走
    }
}

/// 函数项天生是 higher-ranked 的：
/// `fn(&str) -> &str` 就是 `for<'a> fn(&'a str) -> &'a str`。
fn second_word(s: &str) -> &str {
    s.split_whitespace().nth(1).unwrap_or("")
}

fn main() {
    let owned = String::from("alpha beta");
    let pick_first = FirstWord {
        f: annotate(|s: &str| -> &str { s.split_whitespace().next().unwrap_or("") }),
    };
    println!("{}", pick_first.run(&owned));

    let pick_second = FirstWord { f: second_word };
    {
        let local = String::from("gamma delta");
        // 同一个实例换成一个更短命的字符串，照样能用
        println!("{}", pick_second.run(&local));
    }
}
