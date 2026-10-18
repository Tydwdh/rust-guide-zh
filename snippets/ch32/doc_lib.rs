//! 迷你键值表：演示文档注释的三种位置，以及 intra-doc link 的写法。
//!
//! 这是 crate 级文档（内层 `//!`），会显示在文档首页。
//! 核心类型是 [`Table`]，最常用的两个操作是 [`Table::set`] 与 [`Table::get`]。

use std::collections::HashMap;

/// 一张字符串键值表。
///
/// 字段的文档要单独写，否则 `missing_docs` 会连字段一起报出来。
pub struct Table {
    /// 底层存储；`HashMap` 本身保证键唯一。
    data: HashMap<String, String>,
}

impl Table {
    /// 建一张空表。
    ///
    /// 与 [`Table::with_capacity`] 的差别：这里不做任何预分配。
    pub fn new() -> Table {
        Table {
            data: HashMap::new(),
        }
    }

    /// 建一张能装 `n` 个键、不立刻扩容的表。
    pub fn with_capacity(n: usize) -> Table {
        Table {
            data: HashMap::with_capacity(n),
        }
    }

    /// 从 `(键, 值)` 列表建表。
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Table {
        let mut t = Table::new();
        for (k, v) in pairs {
            t.set(k, v);
        }
        t
    }

    /// 插入或覆盖一个键，返回被覆盖的旧值。
    ///
    /// 读回用 [`Table::get`]。
    pub fn set(&mut self, key: &str, value: &str) -> Option<String> {
        self.data.insert(key.to_string(), value.to_string())
    }

    /// 取回一个键对应的值；键不存在时返回 `None`。
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(String::as_str)
    }
}
