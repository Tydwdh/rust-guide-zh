# 28 第 17 章 智能指针与内部可变性

Type: task
Status: claimed

## Question

写第 17 章：**智能指针与内部可变性** —— 共享、可变、跨线程三者的工具箱。

- **范围**：`Box`（堆分配、递归类型、与 `Box<dyn>` 的第 13 章衔接）；`Rc`/`Arc`（共享所有权、`Weak` 防环、`Rc` 不是 `Send`/`Sync`）；`Cell`/`RefCell`（运行时借用检查、`RefCell` 越界借用会 panic 的语义、`Cell` 的 `get/set/take/replace`）；`Mutex`/`RwLock`（跨线程内部可变性、锁中毒一句、与第 19 章的衔接）；`Cow`（写时克隆的典型场景）；`Deref`/`DerefMut` 的陷阱（自动强转的边界、`*` 的行为）；一张「`Send`/`Sync` 在这些类型上的分布」总表。
- **素材**：`research/02`（I7 + §5.2 里 `Rc`/`RefCell` 跨线程那条）；std 对应类型页 + book ch15；`reference` 的 interior-mutability 相关页。
- **边界**：并发基础（thread/Send/Sync 系统讲）→ 19；原子 → 20；别名模型 → 35。
- 篇幅 4500–6000 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注；可选「语言对照角」（C++ shared_ptr/unique_ptr、Java 引用）。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch17-smart-pointers.md`）。

## Comments

- 2026-10-18 初稿（票 28，写作者 `Ch17SmartPointers`）
  - **文件**：`src/ch17-smart-pointers.md`（H1 = 第 17 章 智能指针与内部可变性，八节模板 + 语言对照角）；片段 `snippets/ch17/tree.rs`（目录树：`Rc` + `Weak` + `RefCell` + `Cell` + `Cow`，完整可编译程序，正文用 `{{#rustdoc_include}}` 引入）。只写了本票范围内文件。
  - **自验**：`mdbook test -c ch17-smart-pointers.md` → **exit 0**。共 25 个被测块：`rust` 19 + `rust,compile_fail` 4 + `rust,should_panic` 2（含 include 的 tree.rs）。反向验证门禁有效：把 `list.sum()` 故意改错成 `list.summ()` → exit 101（`26 passed; 1 failed`），恢复后 exit 0。未跑 `mdbook build`/`linkcheck`（批次协议禁止）。
  - **实测输出清单**（全部本机 rustc 1.98.0 / edition 2024 真跑，正文 `text` 块逐字来自这些运行）：
    1. `Box` 递归链表：`sum = 3`、`size_of::<List>() = 16`；尺寸表 `Box<u64>=8`、`Box<[u64]>=16`、`Box<str>=16`、`Rc<String>=8`、`String=24`、`Cell<u64>=8`、`RefCell<u64>=16`、`RefCell<String>=32`、`Rc<str>=16`。
    2. `Rc`/`Weak`：`strong = 2, weak = 1` → 强引用清零后 `upgrade -> None`。
    3. 引用环泄漏（两个 `Rc` 互持，作用域结束无 `Drop` 输出）与 `Weak` 修法（`drop b` / `drop a`）；`Rc::try_unwrap`（唯一句柄 `Ok`，否则 `Err`）。
    4. `Cell`：`replace`/`take`/`update`/`Counter::hit(&self)` 全路径输出。
    5. `RefCell`：正/反向越界 panic 原文（`RefCell already mutably borrowed` / `RefCell already borrowed`，`src/main.rs:7:15`）；`try_borrow` → `Err(BorrowError)`、`try_borrow_mut` → `Err(BorrowMutError)`；`if let` 临时值变体在 2021 与 2024 **两个 edition 下都 panic**（2024 的临时值作用域调整只影响 `else`，已对 `edition-guide/rust-2024/temporary-if-let-scope.html` 逐字核对）。
    6. `Arc<Mutex>` 四线程计数 = `4000`；`RwLock` 两个读锁并存 = `1`/`1`，写后 `42`。`RwLockWriteGuard::downgrade` 实测**必须关联函数调用**（`g.downgrade()` 报 E0599 `this is an associated function, not a method`；`RwLockWriteGuard::downgrade(g)` 通过，1.92 起稳定）。
    7. `LazyLock`：初始化只发生一次；`CONFIG.force()` 报 E0599 `this is an associated function, not a method`，`LazyLock::force(&CONFIG)` / `*CONFIG` 可用。
    8. `Cow`：借用路径不分配、`to_mut()` 变 `Owned`（用 `matches!(.., Cow::Borrowed(_))` 断言）。
    9. `Deref`：`& String`/`&mut String`/`&&String` 都强转到 `&str`；泛型处**不强转**（`size_of_val` 量到 `String` 本体 24，`as_str()` 后 5）；同名方法外层遮蔽内层（`w.name() -> Wrapper`，`(*w).name() -> Inner`）；`*box` 可搬出、`*rc` 报 E0507。
    10. 报错现场：`Rc` 进线程 E0277、`Cell::get` 非 `Copy` E0599、`*rc` 搬出 E0507 —— 关键行均来自 1.98 真实诊断（正文按「最小复现放进 `src/main.rs`」给出行号，已核对与探针行号一致）。
    11. **`Send`/`Sync` 总表取证**：11 行 × `Send`/`Sync` 两列取自本机 `share/doc/rust/html/std` 各类型页的 `Send`/`Sync` 实现（脚本逐页解析并**按段归属**：`Rc`/`rc::Weak` 的 `!Send`/`!Sync`、`Arc`/`sync::Weak`、`Cell`/`RefCell`、`UnsafeCell` 的 `!Sync`、`Mutex`/`RwLock`/`OnceLock`、`LazyLock::Sync` 在 **Trait Implementations**；`Box`、`Cow` 的 Send+Sync、`UnsafeCell::Send`、`LazyLock::Send` 在 **Auto Trait Implementations**），再用 `fn send<T: Send>()` / `fn sync<T: Sync>()` 断言做正反样本交叉验证（通过：`Box/i32`、`Arc/i32`、`Cell<i32>`（仅 Send）、`RefCell<i32>`（仅 Send）、`Mutex/i32`、`RwLock/i32`、`Cow<str>`、`OnceLock/LazyLock`、`sync::Weak`、`Mutex<RefCell<i32>>`、`Arc<Mutex<Cell<i32>>>`；拒绝：`Rc`、`rc::Weak`、`Cell`/`RefCell`/`UnsafeCell` 的 `Sync`、`Arc<RefCell<i32>>`、`RwLock<Cell<i32>>`、`&Cell<i32>`）。判据四条（`&T: Send ⟺ T: Sync` 等）取自 `std/marker/trait.Sync.html` 原文。
  - **遗留 / 不确定**：
    - **篇幅**：正文 **6000 汉字**（不含代码块；含代码块与中文串共 6507），落在票面 4500–6000 区间内（审校修订后仍在内，见下一条）。初稿 7266 汉字，经过两轮压缩（7266 → 6574 → 5994），全程只删冗余表述、**没有删掉任何票面要求项**：11 类类型 + `Send`/`Sync` 总表 + 4 个报错现场 + 3 道练习（带答案）+ 八节模板 + 可选的「语言对照角」全部保留。
    - 未做 SVG：本章图示目前是 1 个 `text` ASCII 图（`Rc`/`Weak` 计数）+ 15 张表，未与票 22 的图示流程合流（见下方「图示需求」）。
    - 未跑 `mdbook build` 与 linkcheck2（批次协议禁止 `book/` 竞态）；延伸阅读 8 条链接按稳定入口给（不钉版本），断链由收尾统一 build 验证。
    - `Mutex<T>` 的大小随平台而变，正文未给具体字节数（避免写死 Windows 值）。
    - 语言对照角的 C++ 引文取自 C++ 工作草案 eel.is/c++draft（`[unique.ptr.general]/4`、`[util.smartptr.shared.general]/1`），Java 引文取自 `docs.oracle.com` Java SE 21 `java.lang.ref.WeakReference`；未引 cppreference（403 不可达）。
  - **新术语（建议进 GLOSSARY，协调者统一并表）**：
    - `引用计数 | reference counting`（建议中文「引用计数」，配 `Rc`/`Arc`/`strong_count`/`weak_count`）
    - `强引用 / 弱引用 | strong / weak reference`（`Rc` 与 `Weak` 的两档；避免只说「引用」）
    - `写时克隆 | clone-on-write`（`Cow`；不建议译「写时复制」，因为 `Cow` 走的是 `Clone` 不是 `Copy`）
    - `自动解引用 | autoderef`（方法解析的候选类型列表那套规则；与表内已有的「解引用强转 / deref coercion」是两个概念，正文已分别标注）
    - `互斥锁 / 读写锁 | mutex / read-write lock`（用 `Mutex`/`RwLock` 时首现给出中文名）
    - `锁中毒 | poisoning`（建议中文「锁中毒」，与 `PoisonError`/`into_inner` 配套）
    - 现有表内已覆盖、本章复用的：智能指针、内部可变性、解引用强转、数据竞争、panic、DST/胖指针、别名规则（无需新增）。
  - **邻章衔接（本章只写「（第 N 章展开）」的地方）**：
    - 第 13 章：`Box<dyn Trait>` 的胖指针/vtable、类型擦除（本章只点一句）。
    - 第 19 章：`thread`/`scope`/channel、`Send`/`Sync` 系统讲、锁的公平性与 `try_lock`、自锁与死锁（本章「留给第 19 章」）。
    - 第 20 章：原子类型与内存序（本章只提「`Cell` 换 `AtomicUsize`」）。
    - 第 22 章：`Box::pin` 与自引用（本章只点一句）。
    - 第 35 章：`UnsafeCell` 背后的别名模型（本章只点一句）。
    - 第 7 章：panic 与 `Result` 的取舍（`RefCell` 用 `borrow` 还是 `try_borrow` 直接复用该判据）。
    - 第 4 章：`Drop` 与移动语义（本章 `Box`/`Rc` 的释放点依赖它）。
  - **图示需求（留给票 22 / 后续补图；本章当前用 ASCII + 表格替代）**：
    1. **`Rc`/`Arc` 的堆布局与两档计数**（现在只有一个 `text` ASCII 图）：两个 `Rc<String>` 句柄 → 右侧共享分配（`strong`/`weak` 两格 + 值），`Weak` 指向同一分配但不占 `strong`。建议 SVG，宽 ~700。
    2. **两层检查并存**：`Rc<RefCell<T>>` 里「`Rc` 的计数（编译期不可见）」与「`RefCell` 的借用计数（运行期）」是两套独立机制——画成两个并排的计数块最直观。
    3. **`Ref`/`RefMut` guard 的借用区间**：`borrow()`/`borrow_mut()` 到 guard 被 drop 之间的区间条，冲突处标 panic，用来解释「guard 的作用域就是借用本身」（对应报错现场二）。
    4. **四格矩阵**（共享×线程）：`Rc`/`Arc` × `Cell`/`RefCell`/`Mutex`/`RwLock` 的 2×2 定位图，可与章末 `Send`/`Sync` 总表互为表里。
    5. 可选：`Box<[T]>`/`Box<str>`/`Rc<str>` 的「瘦指针 vs 胖指针（地址+长度）」对比条。

- 2026-10-18 审校修订（reviewer `RvCh17`：1 blocker + 4 major + 4 minor，全部落地；只动本章文件与本票）
  - **1（blocker）`Cell::update` 语义**：方法表由「用 `f(&mut T)` 就地改」改为「把当前值交给 `f`，用返回值替换（`FnOnce(T) -> T`；1.88 起）」。核实：`std/cell/struct.Cell.html` 的签名为 `pub fn update(&self, f: impl FnOnce(T) -> T)`（1.88.0 稳定），与同章示例 `k.update(|v| v * 10)` 一致。
  - **2（major）表格出处**：表格前一句改为「多数写在 Trait Implementations 段，`Box`、`Cow`、`UnsafeCell` 的 `Send`、`LazyLock` 的 `Send` 写在 Auto Trait Implementations 段」。**11×2 数值未动**（reviewer 复核全对）。
  - **3（major）`&mut String` 那一格缺证据**：示例改为 `let mut m = String::from("world");` + `takes_str(&mut m)`（本机实测编译通过、输出仍 `5`、无 warning），现在三条强转路径都有真实代码。
  - **4（major）`List` 布局说法**：删掉「判别字段 4 字节 + 4 字节填充」，改为「`i32` 4 字节 + 4 字节填充 + 8 字节 `Box` 指针；`Nil` 折进 `Box` 的空指针 niche，没有独立判别字段」。证据（本机实测）：`Box<u64>` = 8、`Option<Box<u64>>` = 8、`enum { T(u64, Box<u64>), N }` = 16（若真用独立判别字段会是 24）。
  - **5（major，协调者裁定）术语统一**：全文 5 处「自动强转」→「**解引用强转**」（含小节标题、小结、延伸阅读），与 `GLOSSARY.md` 及 ch04/ch18 对齐；「自动解引用（autoderef）」按裁定作为另一概念保留。
  - **6（minor）`Deref` 理由**：删「头号理由」，改为「列进三条理由之一（另两条：解引用可能意外失败、承诺公开 API 代价太大）」——核实 `std/ops/trait.Deref.html` 确是三条并列。
  - **7（minor）`LazyLock` WARNING**：接收者按 `&`/`&mut` 区分（`force`/`get` 取 `&`，`get_mut`/`force_mut` 取 `&mut`），版本按方法标注（`force` 1.80.0；`get`/`get_mut`/`force_mut` 1.94.0），「不是新坑」限定到 `force`。核实：各方法页版本徽标与签名（`force(this: &LazyLock<T, F>)`、`get_mut(this: &mut LazyLock<T, F>)`）。
  - **8（minor）E0507 的 `help` 不可照抄**：补一句「它建议的 `r.clone()` 换的是句柄，照抄会报 E0308（`expected String, found Rc<String>`）；要拿 `String` 得写 `(*r).clone()`」。实测：`let t: String = r.clone();` → E0308。
  - **复跑自验**：`mdbook test -c ch17-smart-pointers.md` → **exit 0**（25 个被测块不变：19 `rust` + 4 `compile_fail` + 2 `should_panic`）。正文 **6000 汉字**（含代码块与中文串 6507），仍在票面 4500–6000 内——修订新增的必要说明已用同量级删减抵消。
  - 按批次协议：未写 `## Answer`、未置 `resolved`。
