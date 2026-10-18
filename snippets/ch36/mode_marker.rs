// 用类型参数 + marker 类型代替枚举 const 参数（stable 写法）
trait Mode {
    const NAME: &'static str;
}

struct Fast;
struct Safe;

impl Mode for Fast {
    const NAME: &'static str = "fast";
}

impl Mode for Safe {
    const NAME: &'static str = "safe";
}

fn describe<M: Mode>() -> &'static str {
    M::NAME
}

fn main() {
    println!("{}", describe::<Fast>());
    println!("{}", describe::<Safe>());
}
