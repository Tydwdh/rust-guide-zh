// ManuallyDrop：显式管理某个字段的析构时机。
use std::mem::ManuallyDrop;

struct Log(&'static str);
impl Drop for Log {
    fn drop(&mut self) {
        println!("drop Log({})", self.0);
    }
}

pub struct Session {
    log: ManuallyDrop<Log>,
    closed: bool,
}

impl Session {
    pub fn new(name: &'static str) -> Self {
        Self {
            log: ManuallyDrop::new(Log(name)),
            closed: false,
        }
    }

    /// 提前收尾：手动析构日志，并把「已收尾」记下来。
    pub fn close(&mut self) {
        if !self.closed {
            // SAFETY: closed 标志保证这里对同一个值是恰好一次析构。
            unsafe { ManuallyDrop::drop(&mut self.log) };
            self.closed = true;
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // 正常走完也走这里，保证日志最终一定会被析构。
        self.close();
    }
}

fn main() {
    let mut s = Session::new("build");
    s.close();
    println!("close 之后 Session 还活着");

    let s2 = Session::new("test");
    println!("s2 一直没 close，将在作用域结束时析构");
    drop(s2);

    // 不做手工析构时，into_inner 是安全出口。
    let inner = ManuallyDrop::into_inner(ManuallyDrop::new(Log("plain")));
    drop(inner);
}
