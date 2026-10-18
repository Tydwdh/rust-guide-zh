# 66 第 38 章 自己实现极简 async 运行时

Type: task
Status: claimed

## Question

写第六篇第五章：**自己实现极简 async 运行时**（素材 = D5：`std::task`/`std::future`/`std::pin` 文档）。

- **范围**：不用 async/await 语法糖，手写一个 `impl Future` 状态机；`RawWaker`/`Waker` 构造与 `Wake`；手写 pin 投影（`unsafe` + `Pin::map_unchecked_mut` 一类，封装成安全 API）；单线程 executor：任务队列 + `block_on` + 唤醒路径。与 ch21 明确划界：ch21 讲「怎么用 executor」，本章讲「怎么从 `Future`/`Waker` 原语搭出来」。
- **要求**：全部 stable 真跑（`snippets/ch38/` 含测试）；`Wake`/`RawWaker`/`Pin` 引文逐字核 1.98 快照。
- **边界**：async/await 语法与 executor 使用 → ch21 回指；Pin 语义 → ch22；不重讲 waker 基础概念。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 内嵌 1–2 张 SVG（poll/唤醒时序、状态机展开）。
- **DoD**：§8 —— 独占 `src/ch38-mini-async-runtime.md` + `snippets/ch38/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch38-mini-async-runtime.md` exit 0 + snippet `cargo test`（私有 target）全绿；探针 `.scratch/tmp-66/`；收尾 `Status: claimed` + `## Comments`。

## Comments

**文件**
- `src/ch38-mini-async-runtime.md`：成稿，八节模板齐全，2 张内嵌 SVG。
- `snippets/ch38/`：新 crate `ch38-runtime`（`Cargo.toml`、`src/lib.rs`、`src/main.rs`）。

**自验命令与结果**
1. `mdbook test -c ch38-mini-async-runtime.md` → exit 0（成稿与审校修复后各复跑一次，结果一致），日志 0 个 ERROR。章内共 14 个代码块：7 个普通块真跑 + 3 个 `compile_fail`（合起来 10 块进测试闸门）+ 4 个 `ignore`（`main.rs` 的 include、38.5 的 API 用法片段、练习 1 的修法片段、练习 3 的 `impl Unpin`）。
   反向验证过一次 include：把它临时改成 `rust` 后 mdbook test 立刻 exit 101 并报 E0432（`use ch38_runtime` 无法解析），说明 `{{#rustdoc_include}}` 确实被展开；probe 已还原为 `rust,ignore`。
2. `cd snippets/ch38 && cargo test --target-dir D:/Rust/.scratch/tmp-66/target` → exit 0，`9 passed; 0 failed; 0 ignored`，main 与 doc-tests 两个 target 也全绿；零 warning。
3. `cargo run -q`（同私有 target）输出与正文「动手看看」逐字一致：`TwoStep(1, 2, 3) => 6` / `队列跑空：7 次 poll，顺序 A#3 B#2 A#2 B#1 A#1` / `Budgeted 用尽预算 => None` / `RawWaker 版：3 次 poll，顺序 D#2 D#1`。
4. 两张 SVG 用「导出独立 svg + 栅格化」自查过版式：文字不溢出、箭头与框不重叠；viewBox 700×218 / 700×268（后者在 F1 修复后加高），无空行、无脚本、无写死颜色。

**实测过的输出（真跑真粘）**
- 正文「动手看看」的 `cargo run -q` 输出（上条第 3 点）。
- 正文「动手看看」的 `cargo test` 输出：用默认 target 目录跑一次取得，与读者本机看到的路径一致；测试顺序由 libtest 并行调度，正文已加注。
- 报错现场一/二/三的三份 rustc 1.98 输出：最小复现都按「放进 `src/main.rs`」跑，行号列号照实；二三的 `compile_fail` 块与正文输出一一对应（现场一另附了第二条 Sync 报错的行）。
- 报错现场四的 panic：探针 crate（path 依赖 `ch38-runtime`）里 `block_on(async { std::future::pending::<u32>().await })` 得到「执行器停滞：队列为空且无人登记唤醒」；粘贴只去掉了随运行变化的线程号并注明，行号 `snippets\ch38\src\lib.rs:255:19` 与终稿一致。

**引文来源（逐字核自 1.98 文档快照 `share/doc/rust/html/`）**
`std/task/trait.Wake.html`（定义句、memory-safe alternative、Arc 与嵌入式那段）、`std/task/struct.Waker.html`（Clone+Send+Sync、`wake` 的 happens-before 与合并唤醒、`from_raw` 的安全条款、`Waker::new` 的 data 指针要求、`From<Arc<W>>` 的 no-heap-allocations）、`std/task/struct.RawWakerVTable.html`（四个函数在构造 `Waker` 时必须线程安全）、`std/pin/struct.Pin.html`（`map_unchecked_mut` 的 Safety）、`std/pin/index.html`（Structural Unpin）、`std/task/trait.LocalWake.html`（本地 waker 的收益）。

**版本口径**
- 全部 stable 1.98 真跑。`LocalWaker` / `LocalWake` 为 nightly-only（feature `local_waker`，issue #118959）：stable 上 `use std::task::LocalWaker;` 实测 E0658，正文按规范用 `> [!WARNING]` 只给文档化描述，未伪造输出。
- `Waker::new` 在正文标注「Rust 1.83 起可用」。

