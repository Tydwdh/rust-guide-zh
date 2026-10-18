# 62 第 34 章 手写集合与智能指针

Type: task
Status: claimed

## Question

写第六篇第一章：**手写集合与智能指针**（素材 = `research/02` 的 D1 行：Nomicon `vec.html`、`arc-and-mutex.html`）。

- **范围**：从能跑的 mini `Vec` 出发——RawVec 式增长与容量台阶（用实测台阶证明）、`Drain` 守卫与「守卫 drop 时收尾」的安全写法、ZST 特例（`cap = usize::MAX` 惯例）；然后简化版 `Arc`（原子计数 + `Acquire`/`Release` 配对理由 + `Weak` 不阻断释放）与 `Mutex`（`UnsafeCell` + park 或自旋）。unsafe 契约写法与 ch23 对齐（`// SAFETY:` 注释、`# Safety` 文档）。
- **要求**：全部 stable 真跑（`snippets/ch34/` 真编译 + 测试；关键结构用 `size_of`/字节转储对照 ch25 结论）；**Miri 无 nightly，不得给 Miri 输出**（只文档化命令并标注）。
- **边界**：智能指针基础 → ch17 回指；布局/niche → ch25；unsafe/UB → ch23；`loom` 只提名字并明说未跑。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 需要处内嵌 1–2 张 SVG（如 RawVec 增长/Drain 生命周期）。
- **DoD**：§8 —— 独占 `src/ch34-handwritten-collections.md` + `snippets/ch34/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch34-handwritten-collections.md` exit 0 + `snippets/ch34/` 的 `cargo test`（私有 target）全绿；探针 `.scratch/tmp-62/`；收尾 `Status: claimed` + `## Comments`。

## Comments

写作者自验完成（未交叉审校 / 未接 `SUMMARY.md` / 未跑全量链 —— 按 §8 留给协调者）。

### 交付物
- `src/ch34-handwritten-collections.md`：新写成稿，八节模板齐（这一章解决什么问题 / 为什么这样设计 / 机制与规则 34.1–34.6 / 动手看看 / 报错现场 3 个 / 小结 / 练习 3 题带答案 / 延伸阅读）。正文汉字 **5475**（脚本统计，不含代码块与 SVG）。
- `snippets/ch34/`：新建零依赖 crate。
  - `Cargo.toml`（`ch34-snippets`，edition 2024，无依赖）、`Cargo.lock`（cargo 生成）
  - `src/lib.rs` + `src/{mini_vec,mini_arc,mini_mutex}.rs`：mini_vec = RawVec 增长 / `Drain` 守卫 / ZST；mini_arc = 引用计数 + `Release`/`Acquire` 配对；mini_mutex = `UnsafeCell` + `AtomicBool` 自旋锁
  - `tests/mini_vec.rs`（11 个测试）、`tests/mini_arc_mutex.rs`（6 个测试，含 4 线程 × 20 000 次累加）
  - `examples/vec_steps.rs`、`examples/arc_mutex.rs`（正文「动手看看」的两份完整程序）
  - `#![deny(unsafe_op_in_unsafe_fn)]` 在 lib.rs；唯一 `unsafe fn` 是私有的 `Drain::new`，带 `# Safety` 段，调用点配 `// SAFETY:`（对齐 ch23 `boundary.rs` 的写法）
- 未碰其它任何文件（SUMMARY.md / book.toml / GLOSSARY.md / 其它票只读）。

### 自验命令与结果（本机 Windows x86-64、stable 1.98）
1. `mdbook test -c ch34-handwritten-collections.md` → **exit 0**。
   门禁非空验证（临时改坏再改回，已复原并核对 md5）：
   - 给 34.4 的 `rust` 块塞一个类型错误 → exit 101（该块确实被编译执行）
   - 删掉 E0509 `compile_fail` 块里的 `Drop` 实现使其能编译过 → exit 101（`compile_fail` 断言确实生效）
