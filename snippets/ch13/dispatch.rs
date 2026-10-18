trait Op {
    fn apply(&self, x: u32) -> u32;
}

struct AddOne;

impl Op for AddOne {
    fn apply(&self, x: u32) -> u32 {
        x + 1
    }
}

struct TimesTwo;

impl Op for TimesTwo {
    fn apply(&self, x: u32) -> u32 {
        x * 2
    }
}

#[inline(never)]
fn run_dyn(op: &dyn Op, x: u32) -> u32 {
    op.apply(x)
}

#[inline(never)]
fn run_generic<O: Op>(op: &O, x: u32) -> u32 {
    op.apply(x)
}

fn main() {
    let a = AddOne;
    let t = TimesTwo;
    println!("{} {}", run_dyn(&a, 1), run_generic(&a, 1));
    println!("{} {}", run_dyn(&t, 1), run_generic(&t, 1));
}
