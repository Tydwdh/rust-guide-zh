// 用 MaybeUninit + 一个 bool 标志做「最多初始化一次」的格子。
use std::mem::MaybeUninit;

/// 只在析构时打印，用来观察「值被析构了几次」。
struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop Noisy({})", self.0);
    }
}

pub struct Once<T> {
    value: MaybeUninit<T>,
    init: bool,
}

impl<T> Once<T> {
    pub const fn new() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            init: false,
        }
    }

    pub fn set(&mut self, value: T) {
        assert!(!self.init, "只能设置一次");
        self.value.write(value);
        self.init = true;
    }

    pub fn get(&self) -> Option<&T> {
        if self.init {
            // SAFETY: init 为真，说明上面写入过一次且没被取走。
            Some(unsafe { self.value.assume_init_ref() })
        } else {
            None
        }
    }

    pub fn take(&mut self) -> Option<T> {
        if self.init {
            self.init = false;
            // SAFETY: 取值前把 init 清零，避免同一份数据被读走两次。
            Some(unsafe { self.value.assume_init_read() })
        } else {
            None
        }
    }
}

impl<T> Drop for Once<T> {
    fn drop(&mut self) {
        if self.init {
            // SAFETY: init 为真，格子里的值是有效的，且此后再没人碰它。
            unsafe { self.value.assume_init_drop() };
        }
    }
}

fn main() {
    let mut cell: Once<Noisy> = Once::new();
    println!("还没放东西：{:?}", cell.get().map(|_| ()));

    cell.set(Noisy("first"));
    println!("放了之后能借到：{}", cell.get().is_some());

    let taken = cell.take();
    println!("取走之后：{:?}", cell.get().map(|_| ()));
    drop(taken);

    cell.set(Noisy("second"));
    println!("再放一次，析构 Once 时应当只 drop second");
}
