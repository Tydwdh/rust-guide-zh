//! 一块「跑在外设上」的库：不带标准库，只用 `core`。
//!
//! 交叉编译时（`--target thumbv7m-none-eabi` 等）走 `#![no_std]` 那条路；
//! 在宿主机上跑 `cargo test` 时打开 std，这样测试框架还能用。
#![cfg_attr(not(test), no_std)]

/// 定长环形缓冲：入队满时丢掉最旧的那个字节。
pub struct RingBuffer<const N: usize> {
    buf: [u8; N],
    head: usize,
    len: usize,
}

impl<const N: usize> RingBuffer<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            head: 0,
            len: 0,
        }
    }

    /// 入队一个字节；缓冲已满则返回被挤出的那个字节。
    pub fn push(&mut self, byte: u8) -> Option<u8> {
        if self.len < N {
            let tail = (self.head + self.len) % N;
            self.buf[tail] = byte;
            self.len += 1;
            None
        } else {
            let evicted = self.buf[self.head];
            self.buf[self.head] = byte;
            self.head = (self.head + 1) % N;
            Some(evicted)
        }
    }

    /// 取出第 `index` 个（从最旧的算起）字节。
    pub fn get(&self, index: usize) -> Option<u8> {
        if index < self.len {
            Some(self.buf[(self.head + index) % N])
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Default for RingBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// 静态库是**最终产物**，链接期必须能找到唯一的 panic 处理器——`std` 平时替你提供，
/// 不带标准库时就得自己写。真实固件会在这里复位看门狗或点亮错误灯，这里只留一个自旋。
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// CRC-32（IEEE 802.3 的反射多项式 `0xEDB88320`）。
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            // 无分支写法：掩码要么全 1 要么全 0。
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut rb = RingBuffer::<3>::new();
        assert_eq!(rb.push(1), None);
        assert_eq!(rb.push(2), None);
        assert_eq!(rb.push(3), None);
        assert_eq!(rb.push(4), Some(1));
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.get(0), Some(2));
        assert_eq!(rb.get(2), Some(4));
        assert_eq!(rb.get(3), None);
    }
}
