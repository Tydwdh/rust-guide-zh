//! `MiniVec` 的行为测试：增长台阶、Drain 守卫、ZST、drop 次数与顺序、布局。

use std::cell::RefCell;
use std::mem::size_of;
use std::rc::Rc;
use std::thread;

use ch34_snippets::mini_vec::MiniVec;

/// 记录自己被 drop 的时刻。
struct DropCounter {
    log: Rc<RefCell<Vec<u32>>>,
    id: u32,
}

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.log.borrow_mut().push(self.id);
    }
}

fn counter(log: &Rc<RefCell<Vec<u32>>>, id: u32) -> DropCounter {
    DropCounter {
        log: Rc::clone(log),
        id,
    }
}

#[test]
fn capacity_grows_by_doubling() {
    let mut v = MiniVec::new();
    let mut steps = Vec::new();
    for i in 0..64u32 {
        v.push(i);
        if steps.last() != Some(&v.capacity()) {
            steps.push(v.capacity());
        }
    }
    // 0 -> 1 -> 2 -> 4 -> 8 -> 16 -> 32 -> 64：每次满了就翻倍。
    assert_eq!(steps, [1, 2, 4, 8, 16, 32, 64]);
    assert_eq!(v.len(), 64);
    assert!(v.capacity() >= v.len());
}

#[test]
fn push_pop_is_lifo() {
    let mut v = MiniVec::new();
    assert_eq!(v.pop(), None);
    v.push(String::from("a"));
    v.push(String::from("b"));
    assert_eq!(v.len(), 2);
    assert_eq!(v.pop().as_deref(), Some("b"));
    assert_eq!(v.pop().as_deref(), Some("a"));
    assert_eq!(v.pop(), None);
    assert!(v.is_empty());
}

#[test]
fn insert_and_remove_keep_order() {
    let mut v = MiniVec::new();
    for i in 0..4 {
        v.push(i);
    }
    v.insert(0, 9);
    assert_eq!(v.as_slice(), [9, 0, 1, 2, 3]);
    v.insert(3, 8);
    assert_eq!(v.as_slice(), [9, 0, 1, 8, 2, 3]);
    assert_eq!(v.remove(0), 9);
    assert_eq!(v.remove(4), 3);
    assert_eq!(v.as_slice(), [0, 1, 8, 2]);
}

#[test]
fn drain_moves_elements_out_and_keeps_the_buffer() {
    let mut v = MiniVec::new();
    for i in 0..5 {
        v.push(i);
    }
    let cap = v.capacity();

    let taken: Vec<i32> = v.drain().collect();
    assert_eq!(taken, [0, 1, 2, 3, 4]);
    assert_eq!(v.len(), 0);
    assert_eq!(v.capacity(), cap, "drain 不动缓冲区");

    // 缓冲区还能接着用。
    v.push(42);
    assert_eq!(v.as_slice(), [42]);
    assert_eq!(v.capacity(), cap);
}

#[test]
fn drain_drop_cleans_up_what_was_left() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut v = MiniVec::new();
    for id in 0..5 {
        v.push(counter(&log, id));
    }

    {
        let mut d = v.drain();
        let first = d.next().expect("drain 里还有元素");
        assert_eq!(first.id, 0);
        drop(first);
        assert_eq!(*log.borrow(), [0], "取出来的元素当场被 drop");
    } // Drain 在这里 drop：剩下的 1..4 被守卫收尾

    assert_eq!(*log.borrow(), [0, 1, 2, 3, 4]);
    assert_eq!(v.len(), 0);
}

#[test]
fn forgetting_the_drain_leaks_instead_of_double_dropping() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut v = MiniVec::new();
    for id in 0..3 {
        v.push(counter(&log, id));
    }

    let d = v.drain();
    std::mem::forget(d);

    // 泄漏（可以接受）：元素一次都没被 drop，但它们绝不会被 drop 第二次。
    assert!(log.borrow().is_empty());
    assert_eq!(v.len(), 0);
    drop(v);
    assert!(log.borrow().is_empty());
}

#[test]
fn drop_drops_every_element_exactly_once_in_order() {
    let log = Rc::new(RefCell::new(Vec::new()));
    {
        let mut v = MiniVec::new();
        for id in 0..7 {
            v.push(counter(&log, id));
            // 中途扩容过，drop 时要走的是扩容后的缓冲区。
        }
    }
    assert_eq!(*log.borrow(), [0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn zst_capacity_is_usize_max_and_pushing_never_allocates() {
    let mut v: MiniVec<()> = MiniVec::new();
    assert_eq!(v.capacity(), usize::MAX);

    for _ in 0..10_000 {
        v.push(());
    }
    assert_eq!(v.len(), 10_000);
    assert_eq!(v.capacity(), usize::MAX);

    let taken: Vec<()> = v.drain().collect();
    assert_eq!(taken.len(), 10_000);
    assert_eq!(v.len(), 0);
}

#[test]
fn zst_reads_through_deref_work() {
    let mut v: MiniVec<()> = MiniVec::new();
    v.push(());
    v.push(());
    assert_eq!(v.as_slice().len(), 2);
    assert_eq!(v.iter().count(), 2);
    assert_eq!(v[1], ());
}

#[test]
fn layout_matches_the_same_three_words_as_std() {
    assert_eq!(size_of::<MiniVec<u8>>(), 3 * size_of::<usize>());
    assert_eq!(size_of::<Option<MiniVec<u8>>>(), size_of::<MiniVec<u8>>());
    assert_eq!(size_of::<MiniVec<()>>(), 3 * size_of::<usize>());
}

#[test]
fn moves_across_threads_and_is_shareable_behind_an_arc() {
    let mut v = MiniVec::new();
    v.push(String::from("from main"));
    let handle = thread::spawn(move || v.len());
    assert_eq!(handle.join().unwrap(), 1);

    let mut shared = MiniVec::new();
    for i in 0..4 {
        shared.push(i);
    }
    let shared = std::sync::Arc::new(shared);
    let handles: Vec<_> = (0..3)
        .map(|_| {
            let mine = std::sync::Arc::clone(&shared);
            thread::spawn(move || mine.iter().sum::<i32>())
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), 6);
    }
}
