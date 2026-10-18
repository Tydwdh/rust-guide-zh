# 46 项目 3：迷你任务队列（并发与异步篇）

Type: task
Status: resolved

## Question

写第三篇收尾项目：**迷你任务队列**（`examples/task-queue/` 已建 stub，crate 名 `task-queue`；写作者替换 stub 为真实现）。

- **目标**：零依赖线程池式任务队列：`spawn` 提交闭包任务；N 个 worker 线程从 `mpsc` 取任务；结果经 `JoinHandle` 或 channel 收集；优雅关闭（drop 全部发送端 + join workers）；可选：`recv_timeout` 超时接收、panic 任务不拖垮池（`catch_unwind`）。覆盖第三篇概念并让读者看见设计取舍：`Send + 'static` 边界（为什么闭包必须满足）、channel-only vs `Arc<Mutex>` 的选择与理由、锁中毒路径（若用了锁）、背压（`sync_channel` 容量）、并发测试的写法（重复跑/小压力/确定性断言）。
- **章结构**：对齐项目 1 成稿；引真实代码；所有行为真跑（含并发输出的确定性/非确定性说明）。
- **素材**：已定稿的 ch19–22（尤其 19/20）；std 文档逐条核对。
- **边界**：std-only；**不用 async**（一句话预告 async 版运行时 → 第 38 章）；不做外部依赖；不用 `unsafe`。
- **篇幅**：4000–5500 汉字（不含代码块）；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/project3-task-queue.md` + `examples/task-queue/**`（含其 `Cargo.toml`）+ 本票；禁 `git`/禁 `mdbook build`；自验：`CARGO_TARGET_DIR=.scratch/tmp-46/target cargo test --manifest-path examples/Cargo.toml -p task-queue` 全绿 + `mdbook test -c project3-task-queue.md` 绿；并发测试若偶发（时序类），在票里说明并给出稳定化方式（如 join 顺序/超时上界），不许留 flaky；探针 `.scratch/tmp-46/`；**勿动** `examples/Cargo.toml`、`examples/README.md`、其他 crate、其他章/票；收尾 `Status: claimed` + `## Comments`。

## Comments

### 交付物（只动了本票文件）

- `examples/task-queue/src/lib.rs`（替换 stub，公开接口 + crate 文档）
- `examples/task-queue/src/pool.rs`（`Job`/`Queue`/`ThreadPool`/worker 循环/`Drop` + 2 个单元测试）
- `examples/task-queue/src/handle.rs`（`TaskHandle`/`Timeout`）
- `examples/task-queue/tests/pool.rs`（13 个集成测试，只用公开 API）
- `examples/task-queue/examples/demo.rs`（可跑演示，章内两段终端记录来自它）
- `examples/task-queue/Cargo.toml` 未改（stub 里已是 `edition = "2024"`、无依赖、无额外 targets）
- `src/project3-task-queue.md`（替换占位；成稿 548 行，正文 5409 汉字，不含代码块，落在 4000–5500 内）
- 本票

未碰：`examples/Cargo.toml`、`examples/README.md`、`examples/todo-cli/**`、`examples/expr-eval/**`、其他章、其他票、`SUMMARY.md`（条目协调者已接好）。

### 自验（真跑，逐条）

1. `CARGO_TARGET_DIR=.scratch/tmp-46/target cargo test --manifest-path examples/Cargo.toml -p task-queue`
   → 全绿：**单元 2 + 集成 13 + 文档测试 3 = 18 passed / 0 failed**（约 0.22s）。
2. `CARGO_TARGET_DIR=.scratch/tmp-46/target cargo build --manifest-path examples/Cargo.toml -p task-queue --all-targets`（先 `touch` 全部源文件强制重建）→ 无 warning、无 error。
3. `mdbook test -c project3-task-queue.md` → exit 0。章内 18 个 rust 块：**6** 个真编译/执行（句柄 `Timeout` include、`Job` include、`Queue` include、3 个 `compile_fail`），12 个 `ignore`，另有若干 `text`/`console` 块不参与。
4. 并发用例稳定性（票面硬要求）：
   - 集成测试二进制连跑 **20 次**：0 失败；
   - `--test-threads=1` 串行跑一遍：13 passed；
   - `cargo test --release -p task-queue`：18 passed / 0 failed。
   - 稳定性靠设计而非重试：闸门/交会通道把「先后」变成同步事实；肯定断言配额 5s 上界（`LONG`），否定断言（「提交尚未完成」）由容量 0／容量 1 的协议保证、200ms（`SHORT`）只决定多快判负；顺序断言只在单 worker 下做；多 worker 只断言值与集合。三个可能挂死的用例都给了上界，坏的时候是**失败并打印**而不是挂死。

### 实测过的输出（正文粘贴处，均可复现）

- `cargo run -q --manifest-path examples/Cargo.toml -p task-queue --example demo` 连跑两次：完成顺序不同（丙 甲 乙 丁 / 甲 乙 丙 丁）、关闭前排队的 5 个任务集合相同而顺序不同（`[0,2,3,4,1]` / `[0,2,1,3,4]`）、panic hook 行带系统线程号。
- 另抽样 8 次 demo：完成顺序出现 6 种，用来支撑「顺序不保证」的论述。
- `cargo test --manifest-path examples/Cargo.toml -p task-queue` 的完整输出（13 个集成用例；省 `Compiling`/`Finished`/`Running` 行，以及末尾的 `all doctests ran …`、`Wall time: …`；doc-test 行号 11/36/94 与实际一致）。
- 三段报错：E0373（闭包借了局部变量）、E0277（`Rc` 不 `Send`）、E0277（`catch_unwind` 要 `UnwindSafe`）。用 `rustc --edition 2024` 单独编译最小复现取得原文（探针 `.scratch/tmp-46/probe{3,4,5}`，收尾已删）；正文只把文件路径写成 `main.rs`、对中段用 `…` 省略（含工具链源码路径前缀），其余逐字。**三块都写成自包含最小复现**（用同签名的 `submit` 函数代替真 crate），这样 `compile_fail` 断言的是我们展示的那个错误，而不是「找不到 crate」。

### 设计决策（正文第 1–6 条取舍的落地）

- 任务 = `Box<dyn FnOnce() + Send + 'static>`；`Send + 'static` 的来路逐条讲清，并对比 `thread::scope` 的作用域池（代价：每个签名拖 `'scope`）。
- 取任务 = `Arc<Mutex<Receiver<Job>>>`（std 文档「This half can only be owned by one thread.」是硬依据）；锁只护 `recv`，任务体在锁外跑；对比「每 worker 一条通道 + 轮询派发」的队头阻塞代价。
- 结果 = 每任务一条私有 `mpsc::channel::<thread::Result<T>>()`，`join` 与 `std::thread::JoinHandle::join` 同形。
- panic 隔离 = `catch_unwind(AssertUnwindSafe(job))`；正文引 std 文档原文说明 `UnwindSafe` 的用途、panic hook 先打消息、abort 型 panic 抓不到。
- 背压 = `bounded(size, capacity)` → `sync_channel(n)`，容量 0 = 交会；用交会通道把「提交会阻塞」变成可确定性断言的事。
- 关闭 = 丢发送端 + join worker（通道「挂断」即信号；排队任务先跑完）；`Drop` 与 `shutdown` 共用 `close`，`Drop` 里忽略 worker join 的 `Err`（再抛会 abort）。
- 不可达分支：`enqueue` 的 `send` 用两个带解释的 `expect`，句柄侧用 `lost_result()`（`unreachable!`）标注不变量 —— 正文明确说明这是「把不变量写进代码」，不是错误处理。

### 复审 findings 的处理（RwProj3 复审：1 major + 12 minor，已全部改）

章面：

1. 【major】块末「第 20 章的锁中毒概念」→ 改为与 `Send`/`'static`/`Mutex`/`mpsc`/`JoinHandle` 同列第 19 章。
2. 公开接口描述由「两个类型」改为「三样东西：`ThreadPool`、`TaskHandle<T>`、`Timeout`」；顺带修掉逐块实现开头的「只重导出两个类型」（同源错误）。
3. `std::thread::Result<T>` 的展开两处补 `+ 'static`。
4. 报错三：把真实的 `help` 块（含 `use parentheses` 那条）补进输出；正文改为「加约束那条来自 `help`，`AssertUnwindSafe` 那条来自 `std` 文档」。
5. 概念表 20–22 行改为「任务跑在固定的 N 个 worker 上，`join` 就是阻塞当前线程等通道」（原文「一个提交对应一个线程」与设计相反）。
6. `worker（工作线程）` → `工作线程（worker）`（对齐第 19 章）。
7. 结构差异按协调者裁定保留：`### 先看效果` 与 `### 并发用例怎么写得能重复跑` 是项目 3 有意新增的两节（见「衔接提示」）。
8. SVG：`viewBox` 560 → 600（坐标重排，文字仍在框内）、`font-size="11"` → 12，重新渲染检查过。
9. 终端记录省略说明写全（末尾的 `all doctests ran …`、`Wall time: …` 也列出来）；报错一补回 `note` 的 `-->` + `^^^^` 整块。

crate：

10. `Timeout.waited` 文档改为「本次等待的请求时长（真正返回时可能略晚一点）」，`Display` 改为「（至多）等了 …」，与实测一致（请求 80ms、实际 80.48ms）。正文对应处也改成「请求等了多久」。
11. 新增 3 个集成用例覆盖 `bounded(size, capacity > 0)`：`bounded(2,4)` 排进去的活 + `drop`、`bounded(1,1)` 缓冲满了挡住提交方、`bounded(3,2)` 多线程提交。集成用例 10 → **13**，章内粘贴的输出与计数同步更新。
12. `enqueue` 第一条 `expect` 的消息与注释改为「close 需要 `&mut self`，此处池必然还活着」（原消息描述的是一个不可达状态），正文那段也补了原因。

票：13. 本文件的块统计改为真编译 6 个（句柄 include、`Job` include、`Queue` include、3 个 `compile_fail`）。

改完后重跑：`cargo test … -p task-queue` 18 passed / 0 failed、`mdbook test -c project3-task-queue.md` exit 0、集成二进制连跑 20 次 0 失败、`--test-threads=1` 13 passed、`--release` 18 passed、`--all-targets` 无 warning；正文 5409 汉字（不含代码块）。

### 遗留不确定 / 交给协调者

1. **SVG 只做了单图渲染检查**（抽出来用图像查看：文字均在框内、箭头对齐），**没跑 `mdbook build`**（票内禁止）；请在收尾 build 时确认 HTML 渲染一次。图为「一次提交的路径」（提交 → 队列 → worker → 结果端 → 句柄），单块无空行、`viewBox`+`width="100%"`、无 `height`、只用主题变量、有 `role`/`aria-label`、实体转义（`&lt;`/`&gt;`）。
2. **`Box` 的章号归属**：正文按「第 13 章 `dyn`/trait 对象 + 第 17 章 `Box` 本体」注明（概念表与块末各一处）。若地图里 `Box` 有别的首选落点，按地图改。
3. `demo` 的 panic 输出走 stderr：直接跑会看到，`cargo test` 里被 libtest 收起来（只在失败时展示）。正文已解释这不是池漏了 —— 若审校认为读者会误解，可在 demo 里装一个 panic hook 把它吞掉，但那就把「真实输出」藏起来了（当前选择：保留并说明）。
4. 章内终端记录按本机实况保留了 Windows 反斜杠（`task-queue\examples\demo.rs`、doc-test 行里的 `task-queue\src\...`）。

### 新增术语（供统一并表）

- **线程池（thread pool）**：固定数量 worker 从共享队列取任务执行的并发结构。
- **worker**：池里的工作线程。正文首现写「工作线程（worker）」，之后保留英文（与第 19 章的「工作线程」一致）。
- **任务队列（task queue）**：存放待执行任务的队列。
- **句柄（handle）**：`TaskHandle` —— 提交任务后用来等结果的那一头；正文首现已括注英文。_Avoid_: 收据（已从代码与正文全部改为「句柄」）
- **背压（backpressure）**：有界队列满时阻塞生产端。
- **队头阻塞（head-of-line blocking）**：慢任务把它后面的任务一起卡住；轮询派发方案的代价。
- 复用既有条目：交会（rendezvous）、锁中毒（poisoning）、通道（channel）、先行发生/内存序本票不涉及、原子本票明确不用。

### 衔接提示

- 概念主依赖第 19 章（`thread::spawn`/`Send`/`Sync`/`Mutex`/`mpsc`/`JoinHandle`/`thread::Result`）；第 17 章 `Arc`/`Drop`、第 13 章 `dyn`、第 10 章 `map`/`drain`、第 16 章 `'static` 各一处；第 20 章只作「为什么不用原子」的反面参照；`async` 版明确指向第 38 章（正文收尾第 5 条）。
- 章结构与项目 1 一致：这一章解决什么问题（要做的东西 / 图 / 验收标准 / 先看效果）→ 设计取舍 → 逐块实现（含概念落点对照表）→ 报错现场（3 段）→ 测试（含「并发用例怎么写得能重复跑」）→ 收尾 → 小结 → 练习（3 题带答案）→ 延伸阅读（只官方一手链接，不钉版本）。
- **模板扩展（有意为之，协调者已裁定保留）**：项目 3 起，项目章模板新增两节 —— `### 先看效果`（效果先行：先用真实终端记录把「确定/不确定」立起来）与 `### 并发用例怎么写得能重复跑`（并发测试方法论）。项目 1/2 无此节；如果后续项目章沿用它，建议写进 `WRITING.md` 的项目章模板说明里。
- 若审校要给本项目配图，候选：① 关闭的两步（丢发送端 → worker 排空队列 → join）；② `Arc<Mutex<Receiver>>` 的共享视图（N 个 worker 各持一份 `Arc`、锁只护 `recv`）。当前只放了 ① 对应的文字与一张总览图，未额外插图。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/project3-task-queue.md`（5409 汉字；含 2 个有意扩展小节并已在票注明）+ `examples/task-queue/`（零依赖、无 unsafe/async；**18 测试** = 2 单测 + 13 集成 + 3 doc；连跑 20× 稳定）。
- 自验：`cargo test -p task-queue` 18 全绿；`--test-threads=1`/`--release` 复跑；`mdbook test -c project3-task-queue.md` exit 0（18 块：真编译 6 + ignore 12）。
- 复审（RwProj3；1 major + 12 minor）全修：锁中毒回指 19 章、公开三类型、`thread::Result` 补 `'static`、报错三按真实 help、概念表 worker 表述、首现「工作线程（worker）」、SVG 600/字号≥12、省略说明写全；crate：`Timeout.waited` 口径、**新增 3 个 `bounded(cap>0)` 用例**、`enqueue` expect 消息。
- 术语并表（协调者）：线程池 / 工作线程 / 任务队列 / 句柄 / 背压 / 队头阻塞 已入 GLOSSARY。
- 全量链（批次级）：同上一档口径全绿。
- 遗留：无。
