# 60 第四批图示：第 26 章（FFI 与 ABI）

Type: task
Status: open

## Question

给 `src/ch26-ffi-abi.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 44 的「图示需求」。

- **图清单**（1 张，必做）：
  1. **FFI 边界与 panic 三条防线**（横向示意）：`Rust 调用方 →（extern "C" 边界）→ 回调方`，标注三种情形——① 直接越界 → abort 点；② 边界内侧 `catch_unwind` → 就地转成返回值；③ 全链 `extern "C-unwind"` → 外层 `catch_unwind` 接住。与 26.6 的三条事实一一对应。
- **放置**：插在 26.6 小节内；图前一句话引出；不写图注编号。`text` 块仅在「被新图完整覆盖」时可删（论证并记票）。
- **边界**：只加图与引出句；正文一字不动（本章刚定稿、修回后）。
- **DoD**：§8 —— 独占 `src/ch26-ffi-abi.md` + 本票；禁 `git`/禁 build；`mdbook test -c ch26-ffi-abi.md` 绿；SVG 自查同票 58 口径；栅格化目视 `.scratch/tmp-60/`（收尾删）；收尾 `Status: claimed` + `## Comments`。

## Comments
