# 47 第三批图示：第 19 章（并发基础）

Type: task
Status: resolved

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

三图全部落地，只动 `src/ch19-concurrency-basics.md`（本票文件）——新增 3 个 SVG 块 + 3 句引出句，正文/代码块/表格一字未改。

**图 1（Send/Sync 与三条路径对照）**
- 位置：`src/ch19-concurrency-basics.md` 第 38–61 行（`### 两条规则覆盖三条路径` 小节，紧接「…第 17 章那张分布表里的每一格都是它们的推论。」之后）。
- 引出句（第 36 行）：「下图把三条路径与各自的 trait 要求摆在一起：」
- aria-label：「三条路径对照：把值搬过去只需要 T: Send，把共享引用 &amp;T 发过去需要 T: Sync、可变引用 &amp;mut T 需要 T: Send，同步原语共享不引入新 trait」
- 内容：三列 —— ① 搬值（`move` 进 `spawn`，要求 `T: Send`）；② 发引用（`&T` 需 `T: Sync`、`&mut T` 需 `T: Send`）；③ 同步原语（`Mutex`/channel 不引入新 trait）。底注两行点出「两条规则覆盖全部」与第三条路建立在前两条之上。

**图 2（spawn vs scope 区间对照）**
- 位置：第 165–187 行（`### thread::scope` 小节「三点值得记住」列表之后、`scope` 不豁免 `Send` 段落之前）。
- 引出句（第 163 行）：「下图把 `spawn` 与 `scope` 的存活区间摆在一起：」
- aria-label：「spawn 与 scope 的存活区间对照：spawn 的闭包捕获必须活到 'static 且要手动 join，scope 允许借用局部数据并在作用域出口自动 join 全部线程」
- 内容：① `spawn` 行——创建线程的作用域矩形 + 可能超出的线程存活虚线，标注「捕获的引用活不到 'static → 只能 move 拿走所有权，join 还要自己动手」；② `scope` 行——作用域外框内三条 `s.spawn #N` 区间条，右侧虚线标出「出口 / 全部 join」，底注「借来的 &words 活到 'scope 结束」。

**图 3（mpsc 拓扑）**
- 位置：第 579–605 行（`### 通道` 小节末，`try_recv()`/`recv_timeout()` 段之后、`### thread_local!` 之前）。
- 引出句（第 577 行）：「下图把一条通道的拓扑摊开：」
- aria-label：「mpsc 拓扑：clone 得到的多个 Sender 把消息送进同一条通道，只有一个 Receiver 消费；所有 Sender 消失后接收端结束」
- 内容：左侧三个 `tx.clone() #N` 生产者框 → 中间 FIFO 通道 → 右侧单个 Receiver（单消费者）框；底注「原件 tx 也要 drop，否则接收端永远等不到结束」「所有 Sender 消失 → recv() 返回 Err(RecvError)，rx.iter() 自然结束」。

**ASCII/`text` 草图裁定**：本章 21 个 `text` 块全部是程序输出（`cargo run`/`compile_fail` 的真实 stdout、报错关键行），无一为结构草图/示意图；三张新图未覆盖任何既有 `text` 块，故**一个都没删**（§9「否则一律保留」）。

**自验证据**
- 门禁：`mdbook test -c ch19-concurrency-basics.md` → exit 0（加图前后均绿；图不参与代码测试，确认未碰坏栅栏）。
- SVG 结构自查（Python 脚本，逐块）：3/3 通过 —— 块内无空行 / 根标签无 `height` / `viewBox` + `width="100%"` / `role="img"` + `aria-label` / 无 `<script>`·外链·`href`·`url()` / 无写死颜色（`fill`/`stroke` 仅 `currentColor`·`var(--links)`·`var(--warning-border)`·`var(--quote-border)`·`none`）/ 无裸 `&`（实体全转义）/ `xml.etree` 解析通过。列对齐 `xml:space="preserve"` 无需加：三图无复刻工具输出的列对齐文本。
- 栅格化目视：`cairosvg` 渲染 3 张 PNG（宽 760）到 `.scratch/tmp-47/`（收尾已删）——框、箭头、对齐、文字落位均正常；CJK 显示为方框是渲染器缺中文字库所致（非 SVG 问题，HTML 渲染用浏览器字体）。

**遗留**：无。HTML 最终渲染留待协调者收尾 build 统一确认（§9）。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch19-concurrency-basics.md` 补 **3 张** —— Send/Sync 三条路径（:38–61）、`spawn` vs `scope` 区间（:165–187）、mpsc 拓扑（:579–605）。
- `text` 块审计：21 个全是程序输出/报错行，**0 删**。
- 自验：`mdbook test -c ch19-concurrency-basics.md` exit 0；结构自查 3/3；栅格化目视；HTML 计数 3/3 与浏览器确认。
- 遗留：无。