2. `cargo test --manifest-path snippets/ch34/Cargo.toml` → **exit 0**，17 测试全绿（`unittests src\lib.rs` 0 + `tests\mini_arc_mutex.rs` 6 + `tests\mini_vec.rs` 11 + `Doc-tests` 0）。target 落在 `snippets/ch34/target/`（仓库根 `.gitignore` 的 `target/` 已覆盖，本票私有）。
3. `cargo run --example vec_steps`、`--example arc_mutex` → exit 0；两份输出逐行粘进正文，并用脚本比对过「与章节代码块完全一致」（`Running …` 行的哈希与 `finished in` 耗时按正文注释说明为随运行变化）。
4. include 解析校验：按 mdbook 0.5.4 的 `take_anchored_lines` 语义写脚本提取全部 `{{#include …:anchor}}`，**11 处全部 anchor_found=True 且内容符合预期**（mdbook 对未命中的 anchor 是「静默空块」，不报错，所以单独查了一遍）。引用方式 = 命名锚点（源文件里的 `// ANCHOR: name`），比行号稳。

### 实测数据（正文数字全部本机真跑）
- 容量台阶（40 次 push，去重）：`Vec<u8>` = 8,16,32,64；`Vec<u64>` = 4,8,16,32,64；`Vec<[u64;300]>` = 1,2,4,8,16,32；mini `MiniVec<u32>` = 1,2,4,8,16,32,64。std 源码 `min_non_zero_cap`（1→8 / ≤1024→4 / 其它→1）与文档「不保证任何增长策略、只保证 O(1) amortized push」都核过原文（表格用来证明「台阶不是接口」）。
- ZST：`Vec::<()>::new().capacity()` = 18446744073709551615（`usize::MAX`）；`MiniVec<()>` push 一万次 cap 不变、零分配。
- 布局（对照 ch25）：`Vec<u8>`/`Option<Vec<u8>>`/`MiniVec<u8>`/`Option<MiniVec<u8>>`/`MiniVec<()>` 全 24 字节；`Arc<u128>`/`Option<Arc<u128>>`/`MiniArc<u128>`/`Option<MiniArc<u128>>` 全 8 字节；另有 `MiniVec<u8>` 的 3 机器字字节转储（word0 = 首元素地址、word1 = cap = 4、word2 = len = 3；输出做成确定性，不含随机地址）。
- 报错现场（rustc 1.98 真跑，路径按本书惯例写成 `src/main.rs`，`aborting`/`--explain` 行按 ch23 约定注明略去）：
  - E0509 `cannot move out of type MiniVec<T>, which implements the Drop trait`
  - E0502 `cannot borrow v as mutable because it is also borrowed as immutable`（`&v[0]` 与 `v.drain(..)`）
  - `Layout::array::<u64>(usize::MAX)` → `Err(LayoutError)`；`Layout::array::<()>(usize::MAX)` → `Ok((0, 1))`；`Vec::with_capacity(usize::MAX)` panic `capacity overflow`
  - 练习 3 引了真跑的 `E0277 … required for MiniArc<Cell<u32>> to implement Send`（临时 path-dependency 探针编的）
- 引文逐字核对：Nomicon `vec-push-pop`/`vec-zsts`/`vec-drain`、std `Vec`/`Layout`/`GlobalAlloc`/`UnsafeCell`/`MutexGuard` 文档、std 源码 `raw_vec/mod.rs` 与 `sync.rs` 注释；转述处都标了「译文」或「大意」。

### nightly-only 内容的处理（未伪造任何输出）
- **Miri**：正文 `> [!WARNING]` 盒，只列文档化命令（`rustup +nightly component add miri`、`cargo +nightly miri test --manifest-path snippets/ch34/Cargo.toml`），明写「本机只有 stable 1.98，没跑、没有输出」。
- **loom**：只提名字并说明未跑；内存序部分用真线程压测（4 × 20 000 = 200000）当证据，并在正文点明「这不等于验证过内存序」。
- 未用到 build-std / specialization / macro_metavar_expr 等其它 nightly 特性。

