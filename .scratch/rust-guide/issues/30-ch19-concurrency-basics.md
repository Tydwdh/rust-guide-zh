# 30 第 19 章 并发基础：thread、Send/Sync 与共享状态

Type: task
Status: claimed

## Question

写第三篇第一章：**并发基础**。第 17 章已给 `Mutex`/`RwLock`/`Arc`/`Cell` 的 API 面；本章接「跨线程的组合与规则」。

- **范围**：`thread::spawn` 与 `thread::scope`（借用如何跨线程：`scope` 的 join 保证）；`Send`/`Sync` 的准确语义（auto trait、为什么 `Rc` 不是 `Send`、`unsafe impl` 与不变量）；`Arc<Mutex<T>>` 的组合（含锁中毒路径与 `RwLock` 的取舍回指第 17 章）；channel（`std::sync::mpsc` 两类发送端、`Sender` 的移动语义、消失的发送端）；`thread_local!` 一小节；收束回「数据竞争在安全代码里被类型系统排除」。
- **素材**：`research/02-knowledge-structure.md` 的 I9 行（依赖 B1/I7；书 ch16、`std/marker`、Nomicon send-and-sync / poisoning）；本机 1.98 快照：`std/thread`、`std/sync`、`std/sync/mpsc`、`std/marker/trait.Send.html`、`std/marker/trait.Sync.html`；实测行为用 rustc 1.98 + edition 2024 真跑。
- **边界**：`Ordering`/CAS/无锁 → 第 20 章；async/`Future` → 第 21 章；`Pin` → 第 22 章；别名模型与数据竞争的形式化 → 第 35 章；`loom` 在测试里的位置 → 第 31 章（一句带过）。第 17 章的 `Mutex`/`RwLock` API 细节不重讲，只写「跨线程组合时新出现的东西」。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；`thread::scope` 标稳定版本（1.63）；nightly-only 只标注不进正文；可选「语言对照角」（若写，Java/Go 的线程模型断言须核一手文档，宁缺勿错）。
- **DoD**：按 `WRITING.md` §8 批次协议 —— 独占 `src/ch19-concurrency-basics.md`（替换占位内容）+ `snippets/ch19/` + 本票；禁 `git`/禁 `mdbook build`；自验 `mdbook test -c ch19-concurrency-basics.md` 绿；探针放 `.scratch/tmp-30/`（收尾删除）；运行输出真跑真粘；英文引文逐字核对；收尾 `Status: claimed` + `## Comments` 一条（文件 / 自验命令与结果 / 实测输出清单 / 遗留不确定 / 新增术语 / 衔接提示 / 图示需求）。

## Comments

**（写作者，收尾一条）**

**文件**
- `src/ch19-concurrency-basics.md`：替换占位，952 行；八节模板齐全（这一章解决什么问题 / 为什么这样设计 / 机制与规则 / 动手看看 / 报错现场 / 小结 / 练习 / 延伸阅读）；正文汉字 **5453**（不含代码块，脚本统计 `[\u4e00-\u9fff]`）；练习 3 题带 `<details>` 答案；`thread::scope` 已标 1.63；未写「语言对照角」（票里是可选项，本章跨语言断言收益低，宁缺勿错）。
- `snippets/ch19/wordcount.rs`：新增，46 行，完整可跑（`{{#rustdoc_include}}` 在「动手看看」内）。
- 本票。未碰 `src/SUMMARY.md`（条目协调者已预置）、`book.toml`、`WRITING.md`、`GLOSSARY.md` 及其他章/票。

**自验（grading 命令与结果）**
- `mdbook test -c ch19-concurrency-basics.md` → **exit 0**（mdbook v0.5.4 / rustc 1.98）。
- 注入反证：把示例里 `let value = handle.join().unwrap();` 临时改成 `let value: String = handle.join().unwrap();` → **exit 101**，输出 `test result: FAILED. 20 passed; 1 failed; 1 ignored`（`1 ignored` = spawn 签名块的 `rust,ignore`）；还原后复跑 exit 0。
- 全部 `rust` / `rust,compile_fail` / `rust,no_run` 块另用 `rustc --edition 2024 src/main.rs` 逐块编译核对（compile_fail 均按预期失败；粘贴的 `--> src/main.rs:行:列` 与实际编译一致，大段删减处加 `…`）。
- 未跑 `git`、未跑 `mdbook build` / `serve`；探针目录 `.scratch/tmp-30/` 已删除。

