# 31 第 20 章 原子与内存序

Type: task
Status: claimed

## Question

写第三篇第二章：**原子与内存序**（依赖 I9 的 `Send`/`Sync`/`UnsafeCell` 语义）。

- **范围**：`Ordering` 五值的语义与选用（`Relaxed`/`Acquire`/`Release`/`AcqRel`/`SeqCst`）；happens-before 的直觉与「没有它会发生什么」（两线程 flag/消息对演示）；`compare_exchange` 的两种失败序与 CAS 循环（`fetch_*` 家族概览）；`UnsafeCell` 与「原子类型为什么需要内部可变性」；无锁入门的判断标准（何时不值得手写 —— 回指第 17 章的锁）。
- **素材**：`research/02-knowledge-structure.md` 的 I10 行（依赖 I9/I13；`std/sync/atomic/enum.Ordering.html`、Nomicon atomics / races）；本机 1.98 快照核对 `Atomic*` API 与文档原文。
- **边界**：别名模型/形式化语义 → 第 35 章；性能与剖析 → 第 29 章；`loom` → 第 31 章（一句）；`Send`/`Sync` 基础 → 第 19 章（回指不重讲）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；内存序演示必须真跑（可跑两种序对照并给真实输出）；nightly-only 只标注。
- **DoD**：同 §8 协议 —— 独占 `src/ch20-atomics-memory-ordering.md` + `snippets/ch20/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch20-atomics-memory-ordering.md`；探针 `.scratch/tmp-31/`；收尾 `claimed` + `## Comments`（同 §8 清单：文件 / 自验命令与结果 / 实测输出清单 / 遗留不确定 / 新增术语 / 衔接提示 / 图示需求）。

## Comments

**2026-10-19 写作者（票 31）— 收尾：初稿完成，单章门禁绿**

