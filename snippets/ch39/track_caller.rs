// #[track_caller] 的传播与丢失：本机实测（Windows x86-64、Rust 1.98）。
// 每个数字都是这份文件的行号，方便对照。
use std::panic::Location;

// 打了标记：能拿到「调用它的那行代码」的位置
#[track_caller]
fn located() -> &'static Location<'static> {
    Location::caller()
}

// 没打标记：往外走一层就停在这里，位置固定在函数体内调用 located() 的那一行
fn wrapper() -> &'static Location<'static> {
    located()
}

// 打了标记：继续往外传到它自己的调用点
#[track_caller]
fn outer() -> &'static Location<'static> {
    located()
}

// 泛型函数一样适用
#[track_caller]
fn generic<T: Default>() -> &'static Location<'static> {
    let _ = T::default();
    Location::caller()
}

struct S;
impl S {
    #[track_caller]
    fn method(&self) -> &'static Location<'static> {
        Location::caller()
    }
}

// trait **声明**上标注：对实现也生效，经 trait 对象调用仍保住调用点
trait Declared {
    #[track_caller]
    fn m(&self) -> &'static Location<'static>;
}
struct A;
impl Declared for A {
    // 实现上故意不写：按 Reference，声明上的属性对该实现生效
    fn m(&self) -> &'static Location<'static> {
        Location::caller()
    }
}

// 只在**实现**上标注：直接调用没问题，走 trait 对象（函数指针）时丢在定义处
trait Undeclared {
    fn m(&self) -> &'static Location<'static>;
}
struct B;
impl Undeclared for B {
    #[track_caller]
    fn m(&self) -> &'static Location<'static> {
        Location::caller()
    }
}

fn show(tag: &str, loc: &Location<'_>) {
    println!("{tag} → {}:{}", loc.file(), loc.line());
}

fn main() {
    show("located()", located());
    show("wrapper() 第一次", wrapper());
    show("wrapper() 第二次", wrapper());
    show("outer()", outer());
    show("generic::<u8>()", generic::<u8>());
    show("S.method()", S.method());

    let a: &dyn Declared = &A;
    show("&dyn Declared（声明已标注）", a.m());
    let b: &dyn Undeclared = &B;
    show("&dyn Undeclared（只标实现）", b.m());

    let fp: fn() -> &'static Location<'static> = located;
    show("函数指针调用", fp());

    // 标准库的 unwrap 也靠它：panic 位置指向调用点
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|info| {
        if let Some(loc) = info.location() {
            println!("panic 位置 → {}:{}", loc.file(), loc.line());
        }
    }));
    let outcome = std::panic::catch_unwind(|| {
        let _ = Err::<u8, &str>("boom").unwrap();
    });
    std::panic::set_hook(hook);
    println!("unwrap 的 panic 被捕获 = {}", outcome.is_err());
}