**实测输出清单（rustc 1.98 + edition 2024，全部真跑）**
- `spawn` + `join` 拿返回值：`join 拿到返回值：42`。
- `thread::scope` 借 `&words` 与 `&mut total_len`：`scope 结束后 total_len = 19`（三线程打印无顺序保证，正文已注明）。
- `join` 的 panic 传播：`join -> Err，负载是：子线程炸了`（panic 头 `src/main.rs:4:30`）。
- `scope` 自动 join 的 panic 传染：`a scoped thread panicked`（进程 exit 101，正文按叙述呈现）。
- `unsafe impl Send/Sync`（裸指针句柄指向 `static`）：三线程读到 `1234`。
- **闭包捕获精度对照**（正文 `[!TIP]` 的依据）：同一段用 `h.0` 的代码，edition 2024 报 `*const i32 cannot be sent between threads safely`，edition 2018 编译并运行；改用方法调用后 2024 也通过。
- `Mutex`：`try_lock` → `WouldBlock`；持锁 panic 后 `lock()` → `poisoned lock: another task failed inside`，`into_inner()` 取回半改值 `初始被改了一半`。
- `sync_channel(0)` 交会语义（`send` 阻塞到接收端取走）与 `sync_channel(2)` 缓冲不阻塞：见正文输出。
- `mpsc` 多生产者 + `drop(tx)`：收到 4 条，`recv -> Err(RecvError)`。
- **发送端存活性**：不 drop 原件时第 4 次接收 `timed out waiting on channel`；`drop(tx)` 后立刻 `channel is empty and sending half is closed`（练习 2 依据）。
- `thread_local!`：每线程独立计数（主线程 2、新线程 0→1、子线程结束后主线程仍 2）。
- Send/Sync 断言表（`assert_send`/`assert_sync` 手法）：`Sender` Send+Sync、`Receiver` !Sync、`JoinHandle` Send+Sync、`MutexGuard` !Send 且（T: Sync 时）Sync；`Rc<Mutex<i32>>`、`Arc<Cell<i32>>` 均非 Send。

**遗留不确定**
- scope 示例的三行线程打印顺序**不保证**；粘贴的是本机一次真实运行，正文已加一句「行序可能不同」。
- 「锁的公平性 / 获取顺序」没有 std 文档承诺，正文只写「不要依赖获取顺序」，未做更强断言（不给无法核实的具体策略）。
- `wordcount.rs` 的 `per_chunk` 已加 `.max(1)` 防 `chunks(0)` panic；`workers == 0` 仍会 panic（演示程序按票要求写死 3，未做参数校验）。
- 报错现场的行号按「最小复现放进 `src/main.rs`」给出，与探针文件名不同——正文已声明口径。

**新增术语（请协调者统一并表）**
- **通道（channel）**：`std::sync::mpsc` 的消息传递设施；正文首现写「通道（channel）」。_Avoid_: 信道。
- **交会（rendezvous）**：容量为 0 的 `sync_channel`，发送方必须等到接收方取走。_Avoid_: 会合。
- （复用）**锁中毒（poisoning）**：第 17 章已首现，本章按需复述「建议性」语义，未另立词条。

**衔接提示**
- 回指：本章不重讲第 17 章的 `Mutex`/`RwLock` API；第 17 章 NOTE 里留给本章的三件事（锁的公平性、`try_lock` 非阻塞语义、死锁与自锁）已在本章「`Arc<Mutex<T>>`：跨线程组合」一节兑现（公平性按「文档未承诺」处理，见上）。
- 外送：`Ordering`/CAS/无锁/`UnsafeCell` → 第 20 章；`async` → 第 21 章；`Pin` → 第 22 章；`unsafe impl` 的健全性判定 → 第 23 章；别名模型形式化 → 第 35 章；`loom` → 第 31 章（正文各一句）。
- 第 17 章那张 `Send`/`Sync` 分布表在本章被「逐字段推导 + 断言手法」补上原理，两章表格不冲突；本章新增的是 `JoinHandle`/`Sender`/`SyncSender`/`Receiver`/`MutexGuard`/`LocalKey` 六行。

