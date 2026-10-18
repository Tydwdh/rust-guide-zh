// 每行一条 "名称,数量" 记录
const DATA: &str = "\
alpha,3
beta,12
gamma,7
delta,20
";

// 返回位置 impl Trait：调用者拿到一个迭代器，不必写出它的具体类型
fn rows(input: &str) -> impl Iterator<Item = (&str, u32)> + use<'_> {
    input.lines().filter_map(|line| {
        let (name, count) = line.split_once(',')?;
        Some((name, count.parse().ok()?))
    })
}

// 一个 RPIT 函数的返回值可以继续喂给适配器
fn big_rows(input: &str, min: u32) -> impl Iterator<Item = (&str, u32)> + use<'_> {
    rows(input).filter(move |(_, count)| *count >= min)
}

fn total(input: &str) -> u32 {
    rows(input).map(|(_, count)| count).sum()
}

fn main() {
    println!("总数：{}", total(DATA));
    let names: Vec<&str> = big_rows(DATA, 10).map(|(name, _)| name).collect();
    println!("≥10 的行：{names:?}");
    for (name, count) in rows(DATA) {
        println!("{name} => {count}");
    }
}
