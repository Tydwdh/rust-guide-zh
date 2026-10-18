# 41 第 23 章 unsafe 与 UB

Type: task
Status: claimed

## Question

写第四篇第一章：**unsafe 与 UB**。

- **范围**：`unsafe` 的四个超能力（解引用裸指针、调用 `unsafe fn`、访问可变静态、实现 `unsafe trait`）；`unsafe fn` 的契约与 2024 的 `unsafe_op_in_unsafe_fn`（unsafe fn 体内也要显式 `unsafe {}`）；`unsafe trait` 与「安全承诺」；UB 清单（Reference 版逐条）与「危险但非 UB」的区分；用 Miri 抓 UB（nightly，命令给全）。
- **素材**：`research/02-knowledge-structure.md` 的 I13 行（依赖 B1/I7；`reference/unsafety.html`、`reference/behavior-considered-undefined.html`、`nomicon/meet-safe-and-unsafe.html`、书 ch20.1）；本机 1.98 快照逐条核对。
- **边界**：指针/provenance/`MaybeUninit` → 第 24 章；布局/`repr` → 第 25 章；FFI → 第 26 章；别名模型 → 第 35 章。第 16/22 章的既有概念只回指。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；Miri 全部标 nightly；`unsafe_op_in_unsafe_fn` 标 2024 变化。
- **DoD**：按 `WRITING.md` §8 —— 独占 `src/ch23-unsafe-and-ub.md` + `snippets/ch23/` + 本票；禁 `git`/禁 `mdbook build`；自验 `mdbook test -c ch23-unsafe-and-ub.md` 绿；探针 `.scratch/tmp-41/`（收尾删）；运行输出真跑真粘；英文引文逐字核对；收尾 `Status: claimed` + `## Comments`（同 §8 清单）。

## Comments

2026-10-19 初稿完成（写作者 Ch23Unsafe），同日按交叉审校（RvCh23）修回（见文末「交叉审校修改记录」）。八节模板齐（`grep '^## '`：这一章解决什么问题 / 为什么这样设计 / 机制与规则 / 动手看看 / 报错现场 / 小结 / 练习 / 延伸阅读），练习 3 道、答案 3 个。正文 **5741 汉字**（最终稿；我的计法：全文去代码块后数 `[\u4e00-\u9fff]`。同法下 ch19=5919、ch20=5535、ch21=5247、ch22=5763，而它们票里分别报 5493/5435/4965/5495 —— 协调者口径约 ×0.95，故本章按同一口径约 5465，在 4000–5500 内）。

### 文件
- `src/ch23-unsafe-and-ub.md`（占位替换为成稿）
- `snippets/ch23/boundary.rs`（新增，本章唯一 snippet：`unsafe fn` 原语 + 安全外壳 + `unsafe trait Even` + 它授权的安全函数）
- 本票；未碰任何其他文件（SUMMARY、book.toml、GLOSSARY 等均只读）。探针 `.scratch/tmp-41/` 收尾已删；仓库根实测产生的 `boundary.exe`/`boundary.pdb` 也已删。

### 自验
- 门禁：`mdbook test -c ch23-unsafe-and-ub.md` → **exit 0**（stable 1.98 / edition 2024，mdbook 0.5.4）。成功时 mdbook 不打印 rustdoc 输出，故用注入反证确认测试真在跑。
- 测试清单（mdbook 按 `<章>::<节>` 逐块命名；最终稿共 **11** 个 Rust 块）：
  - 真跑 2：`动手看看` 里 `{{#rustdoc_include}}` 的 boundary.rs、`报错现场::现场二`；
  - `no_run`（编译不运行）3：`机制与规则::_3_4_`、`动手看看` 的 UB 演示、`练习::练习_1_`（交叉审校后新增）；
  - `compile_fail` 3：`机制与规则::_3_2_`（E0499）、`机制与规则::_3_5_可变静态`（static_mut_refs）、`报错现场::现场一`（E0133）；
  - `rust,ignore` 3：23.3 的 SAFETY 片段、练习 1 的片段、练习 2（都是无法独立编译的片段）。
- 注入反证（三次都还原并复跑绿）：
  1. `snippets/ch23/boundary.rs` 末尾追加 `fn _inject() -> i32 { "not an i32" }` → exit 101，失败点正是 `动手看看 (line 322)`（证明 include 的内容真被编译执行，不是「绿了但没测」）。
  2. 现场一的 `let v = *p;` 临时改成 `let v = unsafe { *p };` → exit 101（`Test compiled successfully, but it's marked compile_fail`）。
  3. 动手看看 UB 演示里的 `{b}` 临时改成 `{b_bad}` → exit 101（E0425），证明 `no_run` 块仍被编译。

