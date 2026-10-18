// 第 17 章那个 List：同一份代码，只把载荷从 i32 换成 u64，
// 编译器就换了另一套布局策略。本机实测（Windows x86-64、Rust 1.98）。
use std::mem::size_of;

enum List {
    Cons(i32, Box<List>),
    Nil,
}

enum BigList {
    Cons(u64, Box<BigList>),
    Nil,
}

// 取字段地址做差得到偏移（只算地址，不读写内存）。
fn off<T, U>(base: &T, field: &U) -> usize {
    (field as *const U as usize) - (base as *const T as usize)
}

fn main() {
    let list = List::Cons(1, Box::new(List::Nil));
    if let List::Cons(v, rest) = &list {
        println!(
            "List     size={} align={}   i32 偏移={}  Box 偏移={}",
            size_of::<List>(),
            std::mem::align_of::<List>(),
            off(&list, v),
            off(&list, rest)
        );
    }
    println!("Option<List>     size={}", size_of::<Option<List>>());

    let big = BigList::Cons(1, Box::new(BigList::Nil));
    if let BigList::Cons(v, rest) = &big {
        println!(
            "BigList  size={} align={}   u64 偏移={}  Box 偏移={}",
            size_of::<BigList>(),
            std::mem::align_of::<BigList>(),
            off(&big, v),
            off(&big, rest)
        );
    }
    println!("Option<BigList>  size={}", size_of::<Option<BigList>>());
}
