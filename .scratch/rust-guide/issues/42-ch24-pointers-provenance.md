# 42 第 24 章 指针、provenance 与未初始化内存

Type: task
Status: claimed

## Question

写第四篇第二章：**指针、provenance 与未初始化内存**。

- **范围**：`NonNull`（与裸指针的取舍）；strict provenance（`ptr.addr()`、`with_addr`、`map_addr` 等 —— **先核对 1.98 快照的实际名字/稳定状态**，unstable 的标 nightly）；`&raw const`/`&raw mut`；`MaybeUninit`（`assume_init` 的安全条件、`assume_init_read`、`zeroed` 的坑）；`ManuallyDrop`；别名模型的直觉（Stacked/Tree Borrows 一小节，形式化 → 第 35 章）。
- **素材**：`research/02-knowledge-structure.md` 的 I14 行（依赖 I13；`nomicon/aliasing.html`、`nomicon/checked-uninit.html`、`nomicon/unchecked-uninit.html`、`std/ptr/`）；本机 1.98 快照逐条核对（provenance API 的稳定面变化快，逐条查 `std/ptr` 与 releases）。
- **边界**：别名模型形式化/UCG → 第 35 章；布局 → 第 25 章；FFI → 第 26 章；unsafe 基础 → 第 23 章（回指）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；unstable 逐条标注；Miri 演示标 nightly。
- **DoD**：§8 —— 独占 `src/ch24-pointers-provenance.md` + `snippets/ch24/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch24-pointers-provenance.md` 绿；探针 `.scratch/tmp-42/`；收尾 `claimed` + `## Comments`。

## Comments

- **文件**：`src/ch24-pointers-provenance.md`（八节齐全，修订后 5434 汉字）；新建 `snippets/ch24/nonnull_list.rs`、`provenance.rs`、`checked_uninit.rs`、`manually_drop.rs`、`raw_borrow.rs`（五个完整程序，全部用 `{{#rustdoc_include}}` 引入正文，因而被 `mdbook test` 编译并执行）。未动任何其他文件；探针目录 `.scratch/tmp-42/` 已删除。
- **自验**：`mdbook test -c ch24-pointers-provenance.md` → **exit 0**（修订后 `13 passed; 2 ignored`；别名反例改写成被测块之前是 `12 passed; 3 ignored`）。反证：临时向章末追加一个 `fn main() { let x: u32 = "…"; }` 块 → exit 101（`13 passed; 1 failed`），删掉后复跑 exit 0。
- **实测输出（stable 1.98 / edition 2024，`rustc --edition 2024 snippets/ch24/<f>.rs`，输出逐字粘进正文）**：
  - `nonnull_list`：`len = 3`、`values = [1, 2, 3]`、`Option<NonNull<Node>> = 8 字节`、`Option<*mut Node> = 16 字节`（niche 优化的真实差异）。
  - `provenance`：`with_addr(base.addr()+4)` 之后仍可解引用（= 20）、`map_addr` 打/清标记位后地址往返一致、`expose_provenance` → `with_exposed_provenance` 往返读到 10、`without_provenance(0x1000).is_null() == false`。
  - `checked_uninit`：`drop Noisy` 恰好出现两次（显式 `drop(taken)` 一次、`Once::drop` 里 `assume_init_drop` 一次）。
  - `manually_drop`：`drop Log(build)` 出现在 `close()` 那一刻；三次构造各析构一次，无重复。
  - `raw_borrow`：`read_unaligned` 读出 `0xdeadbeef`、`&raw mut` 写回 `tag`。