**文件（本票独占）**
- `src/ch20-atomics-memory-ordering.md`：占位替换为全章初稿（653 行；正文汉字 **5356**，已剔除代码块与 SVG，落在 4000–5500 区间）
- `snippets/ch20/litmus.rs`（新建，68 行）：store buffering 对照程序（2 000 000 轮 × Relaxed / Release+Acquire / SeqCst），正文以 ` ```rust,no_run ` + `{{#rustdoc_include ../snippets/ch20/litmus.rs}}` 引入
- 未碰任何其他文件。探针 `.scratch/tmp-31/` 已删除；另删掉一个手工误建的根目录文件 `snippets_ch20_litmus.rs`

**结构自查（WRITING §4）**：八节模板齐全（这一章解决什么问题 / 为什么这样设计 / 机制与规则 / 动手看看 / 报错现场 / 小结 / 练习 / 延伸阅读）；练习 3 题均带 `<details><summary>参考答案</summary>`；正文内嵌 1 张手写 SVG（Release/Acquire 配对 → happens-before 时间线）——单块无空行、无脚本、无外部资源、`viewBox` + `width="100%"` 且不写 `height`、只用 `currentColor`/`var(--links)`、`role="img"` + `aria-label`、文本里无裸 `&`/`<`/`>`。

**自验（唯一命令）**
- `mdbook test -c ch20-atomics-memory-ordering.md` → **exit 0**
- doctest 构成（首次运行日志）：12 个 —— 7 个真跑、3 个 `compile_fail`（负向断言：`load` 传 `Release`、`static Cell<u64>`、练习 1 的 `store` 传 `Acquire`）、1 个 `no_run`（只编译 litmus 片段）、1 个 `ignore`（`Atomic<T>` 泛型骨架，该拼写 nightly-only，不能编译）
- **注入反证（已还原）**：把「发布/订阅」示例里的 `Ordering::Acquire` 改成 `Ordering::Acquiree` → 同命令 **exit 101**，rustdoc 报 `E0599`、`variant, associated function, or constant not found in std::sync::atomic::Ordering` 并给出 `Acquire` 修法建议；还原后再次 exit 0。另有一次真实反证：练习 1 初稿漏标 `rust,compile_fail`，门禁立刻 exit 101 并打出 `atomic stores cannot have Acquire or AcqRel ordering`，补标记后转绿 —— 门禁确实在编译这些块。

**实测过的输出清单（真跑真粘；Windows x86-64 / rustc 1.98.0 (88d9e12ae 2026-08-18) / edition 2024）**
1. 丢更新对照（默认档位）：`load + store` → 110023 / 141000 / 114928 / 175575 / 124777（每次不同）；`fetch_add` → 恒 400000
2. 同程序加 `-O`：5 次全部 400000；更早一次同类运行出现 391785；`-O --emit asm` 的坏循环体是 5 条**不带 `lock` 的 `incq`**（写 `ArcInner` 的 data 字段，偏移 16 —— 已核对 `alloc/src/sync.rs:388` 的 `strong`/`weak`/`data` 布局）
3. 发布/订阅（Release + Acquire）：`消费者读到数据：42`（多次运行一致）
4. CAS/RMW 巡礼：`79999`、`update 返回旧值 0，现在是 100`、`失败，当前值其实是 7`、`x 仍然是 7`、`fetch_or 0b0000→0b0101`、`swap 0b0101→0b1000`、`fetch_and 0b1000→0b0000`
5. store buffering 对照（每项分母 2 000 000）：① Relaxed 20 / Release+Acquire 77 / SeqCst 0；② 51 / 313 / 0；③ 按书中命令（仓库根 + `-O`）再跑一次：400 / 127 / 0 —— 三次里 SeqCst 恒为 0
6. `--emit asm`（`-O`）：`store` 的 Relaxed 与 Release 都是 `movq %rdx, (%rcx)`，SeqCst 是 `xchgq %rdx, (%rcx)`；三种 `load`（Relaxed/Acquire/SeqCst）全是 `movq (%rcx), %rax`；`fetch_add` 无论什么序都是 `lock addq`；`compare_exchange(AcqRel, Acquire)` 是 `lock cmpxchgq`
7. 报错现场两段真实输出：`error: atomic loads cannot have \`Release\` or \`AcqRel\` ordering`（help 建议 `Acquire`/`SeqCst`/`Relaxed`，`#[deny(invalid_atomic_ordering)]` 默认 deny）；`error[E0277]: \`Cell<u64>\` cannot be shared between threads safely`（note 直接建议改用 `RwLock` 或 `std::sync::atomic::AtomicU64`）
8. 运行期细节核对：`compare_exchange(1, 2, Relaxed, SeqCst)` 实测**不 panic**（`core` 只拦 `Release`/`AcqRel` 作失败序）→ 正文因此只断言「失败序不能是 `Release`/`AcqRel`」，没写「必须弱于成功序」
9. 术语来源核对：`Arc::clone` 用 `strong.fetch_add(1, Relaxed)`、`Drop` 用 `strong.fetch_sub(1, Release)` + Acquire 屏障（`alloc/src/sync.rs:2411` / `:1235`），练习 3 答案据此写 `Arc` 的选用

**遗留不确定**
- SB 实验命中率极低（百万分之一量级）且随负载变化；正文已明确写「概率性、可能跑几次全是 0」，只把「SeqCst 恒为 0」当稳定对照，避免读者复现不出而误解。
- `-O` 下那对 `Relaxed` load/store 被合并成不带 `lock` 的 `incq`，是**本机 rustc 1.98 / LLVM 的观察**（反汇编为证）；正文只描述现象与后果，未追究优化器依据，也未断言这是规范行为。
- `fetch_update` 的 1.99 弃用标记取自本机 1.98 快照的 `core/src/sync/atomic.rs:1312`；1.98 上不告警（实测 stderr 为空）。正文已注明「1.98 上还能用、不告警」。
- 无任何弱序架构（ARM）实测；凡涉及 x86/ARM 差异的句子全部改引 Nomicon 原文（`atomics.html`：strong/weak ordering、release/acquire 在强序平台上「often totally free」、弱序平台上测试的建议）。
- `AtomicU128`/`AtomicI128` 的 `target_has_atomic = "128"` 门控、`thumbv6m-*` 无 CAS、32 位 PowerPC/MIPS 无 `AtomicU64`，均只从 std 源码/模块文档读到，未在对应目标上实测。
- 按协议**未跑** `mdbook build`/`serve`：SVG 渲染、表格与 linkcheck2 由协调者收尾确认（正文外链全是 `doc.rust-lang.org` 稳定入口，无版本钉扎；`[…]` 已全部收进完整代码 span 或去括号，规避 §7 的潜在链接坑）。

**新增术语（请协调者统一并表）**
原子类型（atomic type）；读-改-写（read-modify-write，RMW）；先行发生（happens-before）；顺序一致性（sequential consistency）；CAS（compare-and-swap）与 CAS 循环；伪失败（spurious failure）；无锁（lock-free）；无等待（wait-free）；存储缓冲（store buffer）；自旋锁（spinlock）；ABA 问题（ABA problem）；屏障 / 完整屏障（memory barrier）。全部按「首现中文（English）」落文；`_Avoid_` 词逐条扫描命中两处并已改（`不安全的部分` → `unsafe 的部分`；`[lock-free]`/`[wait-free]` 的方括号按 §7 去掉以免被 linkcheck2 当链接）。沿用既有术语：内存序、数据竞争、未定义行为、锁中毒、内部可变性、析构。

**衔接提示**
- 第 19 章目前仍是占位（3 行），本章只回指「第 19 章」负责 `thread` / `Send`+`Sync` / channel / 锁中毒，不依赖其细节；`Ch19Conc` 定稿后若要在第 19 章反向指到「原子量在第 20 章」，由协调者处理（本章无需改动）。
- 回指与边界（均按票）：第 17 章（内部可变性、`UnsafeCell`、`Cell` 是 `!Sync`）、第 29 章（性能与剖析）、第 31 章（`loom`/Miri，一句）、第 35 章（别名模型与形式化语义）。
- `Atomic<u32>` 泛型拼写仍不稳定（E0658）、`from_mut(&mut u32) -> &AtomicU32` / `from_mut_slice` / `get_mut_slice` 的方向，均对应 `research/02` §5.2#17，已逐条核对后写进「操作家族」表。
- 票面写的素材路径 `research/02-knowledge-structure.md` 是笔误：实际在 `.scratch/rust-guide/research/02-knowledge-structure.md`，**已对照**其 I10 行与 §5 易错清单（含 §5.2#17）。

**图示需求**
- 本章已自绘 1 张时序图（Release/Acquire 配对 → happens-before），放在「机制与规则」。若线 B（票 34–40）要按统一风格补图，本章**不需要**再加；可选增强是「五种序强度阶梯」示意——目前由 `Ordering` 五值表承载，符合「一张表能说清就不画」。

---

**2026-10-19 第二版：交叉审校 RvCh20 的 8 条 findings 已逐条处理（2 blocker + 1 major + 6 minor 里实际 8 条）**

只动 `src/ch20-atomics-memory-ordering.md` 与本票；其余四条轴（Nomicon/std 引文、litmus/汇编复跑、术语、结构）未改动对应内容。

1. **[blocker] CAS 失败序的「会 panic」说法不准确 → 已改并补成「报错现场」第三例。** 自行复核两种形态（探针 `.scratch/tmp-31/proj/src/main.rs`）：
   - **常量**写法 `compare_exchange(1, 2, Ordering::AcqRel, Ordering::Release)` → **编译错误**（同一 lint `invalid_atomic_ordering`，默认 deny）：`` error: `compare_exchange`'s failure ordering may not be `Release` or `AcqRel`, since a failed `compare_exchange` does not result in a write ``，`--> src/main.rs:5:56`，`help: consider using `Acquire` or `Relaxed` failure ordering instead`。
   - **非常量**（失败序由函数参数传入）→ 编译通过，运行期在 `...\core\src\sync\atomic.rs:4011:29` panic：`there is no such thing as a release failure ordering`（panic 行路径的工具链前缀已在正文注明「已略」）。
   正文「机制与规则」的 `failure` 条目改为「常量被 lint 在编译期拦下；非常量才轮到运行期 panic」，并新增「报错现场」第三例（`rust,compile_fail` + `rust,should_panic` 两段，均真跑、均进门禁），判据引 std 原话（`(A `compare_exchange` or `compare_exchange_weak` that does not succeed is not considered a write.)`）。
2. **[blocker] 小结第 3 条同事实错误 → 已改**为「三条都由同一 lint `invalid_atomic_ordering` 在编译期拦下（默认 deny）；失败序那条只拦得住常量写法，非常量要等运行期 `panic!`」。
3. **[major] 延伸阅读 `struct.AtomicUsize.html` 404 → 改 `type.AtomicUsize.html`**（实测 200；该页给出 `pub type AtomicUsize = Atomic<usize>;`，与 1.98 的类型别名实况一致）。按评审建议，全章 6 条外链用 `read` 实测：`/std/sync/atomic/`、`enum.Ordering.html`、`type.AtomicUsize.html`、`std/cell/struct.UnsafeCell.html`、`nomicon/atomics.html`、`nomicon/races.html` —— **全部 200**。
4. **[minor] nightly-only 块补 admonition**：`Atomic<T>` 骨架前加 `> [!WARNING]`（`generic_atomic` tracking issue #130539；stable 用 `AtomicU32`/`AtomicUsize` 这类具体名字，它们是 `Atomic<T>` 的类型别名）。全章 admonition 计数由 0 变 1。
5. **[minor]「完整屏障（memory barrier）」首现补英文括注**（在「五、序在 x86-64 上到底编译成了什么」的第 2 条；更早的 `text` 汇编块里那处属代码块内容，未动）。
6. **[minor] 可移植清单纠偏**：原文「优先 `AtomicUsize`/`AtomicBool`」与 std 不符 → 改引模块文档原话 *`AtomicUsize` and `AtomicIsize` are generally the most portable, but even then they’re not available everywhere.*
7. **[minor] 报错现场口径统一**：三处文件名改 `src/main.rs`（`5:27` / `3:17` / `5:56`——改后又用 `src/main.rs` 真跑一遍取输出，行号列号不变）；输出标签统一为「以 Rust 1.98 为例，关键几行：」。
8. **[minor] 引文弯引号**：`` without the “consume” memory ordering ``（按渲染后的 std 页面照抄；本地 `.rs` 源里是直引号，正文以读者看到的页面为准）。

**第二版再验证**
- `mdbook test -c ch20-atomics-memory-ordering.md` → **exit 0**。rust 块 14 个 = 7 个真跑 + 4 个 `compile_fail`（`load` 传 `Release`、`static Cell<u64>`、CAS 常量失败序、练习 1）+ 1 个 `should_panic`（CAS 非常量失败序）+ 1 个 `no_run`（litmus）+ 1 个 `ignore`（`Atomic<T>` 骨架）。
- **反证（已还原）**：把新增 `should_panic` 块里的 `Ordering::Release` 改成 `Ordering::Acquire`（这样不会 panic）→ 门禁 **exit 101**，证明该块确实被执行；还原后 exit 0。
- 篇幅：正文汉字 **5435**（剔除代码块与 SVG），仍在 4000–5500 区间内（第一版 5356，第二版因新增第三例与措辞调整一度到 5669，已按不删信息点的原则压缩行文回落）。
- 探针目录 `.scratch/tmp-31/` 已删除；`snippets/ch20/litmus.rs` 未改动（其输出被复跑复核一致）。
- 仍未跑 `mdbook build`/`serve`（协议）；外链已改为用 `read` 逐条实测（这次 404 说明单靠 build 抓不到这类断链）。
