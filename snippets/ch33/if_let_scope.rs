// `if let` 的临时值作用域：本机实测（Windows x86-64、Rust 1.98）。
use std::sync::RwLock;

fn main() {
    let value: RwLock<Option<bool>> = RwLock::new(None);

    if let Some(x) = *value.read().unwrap() {
        println!("then branch, x = {x}");
    } else {
        // `try_write` 不会阻塞：拿不到就立刻返回 Err。
        println!("write lock available = {}", value.try_write().is_ok());
        let mut v = value.write().unwrap();
        if v.is_none() {
            *v = Some(true);
        }
    }

    println!("final = {:?}", *value.read().unwrap());
}
