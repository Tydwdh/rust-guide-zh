//! 第 23 章「动手看看」的完整程序：把 unsafe 关在安全 API 后面。
//!
//! 分三块：
//! 1. `unchecked` 模块：一个 `unsafe fn` 原语 + 一个做掉边界检查的安全外壳；
//! 2. `Even`：一个 `unsafe trait`，实现方承诺「一定返回偶数」；
//! 3. `half_of_even`：安全函数，靠上面的承诺去调用 `unreachable_unchecked`。

mod unchecked {
    // 2024 起这条 lint 默认就是 warn；这里提成 error，让「忘了写 unsafe 块」直接编译不过。
    #![deny(unsafe_op_in_unsafe_fn)]

    /// 取第 `i` 个元素，不做边界检查。
    ///
    /// # Safety
    ///
    /// 调用者必须保证 `i < x.len()`。
    pub unsafe fn get_i32_unchecked(x: &[i32], i: usize) -> i32 {
        // SAFETY: 调用者已保证 i 在界内，这是本函数唯一的前提。
        unsafe { *x.get_unchecked(i) }
    }

    /// 安全外壳：先把「界内」检查掉，再把剩下的交给上面的原语。
    pub fn get_or(x: &[i32], i: usize, fallback: i32) -> i32 {
        if i < x.len() {
            // SAFETY: 上面的判断保证了 i < x.len()。
            unsafe { get_i32_unchecked(x, i) }
        } else {
            fallback
        }
    }
}

/// 安全承诺：`value` 必须返回偶数。
///
/// # Safety
///
/// 实现者必须保证 `value` 的返回值能被 2 整除。
unsafe trait Even {
    fn value(&self) -> i32;
}

// SAFETY: `<< 1` 的结果一定能被 2 整除。
unsafe impl Even for i32 {
    fn value(&self) -> i32 {
        self << 1
    }
}

/// 安全函数：它敢用 `unreachable_unchecked`，因为 `Even` 的契约替它兜底。
fn half_of_even<T: Even>(x: T) -> i32 {
    if x.value() % 2 == 0 {
        x.value() / 2
    } else {
        // SAFETY: 实现 `Even` 的类型必须保证 value() 是偶数，这个分支不可达。
        unsafe { std::hint::unreachable_unchecked() }
    }
}

fn main() {
    let v = [1, 2, 3];
    println!("get_or(1) = {}", unchecked::get_or(&v, 1, -1));
    println!("get_or(9) = {}", unchecked::get_or(&v, 9, -1));

    let x = 21i32;
    println!("value = {}, half = {}", x.value(), half_of_even(x));
}
