// 尾表达式里的临时值：本机实测（Windows x86-64、Rust 1.98）。
struct Droppy(i32);

impl Droppy {
    fn get(&self) -> i32 {
        self.0
    }
}

impl Drop for Droppy {
    fn drop(&mut self) {
        println!("  drop {}", self.0);
    }
}

fn edition_dependent() -> i32 {
    // 局部变量：Type 有自定义析构。
    let another_droppy = Droppy(0);
    // 尾表达式：这里临时造出的 `Droppy(1)` 也有自定义析构。
    Droppy(1).get() + another_droppy.get()
}

fn main() {
    println!("begin");
    let n = edition_dependent();
    println!("  got {n}");
}
