use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

/// 把文本切成 `workers` 片并行统计词频，每片一个线程，结果用 channel 汇总。
fn count_words(text: &str, workers: usize) -> Vec<(String, usize)> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let (tx, rx) = mpsc::channel::<HashMap<&str, usize>>();

    thread::scope(|s| {
        // max(1)：避免空输入或 workers 大于词数时出现 chunks(0)（会 panic）
        let per_chunk = words.len().div_ceil(workers).max(1);
        for chunk in words.chunks(per_chunk) {
            let tx = tx.clone(); // 每个线程拿一份发送端
            s.spawn(move || {
                let mut local = HashMap::new();
                for w in chunk {
                    *local.entry(*w).or_insert(0) += 1;
                }
                tx.send(local).unwrap();
            });
        }
        drop(tx); // 主线程不再发送，否则 rx 永远等不到「全部挂断」
    });
    // scope 返回时所有分片线程都已 join，发送端也全部消失

    let mut total: HashMap<&str, usize> = HashMap::new();
    for partial in rx.iter() {
        for (word, n) in partial {
            *total.entry(word).or_insert(0) += n;
        }
    }

    let mut counts: Vec<(String, usize)> =
        total.into_iter().map(|(w, n)| (w.to_owned(), n)).collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}

fn main() {
    let text = "the quick brown fox jumps over the lazy dog the fox";
    for (word, n) in count_words(text, 3) {
        println!("{word:<6} {n}");
    }
}
