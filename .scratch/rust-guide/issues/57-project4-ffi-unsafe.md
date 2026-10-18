# 57 项目 4：封装系统 API（FFI 与 unsafe）

Type: task
Status: open

## Question

写第四篇收尾项目：**封装系统 API**（`examples/sys-probe/` 已建 stub；crate 名 `sys-probe`；作者替换 stub）。

- **目标**：把 Windows 系统 API 安全封装成零依赖库：`extern "system"` + `#[link(name = "kernel32")]`（如 `GetTickCount64`；可选加 `GetSystemTimeAsFileTime` 走 `#[repr(C)]` 结构体 + 裸指针路径）；对外只暴露安全 API（如 `uptime_millis() -> Result<u64, ProbeError>`）；覆盖 ch23–26 概念并让读者看见取舍：`unsafe fn` 的契约与 `# Safety` 文档、`unsafe {}` 块的最小化、`#[repr(C)]` 布局契约、FFI 边界与 panic（`catch_unwind` 必要性说明）、错误设计。
- **跨平台**：`#[cfg(windows)]` 走真 FFI；`#[cfg(not(windows))]` 提供 `Err(ProbeError::Unsupported)` 降级分支并配测试——保证 `cargo test` 在 Linux CI 也能编译/通过（本机只实测 Windows 路径，票里注明）。
- **章结构**：对齐项目 1 成稿（逐块实现 / 设计取舍 / 报错现场 / 测试 / 小结 / 练习 2–3 题带答案）；正文引 crate 真实代码；Windows 实跑输出真粘。
- **素材**：已定稿 ch23–26 与项目 1；Windows API 须引一手文档（learn.microsoft.com）；本机 1.98 实测。
- **DoD**：§8 —— 独占 `src/project4-ffi-unsafe.md` + `examples/sys-probe/**`（含其 `Cargo.toml`）+ 本票；禁 `git`/禁 build；自验：`CARGO_TARGET_DIR=.scratch/tmp-57/target cargo test --manifest-path examples/Cargo.toml -p sys-probe` 全绿 + `mdbook test -c project4-ffi-unsafe.md` exit 0；探针 `.scratch/tmp-57/`；**勿动** `examples/Cargo.toml`（members 已配）、`examples/README.md`、其他 crate；收尾 `claimed` + `## Comments`。

## Comments
