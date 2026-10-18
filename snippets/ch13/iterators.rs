// 只借一个「未知实现」的迭代器：不知道具体类型，只知道 Item。
fn sum_all(items: &mut dyn Iterator<Item = u32>) -> u32 {
    items.sum()
}

fn take_n(items: &mut dyn Iterator<Item = u32>, n: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        match items.next() {
            Some(v) => out.push(v),
            None => break,
        }
    }
    out
}

// 返回位置也想按条件给出不同迭代器：用 Box<dyn Iterator> 抹掉类型。
fn evens_up_to(limit: u32) -> Box<dyn Iterator<Item = u32>> {
    Box::new((1..=limit).filter(|n| n % 2 == 0))
}

fn main() {
    println!("sum_all(&mut (1..=4)) = {}", sum_all(&mut (1..=4)));

    let v = vec![10u32, 20, 30, 40];
    println!(
        "sum_all(&mut v.iter().copied()) = {}",
        sum_all(&mut v.iter().copied())
    );

    println!("take_n(&mut (100..), 3) = {:?}", take_n(&mut (100..), 3));
    println!("evens_up_to(10) = {:?}", evens_up_to(10).collect::<Vec<_>>());
}
