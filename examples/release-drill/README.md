# release-drill

把字节数格式化成人类可读的大小字符串，并把这种字符串解析回字节数。零依赖，只用标准库。

```rust
use release_drill::{format, parse};

assert_eq!(format(1536), "1.5 KiB");
assert_eq!(parse("1.5 KiB").unwrap(), 1536);
assert_eq!(parse("1.5MB").unwrap(), 1_500_000);
```

## 单位规则

| 方向 | 接受的单位 |
|---|---|
| `format` | 1024 进制：`B`、`KiB`、`MiB`、`GiB`、`TiB`、`PiB`、`EiB` |
| `parse` | 上面那一套，外加 1000 进制：`kB`、`MB`、`GB`、`TB`、`PB`、`EB` |

输出只有一种样子：写出去用 1024 进制，读进来两套都认。`kB` 是 1000 字节、`KiB` 是 1024 字节，两者不会互相换算。

## 用法

- `format(bytes: u64) -> String`：档位每级除以 1024；尾数小于 10 时保留一位小数（`1.5 KiB`），否则取整（`10 KiB`）。
- `parse(text: &str) -> Result<u64, ParseError>`：数字可带小数点，单位大小写不敏感、与数字之间可有空格；没有单位后缀时按字节算，乘出来不是整数时四舍五入到最近的字节。

## 错误

`ParseError` 是一个 `#[non_exhaustive]` 枚举，四个变体：`Empty`、`InvalidNumber`、`UnknownUnit`、`TooLarge`。它实现了 `Display` 与 `std::error::Error`，可以直接配 `?` 用。

## 许可

MIT OR Apache-2.0，Rust 生态惯用的双许可。
