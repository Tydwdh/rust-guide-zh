//! Rust 与 C 之间的字符串：`CString`（拥有）、`CStr`（借用）、`c"..."`（字面量）。

use std::ffi::{c_char, CStr, CString};

/// 「C 侧」风格的函数：只认裸指针，长度靠扫描 nul 得到。
unsafe fn c_style_len(s: *const c_char) -> usize {
    // SAFETY: 调用方保证 `s` 指向一块以 nul 结尾、且在调用期间不被改写的内存。
    unsafe { CStr::from_ptr(s) }.to_bytes().len()
}

fn main() {
    // 1. 拥有型：`CString` 保证尾部有 nul，且内部没有 nul。
    let owned = CString::new("hello").unwrap();
    println!("CString::new        -> {:?}", owned.as_c_str());

    // 2. 借用型：`c"..."` 直接给出 `&'static CStr`，不需要分配。
    let literal = c"hello";
    assert_eq!(owned.as_c_str(), literal);

    // 3. 把借用指针交出去，再从 C 的规则读回来。
    let len = unsafe { c_style_len(literal.as_ptr()) };
    println!("c_style_len(c\"hello\") -> {len}");

    // 4. 从 C 的缓冲区读：只知道「里面有 nul」，不知道在哪。
    let mut buffer = [0u8; 16];
    buffer[..5].copy_from_slice(b"bytes");
    let parsed = CStr::from_bytes_until_nul(&buffer).unwrap();
    println!("from_bytes_until_nul -> {:?}", parsed.to_str().unwrap());

    // 5. 内部 nul 会被 `CString::new` 拒收——C 字符串没地方安放它。
    assert!(CString::new("a\0b").is_err());
    println!("CString::new(\"a\\0b\")  是 Err");
}