- **报错现场**：正文给两例真实诊断（逐字粘，路径按 doctest 惯例写成 `src/main.rs`）——`E0793`（对 packed 字段取引用，含 `this struct is 1-byte aligned…` 与 `creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)` 两行 note 及完整 help）、`deref_nullptr`（默认 deny 的 lint）。另有三个 `compile_fail` 佐证：`E0308`（`*mut u8` 传给 `NonNull<u8>`）、`E0015`（`addr` 不是 const fn）、`` `ManuallyDrop::<T>::take` is not yet stable as a const fn ``。六个 `compile_fail` 块逐块抽出单独 `rustc --edition 2024` 复跑，确认失败原因与行列同正文一致。
- **provenance API 逐条核对（本机 1.98 快照 `std/ptr`（含 `src/core/ptr/*.rs.html` 源码页）+ `releases.md`）**：
  - 稳定 **1.84**：`<ptr>::addr` / `expose_provenance` / `with_addr` / `map_addr`，`ptr::{with_exposed_provenance, with_exposed_provenance_mut, without_provenance, without_provenance_mut}`；`NonNull::{addr, with_addr, map_addr}` 同为 1.84（与 releases 1.84 段落的 Stabilized APIs 逐条对上）。
  - 稳定 **1.89**：`NonNull::{expose_provenance, with_exposed_provenance, without_provenance, from_ref, from_mut}`（releases 1.89 段落）。
  - **const 状态**：`ptr::without_provenance`/`_mut` 1.84 起 const；`ptr::with_exposed_provenance`/`_mut` 1.84 稳定、1.91 const（releases 1.91 段落列它）；`NonNull::without_provenance`/`with_exposed_provenance` 1.89 起 const；**`addr`/`with_addr`/`map_addr`/`expose_provenance` 在 1.98 仍不是 `const fn`**（快照源码签名 `pub fn addr(self) -> usize` 无 `const`，并用 `E0015` 实测）。
  - **旧名**：`expose_addr`/`from_exposed_addr` 在 1.98 全套 HTML 快照里已搜不到（正文提醒别照抄旧资料）。
  - **nightly（正文单独成表、逐条标 feature 门）**：`ptr::from_raw_parts`/`to_raw_parts`/`NonNull::from_raw_parts`（`ptr_metadata`）；`NonNull::{cast_slice, cast_array, cast_init, cast_uninit}`（`ptr_cast_slice`/`ptr_cast_array`/`cast_maybe_uninit`）；`MaybeUninit::array_assume_init`、`MaybeUninit::<[T; N]>::transpose`；`ManuallyDrop::{take, drop}` 与 `MaybeUninit::assume_init_drop` 的 **const** 用法。
  - 顺带核对的稳定版本：`&raw const`/`&raw mut`（`raw_ref_op`）= 1.82（RFC 2582，releases 1.82）；`addr_of!(/_mut!)` = 1.51；`MaybeUninit::{new, uninit}` 1.36、`assume_init` 1.36（const 1.59）、`assume_init_ref` 1.55、`assume_init_read` 1.60、`assume_init_drop` 1.60、`write` 1.55（const 1.85）、`zeroed` 1.36（const 1.75）；`ManuallyDrop::{new, into_inner}` 1.20（const 1.32）、`take` 1.42、`drop` 1.20；`NonNull::new` 1.25（const 1.85）、`dangling` 1.25；`<[MaybeUninit<T>]>::assume_init_*` 与 `MaybeUninit<[T; N]>` 的 `From`/`AsRef`/`AsMut` = 1.93 / 1.95；`NonNull` 是 `!Send`/`!Sync`（文档原文：`NonNull` pointers are not `Send` because the data they reference may be aliased.）。
- **Miri**：本机只有 `stable-x86_64-pc-windows-msvc`，**没有跑 Miri**。正文对 Miri 只写「nightly 才可用」（依据：Cargo Book 的 `cargo miri` 手册页原文 "This command is only available on the nightly channel."）外加一个 `rust,ignore` 反例，**没有粘贴任何 Miri 输出**。Stacked Borrows / Tree Borrows 只在「别名模型的直觉」一小节提名并强调是未定案的研究模型；`-Zmiri-tree-borrows*` 的说法只出现在指向 Unstable Book `llvm-writable` 一节（该节确实提到 `-Zmiri-tree-borrows-implicit-writes`）时。
- **遗留不确定**：
  1. 「别名模型的直觉」三条是 Reference 现行原则的中文转述（原文已逐字引在正文，含 "The exact aliasing rules are not determined yet"）；第 35 章若换措辞，需回改本章那一节。
  2. 正文那个 `rust,ignore` 反例（从共享借用引出可变裸指针后写入）按 Reference 的别名原则越线，但本机无 nightly，无法用 Miri 复现，故措辞是「按 Reference 已经越线，Miri（nightly）会判 UB」，未给 Miri 报文。
  3. 第 23 章此刻仍是占位；本章按「unsafe 四能力与 Miri 操作步骤在第 23 章、本章只回指」来写。若第 23 章最终也大段讲 Miri 能查出什么，建议收尾时分工（第 23 章留用法、本章留预期）。
- **新增术语（建议并表 `GLOSSARY.md`；正文首现已按「中文（English）」写）**：
  - **来历（provenance）**：指针携带的「这块地址属于哪块分配、由哪条引用派生而来」的信息。_Avoid_: 出处、来源（易与「引用的来源」混淆）。
  - **裸指针（raw pointer）**：`*const T`/`*mut T`。与「引用」（`&T`/`&mut T`）严格区分（沿用 `unsafe` 条与「悬垂引用」条的既有口径）。
  - **严格来历 / 暴露来历（strict / exposed provenance）**：std 文档对两组 API 的分类标签（原文 "This is a Strict Provenance API." / "This is an Exposed Provenance API."），正文沿用。
  - **别名模型（aliasing model）**：Reference 的别名规则（已有条目）加上在 Miri 里实现的候选模型（Stacked Borrows / Tree Borrows）。与「别名规则」区分：规则是要求，模型是用来做检查/推理的形式化。
  - **未初始化（uninitialized）**：内存尚未写入有效值；`MaybeUninit` 处理的那段时间。
  - **裸借用运算符（raw borrow operators）**：`&raw const` / `&raw mut`（Reference 用语）。
- **衔接提示**：正文回指第 23 章（五项能力、Miri 用法、「引用必须始终有效」），并把字段布局 → 第 25 章、跨语言 ABI → 第 26 章、别名模型形式化 → 第 35 章明确划界。`SUMMARY.md` 条目由协调者预置，本章未改动。
- **图示需求**：本章**未画 SVG**——「三种指针形态 × 地址/来历」与「`MaybeUninit` 的初始化状态」都用表格或三行文字说清了（按 WRITING §9「一张表或三行文字能说清的就不画」）。若审校认为仍需图，建议只加一张：`*mut T` / `NonNull<T>` / `usize` / `without_provenance` 四行的「地址 | 来历」两栏对照。

