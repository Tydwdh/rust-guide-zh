# 44 第 26 章 FFI 与 ABI

Type: task
Status: open

## Question

写第四篇第四章：**FFI 与 ABI**。

- **范围**：`extern "C"`（2024 起 `unsafe extern` 块；未写 `unsafe` 是 2024 的 hard error 还是 lint —— 以实测为准）；`#[no_mangle]` → `#[unsafe(no_mangle)]`（2024）；C 字符串（`CStr`/`CString`、`c"..."` 字面量）；回调（`extern "C" fn`、跨 FFI 边界 `catch_unwind` 防 panic 展开）；`#[repr(C)]` 契约；`c_variadic` 状态（nightly 标注）。给一个**可跑的最小端到端例子**（环境允许就用 `extern "C"` + 自编译 C 或 `link`；若本机无 C 编译器，用「同进程 Rust 内 `extern "C" fn` 互调 + `#[unsafe(no_mangle)]` 导出符号」替代，并说明局限）。
- **素材**：`research/02-knowledge-structure.md` 的 I16 行（依赖 I15/I13；`nomicon/ffi.html`、`reference/items/external-blocks.html`、`reference/abi.html`、`std/ffi/`）；本机 1.98 快照核对 `CStr`/`CString` API。
- **边界**：UB 清单 → 第 23 章；布局/repr → 第 25 章（回指）；`no_std`/嵌入式 → 第 41 章（一句）。Windows 本机若有 `cl.exe`/`gcc` 可尝试真编译 C 片段（先探测，失败就退回纯 Rust 演示并在票里说明）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；与 C 的引文须核一手文档（C 标准草案/CPython 文档等，别引二手）。
- **DoD**：§8 —— 独占 `src/ch26-ffi-abi.md` + `snippets/ch26/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch26-ffi-abi.md` 绿；探针 `.scratch/tmp-44/`；收尾 `claimed` + `## Comments`。**注意**：crate 级编译（若做独立 C 工程）也放 `.scratch/tmp-44/`，别落 `target/`。

## Comments