**规模**
正文汉字 5014（连代码注释与图示文字共 5681 个汉字字符；审校修复后比成稿略增，增量来自 F3/F5 的代码注释与断言、以及 SVG 改版）。

**新增术语**
无。沿用 `GLOSSARY.md` 既有条目（执行器 / 任务 / 运行时 / 唤醒与唤醒句柄 / 状态机 / pin 投影）。`RawWaker`、`vtable`、结构化 pinning、happens-before 未进表，若认为需要并表请协调者处理。

**衔接提示**
- 第 21 章：开头划界（用法在 21、搭法在此），正文两处回指（`poll` 契约三条、丢唤醒窗口），未重讲 waker 基础概念。
- 第 22 章：补上 22.7 预告的手写 pin 投影，并三处回指其结论（`Unpin` 语义、三个 safe 入口、投影是 unsafe 入口）。
- 第 19 章：`Send + Sync` 只提一句并回指。
- 第 39 章若要讲宏生成状态机，可直接引用本章的不变量。

**遗留 / 需协调者收尾**
- 章内 `{{#rustdoc_include ../snippets/ch38/src/main.rs}}` 放在 `rust,ignore` 块（`mdbook test` 把每个块当独立 crate 编译，该文件的 `use ch38_runtime::…` 拿不到本 crate）；渲染由收尾 build 统一确认（include 展开已自查）。
- `snippets/ch38/` 未提交 `Cargo.lock`（与 `snippets/ch11/`、`snippets/ch31/` 一致；构建产物 target/ 与探针 `.scratch/tmp-66/` 已删）。若仓库要保留 lock，收尾时补跑一次 `cargo test` 由 git 一并提交。
- 过程申明：收尾列目录时误敲了一次只读的 `git status --porcelain`（未改任何文件、未用于任何决策）；除此之外全程未使用 git，未跑 `mdbook build`。

## Comments · 审校修复（RvCh38 回执，全部已处理）

F1–F12 逐条处置；修复后重跑两条闸门：`mdbook test -c ch38-mini-async-runtime.md` exit 0（0 ERROR）、`cargo test`（私有 target）`9 passed; 0 failed`、零 warning。

- **F1（major）已改**：`Waker` 布局写错。已核 rust-src 1.98 `core/src/task/wake.rs`：`pub struct RawWaker { data: *const (), vtable: &'static RawWakerVTable }`、`#[repr(transparent)] pub struct Waker { waker: RawWaker }`。正文改为「`Waker` 是个 `#[repr(transparent)]` 的薄包装——源码里就是 `pub struct Waker { waker: RawWaker }`，那两个字段其实在 `RawWaker` 上」；SVG 里盒子改成三行 `Waker { waker: RawWaker }` / `RawWaker { data: *const (),` / `vtable: &'static RawWakerVTable }`，标题改「Waker 包着 RawWaker」，viewBox 由 246 调高到 268 并重新栅格化自查版式。
- **F2（minor）已改**：`mdbook test` 的解释改成「把每个代码块当独立的 crate 编译，块里的 `use ch38_runtime::…` 拿不到本 crate，编不过；`ignore` 只让它不进闸门，include 照常展开」。
- **F3（minor）已改**：正文措辞改为「把 `clone`/`wake`/`wake_by_ref`/`drop` 四个 vtable 入口依次走一遍，逐步检查引用计数」；测试 `raw_waker_keeps_refcounts_balanced` 补齐断言：`wake_by_ref` 不消费（引用不变）、`clone` 后 +1、`drop` 后回到 +1、消费式 `wake` 后回到基准。
- **F4（minor）已改**：删除死代码 `Task.polls` 字段、`polls()` 方法、`fetch_add` 与初始化（选「删」而非「用上」，因为正文没有要展示的用途）；随之把 `AtomicUsize` 的导入下移到 `mod tests`，避免 unused_import 警告。
- **F5（minor）已改**：38.6 片段与 crate 对齐——改成先取 guard（`let mut slot = self.future.lock().unwrap();`）再 `slot.take()`，guard 持有到 poll 结束；并加一句「crate 里的 `run` 与这段逐行对应，只多一个 `with_raw_waker()` 分支」，把两处差异交代清楚。
- **F6（minor）已改**：「第 21 章末尾提到的那个窗口」→「第 21 章讲执行器那节提到的那个窗口」；现场二的「理由见 38.4」→「这条约束为什么躲不掉，见本段末」。
- **F7（minor）已改**：现场三输出补回被漏掉的两行（`|                   |` 与 `|                   required by a bound introduced by this call`），并在末尾加 `...` 表示省略了 `Pin::<Ptr>::new` 的 toolchain 路径 note。
- **F8（minor）已改**：代价表限定为「沿用默认的 `wake_by_ref` 时每次唤醒有一次 `Arc::clone` 原子操作（`wake` 与自带实现不克隆）」。
- **F9（minor）已改**：练习 2 答案的不可操作建议换成三条可行路线：换 `Arc` 等线程安全类型 / 把 `!Send` 值留在 future 外面 / 自己 `unsafe impl Send` 并承担证明责任。
- **F10**：确认无需处理（Cargo.lock 卫生交协调者收尾）。
- **F11（minor）已改**：延伸阅读拆成两条——`RawWaker` 与 `RawWakerVTable` 各给一条官方链接。
- **F12（minor）已改**：本票自验口径按「7 个普通块 + 3 个 `compile_fail` = 10 块进闸门，另 4 块 `ignore`，共 14 块」重写（见上方「自验命令与结果」第 1 条）。
