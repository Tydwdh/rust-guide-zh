// 喂给 --emit 的最小源码：一个孤立函数 + 一个带循环的调用方。
// `add` 特意写成 extern "C" + no_mangle：符号名不被修饰，
// 下面各种 dump 里能一眼认出它。
#[unsafe(no_mangle)]
pub extern "C" fn add(a: u32, b: u32) -> u32 {
    a + b
}

pub fn sum_slice(xs: &[u32]) -> u32 {
    let mut acc = 0;
    for x in xs {
        acc = add(acc, *x);
    }
    acc
}

fn main() {
    println!("{}", sum_slice(&[1, 2, 3]));
}
