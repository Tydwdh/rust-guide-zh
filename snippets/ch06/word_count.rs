use std::collections::HashMap;

fn main() {
    // 同时含英文、中文和 emoji 的一小段文本
    let text = "the quick brown fox 你好 世界 你好 Rust 🦀 Rust";

    // 1) 字节数 vs 字符数：len() 数的是 UTF-8 字节
    println!("字节数 = {}", text.len());
    println!("字符数 = {}", text.chars().count());

    // 2) 词频：entry API 把「没有就插入 0，然后加一」写成一步
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    println!("不同词数 = {}", counts.len());

    // 3) 想按次数排序，就先把 HashMap 收进 Vec（顺序是确定的，因为我们排了序）
    let mut pairs: Vec<(&str, u32)> = counts.iter().map(|(w, n)| (*w, *n)).collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (word, n) in pairs.iter().take(3) {
        println!("{word}: {n}");
    }

    // 4) 逐字符遍历是 UTF-8 安全的：切在 char 边界上
    let longest = text.split_whitespace().max_by_key(|w| w.chars().count());
    println!("最长词（按字符数）= {}", longest.unwrap_or(""));
}
