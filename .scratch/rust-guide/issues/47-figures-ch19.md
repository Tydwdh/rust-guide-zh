# 47 第三批图示：第 19 章（并发基础）

Type: task
Status: open

## Question

给 `src/ch19-concurrency-basics.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 30 的「图示需求」（三张全做）。

- **图清单**：
  1. **Send/Sync 与三条路径对照**：搬值（`move` 进线程）/ 发引用（`&T`/`&mut T`）/ 同步原语（`Mutex`/channel），标出每条例各要求哪个 trait（`Send`/`Sync`）。
  2. **`thread::spawn` vs `thread::scope` 区间对照**：`'static` 要求 vs `'scope`/`'env` 与「作用域出口自动 join」。
  3. **mpsc 拓扑**：`Sender`（clone × N）→ `Receiver`（单消费者）+「全部发送端消失 ⇒ 接收端结束」。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。若某 ASCII/`text` 草图被新图**完整覆盖**可删（逐条论证并记票）；否则一律保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动（本章刚经审校定稿）。
- **DoD**：§8 协议 —— 独占 `src/ch19-concurrency-basics.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch19-concurrency-basics.md` 绿；SVG 自查（块内无空行 / 无脚本外链 / `viewBox`+`width="100%"` 无 height / 仅主题变量 / `role="img"`+`aria-label` / 实体转义 / 列对齐加 `xml:space="preserve"`）；栅格化目视放 `.scratch/tmp-47/`（收尾删）；收尾 `Status: claimed` + `## Comments`（每图：位置行号 / aria-label / 引出句 / 自查结果 / 目视结论 / 遗留）。

## Comments
