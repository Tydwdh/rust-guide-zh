# 40 进阶篇图示：第 18 章（错误处理与 API 设计）

Type: task
Status: open

## Question

给 `src/ch18-error-api-design.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 29 的「图示需求」（1、2 必做；3 非必需）。

- **图清单**：
  1. **库 ↔ 应用 责任分工与错误类型形状**（§18.1，现为 5 行表）：分层示意 —— 库侧具体 enum ／ 应用侧 `Box<dyn Error>`，中间是 `source()` 与 `?` 的流转。
  2. **`#[non_exhaustive]` 两个视角**（§18.3，现为 3 行表）：定义 crate 内 / 外「同一份代码、两种语义」对照（枚举 / 变体 / struct 三种位置的对外行为）。
  3. （非必需，可略）§18.6 的 17 行判定表 —— 图非必需。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若表格仍不可替代（信息更密），可只画 1、2 并说明。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch18-error-api-design.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch18-error-api-design.md` 绿；SVG 自查项同票 34；栅格化目视 `.scratch/tmp-40/`（收尾删）；票尾 `## Comments` 逐图记录。

## Comments
