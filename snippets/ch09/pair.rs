#[derive(Debug)]
struct Pair<T> {
    first: T,
    second: T,
}

// 第一段 impl：不要求 T 有任何能力，只要能把两个值装进来。
impl<T> Pair<T> {
    fn new(first: T, second: T) -> Self {
        Self { first, second }
    }
}

// 第二段 impl：只有能比较大小的 T 才有 larger。
impl<T: PartialOrd + std::fmt::Display> Pair<T> {
    fn larger(&self) -> &T {
        if self.first >= self.second {
            &self.first
        } else {
            &self.second
        }
    }

    fn describe_larger(&self) -> String {
        format!("{} 与 {} 中较大的是 {}", self.first, self.second, self.larger())
    }
}

fn main() {
    let numbers = Pair::new(3, 7);
    println!("{}", numbers.describe_larger());

    let words = Pair::new(String::from("apple"), String::from("pear"));
    println!("{}", words.describe_larger());
}
