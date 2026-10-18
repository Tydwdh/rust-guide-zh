use std::fmt;

// Vec<String> 是外部类型，Display 是外部 trait，
// 直接 `impl fmt::Display for Vec<String>` 违反孤儿规则。
// newtype：包一层本地类型，就有了「本地类型」的位置。
struct WordList(Vec<String>);

impl fmt::Display for WordList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} 个词：{}", self.0.len(), self.0.join(" / "))
    }
}

fn main() {
    let words = WordList(vec![
        String::from("alpha"),
        String::from("beta"),
        String::from("gamma"),
    ]);
    println!("{words}");
}
