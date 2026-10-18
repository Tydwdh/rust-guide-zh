// 同一份泛型代码，用五种类型各调一次：单态化会为每种类型各生成一份。
pub fn wrap<T: Copy>(x: T) -> (T, T) {
    (x, x)
}

fn main() {
    let a = wrap(1u32);
    let b = wrap(1u64);
    let c = wrap(1.5f64);
    let d = wrap('x');
    let e = wrap(true);
    println!("{} {} {} {} {}", a.0, b.0, c.0, d.0, e.0);
}
