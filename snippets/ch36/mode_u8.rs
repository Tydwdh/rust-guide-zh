// adt_const_params 的 stable 替代之一：把模式编码成 u8，用 inline const 做编译期断言
fn describe<const MODE: u8>() -> &'static str {
    const { assert!(MODE < 2, "MODE 只能是 0 或 1") };
    match MODE {
        0 => "fast",
        _ => "safe",
    }
}

fn main() {
    println!("{}", describe::<0>());
    println!("{}", describe::<1>());
}
