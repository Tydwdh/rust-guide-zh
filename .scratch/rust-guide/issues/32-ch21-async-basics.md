# 32 第 21 章 async 基础：Future、执行器与运行时

Type: task
Status: resolved

## Question

写第三篇第三章：**async 基础**。

- **范围**：`Future` 的契约（惰性、`Poll`、谁负责推进）；执行器概念（`Waker` 一句 + 「为什么要被执行器管理」）；`async`/`.await` 展开（状态机直觉，具体展开 → 第 38 章）；`join!`/`select!` 的概念（外部 crate 只作说明性引用，不进被测块）；`!Send` future 与 `Send` 边界（跨 `.await` 持锁的最小复现）；取消与 drop（`select` 掉的分支去哪了）；运行时选型一段（取舍标准，不点名背书）。
- **素材**：`research/02-knowledge-structure.md` 的 I11 行（依赖 B1/I3/I7/I9；书 ch17、`std/future`、`std/task`）；本机 1.98 快照：`std/future/trait.Future.html`、`std/task`（`Waker`/`Context`）；`std/pin` 只回指第 22 章。
- **边界**：`Pin`/`Unpin` 细节 → 第 22 章；手写运行时 / `Waker` 深水 / pin 投影 → 第 38 章；AFIT 语法在第 14 章（回指）；`async fn in trait` 的 `Send` 坑在第 14 章（回指）。执行器演示可用 std-only 的极简 `block_on`（把「Waker 被用来叫醒」说清即可，完整运行时 → 第 38 章）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；`async`/`.await` 在 edition 2024 + 1.98 stable 上真跑；外部 crate 的代码块一律 `rust,ignore` 并注明原因。
- **DoD**：同 §8 协议 —— 独占 `src/ch21-async-basics.md` + `snippets/ch21/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch21-async-basics.md`；探针 `.scratch/tmp-32/`；收尾 `claimed` + `## Comments`（同 §8 清单）。

## Comments

（写作者 32，2026-10-19）

**文件**
- `src/ch21-async-basics.md`：替换占位，八节模板齐全，574 行；正文汉字约 4935（不含代码块），落在 4000–5500 区间内。
- 新增 `snippets/ch21/executor.rs`：std-only 极简执行器（`Parker` 实现 `Wake` + `block_on` + `block_on_all` + `Delay`）。
- 新增 `snippets/ch21/cancel.rs`：drop 一个停在等待点上的 future，看局部变量如何析构。
- 本票。未碰其他文件（`src/SUMMARY.md` 已有本章条目，无需改动）。

**自验**
- `mdbook test -c ch21-async-basics.md` → **exit 0**。共 17 个 doctest：15 个编译/运行通过 + 2 个 `rust,ignore`；两个 `rustdoc_include` 片段（executor / cancel）都在被编译之列。
- 注入失败反证：把 `poll_once` 的 `Some(v)` 改成 `Some(v + 1)` → 报 `error[E0369]`、`test result: FAILED. 14 passed; 1 failed; 2 ignored`、**exit 101**；已还原并复测 exit 0。说明门禁确实在编译本章（成功时 mdbook 不打印逐条结果，只留一行 INFO）。
- 旁注（非本票问题）：该命令日志里会出现 `ERROR Error updating "{{#rustdoc_include ../snippets/ch20/litmus.rs}}"`，来自写这条时第 20 章（票 31）尚未落盘的片段；不影响本章 exit 0，未去碰它。

**实测过的输出（均真跑真粘；1.98 + edition 2024；探针在 `.scratch/tmp-32/`，已删）**
- 惰性：`greet()` 只创建不 await → 只打印 `future 已创建，但函数体还没跑`。
- 忘了 await → ``warning: unused implementer of `Future` that must be used`` + ``= note: futures do nothing unless you `.await` or poll them``。
- `poll_once`（`Waker::noop()`）→ `Some(2) None`；忙等版 `block_on` → `42` / `2`。
- `snippets/ch21/executor.rs` → `顺序 await 两个 Delay：150 ms` / `并发跑两个 Delay：90 ms -> ["A(60ms)", "B(90ms)"]`（Delay 分别 60/90 ms）。
- `snippets/ch21/cancel.rs` → `任务 A 开始，停在等待点` / `drop future` / `任务 A 的局部变量 被 drop` / `main 结束`。
- `.await` 在非 async 上下文 → ``error[E0728]: `await` is only allowed inside `async` functions and blocks``（关键行见章内）。
- 跨 `.await` 持锁 → `error: future cannot be sent between threads safely`，附 ``= help: within `{async block@src/main.rs:7:17: 7:22}`, the trait `Send` is not implemented for `std::sync::MutexGuard<'_, u32>``` 与 ``note: future is not `Send` as this value is used across an await``；两种修法（把 guard 关进块 / 抽成同步函数）实测均编译通过。
- async 闭包 `async |x: u32| x + 1` 在 1.98 + 2024 编译并跑出 `42`（正文只用一句话提及，无代码块）。
- 命令行口径：章内 console 块用的就是实跑的那两条 `rustc --edition 2024 snippets/ch21/… -o …exe` 与 `.\….exe`。