### 图示（§9，2 张内嵌 SVG）
结构与规则自查通过（脚本）：单块无空行、无脚本/外链、`viewBox` + `width="100%"` 无 `height`、颜色只用 `currentColor`/`var(--links)`/`var(--quote-border)`、`role="img"` + `aria-label`、图内 `&`/`<`/`>` 已转义、图前后各留空行。
1. 34.2「40 次 push 的累计扩容次数」双台阶折线（MiniVec 7 次 vs std `Vec<u8>` 4 次）。
2. 34.3「Drain 守卫生命周期」三行图（drain 前 / drain 后 len=0 且守卫持有 `[start,end)` / 守卫 drop 的三种结局）。
**遗留：本机禁 `mdbook build`，两张图的目视渲染（比例、右端文字是否溢出、深浅主题对比）请协调者收尾 build 时确认。**

### 新增术语（供协调者并表；写作者未改 GLOSSARY）
- **守卫（guard）**：RAII 型守卫（`Drain`、`MutexGuard`）。GLOSSARY 现有「匹配守卫（match guard）」条目写着 `_Avoid_: 守卫（单用太泛）`——建议新增「守卫（guard）」一条并说明区别。本章首现写「守卫（guard）」，其余处写「`Drain` 守卫」「锁守卫」，避免裸用。
- **增长台阶（capacity growth steps）**：容量发生变化的那些时刻（本章用它对比 `min_non_zero_cap`）。这不是标准库术语，属叙述性表达，协调者可决定是否入表。
- 其余术语（智能指针 / 内部可变性 / 引用计数 / 强引用·弱引用 / 锁中毒 / 原子类型 / 数据竞争 / 未初始化 / 空位 niche / 契约 / 健全性）GLOSSARY 已有，照表执行。

### 衔接提示
- 回指章：6（`Vec` 用法）、16（variance / drop check）、17（智能指针；`Weak` 父子目录例）、19/20（线程、原子与内存序）、23（unsafe 契约、UB、泄漏 ≠ UB）、24（未初始化）、25（布局 / niche / 量法）。分工：本章不复述 UB 清单与 Miri 用法，布局数字只引 ch25 结论。
- 自检时改掉两处错误回指：原稿「第 31 章讨论过 panic 之后的类型状态」→ 实测 ch31 无此内容，改为「第 19 章讲过锁中毒」；原稿「裸指针让编译器给出过宽的可变性」→ Nomicon 原文是 too strict（`*mut T` 在 `T` 上不变），已改为「过于严格」并补 `Arc<&'static str>` 的例子。
- 若第 35 章（别名模型）要拿「合法别名序列」举例，可直接复用 `snippets/ch34/` 的 `Drain` / `MiniArc`。

### 遗留不确定
1. 两张 SVG 的目视渲染（见「图示」段）。
2. ~~`snippets/ch34/Cargo.lock`~~ / ~~`snippets/ch34/target/`~~：审校指出 snippets 不下沉 lock/target，已删除（重跑 `cargo test` 会一次性生成，不入库）。删除发生在最后一轮验证之后，验证时的私有 target 即 `snippets/ch34/target/`。
3. `Weak::weak_count` 在强引用归零后返回 0 是 std 文档约定，正文以「大意如此」转述；代码块里的 `weak_count = 0` 是实测输出。
4. 探针目录 `.scratch/tmp-62/` 已按要求删除。第二轮修复用的私有探针目录 `.scratch/tmp-62b/`（`&MutexGuard<Cell<u32>>` 共享探针、锚点校验脚本、输出比对脚本）收尾时一并删除。

---

## Comments（第二轮：审校 RvCh34 回执处理）

回执共 1 blocker + 1 major + 9 minor（R6-34-08 归协调者）。逐条处理如下，全部已改、已重跑。**只动了 `src/ch34-handwritten-collections.md` + `snippets/ch34/` + 本票。**