### 交叉审校修订（RvCh24：4 major + 6 minor + 1 可选；逐条处理）

1. **[major] 别名反例**：取 (b) 方案。旧写法实测确会撞上 `` error: assigning to `&T` is undefined behavior, consider using an `UnsafeCell` ``（`#[deny(invalid_reference_casting)]`，另带 `unused_mut` 警告）；现改为 `let mut v` + `let raw = &raw mut v;` + `let shared = &v;` + `unsafe { *raw = 2 };`，实测 rustc 1.98 **零警告编译通过并打印 2**，且已从 `rust,ignore` 改成普通 `rust` 块——所以它现在真的被 `mdbook test` 编译并执行。「rustc 抓不到」的口径改为：`rustc` 只拦得下最露骨的一类（从 `&T` 强转 `*mut T` 再写），绕一下就只能放行，别名规则是运行时语义层面的要求（教学点落在 Miri 上，保留）。
2. **[major] const 口径**：实测 `const fn` 里调 `NonNull::with_exposed_provenance` 报 `` `NonNull::<T>::with_exposed_provenance` is not yet stable as a const fn ``（徽标 `1.89.0 (const: unstable)`）；同一探针里的 `NonNull::without_provenance` 无错。正文三处已按「`ptr::without_provenance`/`_mut`（1.84）、`ptr::with_exposed_provenance`/`_mut`（1.91）、`NonNull::without_provenance`（1.89）**是** `const fn`；`NonNull::with_exposed_provenance` **不是**」改写：细节 bullet、nightly 表（新增一档）、小结。
3. **[major] 五项能力**：`:5`、`:19` 由「四个 unsafe 超能力」改为「unsafe 的五项能力」（对齐已裁定的 GLOSSARY 与定稿 ch23；std `keyword.unsafe` 页的五条也复核过：解引用裸指针 / 调用 `unsafe fn` / 改可变静态 / 实现 `unsafe trait` / 访问联合体字段）。
4. **[major] `checked_uninit` 反事实**：按本机探针实测重写。删掉 `self.init = false;` 后输出为：第 3 行 `Some(())` → `drop Noisy(first)` → `thread 'main' panicked …: 只能设置一次`（退出码 101）→ `drop Noisy(second)`（panic 展开析构 `set` 实参）→ `drop Noisy(first)`（重复释放）。正文改成「要害不是多打一行，而是 `drop Noisy(first)` 出现两次」，并附这段实测节选（路径与线程号已略去）。
5. **[minor] nightly 表**：加 `> [!WARNING]`（表前），写明 stable 上不可用与两种报错措辞。
6. **[minor] 术语首现**：`:14` 改「严格来历（strict provenance）」、`:17` 改「别名模型（aliasing model）」；其后以中文为主（`:35`「严格来历就是……」，小结「这套严格来历 API」），满足「正文至少再出现一次」。
7. **[minor] `:43` 无依据半句**：表格改为「『尽力而为』：文档说这类指针可能不被检查工具支持」，并在下方 bullet 补文档逐字原文："Due to its inherent ambiguity, `with_exposed_provenance` may not be supported by tools that help you to stay conformant with the Rust memory model. It is recommended to use Strict Provenance APIs such as `with_addr` wherever possible, in which case `addr` should be used instead of `expose_provenance`."
8. **[minor] `ptr::to_raw_parts`**：改为 `ptr::from_raw_parts`、`<ptr>::to_raw_parts`、`NonNull::from_raw_parts`（feature 门仍 `ptr_metadata`）。
9. **[minor] `:193`**：「两类场合」→「三类场合」；第二个例子按 Reference 原文对齐（注释里的 `&uninit.as_mut().field` 会造出指向未初始化 `bool` 的引用，替代写法是 `&raw mut (*uninit.as_mut_ptr()).field` 配 `write`）。
10. **[minor] NOTE 承诺**：删掉「Miri 的演示另标」，改为「本机只装了 stable 工具链，本章不含 Miri 实测输出」；紧邻别名反例另加一句「本机只有 stable，未用 Miri 复跑；判定依据是 Reference 的别名原则与两套候选模型的规则」。
11. **[可选] `:334`**：改为「`as` 属 std 所说的 Exposed Provenance 家族、结果只是『尽力而为』；而 `addr()` 本身不暴露来历，所以它什么也捡不到——地址对，不能解引用」，与 `:43` 表格口径一致。

**修订后复跑**：`mdbook test -c ch24-pointers-provenance.md` → **exit 0**（`13 passed; 2 ignored`——别名反例现为被测块；ignored 两个 = `MaybeUninit::zeroed` 的 UB 示例、练习 3）；再次注入失败反证 → exit 101（`13 passed; 1 failed`），还原后 exit 0。修订引入的扩写用同量级删减抵消，汉字数 **5434**（仍在 4000–5500）。
