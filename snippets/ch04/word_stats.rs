//! 统计一行文本里的「词」：每个词的起始字节偏移，以及它在原文里的切片。
//!
//! 运行：
//!   rustc --edition 2024 word_stats.rs -o word_stats
//!   ./word_stats

fn main() {
    let line = String::from("Rust 让 借用 变成 编译期检查");
    for (start, word) in words(&line) {
        // word 是 line 里的一段视图：不复制任何字节
        println!("字节 {start:>2}..{:<2}  {word}", start + word.len());
    }
    // words() 只借用了 line，所以 line 在这里仍然可用
    println!("整行 = {line}");
}

/// 按空格切词，返回「起始字节偏移 + 切片」。
/// 返回的 `&str` 借用自参数 `s`：只要 `s` 还活着，这些切片就有效。
fn words(s: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if c == ' ' {
            if let Some(begin) = start.take() {
                out.push((begin, &s[begin..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(begin) = start {
        out.push((begin, &s[begin..]));
    }
    out
}
