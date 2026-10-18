//! `FILETIME`：Windows 的 64 位时间表示，以及它到 Unix 毫秒的换算。
//!
//! 这个文件里只有一处 `unsafe` —— 从裸指针里读值的那个构造函数；其余全是普通算术。

use crate::error::{ProbeError, Result};

/// 自 1601-01-01 00:00:00 UTC 起、以 100 纳秒为一格的计数。
///
/// `#[repr(C)]` + 两个 `u32` 是 Windows 头文件里 `FILETIME` 的布局，
/// 所以这个类型可以直接出现在 FFI 签名里（`*mut FileTime` 就是 `LPFILETIME`）。
///
/// 字段名不是 C 的名字（`dwLowDateTime` / `dwHighDateTime`）：`repr(C)` 的布局
/// 按声明顺序排，字段名只在 Rust 这边有意义，取短名是为了不违反命名习惯。
///
/// ```
/// use sys_probe::FileTime;
///
/// // Unix 纪元（1970-01-01 UTC）恰好是零：
/// let epoch = FileTime::from_ticks(FileTime::EPOCH_OFFSET_TICKS);
/// assert_eq!(epoch.unix_millis().unwrap(), 0);
///
/// // 1601-01-01 早于 Unix 纪元，换不出来：
/// assert!(FileTime::from_ticks(0).unix_millis().is_err());
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTime {
    /// 低 32 位（C 里的 `dwLowDateTime`）。
    low: u32,
    /// 高 32 位（C 里的 `dwHighDateTime`）。
    high: u32,
}

impl FileTime {
    /// 1601-01-01 到 1970-01-01 之间的 100 纳秒格数。
    ///
    /// 11644473600 秒 × 10 000 000 格/秒。
    pub const EPOCH_OFFSET_TICKS: u64 = 116_444_736_000_000_000;

    /// 一毫秒有多少格（1 ms = 1 000 000 ns = 10 000 × 100 ns）。
    pub const TICKS_PER_MILLI: u64 = 10_000;

    /// 用一个 64 位计数构造：低位在前，和 `FILETIME` 的内存布局一致。
    ///
    /// ```
    /// use sys_probe::FileTime;
    ///
    /// let ft = FileTime::from_ticks(0x0000_0002_0000_0001);
    /// assert_eq!(ft.ticks(), 0x0000_0002_0000_0001);
    /// ```
    pub const fn from_ticks(ticks: u64) -> Self {
        Self {
            low: ticks as u32,
            high: (ticks >> 32) as u32,
        }
    }

    /// 把两个 32 位字段拼回 64 位计数。
    pub const fn ticks(self) -> u64 {
        ((self.high as u64) << 32) | self.low as u64
    }

    /// 换算成「自 Unix 纪元起的毫秒数」。
    ///
    /// 1970 年以前的值没有 `u64` 毫秒可以表示，所以这里返回
    /// [`ProbeError::BeforeUnixEpoch`] 而不是悄悄回绕。
    ///
    /// ```
    /// use sys_probe::FileTime;
    ///
    /// // 纪元之后一秒：±1 s 就是 10 000 000 格
    /// let one_second_in = FileTime::from_ticks(FileTime::EPOCH_OFFSET_TICKS + 10_000_000);
    /// assert_eq!(one_second_in.unix_millis().unwrap(), 1_000);
    /// ```
    pub fn unix_millis(self) -> Result<u64> {
        let ticks = self.ticks();
        ticks
            .checked_sub(Self::EPOCH_OFFSET_TICKS)
            .map(|since_epoch| since_epoch / Self::TICKS_PER_MILLI)
            .ok_or(ProbeError::BeforeUnixEpoch { ticks })
    }

    /// 从 C 交给我们的指针里读一份 `FileTime`。
    ///
    /// Windows 的很多结构体里嵌着 `FILETIME`（比如 `WIN32_FIND_DATA`），
    /// C 侧回调常常把「指向它的指针」递过来。指针能不能读，只有调用方知道，
    /// 所以这个构造函数只能是 `unsafe`：库里没有任何办法替你检查。
    ///
    /// # Safety
    ///
    /// - `ptr` 必须指向一个**已初始化**、**按 4 字节对齐**的 `FileTime`；
    /// - 读取期间，那块内存不得被别的线程改写（否则读到的是撕裂值）。
    ///
    /// ```
    /// use sys_probe::FileTime;
    ///
    /// let ft = FileTime::from_ticks(42);
    /// // SAFETY: `ft` 就在栈上、已初始化、没有被别人借用。
    /// let copied = unsafe { FileTime::read_raw(&ft) };
    /// assert_eq!(copied, ft);
    /// ```
    pub unsafe fn read_raw(ptr: *const FileTime) -> FileTime {
        // SAFETY: 上面 `# Safety` 的两条由调用方保证。
        unsafe { ptr.read() }
    }
}

// 布局断言：这是「和 Windows 对得上」的合同，编译期就该成立。
// 任何一条不成立，这个 crate 都编不过 —— 这正是我们想要的效果。
const _: () = {
    use std::mem::{align_of, offset_of, size_of};

    assert!(size_of::<FileTime>() == 8);
    assert!(align_of::<FileTime>() == 4);
    assert!(offset_of!(FileTime, low) == 0);
    assert!(offset_of!(FileTime, high) == 4);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_round_trip_through_ticks() {
        for ticks in [0, 1, 0x0000_0001_FFFF_FFFF, u64::MAX] {
            assert_eq!(FileTime::from_ticks(ticks).ticks(), ticks);
        }
    }

    #[test]
    fn unix_epoch_is_zero_milliseconds() {
        let epoch = FileTime::from_ticks(FileTime::EPOCH_OFFSET_TICKS);
        assert_eq!(epoch.unix_millis().unwrap(), 0);
    }

    #[test]
    fn sub_millisecond_remainder_is_truncated() {
        let ft = FileTime::from_ticks(FileTime::EPOCH_OFFSET_TICKS + 9_999);
        assert_eq!(ft.unix_millis().unwrap(), 0, "不到 1 ms 应该被截掉");
    }

    #[test]
    fn before_the_epoch_is_an_error_not_a_wrap() {
        let ft = FileTime::from_ticks(FileTime::EPOCH_OFFSET_TICKS - 1);
        match ft.unix_millis() {
            Err(ProbeError::BeforeUnixEpoch { ticks }) => {
                assert_eq!(ticks, FileTime::EPOCH_OFFSET_TICKS - 1);
            }
            other => panic!("应该报 BeforeUnixEpoch，实际是 {other:?}"),
        }
    }

    #[test]
    fn read_raw_copies_the_value_behind_the_pointer() {
        let original = FileTime::from_ticks(0x0123_4567_89AB_CDEF);
        // SAFETY: `original` 已初始化、对齐，且在这次读取期间没有可变借用。
        let copy = unsafe { FileTime::read_raw(&original) };
        assert_eq!(copy, original);
    }
}