**引文核对（本机 1.98 快照逐字，非转述）**
- Book ch17.6：`The runtime—specifically, its executor—manages tasks, and tasks manage futures.`
- Book ch17.1：`futures in Rust are lazy: they don’t do anything until you ask them to with the await keyword.`
- Reference `expressions/await-expr.html`：`An await expression is a syntactic construct for suspending a computation provided by an implementation of std::future::IntoFuture until the given future is ready to produce a value.`（近似展开那段落整段照抄，标 `rust,ignore` 并注明 `yield` 是伪代码）
- Reference `items/functions.html`（Async functions）：`Async functions do no work when called: instead, they capture their arguments into a future. When polled, that future will execute the function’s body.`
- std `Future`：`This method does not block if the value is not ready. Instead, the current task is scheduled to be woken up when it’s possible to make further progress by polling again.`；`Once a future has finished, clients should not poll it again.`；`Panics` 一节 `may panic, block forever, or cause other kinds of problems`。
- std `Poll`：`When a function returns Pending, the function must also ensure that the current task is scheduled to be awoken when progress can be made.`
- std `Waker`：`A Waker is a handle for waking up a task by notifying its executor that it is ready to be run.`；`a waker may be invoked from any thread, including ones not in any way managed by the executor.`
- std `Waker::noop`（1.85 起稳定，const）：`Returns a reference to a Waker that does nothing when used.`；`discarding the notification of when the future should be polled again`。
- std `IntoFuture`：`The .await keyword desugars into a call to IntoFuture::into_future first before polling the future to completion.`

**范围与边界（已对照 `research/02-knowledge-structure.md` 的 I11 行与 §5）**
- I11 清单逐条落地：`Future` 契约、executor 概念、`async`/`.await`、`join!`/`select!`（说明性、`rust,ignore`）、`!Send` future 与 `Send` 边界、取消与 drop、运行时选型。
- 边界守住：`Pin`/`Unpin` 细节、手写 `Waker` 深水、pin 投影 → 第 22/38 章；AFIT 的 `Send` 坑与「含 `async fn` 的 trait 不能做 trait 对象」只回指第 14 章，不重讲；执行器演示是 std-only 极简版（`Wake` + `Condvar` 挂起/唤醒，`Delay` 由后台线程 `wake`）。
- §5.1 #3「async 闭包不支持」是过时说法：按 1.85 起稳定写了一句并实测（见上）。

**新增术语（请协调者统一并表）**
- 执行器（executor）、任务（task）、运行时（runtime）、唤醒（wake）／唤醒句柄（`Waker`）、状态机（state machine）、取消（cancellation）、惰性（laziness；已有「惰性求值（lazy evaluation）」条，本次按形容词用法出现）。
- `_Avoid_` 遵守：没用「竞态」（正文写作「唤醒与阻塞之间的时序窗口」），没用「销毁/释放」指 drop（统一用「析构」），没用「异步 trait 方法」（写 AFIT）。

**遗留不确定**
- 执行器演示的两个耗时（150/90 ms）随机器浮动；正文已补一句「要紧的是顺序≈相加、并发≈较慢的那段」，不把那两个数字当断言。
- 「只驱动一次」的忙等 `block_on` 与「动手看看」里的真 `block_on` 同名但能力不同；正文用 `> [!WARNING]` 明确划清。若审校觉得同名易混，可改名为 `block_on_ready`（一行之内）。
- 选型一节刻意不点名任何运行时；`join!`/`select!` 的 `rust,ignore` 块以 `futures` 为例，未引运行时名。

