//! 改编自 Miri 仓库的测试 `tests/pass/tree_borrows/sb_fails.rs`（写书时主分支）。
//! 下面五段都**通不过 Stacked Borrows**，但**通过 Tree Borrows**。
//! 它们在本机 stable 1.98 上照常编译、照常跑完：稳定的编译器不做别名模型检查。
//! 与上游的唯一区别：`main` 里的顺序调过（上游把 `fnentry_invalidation` 放在最前）。

mod fnentry_invalidation {
    pub fn main() {
        let mut x = 0i32;
        let z = &mut x as *mut i32;
        x.do_bad(); // 可变借用被传进函数 → FnEntry retag（带保护符）
        unsafe {
            let _oof = *z; // SB：z 已被那次 retag 失效；TB：那次重借并不禁止读 z
        }
    }

    trait Bad {
        fn do_bad(&mut self) {}
    }

    impl Bad for i32 {}
}

mod pass_invalid_mut {
    fn foo(_: &mut i32) {}

    pub fn main() {
        let x = &mut 42;
        let xraw = x as *mut _;
        let xref = unsafe { &mut *xraw };
        let _val = unsafe { *xraw }; // SB：这次读让 xref 失效
        foo(xref); // SB：到这里就是 UB；TB：xref 还是 Reserved，仍可写
    }
}

mod return_invalid_mut {
    fn foo(x: &mut (i32, i32)) -> &mut i32 {
        let xraw = x as *mut (i32, i32);
        let ret = unsafe { &mut (*xraw).1 };
        let _val = unsafe { *xraw }; // SB：这次读让 ret 失效
        ret // SB：返回处重借 → UB；TB：ret 是 Reserved，仍可写
    }

    pub fn main() {
        let _ = foo(&mut (1, 2));
    }
}

mod static_memory_modification {
    static X: usize = 5;

    #[allow(mutable_transmutes)] // 少这一行，stable 上连编译都过不去
    pub fn main() {
        let x = unsafe {
            std::mem::transmute::<&usize, &mut usize>(&X) // SB：造这个可变重借就算越线
            // TB：只要不写就允许
        };
        assert_eq!(*&*x, 5);
    }
}

#[allow(unused_assignments)] // 上游文件里也标了这个「假警告」
fn interior_mut_reborrow() {
    use std::cell::UnsafeCell;

    let mut c = UnsafeCell::new(42);
    let ptr = c.get(); // 先拿到内部可变的指针
    c = UnsafeCell::new(13); // 再写父位置
    assert_eq!(unsafe { ptr.read() }, 13); // 然后通过先前的指针读
}

fn main() {
    pass_invalid_mut::main();
    fnentry_invalidation::main();
    return_invalid_mut::main();
    static_memory_modification::main();
    interior_mut_reborrow();
}