**图示需求（第三批 B 线不含本章，供后续图票）**
1. Send/Sync 与三条路径（搬值 / 发引用 / 同步原语）的对照图：哪条路要求哪个 trait。
2. `thread::spawn` vs `thread::scope` 的区间对照图：`'static` 要求 vs `'scope` / `'env` 与「作用域出口自动 join」。
3. mpsc 拓扑图：`Sender`(clone×N) → `Receiver`（单消费者）+「全部发送端消失 ⇒ 接收端结束」。

**素材核对**：已对照 `research/02-knowledge-structure.md` 的 **I9 行**（`thread`/`scope`、`Send`/`Sync`（auto trait 与 `unsafe impl`）、`Arc<Mutex<T>>`、poisoning、channel、`thread_local!` —— 逐项覆盖）与 **§5 易错清单**（§5.2 #14「`Rc`/`RefCell` 不是 `Send`/`Sync`；`unsafe impl Send` 是安全承诺不是解锁开关」已在正文 WARNING 框落实；§4.2 的 1.63 `thread::scope` 版本号已采用）。

**（写作者，交叉审校修复一轮）**

7 条 minor 全部处理（只动 `src/ch19-concurrency-basics.md` 与本票）：

1. **分布表 `LocalKey<T>` 行**：改「是 / 是」，说明列写「句柄本身是 auto trait、可随线程移动；但 `with` 取到的永远是**当前线程**那一份」。复核：`assert_send/assert_sync::<LocalKey<Cell<u32>>>()` 与 `::<LocalKey<Rc<i32>>>()` 全部通过（1.98）——`LocalKey` 的唯一字段是 `fn(Option<&mut Option<T>>) -> *const T`，`fn` 指针的 Send/Sync 与 `T` 无关。
2. **`:64` 自相矛盾**：改为「这三个 `where` 子句里，std 专门给了理由的是两条（译自原文）…」，并补一句返回值的 `T: Send` 出自同一条理由。
3. **直引号 → 弯引号**：`determines it's appropriate` → `it’s`；`having "hung up"` → `having “hung up”`，已改。证据与结论：快照源码里这两处本是 ASCII，但 **mdBook 的 smart punctuation 会把它们渲染成弯引号**（实测 `book/html/ch17-smart-pointers.html`：ch17 源码的 ASCII `"Reference Counted"` 渲染为 `“Reference Counted”`、`isn't` 渲染为 `isn’t`），且 ch16/ch18 定稿章的源码直接写弯引号 —— 为与定稿章一致（也免依赖渲染行为）改弯引号；词面逐字未动，仅引号字形变化。
4. **TLS 引文**：补回句中 `(e.g. with)` 与 `and values that implement Drop get destructed when a thread exits.`，删掉原来的截断拼接。
5. **`join` 返回类型**两处（正文 + 小结）改为 `std::thread::Result<T>`，展开为 `Result<T, Box<dyn Any + Send + 'static>>`（与快照 `thread/mod.rs:263` 的别名定义一致）。
6. **报错现场导语**：改为「按报错来源排列：生命周期 → 类型约束 → 所有权 → 跨线程能力」。
7. **练习 2 输出无来源**：补上产生它的最小程序（先收固定 3 条 → `recv_timeout` → `drop(tx)` 再试一次），输出块换成该程序的**确定性**输出：`收到 3 条，合计 3` / `第 4 次 recv：timed out waiting on channel` / `drop(tx) 之后：channel is empty and sending half is closed`（原来的 `收到 0/1/2` 行序不定，已弃用）。

复跑与复核：
- `mdbook test -c ch19-concurrency-basics.md` → **exit 0**（修复后）。
- 另用脚本把本章 23 个 rust 块逐个落成 `src/main.rs`、`rustc --edition 2024` 核对：13 个可运行块 rc=0，`compile_fail` 均按预期失败，`no_run` 只编译、`ignore` 跳过；无挂起、无 mismatch。
- 正文汉字复计 **5493**（仍 ≤ 5500；为容纳修复内容压缩了两处句子）。
- 探针目录 `.scratch/tmp-30/` 复删。
