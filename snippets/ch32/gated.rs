/// 只在打开 `extra` feature 时编译的模块。
///
/// `cfg_attr(docsrs, ...)` 让 docs.rs 的页面标出「需要 extra」；
/// 普通构建里 `docsrs` 不成立，这行属性整条不生效。
#[cfg(feature = "extra")]
#[cfg_attr(docsrs, doc(cfg(feature = "extra")))]
pub mod gated {
    /// 门后的一把钥匙。
    pub fn key() -> u32 {
        42
    }
}
