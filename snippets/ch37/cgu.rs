// 12 个小函数，用来看 codegen-units 怎么切。
pub fn f0(x: u64) -> u64 { x.wrapping_mul(3).rotate_left(0) }
pub fn f1(x: u64) -> u64 { x.wrapping_mul(4).rotate_left(1) }
pub fn f2(x: u64) -> u64 { x.wrapping_mul(5).rotate_left(2) }
pub fn f3(x: u64) -> u64 { x.wrapping_mul(6).rotate_left(3) }
pub fn f4(x: u64) -> u64 { x.wrapping_mul(7).rotate_left(4) }
pub fn f5(x: u64) -> u64 { x.wrapping_mul(8).rotate_left(5) }
pub fn f6(x: u64) -> u64 { x.wrapping_mul(9).rotate_left(6) }
pub fn f7(x: u64) -> u64 { x.wrapping_mul(10).rotate_left(7) }
pub fn f8(x: u64) -> u64 { x.wrapping_mul(11).rotate_left(8) }
pub fn f9(x: u64) -> u64 { x.wrapping_mul(12).rotate_left(9) }
pub fn f10(x: u64) -> u64 { x.wrapping_mul(13).rotate_left(10) }
pub fn f11(x: u64) -> u64 { x.wrapping_mul(14).rotate_left(11) }
fn main(){ let mut t=0u64; for i in 0..10 { t = t.wrapping_add(f0(i) + f1(i) + f2(i) + f3(i) + f4(i) + f5(i) + f6(i) + f7(i) + f8(i) + f9(i) + f10(i) + f11(i)); } println!("{t}"); }
