//! 一个自引用类型：字段 `ptr` 指向自己的 `data`。
//!
//! 只有在值被 pin 住（地址不再变化）之后，`ptr` 才允许存在。

use std::marker::PhantomPinned;
use std::pin::{pin, Pin};

struct SelfRef {
    data: String,
    ptr: *const String,
    // 关掉自动推导出来的 `Unpin`：这个类型必须被 pin 住才安全。
    _pin: PhantomPinned,
}

impl SelfRef {
    /// 此时还谈不上自引用，`ptr` 是空的；值可以随便移动。
    fn new(data: &str) -> Self {
        Self {
            data: data.to_owned(),
            ptr: std::ptr::null(),
            _pin: PhantomPinned,
        }
    }

    /// 地址定下来之后再记录「自己在哪里」。
    fn init(self: Pin<&mut Self>) {
        // SAFETY: 只写字段，不把 self 移出去，也不把拿到的引用泄漏出去。
        let this = unsafe { self.get_unchecked_mut() };
        this.ptr = &this.data;
    }

    fn data(&self) -> &str {
        &self.data
    }

    /// `ptr` 是否仍然指向自己的 `data`。
    fn points_to_self(&self) -> bool {
        std::ptr::eq(self.ptr, &self.data)
    }
}

/// 堆上的自引用值可以跨作用域传递：返回的是指针，值本身不会跟着搬家。
fn spawn(data: &str) -> Pin<Box<SelfRef>> {
    let mut task = Box::pin(SelfRef::new(data));
    task.as_mut().init();
    task
}

fn main() {
    // 1. 就地 pin：值留在当前栈帧里，不分配。
    let value = SelfRef::new("stack");
    let mut pinned = pin!(value);
    pinned.as_mut().init();
    assert!(pinned.points_to_self());
    println!("pin!     {} -> {}", pinned.data(), pinned.points_to_self());

    // 2. 堆上 pin：`Pin<Box<..>>` 自己可以到处移动，堆上的值不动。
    let task = spawn("heap");
    let moved = task;
    assert!(moved.points_to_self());
    println!("Box::pin {} -> {}", moved.data(), moved.points_to_self());
}