### 真跑真粘的输出（全部本机 1.98 stable / edition 2024）
1. `rustc --edition 2024 snippets/ch23/boundary.rs -o boundary.exe` + `.\boundary.exe` → `get_or(1) = 2` / `get_or(9) = -1` / `value = 42, half = 21`，exit 0（pasted 在「动手看看」）。
2. UB 演示（`transmute` 出非法 `bool` + 释放 `Vec` 后读悬垂指针）：debug → `b = true` / `old[0] = 1`；`-O` → `b = false` / `old[0] = 0`；两次都 exit 0、零警告。
3. 撒谎的 `unsafe impl Even for i32` + `unreachable_unchecked`：debug → `thread 'main' (5456) panicked at src/main.rs:17:18:` + `unsafe precondition(s) violated: hint::unreachable_unchecked must never be reached` + `This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.`（进程以 0xC0000409 abort）；`-O` → 无输出、退出码 `0xC000001D`。
4. E0499：`error[E0499]: cannot borrow x as mutable more than once at a time`（另有 `unused_unsafe` 警告，正文注明节选把它省了）。
5. 现场一 E0133：`dereference of raw pointer is unsafe and requires unsafe block`（`src/main.rs:4:13`）+ `note: raw pointers may be null, dangling or unaligned; ...`。
6. 现场二（2024）：`warning[E0133]: call to unsafe function core::slice::<impl [T]>::get_unchecked is unsafe and requires unsafe block`（`src/main.rs:2:6`）+ `note: an unsafe function restricts its caller, but its body is safe by default` + `#[warn(unsafe_op_in_unsafe_fn)] (part of #[warn(rust_2024_compatibility)])`。
7. 2024 `static_mut_refs`：`error: creating a shared reference to mutable static`（`src/main.rs:6:24`）。另实测其修法：`let v = COUNTER;` 取只读副本可编译（正文写了这条）。
8. 顺带实测（未进正文，作为判断依据）：`invalid_reference_casting` 在 1.98 默认 deny（拦住「`&T` → `*mut T` 后写」这类历史 UB 写法）；`slice::get_unchecked` 越界在 debug 会撞 std 的可选 UB 检查（`unsafe precondition(s) violated: slice::get_unchecked requires that the index is within the slice`），`-O` 下无检查、进程以 0xC000001D 消失。

### 遗留 / 不确定
- **Miri 段（23.8）只有命令、没有运行结果**：本机只有 stable 1.98。我试装过 `rustup toolchain install nightly --profile minimal`（后台，600s 上限），下载未完成即被中断，留下一个坏的 nightly 目录；已 `rustup toolchain uninstall nightly` 清理，`rustup toolchain list` 回到只有 stable（+ 具名 1.98），机器恢复原状。所以正文 23.8 用 `> [!WARNING]` 明确标注：命令给全（`rustup +nightly component add miri`、`cargo +nightly miri test`、`cargo +nightly miri run`、`set MIRIFLAGS=-Zmiri-strict-provenance` 及 PowerShell/Unix 等价写法），并说明本章的 UB 证据改用「同一份代码 debug 与 `-O` 结果不同」。若验收机上有 nightly + miri，建议补一段真 Miri 输出替换该警告。
- UB 演示里的**具体数值与退出码**是本机 1.98 实测，但 UB 本身就是「没有任何结果被保证」——正文把这个性质讲成了论点，不是回避；换平台/版本出现别的数值属预期。
- 23.4 的 abort 输出含运行时线程号（`(5456)`），正文已注明「每次运行都不同」。
- **求协调者裁定：GLOSSARY 的 `unsafe` 条与官方口径不一致。** GLOSSARY 写「开启四个超能力（解引用裸指针、调用 `unsafe fn`、访问可变静态、实现 `unsafe trait`）」；`std` 关键词页与 The Book 都列**五条**（多出「访问联合体字段」，且 The Book 的措辞是「访问或修改可变静态变量」），Reference 列**八条**（另加 target_feature 调用、`extern` 块、unsafe 属性）。本章 23.1 按「常用五条 + Reference 八条」写。我没动 GLOSSARY（只读），请协调者并表时统一（建议对齐五条，并把「四」这一说法去掉）。

