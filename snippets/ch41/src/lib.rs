// 第 41 章的 no_std 示例：只用 core，能在 thumbv7m-none-eabi 上编过。
// 宿主上 `cargo test`，目标上 `cargo build --target thumbv7m-none-eabi`。
#![no_std]

/// 定长环形缓冲：保存最近 `N` 次采样。
///
/// 长度写在类型里（const 泛型，第 15 章），整块存储就地放在栈上或 `static` 里 ——
/// 裸机目标上没有堆。
pub struct RingBuffer<const N: usize> {
    buf: [u16; N],
    head: usize,
    len: usize,
}

impl<const N: usize> RingBuffer<N> {
    /// 全零初始化；`const fn` 让它能直接当 `static` 的初值。
    pub const fn new() -> Self {
        Self { buf: [0; N], head: 0, len: 0 }
    }

    /// 写入一个采样；缓冲已满时返回被覆盖的旧值。
    pub fn push(&mut self, sample: u16) -> Option<u16> {
        let overwritten = if self.len == N { Some(self.buf[self.head]) } else { None };
        self.buf[self.head] = sample;
        self.head = (self.head + 1) % N;
        if self.len < N {
            self.len += 1;
        }
        overwritten
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 按写入顺序遍历。返回 `impl Iterator` 而不是具体类型（第 14 章）。
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        let start = (self.head + N - self.len) % N;
        (0..self.len).map(move |i| self.buf[(start + i) % N])
    }

    /// 整数平均：不碰浮点，累加时先升到 `u32` 防溢出。
    pub fn mean(&self) -> Option<u32> {
        if self.len == 0 {
            return None;
        }
        let sum: u32 = self.iter().map(u32::from).sum();
        Some(sum / self.len as u32)
    }
}

/// ADC 原始读数换算成毫伏：`raw * v_ref_mv / (2^bits - 1)`。
///
/// 全程整数：没有硬件浮点的核上，一个 `f32` 除法会拖进整套软浮点例程。
pub const fn millivolts(raw: u16, v_ref_mv: u16, bits: u8) -> u16 {
    let full_scale = if bits == 0 { 1 } else { (1u32 << bits) - 1 };
    ((raw as u32 * v_ref_mv as u32) / full_scale) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_keeps_last_n() {
        let mut ring = RingBuffer::<3>::new();
        assert_eq!(ring.push(1), None);
        assert_eq!(ring.push(2), None);
        assert_eq!(ring.push(3), None);
        assert_eq!(ring.push(4), Some(1));
        assert!(ring.iter().eq([2, 3, 4]));
        assert_eq!(ring.len(), 3);
    }

    #[test]
    fn mean_is_integer() {
        let mut ring = RingBuffer::<4>::new();
        for sample in [10, 20, 31] {
            ring.push(sample);
        }
        assert_eq!(ring.mean(), Some(20));
        assert!(RingBuffer::<4>::new().is_empty());
        assert_eq!(RingBuffer::<4>::new().mean(), None);
    }

    #[test]
    fn millivolts_covers_full_range() {
        assert_eq!(millivolts(0, 3300, 12), 0);
        assert_eq!(millivolts(2048, 3300, 12), 1650);
        assert_eq!(millivolts(4095, 3300, 12), 3300);
    }
}