**衔接提示**
- 回指：第 10 章（迭代器惰性类比）、第 13 章（`Pin<Box<dyn Future<...>>>` 那行持有形式）、第 14 章（RPIT/AFIT 与 `Send` 坑）、第 19 章（线程模型与 `Send`）。
- 前指：第 22 章（`Pin`/`Unpin`：为什么 `poll` 收 `Pin<&mut Self>`）、第 38 章（`Waker` 深水、手写状态机、pin 投影、丢唤醒的完整处理——本章「动手看看」第二条已预告）。
- 第 22 章可放行回指「第 21 章已引出 `Pin`」：本章只在「`Future` 的契约」末尾与小结各提一句，不展开。

**图示需求**
- 本章未插 SVG（票面无要求）。若后续统一补图，有一处值得画：`async fn` 的写法 ↔ 编译器生成的状态机（字段=跨 `.await` 存活的局部变量、切点=`.await`），以及「poll → `Pending` → 登记 waker → `wake` → 再 poll」的循环；都是空间/流程关系，建议 600×220 左右、单块无空行。

### 交叉审校修回（RvCh21；1 major + 4 minor；2026-10-19）

只动 `src/ch21-async-basics.md` 与本票；修完复跑 `mdbook test -c ch21-async-basics.md` → **exit 0**。

1. **【major】「动手看看」第三条**（原「睡一次、醒一次」）：按实测改写为「两个 `Delay` 各自返回 `Pending` 后，执行器先睡下；60 ms 时 A 的线程唤醒它，推进一轮后再次睡下，90 ms 时 B 的线程再唤醒它——两次睡眠、两次唤醒，总耗时由最慢的一个决定」。自己另行插桩复核过：`.scratch/tmp-32/count.rs`（`executor.rs` 的插桩副本，数 `Parker::park` 与 `wake_by_ref`）输出顺序为 `[park]1 → [wake]1 → [poll]完成 → [park]2 → [wake]2 → [poll]完成`，`合计：park 2 次，wake 2 次`，耗时仍 `90 ms`（探针目录已删）。
2. **【minor】`:77`**：`Poll` 不在 `std::future`。改为「你看到的只有 `Future`、`IntoFuture` 与 `pending`/`ready`/`poll_fn` 这类现成的小积木（`Poll` 在 `std::task`）」，与后文抄 `Poll` 定义处（来自 `std::task`）不再互相矛盾。
3. **【minor】`:257`**：改为「这两个宏在 stable 上都用不上：`std::future::join!` 至今是 nightly-only 实验 API（`future_join`），`select!` 则只在 `futures` 这类 crate 里」。回查本机 1.98 快照 `std/future/macro.join.html` 原文确为 `This is a nightly-only experimental API. (future_join #91642)`，页内无 `Experimental` 字样——之前是把模块索引那行的徽标文字当成页面原文了。
4. **【minor】`:383`**：小标题改为「就绪标记与唤醒标记的先后不能反」，正文点明「`Delay` 那边先置 `done` 再 `wake()`；执行器这边 `park()` 先看 `Parker::woken` 再阻塞」，不再出现代码里没有的 `ready`。
5. **【minor】报错块保真**：三处 `text` 报错块（E0728 / 忘记 `.await` / `!Send`）统一补「以 Rust 1.98 为例，关键几行：」（对齐 ch19 口径）；`!Send` 那块按本章 `compile_fail` 代码本身重新真跑后重贴，删掉 caret 行多出的那一个前导空格，并补齐 `--> src/main.rs:9:32` 与出错源行；源行与 caret 行已用 `cmp` 与实况输出**逐字节**比对通过。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch21-async-basics.md`（4965 汉字；八节模板 + 3 道练习）+ `snippets/ch21/{executor.rs,cancel.rs}`（std-only `block_on` 执行器 + 取消演示）。
- 自验：`mdbook test -c ch21-async-basics.md` → exit 0（17 doctest = 15 通过 + 2 `ignore`）；注入反证 `14 passed; 1 failed; 2 ignored` → 还原；`executor.rs` 真跑 `150–151 ms + 90 ms`。
- 交叉审校（RvCh21；1 major + 4 minor）全修：`block_on_all` 睡/醒次数按插桩实测改写（两次睡、两次醒）；`Poll` 归属 `std::task`；`join!` 措辞按 1.98 徽标原文；报错块补版本标注 + caret 逐字节对齐。
- 术语并表（协调者）：执行器 / 任务 / 运行时 / 唤醒·Waker / 状态机 / 取消 已入 GLOSSARY。
- 全量链（批次级）：同上一档口径全绿。
- 遗留：执行器耗时数字随机器浮动（正文已注明「要紧的是顺序≈相加、并发≈较慢那段」）。
