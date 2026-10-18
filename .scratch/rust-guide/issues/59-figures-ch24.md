# 59 第四批图示：第 24 章（指针、provenance 与未初始化内存）

Type: task
Status: open

## Question

给 `src/ch24-pointers-provenance.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 42 的「图示需求」。

- **图清单**（1 张，必做）：
  1. **四种指针形态 × 地址 / 来历**：`*mut T` / `NonNull<T>` / `usize` / `without_provenance` 四行，两栏「有没有地址、有没有来历、能不能解引用」，把 provenance 的「地址对 ≠ 可用」一眼摊开（与 24.1/24.2 的严格来历叙述一致）。
- **放置**：插在 provenance 小节内（按正文位置就近）；图前一句话引出；不写图注编号。`text` 块仅在「被新图完整覆盖」时可删（论证并记票）。
- **边界**：只加图与引出句；正文一字不动（本章刚定稿、修回后）。
- **DoD**：§8 —— 独占 `src/ch24-pointers-provenance.md` + 本票；禁 `git`/禁 build；`mdbook test -c ch24-pointers-provenance.md` 绿；SVG 自查同票 58 口径；栅格化目视 `.scratch/tmp-59/`（收尾删）；收尾 `Status: claimed` + `## Comments`。

## Comments
