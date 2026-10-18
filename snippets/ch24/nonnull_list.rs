// 用 NonNull 表达「一定非空」的裸指针：一个最小的单链表。
use std::ptr::NonNull;

struct Node {
    value: i32,
    next: Option<NonNull<Node>>,
}

pub struct IntList {
    head: Option<NonNull<Node>>,
    len: usize,
}

impl IntList {
    pub fn new() -> Self {
        Self { head: None, len: 0 }
    }

    pub fn push_front(&mut self, value: i32) {
        let mut node = Box::new(Node { value, next: self.head });
        // 先取指针再交出 Box 的所有权；顺序反了就是悬垂指针。
        let ptr = NonNull::from_mut(&mut *node);
        let _ = Box::into_raw(node);
        self.head = Some(ptr);
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        let mut cur = self.head;
        std::iter::from_fn(move || {
            cur.map(|ptr| {
                // SAFETY: 节点由 Box 拥有，只要列表还活着就不会失效。
                let node = unsafe { ptr.as_ref() };
                cur = node.next;
                node.value
            })
        })
    }
}

impl Drop for IntList {
    fn drop(&mut self) {
        while let Some(ptr) = self.head {
            // SAFETY: 每个指针都由 Box::into_raw 产出，且只在这里回收一次。
            let node = unsafe { Box::from_raw(ptr.as_ptr()) };
            self.head = node.next;
        }
    }
}

fn main() {
    let mut list = IntList::new();
    for v in [3, 2, 1] {
        list.push_front(v);
    }
    println!("len = {}", list.len());
    println!("values = {:?}", list.iter().collect::<Vec<_>>());

    // niche 优化：NonNull 已知非空，所以 Option<NonNull<T>> 与裸指针同宽。
    println!(
        "Option<NonNull<Node>> = {} 字节",
        size_of::<Option<NonNull<Node>>>()
    );
    println!("Option<*mut Node>   = {} 字节", size_of::<Option<*mut Node>>());
}