### 新增术语（供协调者并表）
- **未定义行为（undefined behavior，UB）**：GLOSSARY 已有，本章沿用。
- **健全的 / 不健全的（sound / unsound）**：对**任意**安全客户端都不会触发 UB 的 unsafe 代码称为健全。_Avoid_: 安全性、可靠的
- **裸指针（raw pointer）**：`*const T` / `*mut T`（GLOSSARY「悬垂引用」条出现过该词但未立条）。
- **可变静态（mutable static）**：`static mut`；读、写、取引用都需要 `unsafe`（2024 起取引用默认 deny）。_Avoid_: 可变全局变量
- **契约（contract）**：编译器无法检查、由人兑现的前提；`# Safety` 文档段与 `// SAFETY:` 注释是它的落点。
- **声明 / 兑现（declare / uphold）**：`unsafe fn`、`unsafe trait` 是声明契约；`unsafe {}`、`unsafe impl` 是兑现契约（取自 `std` 关键词页的两种用法）。
- **可选 UB 检查（optional UB check）**：1.98 在未优化构建里对部分 unsafe 前提插入的运行时检查（panic 文案：`unsafe precondition(s) violated: ... This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.`）。

### 衔接
- 回指：第 22 章 22.6（开篇引用其「不用 unsafe 就写不坏 pin」的结论）；第 16 章 `#[may_dangle]` + `unsafe impl<...> Drop`（23.4 末段）；第 19 章 `unsafe impl Send`（23.4）、`Mutex`（23.5 与练习 2）；第 20 章 `AtomicU32`（23.5）；第 9 章 `Ord` 的语义契约（「为什么这样设计」）。
- 前指：第 24 章（指针 / provenance / `MaybeUninit`；Miri 严格 provenance 模式）、第 25 章（布局与 `repr`）、第 26 章（`unsafe extern`、`#[unsafe(no_mangle)]`）、第 31 章（Miri/loom 入 CI）、第 33 章（`cargo fix --edition` 自动迁移 `unsafe_op_in_unsafe_fn`）、第 35 章（别名模型）。
- 给第 24/35 章的接口：23.6 第 4 条（别名规则）的原文引文与练习 1（用裸指针绕过借用检查后构造出两个 `&mut`）正是那两章的入口；本章刻意不展开。

### 图示需求（本章未画图）
- 首选：一张「`unsafe` 的两种用法」对照图——左栏 `unsafe fn` / `unsafe trait`（**声明契约**，箭头指向上方「调用方/实现方」），右栏 `unsafe {}` / `unsafe impl`（**兑现契约**，箭头指向下方「被调用的操作 / trait 的不变量」），中间一条竖线代表「编译器检查的边界」。
- 备选：两栏「UB 清单 vs 危险但非 UB」（左：能触发 UB 的 11 类；右：死锁、泄漏、整数溢出、逻辑错误）——正好承载 23.6/23.7 的对照。
- 若走图票，按 §9 硬规则（单块无空行、`viewBox` + `width="100%"` 无 height、只用主题变量、`role` + `aria-label`、实体转义 `<`/`>`/`&`）。

### 交叉审校（RvCh23）修改记录

协调者转来 1 × major + 13 × minor（四轴其余全过），已逐条修；只动 `src/ch23-unsafe-and-ub.md`、`snippets/ch23/` 与本票。major 那条与后续几处口径修正都**重新跑过探针核对**，不照抄结论。

