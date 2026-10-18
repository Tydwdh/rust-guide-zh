// 关联类型自己接收生命周期参数：每次借 `self` 得到一个新类型
trait LendingIterator {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

// 按行切分一个自己持有的缓冲区；每行都是对内部 buffer 的借用
struct Lines {
    buf: String,
    pos: usize,
}

impl Lines {
    fn new(text: &str) -> Self {
        Self { buf: text.to_owned(), pos: 0 }
    }
}

impl LendingIterator for Lines {
    type Item<'a> = &'a str where Self: 'a;

    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>> {
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        match self.buf[start..].find('\n') {
            Some(off) => {
                self.pos = start + off + 1;
                Some(&self.buf[start..start + off])
            }
            None => {
                self.pos = self.buf.len();
                Some(&self.buf[start..])
            }
        }
    }
}

// 消费方：bound 里用 HRTB 说「任何生命周期借出来的 Item 都能当 &str 用」
fn longest<I>(it: &mut I) -> Option<usize>
where
    I: LendingIterator,
    for<'a> I::Item<'a>: AsRef<str>,
{
    let mut best: Option<usize> = None;
    while let Some(line) = it.next() {
        let n = line.as_ref().chars().count();
        if best.is_none_or(|b| n > b) {
            best = Some(n);
        }
    }
    best
}

fn main() {
    let mut lines = Lines::new("alpha\nbe\ngamma\n");
    while let Some(line) = lines.next() {
        println!("[{line}]");
    }

    let mut lines = Lines::new("alpha\nbe\ngamma\n");
    println!("最长一行 {} 个字符", longest(&mut lines).unwrap());
}
