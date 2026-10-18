fn main() {
    // ① 绑定与遮蔽
    let x = 5;
    let x = x + 1;
    println!("遮蔽后 x = {x}");

    // ② 默认类型与 char
    println!(
        "42 默认是 {}，3.5 默认是 {}",
        std::any::type_name_of_val(&42),
        std::any::type_name_of_val(&3.5)
    );
    println!(
        "char 占 {} 字节，'中' 的标量值 = {}",
        std::mem::size_of::<char>(),
        '中' as u32
    );

    // ③ 元组与数组
    let pair = (1, "两个");
    let (a, b) = pair;
    println!("元组：{} 和 {}，解构后 {a} / {b}", pair.0, pair.1);
    let arr: [i32; 3] = [10, 20, 30];
    println!("数组长度 {}，第一个元素 {}", arr.len(), arr[0]);

    // ④ 表达式：if 有值
    let n = 7;
    let parity = if n % 2 == 0 { "偶" } else { "奇" };
    println!("{n} 是{parity}数");

    // ⑤ 函数：定义在调用之后也没问题
    println!("double(21) = {}", double(21));

    // ⑥ 循环
    let mut count = 0;
    let found = loop {
        count += 1;
        if count == 3 {
            break count * 10;
        }
    };
    println!("loop 返回 {found}");

    let mut i = 0;
    while i < 3 {
        i += 1;
    }
    println!("while 结束 i = {i}");

    let mut odd_sum = 0;
    for v in [1, 2, 3, 4] {
        if v % 2 == 0 {
            continue;
        }
        odd_sum += v;
    }
    println!("奇数之和 = {odd_sum}");

    // ⑦ 两种转换
    let big = 300_i32;
    println!("300 as u8 = {}", big as u8);
    let fits: i64 = 200;
    let small: u8 = fits.try_into().unwrap();
    println!("200_i64.try_into::<u8>() = {small}");
}

/// 返回 `n` 的两倍。
fn double(n: i32) -> i32 {
    n * 2
}
