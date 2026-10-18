# 变更日志

本文件记录这个 crate 的显著变更。格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

未发布版本写在 `Unreleased` 下；每次发布把那一节改名为版本号并补上日期（`YYYY-MM-DD`）。

## [Unreleased]

## [0.1.0] - 2026-10-19

### 新增

- `format`：把字节数格式化成 1024 进制的大小字符串（`1.5 KiB`、`10 MiB`）。
- `parse`：把大小字符串解析回字节数，同时接受 1000 进制与 1024 进制单位。
- `ParseError`：`Empty` / `InvalidNumber` / `UnknownUnit` / `TooLarge` 四个变体，
  实现 `Display` 与 `std::error::Error`。

### 说明

- 0.1.0 之前的 API 不保证兼容。
