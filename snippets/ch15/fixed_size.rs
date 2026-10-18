// 行数、列数进入类型：转置后的类型由编译器算出
fn transpose<const R: usize, const C: usize>(m: [[i32; C]; R]) -> [[i32; R]; C] {
    std::array::from_fn(|i| std::array::from_fn(|j| m[j][i]))
}

// 长度不匹配的两个数组无法调用：`N` 是同一个参数
fn dot<const N: usize>(a: [i32; N], b: [i32; N]) -> i32 {
    const { assert!(N > 0, "空数组没有点积") };
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn main() {
    let m = [[1, 2, 3], [4, 5, 6]];
    println!("转置 = {:?}", transpose(m));
    println!("点积 = {}", dot([1, 2, 3], [4, 5, 6]));
}