1. 【major】练习 2 答案第 4 条与实测相反：实测 2021 下只有 `static_mut_refs` 警告（`creating a mutable reference to mutable static`，指向 `log` 里的 `LOGS.push`，exit 0），2024 下同一条直接是 error（exit 1），**没有任何** `unnecessary unsafe block`。已改写为「这四处 `unsafe` 标记本身都用对了地方——`log` 要改可变静态、`first_char` 调 `unwrap_unchecked`、`main` 调 `unsafe fn cast`……问题在于它们背后没有契约：`log` 与 `first_char` 是不健全的安全 API，`cast` 是多余且缺 `# Safety` 的 `unsafe fn`」，并补上「这份代码在 2024 下其实编译不过（`static_mut_refs` 默认 deny），同一条在 2021 下只是警告」。
2. `Mutate statics (including external ones)` —— 括号是 std 原文的一部分，已按原文恢复（原先是我的译注式写法）。
3. 五条 vs 八条的差集删掉一处漏项：Reference 第 2 条是「读**或**写可变或 unsafe 的**外部**静态变量」（读也算在内、点名外部静态），现与联合体那条的收紧（「给字段赋值除外」）一起写在差集里。
4. UB 检查口径两处订正：(a) 机制**不是** 1.98 新增 —— 正文改引 1.78 Libraries 段的原文 "Many unsafe precondition checks now run for user code with debug assertions enabled" 与 1.80 的 `assert_unsafe_precondition`（`unchecked_{add,sub,neg,mul,shl,shr}`），两条都逐字取自本机 1.98 快照的 `releases.md`；(b) 删掉「未优化构建里生效」，改为「**尽力而为**，可能不触发；开关跟随 `-C debug-assertions`（默认在未优化构建里开着）」，并给出本机实测矩阵（见下）。
5. Cargo Book 引文位置改为「`cargo miri` 手册页（DESCRIPTION）里写着」，去掉「第一句」（它其实是第三句）。
6. 「Miri 抓不到死锁」改为以 UB 为锚：「死锁与逻辑错误都不是 UB，也就不在 Miri 的 UB 检查清单里」。
7. 泄漏措辞改准：「23.7 那一类『危险但非 UB』它不当作 UB 报——内存泄漏另算，Miri 会单独报（除非加 `-Zmiri-ignore-leaks`）」。依据 Miri README：「Miri will also tell you about memory leaks: when there is memory still allocated at the end of the execution, and that memory is not reachable from a global `static`, Miri will raise an error.」与 flag 条目「`-Zmiri-ignore-leaks` disables the memory leak checker」。
8. 悬空回指修掉：`get_i32_unchecked` 此时还未出现 → 改为「待会儿 `snippets/ch23/boundary.rs` 里 `get_or` 的调用点就是这个样子」。
9. 「（23.9 的『安全抽象』）」与 23.9 用词不符 → 统一为「（23.9 说的『把 unsafe 关进安全 API』）」。
10. 「契约」首现补英文：开篇「每扇门后面的契约（contract）」。
11. 粘贴块**内部**的省略全部标注或补回：现场二补 `= note: for more information, see <…rust-2024/unsafe-op-in-unsafe-fn.html>` 一行，并注明末尾 `warning: 1 warning emitted`、`For more information…` 略去；23.5 补 `= note: `#[deny(static_mut_refs)]` (part of `#[deny(rust_2024_compatibility)]`) on by default`（外加同组的 `for more information` 行，正是「默认 deny」的直接证据）；23.2 的 E0499 块后补「输出按本章约定节选：开头还有一条 `unused_unsafe` 警告，末尾的 `error: aborting…` 与 `For more information…` 略去」。
12. 练习 1 答案补进被引用的那段改写代码（`rust,no_run`：`let p = &raw mut x;` + `let a = &mut *p;` + `let b = &mut x;` + `*a += *b;` + `println!("{x}")`），并把「`unsafe` 只额外开放五项能力、借用检查照跑」的说法理顺。
13. 23.6 第 7 条补 Reference 的限定：「——除非该平台明确文档化过这样用是安全的」。
14. 「代价是慢」改为「比本机执行慢得多——Miri 把程序当解释执行，不生成机器码」（Miri README 首段即「Miri is an Undefined Behavior detection tool … It can run binaries and test suites」，同段还给出它检出的 UB 类别，与 `-Zmiri-*` 的解释器语义一致）。

### 复跑与复检（修改后，2026-10-19）

- 门禁：`mdbook test -c ch23-unsafe-and-ub.md` → **exit 0**。代码块从 10 增至 **11**（练习 1 答案新增 1 个 `no_run` 块），分布：真跑 2 / `no_run` 3 / `compile_fail` 3 / `rust,ignore` 3（与上文「测试清单」一致）。
- 新增块的反证：把练习 1 答案里的 `let b = &mut x;` 临时改成 `let b = &mut y;` → exit 101（E0425），mdbook 报失败点就在该块；已还原并复跑绿。
- 为复核本条 major 与第 4、6、7、14 条另跑探针（`.scratch/tmp-41/`，收尾已删）：
  1. 练习 2 那份源码：`rustc --edition 2021` → 仅 1 条 `static_mut_refs` 警告、exit 0；`rustc --edition 2024` → `static_mut_refs` error、exit 1。
  2. UB 检查矩阵（`slice::get_unchecked` 越界，1.98）：默认构建 → 触发（非展开式 panic，退出 9）；`-C opt-level=2 -C debug-assertions=on` → 触发；`-C debug-assertions=off` → 不触发（读到垃圾值后 exit 0）；`-C opt-level=1`、`-C opt-level=2` → 不触发（进程以 0xC000001D 消失）。
- 篇幅：正文 **5741 汉字**（我的计法，评审修订净增约 149）；按协调者口径约 5465，仍在 4000–5500 内。
- 仍未做（与初稿同一处遗留）：Miri 真输出；`.scratch/tmp-41/` 已删，仓库根无残留可执行文件。