| 编号 | 处理 | 自查证据 |
|---|---|---|
| **R6-34-01 blocker** | 已修。`src/mini_mutex.rs` 增 `unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}`（SAFETY 注释写明：不写这条，守卫的 `Sync` 会从字段 `&'a Mutex<T>` 自动推导、边界退化成 `T: Send`）。正文 34.6 拆成两条：`Mutex<T>` 的边界是 `T: Send`，**`MutexGuard<T>` 的边界是 `T: Sync`**（它要发 `&T`）且必须手工写；小结同步修正。另外把 `guard` 锚点从「守卫结构体」扩到模块尾部，正文代码块现在完整含 `Mutex` 结构体 + 两组 `Send/Sync` impl（原先读者看不到 `UnsafeCell` 字段） | 独立复现并验证：探针 `.scratch/tmp-62b/share`（`&MutexGuard<Cell<u32>>` 双线程各写 200 万次）——**修复前编译通过、末值 2355873（丢 164 万）**；**修复后 `E0277 ... required for MutexGuard<'_, Cell<u32>> to implement Sync`**。crate 自身 17 测试仍全绿 |
| **R6-34-02 major** | 已修（取方案 a）。`examples/vec_steps.rs` 的 `Vec<[u64;300]>` 改成 40 次 push，打印头不再与实际不符；表格行与粘贴输出同步为 `1, 2, 4, 8, 16, 32, 64` | `cargo run --example vec_steps` 三行 `Vec` 均为 40 次，`min_non_zero_cap` 的分档对比仍成立（8 / 4 / 1 起步） |
| R6-34-03 minor | 已修：E0509 输出补回 `   \|          \|` 连接线 | 探针输出逐行比对 |
| R6-34-04 minor | 已修：panic 行改回实况反斜杠 `library\alloc\src\raw_vec\mod.rs:28:5`；注里补明「线程号、`/rustc/<hash>/` 前缀、末尾 `note: run with RUST_BACKTRACE=1 …` 一行均已略去」 | 探针原始输出 |
| R6-34-05 minor | 已修：`src/lib.rs` 文档改「没有**公开的** `unsafe fn`；唯一一个是私有的 `mini_vec::Drain::new`，带 `# Safety` 段，调用点写 `// SAFETY:`」 | — |
| R6-34-06 minor | 已修：`min_non_zero_cap` 恢复 stdout 源码排版（原来的压缩版删除） | 与 std 1.98 渲染源码逐行一致 |
| R6-34-07 minor | 已修：正文改成「两条真正会触发的失败路径（翻倍溢出 `usize`、`Layout::array` 返回 `Err`）+ 一条按契约不可达的防御检查」；`grow` 代码里那条 `size() <= isize::MAX` 分支补注释说明不可达 | — |
| R6-34-09 minor | 已修：注改为「`Running …` 行里的哈希由 crate 内容与工具链决定，同一次提交重跑逐字一致；随运行变化的只有打印顺序与耗时」 | 自验：连续两次 `cargo test` 的哈希完全一致（`mini_arc_mutex-9097e61abfd4dfd5`、`mini_vec-72deb703ab0e5e92`） |
| R6-34-10 minor | 已修：E0502 现场补修法——别先存 `&v[0]`（要值就 `let first = v[0];`），或改用 `for x in v.drain(..)` 边取边用 | — |
| R6-34-08 | 按指示不动（GLOSSARY 并表归协调者）；「守卫（guard）」首现体例保持 | — |

**第二轮重跑（本机 Windows x86-64、stable 1.98）**
1. `mdbook test -c ch34-handwritten-collections.md` → **exit 0**。
2. `cargo test --manifest-path snippets/ch34/Cargo.toml` → **exit 0**，17 测试全绿（6 + 11，私有 target `snippets/ch34/target/`）。
3. 粘贴输出现场复核：`vec_steps` / `arc_mutex` 两份示例输出**逐行命中**（脚本比对，缺失行 0）；`cargo test` 段的 `Running …` 行与全部 `test … ok` 行命中，只有 `finished in` 的耗时按正文注释浮动、以及正文声明略去的 `Doc-tests` 段。
4. 锚点 include：11 处全部命中（含改过的 `mini_mutex.rs:guard`，现在 65 行、首行 `pub struct Mutex<T> {`）。
5. 篇幅：正文 **5480 汉字**（修 blocker 时同步压缩了若干处冗余叙述，仍在 4000–5500 内）。

**遗留**：图中两张 SVG 的目视渲染仍待协调者收尾 build 确认（同第一轮）。此外无未处理项；若审校的 10 个安全探针里还有未闭环的怀疑，请把对应编号发我。
