# 第 35 章 别名模型与形式化

## 这一章解决什么问题

第 24 章在结尾留了一个钩子：Rust Reference 只说「别名规则的确切形式还没定案」，然后给出几条原则；官方真正在用的两个候选模型叫 Stacked Borrows 与 Tree Borrows，它们活在 Miri 里。这一章把钩子收掉。

三个问题，逐个回答：

- **规范上现在有什么？** 只有原则，没有定案；Unsafe Code Guidelines（UCG）不是规范，而是一份基本停摆的研究文档。
- **两个候选模型差在哪？** Stacked Borrows 给每个字节维护一条「借用栈」；Tree Borrows 换成每块分配一棵「许可树」。差别落在对**读**的处理：栈模型里读会让上方的 `Unique` 失效；树模型里要先看那个节点**激活了没有**——没写过的 `Reserved` 读一下毫发无损，写过的 `Unique` 才被冻住。
- **编译器按哪一套在优化？** 都不是。stable 的 rustc 不做别名模型检查，它只是把「引用不别名」直接写进 LLVM 属性；模型只存在于 Miri（nightly）。

读完这一章，你应该能够：

- 分清「别名规则」「别名模型」「规范承诺」三件事，知道引用现成结论时该引哪一层；
- 读懂 Miri 的别名报错（tag 从哪来、被谁干掉、怎么干掉）；
- 说出哪些写法在两套模型下结论不同，以及为什么；
- 用 `-Zmiri-tree-borrows` 切换检查器，并知道切换后结论的强弱；
- 把第 24 章的 provenance API 放回抽象机的语境里。

边界：unsafe 基础与 Miri 安装见第 23 章，provenance API 用法见第 24 章；本章只补它们在模型里的位置。

## 为什么这样设计

### 优化器需要一份「什么算越线」的定义

第 24 章从 Nomicon 的 `compute` 例子出发解释过动机：只有知道 `&u32` 与 `&mut u32` 不重叠，编译器才敢把 `*input` 缓存进寄存器、才敢把 `*output = 1; *output *= 2;` 折成一次 `*output = 2;`。

问题是这种推理需要前提：得有一份可判定的「什么写法算越线」。太松，优化器不敢下注，Rust 丢掉最主要的性能承诺；太紧，大量真实的 unsafe 代码被判死，`std` 自己第一个过不去。所以需要的是**模型**，不是几行散文——它既是优化器可以依赖的下界，也是 unsafe 代码作者可以指望的上界。

### 三件事，别混

| 东西 | 是什么 | 谁维护 | 一句话定位 |
|---|---|---|---|
| 别名规则 | 「同一时刻谁能碰这块内存」的要求 | 语言规范（Reference） | 目前只有原则，**未定案** |
| 别名模型 | 把规则变成能逐步执行的判定器（tag、栈/树、状态机） | 研究项目 + Miri | Stacked Borrows / Tree Borrows，**非规范** |
| 规范承诺 | 官方文档写下的、可以依赖的保证 | Reference 与 std 文档 | 目前**没写**任何一条别名模型的内容 |

> [!NOTE]
> 这一章引用的一切，都在本机 1.98 文档快照（`reference/`、`nomicon/`、`unstable-book/`、`cargo/`、标准库源码）或写书时的官方源（UCG、Miri 仓库、`rust-lang/opsem-team`、PLDI'25 论文页）上逐条核对过。凡引用 Miri 侧的结论都标了来源。**本机只装 stable，没有 nightly，所以本章不含 Miri 的实跑输出**——出现 Miri 输出的地方是从 Miri 仓库里抄下来的，来源写在该处。

### 两个模型的先后

Stacked Borrows 从 2018 年开始设计与实现，最初的目标很朴素：给 Miri 一个可执行的判据，让它能报出「越线了」。Tree Borrows 是后来者，动机不是「换一种画法」，而是修栈模型的几处硬伤（下一节会看到）。PLDI 2025 那篇论文给的一个结论是：在 3 万个最常用的 crate 上，Tree Borrows 拒绝的测试用例比 Stacked Borrows 少 54%（原文："our evaluation on the 30 000 most widely used Rust crates shows that Tree Borrows rejects 54% fewer test cases than Stacked Borrows does."）。

## 机制与规则

### 规范这一层说的是什么话

Rust Reference 关于别名的整段话，1.98 快照与在线版逐字一致，开头是：

> Breaking the pointer aliasing rules. The exact aliasing rules are not determined yet, but here is an outline of the general principles:
>
> `&T` must point to memory that is not mutated while they are live (except for data inside an `UnsafeCell<U>`), and `&mut T` must point to memory that is not read or written by any pointer not derived from the reference and that no other reference points to while they are live. `Box<T>` is treated similar to `&'static mut T` for the purpose of these rules.

三个可以直接用的事实：规则**未定案**、只给了原则、措辞里没提任何模型。还有一条可验证的现状：1.98 快照里 "Stacked Borrows" 与 "Tree Borrows" 这两个名字出现在 `unstable-book` 的一页和标准库源码的注释里，但**没有**出现在 Reference 里。

### UCG 是什么，不是什么

Unsafe Code Guidelines 常被当成「Rust 的 unsafe 规范」。逐字看它自己的首页：

> This document is a past effort by the UCG WG to provide a "guide" for writing unsafe code that "recommends" what kinds of things unsafe code can and cannot do, and that documents which guarantees unsafe code may rely on. It is largely abandoned right now. However, the glossary is actively maintained.
>
> Unless stated otherwise, the information in the guide is mostly a "recommendation" and still subject to change.

"a past effort"、"largely abandoned right now"、"mostly a "recommendation""——三层免责声明。那份仓库现在的自我介绍是：

> The purpose of this repository is to collect and discuss all sorts of questions that come up when writing unsafe code. It is primarily used by the opsem team to track open questions around the operational semantics \[…\] Current consensus is documented in t-opsem FCPs and the Rust Language Reference.

也就是说：问「现在能依赖什么」要去 Reference 与 std 文档；t-opsem 的 FCP 清单是「哪些问题已经达成共识」的索引，而它自己写着：

> Here we track all the FCPs of the team. These reflect consensus decisions and are thus a useful starting point for figuring out what is (and is not) guaranteed. Note however that only the official documentation (the Reference and standard library docs) are actually normative stable guarantees.

写书时清单里没有任何一条是关于别名模型的。所以下面这些结论，**只能当成「检查器怎么判」来用，不能当成「语言保证了什么」**。

### Stacked Borrows：每个字节一条栈

Stacked Borrows（SB）给抽象机加了两样东西：每个指针带一个 **tag**（唯一编号），每个内存位置带一条**栈**。栈里每一项是 `Item { perm, tag, protector }`，权限只有四种：

| 权限 | 允许什么 |
|---|---|
| `Unique` | 唯一可变访问（`&mut`/`Box` 的重借得来） |
| `SharedReadWrite` | 可读可写，但和同「块」内其他 `SharedReadWrite` 等价（裸指针） |
| `SharedReadOnly` | 只读（普通 `&` 的非 `UnsafeCell` 部分） |
| `Disabled` | 什么都不给，只用来隔开两段 `SharedReadWrite` |

访问内存时的规则，逐字来自 UCG 的 `wip/stacked-borrows.md`：

> For write accesses, pop all *blocks* above the one containing the granting item. \[…\] For read accesses, disable all `Unique` items above the granting one: change their permission to `Disabled`.

译过来：
- **写**：找到批准这次访问的那一项，把它**上面**的一切弹掉——被弹掉的 tag 从此在这个位置不存在了，再用就是 UB。
- **读**：不弹，把上面所有 `Unique` 改成 `Disabled`（只挡住后续访问；位置留着，免得两段 `SharedReadWrite` 被合并）。

新权限只通过**重借（reborrow）**加进来。SB 在抽象机上新加了一条 MIR 语句叫 `Retag`，有四种场合：函数入口（`FnEntry`）、两阶段借用的准备（`TwoPhase`）、从引用转裸指针（`Raw`）、普通重借（`Default`）。`FnEntry` 还会给引用加**保护符（protector）**：在函数调用期间这个借用不允许被别的指针顶掉，顶掉就是 UB——这一条正是编译器敢给 `&mut` 形参贴 `noalias` 的依据。`rustc` 侧对应的开关是 nightly 的 `-Zcodegen-emit-retag`（Unstable Book 原话："emit retag function calls in generated code"，并注明 "This feature has no tracking issue, and is therefore likely internal to the compiler, not being intended for general use."）。

这套设计有两处公认的硬伤，UCG 的 SB 文档把它们写在「Biggest conceptual issues」一节，第一条原文是：

> Raw pointer casts generate fresh tags. This is a problem because it means we need to detect the *transitions* from references to raw pointers, which is not always easy. \[…\] It is also frequently surprising to programmers, e.g. when `addr_of_mut!(local)` is invalidated by direct writes to `local`.

第二条是「读不守栈纪律」（"On reads we do not follow a proper stack discipline."）——Tree Borrows 就是冲着它来的。

下面的图把两种记账方式摊开：

<svg viewBox="0 0 720 250" width="100%" role="img" aria-label="Stacked Borrows 与 Tree Borrows 的结构对照：左边是每个内存位置一条自底向上的借用栈，写会弹掉批准项上方的一切、读把上方的 Unique 置为 Disabled；右边是每块分配一棵许可树，节点按派生关系相连，外来读对未写过的 Reserved 无影响、对已激活的 Unique 才冻成 Frozen，外来写才把它判死成 Disabled" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">同一个内存位置，两种记账方式</text>
<g stroke="var(--quote-border)" stroke-width="1.5"><line x1="366" y1="32" x2="366" y2="238" /></g>
<text x="0" y="46" font-size="13" fill="var(--links)">Stacked Borrows：每个字节一条栈</text>
<text x="380" y="46" font-size="13" fill="var(--links)">Tree Borrows：每块分配一棵树</text>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<rect x="24" y="62" width="196" height="28" rx="4" />
<rect x="24" y="94" width="196" height="28" rx="4" />
<rect x="24" y="126" width="196" height="28" rx="4" />
<rect x="24" y="176" width="196" height="32" rx="4" stroke="var(--links)" />
</g>
<text x="34" y="81" font-size="12" font-family="monospace" fill="currentColor">#3 Unique（xref）</text>
<text x="34" y="113" font-size="12" font-family="monospace" fill="currentColor">#2 SharedReadWrite（raw）</text>
<text x="34" y="145" font-size="12" font-family="monospace" fill="currentColor">#1 Unique（x）</text>
<text x="34" y="197" font-size="12" fill="currentColor">内存位置：x 的那个字节</text>
<text x="232" y="74" font-size="12" fill="currentColor">← 栈顶：最新</text>
<text x="232" y="166" font-size="12" fill="currentColor">← 栈底：这块分配</text>
<text x="0" y="232" font-size="12" fill="var(--links)">写：弹掉批准项上方的一切　读：上方的 Unique 置为 Disabled</text>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<rect x="470" y="62" width="150" height="28" rx="4" />
<rect x="382" y="132" width="140" height="28" rx="4" />
<rect x="562" y="132" width="140" height="28" rx="4" />
<line x1="530" y1="90" x2="452" y2="132" stroke="var(--quote-border)" />
<line x1="552" y1="90" x2="632" y2="132" stroke="var(--quote-border)" />
</g>
<text x="486" y="81" font-size="12" font-family="monospace" fill="currentColor">#1（分配根）</text>
<text x="392" y="151" font-size="12" font-family="monospace" fill="currentColor">#2 Reserved → Unique</text>
<text x="572" y="151" font-size="12" font-family="monospace" fill="currentColor">#3 Frozen → Disabled</text>
<text x="396" y="182" font-size="12" fill="currentColor">边 = 「从谁派生」；每个节点对每个字节各有一份权限</text>
<text x="380" y="212" font-size="12" fill="var(--links)">读：Reserved 不变；Unique → Frozen</text>
<text x="380" y="232" font-size="12" fill="var(--links)">写：一律 → Disabled</text>
</svg>

### Tree Borrows：每块分配一棵树

Tree Borrows（TB）保留了 tag、重借、保护符这些概念，把「每个字节一条栈」换成「每块分配一棵树」：指针的 tag 是树上一个节点，每个节点对分配里的每个字节持有一份权限，权限随访问演化。Miri 里这套状态机的取值是（源码 `tree_borrows/perms.rs` 的注释）：

| 权限 | 谁会处于这个状态 | 外来读 | 外来写 |
|---|---|---|---|
| `ReservedFrz` | `&mut` 刚重借出来、还没被写过 | 无影响（仍可写） | `Disabled` |
| `ReservedIM` | 同上，且目标类型内部可变 | 无影响 | 无影响 |
| `Unique` | 被写过之后（已激活）的 `&mut` | `Frozen` | `Disabled` |
| `Frozen` | `&` 本身，或被冻住的 `&mut` | 无影响 | `Disabled` |
| `Disabled` | 已经判死的 | 无影响 | 无影响 |
| `Cell` | 内部可变（`UnsafeCell`）的那部分字节 | 无影响 | 无影响 |

子访问那半张表：`Reserved*` 的子写把它**激活**成 `Unique`（受保护、且已被外来读标过 `conflicted` 时子写 UB）；`Frozen`/`Disabled` 的子写 UB，`Disabled` 连子读都 UB。

关键差别在「单个节点被外来读时怎么变」，而且 TB 要先看这个节点**激活了没有**：

- SB 里没有「还没写过」这个区分——`&mut` 一重借就在栈上压一个 `Unique`；外来**读**把它直接判死（置 `Disabled`）。
- TB 里，未写过的 `&mut` 是 `ReservedFrz`/`ReservedIM`，外来读**什么都不做**——`tree_borrows/perms.rs` 的注释逐字是："Before activation and without protectors, foreign reads are fine. That's the entire point of 2-phase borrows."；写过之后的 `Unique` 才会被外来读冻成 `Frozen`（仍可读、不能再写）。
- 外来**写**两边都狠：TB 里除了 `Cell` 与未受保护的 `ReservedIM`，其余一律 `Disabled`。

另外几处细差别值得记住：

- **TB 的「重借」算读。** Miri 测试文件 `tests/pass/tree_borrows/reborrow-is-read.rs` 的注释逐字是："To check that a reborrow is counted as a Read access, we use a reborrow with no additional Read to Freeze an Unique pointer."——所以一次重借就相当于一次读：来自别的节点时，已激活的 `Unique` 被冻成 `Frozen`，未写过的 `Reserved` 不受影响（受保护时才会直接判死）。
- **裸指针的重借在 TB 里是空操作**：UCG 的 `wip/tree-borrows.md` 写着 "Tree Borrows treats raw pointer retags as NOPs, i.e. it does not distinguish a raw pointer from the references it is derived from."
- **TB 没有子对象来历（subobject provenance）**：同一份文档写着 "Tree Borrows does *not* have subobject provenance, meaning that retags do not shrink the set of offsets that a reference can be used to access."——SB 会缩：从 `&mut` 重借出指向某个字段的引用之后，原引用的可访问范围就变小了。

代价也有，Miri 的 README 说得很直白：

> Tree Borrows is even more experimental than Stacked Borrows. \[…\] it is likely that the eventual final aliasing model of Rust will be stricter than Tree Borrows. In other words, if you use Tree Borrows, even if your code is accepted today, it might be declared UB in the future. This is much less likely with Stacked Borrows.

一句话：**TB 更宽松，因此未来被收紧的风险更大**。它还缺一块：`&mut` 第一次写之前不算 writable，与编译器将来可能启用的 `writable` 属性对不上（UCG #584，写书时 open）。

### 同一段代码，两个结论

Miri 仓库里有个测试文件名字就叫 `sb_fails.rs`，注释第一行是 "These tests fail Stacked Borrows, but pass Tree Borrows."。它是本章最省事的对照材料（下面是它的节选，五处用例都在，只调了 `main` 里的顺序）：

```rust
{{#rustdoc_include ../snippets/ch35/sb_fails.rs}}
```

这个程序在本机 stable 1.98 上编译、运行**都没有输出，退出码 0**——稳定编译器根本不做别名模型检查。换成 Miri，SB 会在第一处 `pass_invalid_mut` 报 UB（上游文件把 `fnentry_invalidation` 放最前，那处一样会报），TB 则一路通过。只看 `pass_invalid_mut` 这一处：

```text
let xref = unsafe { &mut *xraw };   // SB：Unique 压在 xraw 的 SharedReadWrite 之上
let _val = unsafe { *xraw };        // 这次「读」按 SB 的规则把 xref 置为 Disabled
foo(xref);                          // 于是这里重借 xref → UB
```

TB 里同一步**根本不动 `xref`**：它还没被写过，状态是 `Reserved`，而外来读对 `Reserved` 无影响——所以它依然可写（上面这段只有读和重借，没有写 `xref`）。会出事的是写：子写把它激活成 `Unique`，此后再被外来读才会冻成 `Frozen`。五个用例整理成表：

| 用例（都取自 Miri 的 `tests/pass/tree_borrows/sb_fails.rs`） | SB | TB |
|---|---|---|
| `fnentry_invalidation`：`&mut x` 转成裸指针，再把 `x` 的可变借用传进函数，随后读那个裸指针 | 被保护的 `Unique` 顶掉了它 → UB | 那次重借没有禁止读它，通过 |
| `pass_invalid_mut`：读一下裸指针，再把它派生出的 `&mut` 传出去 | 那个 `&mut` 判死 → UB | `Reserved` 不受这次读影响，仍可写 |
| `return_invalid_mut`：同上，区别在返回值位置 | 同上 → UB | 同上 |
| `static_memory_modification`：`transmute::<&usize, &mut usize>` 但从不写 | 造这个 `&mut` 就算越线 → UB | 允许（注释原话："in TB we are allowed to transmute as long as we don't write."） |
| `interior_mut_reborrow`：直接写本地变量，其间还有一个内部可变重借 | 会误报 | 通过 |

最后一行上游的注释是 "This one is about direct writes to local variables not being in conflict with interior mutable reborrows."

不过改「读」的语义不是免费的。UCG 的 SB 文档在同一节里说，把「读」从「判死」改成「冻结上方的 `Unique`」（文档语：the Tree Borrows route）会让一大票常见写法变成 UB，给的例子是 `ptr::copy_nonoverlapping(src.as_ptr(), dest.as_mut_ptr(), dest.len());`：`dest.len()` 这次共享重借会冻住刚拿到的裸指针，之后的写就 UB；文档的结论原话是 "It's not clear how this could be fixed without going all the way to trees."。（这是 SB 文档自己的论证，写作时间早于 TB 落地；TB 现行实现里裸指针重借是空操作，具体情形并不完全相同。）

<svg viewBox="0 0 720 210" width="100%" role="img" aria-label="外来读对同一份可变许可的两种处理：Stacked Borrows 把栈上方的 Unique 置为 Disabled，该 tag 从此不能再用；Tree Borrows 要先看节点激活了没有——未写过的 Reserved 不受影响、仍可写，已激活的 Unique 才冻成 Frozen（可读、不可写）" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">同一次「读」，两种后果</text>
<g stroke="var(--quote-border)" stroke-width="1.5"><line x1="358" y1="34" x2="358" y2="196" /></g>
<text x="0" y="46" font-size="13" fill="var(--links)">Stacked Borrows</text>
<text x="0" y="68" font-size="12" fill="currentColor">读之前：</text>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<rect x="76" y="56" width="120" height="24" rx="4" />
<rect x="76" y="84" width="120" height="24" rx="4" />
<rect x="76" y="112" width="120" height="24" rx="4" />
</g>
<text x="84" y="73" font-size="12" font-family="monospace" fill="currentColor">#3 Unique</text>
<text x="84" y="101" font-size="12" font-family="monospace" fill="currentColor">#2 SharedReadWrite</text>
<text x="84" y="129" font-size="12" font-family="monospace" fill="currentColor">#1 Unique</text>
<text x="210" y="73" font-size="12" fill="currentColor">← 被读到的这一层以上，全部作废</text>
<text x="0" y="164" font-size="12" fill="currentColor">读之后：#3 变成 Disabled，它的 tag 从此</text>
<text x="0" y="184" font-size="12" fill="var(--warning-border)">不能读也不能写；再用就是 UB。</text>
<text x="372" y="46" font-size="13" fill="var(--links)">Tree Borrows（还要看节点状态）</text>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<rect x="372" y="56" width="340" height="46" rx="4" />
<rect x="372" y="110" width="340" height="46" rx="4" />
</g>
<text x="382" y="76" font-size="12" font-family="monospace" fill="currentColor">#3 Reserved（没被写过）</text>
<text x="382" y="94" font-size="12" fill="var(--links)">这次读什么都不做 → 仍然可写</text>
<text x="382" y="130" font-size="12" font-family="monospace" fill="currentColor">#3 Unique（写过了）</text>
<text x="382" y="148" font-size="12" fill="var(--links)">这次读 → Frozen：可读、不可写</text>
<text x="372" y="180" font-size="12" fill="currentColor">外来写则一律 → Disabled</text>
<text x="372" y="198" font-size="12" fill="currentColor">（Cell、未受保护的 ReservedIM 除外）</text>
</svg>

### provenance 的深化：抽象机里多出来的状态

第 24 章讲了严格来历与暴露来历两组 API 怎么用，这里补它们在模型里的位置。UCG 词汇表（UCG 仓库里 "actively maintained" 的那部分）对「指针来历」的定义是：

> The provenance of a pointer is used to distinguish pointers that point to the same memory address (i.e., pointers that, when cast to `usize`, will compare equal). Provenance is extra state that only exists in the Rust Abstract Machine; it is needed to specify program behavior but not present any more when the program runs on real hardware. In other words, pointers that only differ in their provenance can not be distinguished any more in the final binary (but provenance can influence how the compiler translates the program).
>
> The exact form of provenance in Rust is unclear.

"exact form is unclear" 和 Reference 的 "exact aliasing rules are not determined yet" 是一对。词汇表还给了一个很好用的具体形态：抽象机里的字节不是 `u8`，而是

```rust
pub enum AbstractByte<Provenance> {
    /// 未初始化的字节。
    Uninit,
    /// 已初始化的字节；如果它编码的是指针，可以带来历。
    Init(u8, Option<Provenance>),
}
```

（上面这段是词汇表里给出的原文结构，注释按原文翻译。）也就是说**来历保存在数据里**：指针被存回内存、之后又读出来，来历跟着字节一起走。这一条直接解释了第 24 章的几件事：

- 为什么 `addr()` 算「不暴露来历」：它把 `Option<Provenance>` 丢掉，又没在全局表里留下记录，所以那个地址回不到任何分配。
- 为什么 `with_addr` 换了地址还照样能用：`Option<Provenance>` 原样留着。
- 为什么暴露来历只能「尽力而为」：一旦允许整数与指针自由互转，抽象机就得维护一张「所有被暴露过的分配」的表，然后**猜**。Miri README 在 `-Zmiri-permissive-provenance` 那条里逐字写着，这类操作 "are not efficiently and accurately implementable in a sanitizer"，所以 "will necessarily miss some bugs"；而 `-Zmiri-strict-provenance` 直接让整数转指针停机："This means that casting an integer to a pointer will stop execution because the provenance of the pointer cannot be determined."
- 为什么 `&raw`（第 24 章那对裸借用运算符）在模型里被单独对待：它不造引用，也就没有「引用型重借」那一步。SB 会给「引用转裸指针」一次 `Raw` retag（派生新 tag），TB 把裸指针重借当空操作——前者被列进 SB 文档的「最头疼的设计问题」（见上述 `addr_of_mut!(local)`）。

还有一条尚未定案的交互：别名模型与并发/数据竞争模型怎么配合，挂在 UCG 的 #585，标题是 "How does the aliasing model interact with the concurrency model?"；TB 文档也承认「保护符结束时的写」与数据竞争模型的交互 "is not fully resolved"。

### 形式化到什么程度，编译器又走到哪一步

分三层说（都以 1.98 为准）：

**一、stable 的 rustc 不启用任何别名模型。** 两个模型都只活在 Miri 里，而 `cargo miri` 手册页（本机 1.98 快照的 Cargo Book）第一句就是 "This command is only available on the nightly channel."。stable 上能拦住别名违规的只有两把刷子：借用检查器（只认引用），以及默认 deny 的两个 lint——`invalid_reference_casting`（抓「从 `&T` 强转出 `*mut T` 再写」）和 `mutable_transmutes`（抓 `transmute::<&T, &mut T>`，两者都见下一节）。Miri 的默认模型是 **Stacked Borrows**，加 `-Zmiri-tree-borrows` 才切到 Tree Borrows；`-Zmiri-disable-stacked-borrows` 则两者都关掉，README 注明那会漏掉别名违规，并且 "Using this flag is **unsound**"。

**二、编译器已经在按「引用不别名」下注。** 这是本章唯一能在本机 stable 上亲手验证的一环，见下一节。形式化没定案不等于编译器不动作——它下的是 LLVM 那一层的注，而这些属性对不对本该由别名模型保证；接口也还在 nightly（`Retag` 是抽象机的新语句，`-Zcodegen-emit-retag` 被 Unstable Book 归为内部设施）。TB 与编译器新想法的关系同样如此，`llvm-writable` 一页写道："The Miri tool can be used to detect some problematic cases. However, note that when using Tree Borrows, you must set -Zmiri-tree-borrows-implicit-writes to ensure that the UB arising from these implicit writes is detected."

**三、模型已经在影响真实代码。** 标准库源码里就留着两处：`alloc/sync.rs` 里 `Arc<[T]>::default` 的注释说，它特意不把 `ArcInner<[u8; 1]>` 的引用交出去，因为 "we don't shrink the range of bytes the ptr is allowed to access under Stacked Borrows"，紧跟一句 "(Note that `NonNull::from(&STATIC_INNER_SLICE.inner)` is fine under Tree Borrows.)"；`core/slice/sort/unstable/quicksort.rs` 里写着某个更顺的写法 "This is only an issue under Stacked Borrows, Tree Borrows accepts the intuitive code using GapGuard as valid."。规范没定案，但 `std` 得先在一个具体模型下跑通。

## 动手看看

### 一、看编译器怎么下注：`--emit=llvm-ir`

`snippets/ch35/ir_probe.rs` 就是 Nomicon 那个 `compute`（去掉 `main`、加上导出用的 `extern "C"`）：

```console
$ rustc --edition 2024 -O --emit=llvm-ir snippets/ch35/ir_probe.rs
$ grep -n "define void @compute" ir_probe.ll
```

```text
7:define void @compute(ptr noalias nofree noundef readonly align 4 captures(none) dereferenceable(4) %input, ptr noalias nofree noundef align 4 captures(none) dereferenceable(4) %output) unnamed_addr #0 {
```

（1.98.0 实跑。）两个形参都带 `noalias`：`input` 还多个 `readonly`，两个都有 `dereferenceable(4)`——编译器替你把别名规则写进了 IR。同一份 IR 里还有 Nomicon 描述的那个折半动作：`> 10` 那条路径直接把常量 `2` 存进 `output`，中间的 `*output *= 2;` 不见了：

```text
bb3:
  %1 = phi i32 [ %0, %bb2.bb3_crit_edge ], [ 2, %start ]
  store i32 %1, ptr %output, align 4
```

把它跑起来（`x = 20`、`out` 初值 `3`）得到 `2`：

```rust
#[unsafe(no_mangle)]
pub extern "C" fn compute(input: &u32, output: &mut u32) {
    if *input > 10 {
        *output = 1;
    }
    if *input > 5 {
        *output *= 2;
    }
}

fn main() {
    let (x, mut out) = (20u32, 3u32);
    compute(&x, &mut out);
    println!("{out}");
}
```

```text
2
```

> [!NOTE]
> 这段优化**依赖** `noalias`：如果 `output` 恰好就是 `input` 那块内存，`*output = 1` 之后 `*input > 5` 的判断就失去意义了。IR 属性是编译器单方面贴的，它不检查你有没有守住规则——这正是「跑起来看着对」不能当证据的原因（第 23 章）。

### 二、SB 报错、TB 放行的那段程序

见上一节的 `snippets/ch35/sb_fails.rs`（真实运行结果：无输出、退出码 0）。补上「交给 Miri 之后两套模型分别怎么说」：

| 检查器 | 结论 |
|---|---|
| `cargo +nightly miri test`（默认 SB） | 第一处 `pass_invalid_mut` 报 UB：`xref` 的 tag 已经不在借用栈里（按本章片段的顺序；上游文件的第一处是 `fnentry_invalidation`） |
| 先 `set MIRIFLAGS=-Zmiri-tree-borrows`，再跑同一个命令 | 通过 |

（这两行是文档化的行为，不是本机输出。SB 的实际报错文本见下一节。）

### 三、Miri 的别名相关开关（命令级）

安装与在测试里跑起来的方法在第 23 章，这里只列与别名模型、provenance 有关的开关；它们全部来自 Miri 仓库的 README（写书时主分支）。

```console
$ rustup +nightly component add miri
$ cargo +nightly miri test
```

切模型、调 provenance 检查强度：

```console
$ set MIRIFLAGS=-Zmiri-tree-borrows
$ cargo +nightly miri test
$ set MIRIFLAGS=-Zmiri-strict-provenance
$ cargo +nightly miri test
```

（PowerShell 里把 `set MIRIFLAGS=…` 写成 `$env:MIRIFLAGS = "…"`，第 23 章有说明。）

排障时最有用的一条是给某个 tag 打开追踪（README 原话：它会在 tag 被创建、以及被从借用栈里弹出时各打一次回溯）：

```console
$ set MIRIFLAGS=-Zmiri-track-pointer-tag=1
$ cargo +nightly miri test
```

> [!WARNING]
> 本机只有 stable 1.98，而 Miri 在 stable 上**不存在**（`cargo miri` 手册页："This command is only available on the nightly channel."）。上面这些命令本章**没有跑过**，也没有输出可贴；它们的语义逐字来自 Miri README 与 UCG 的模型文档。真要跑，请按第 23 章装 nightly 组件。
>
> 两条 caveat 也记住：Miri **不查全部** UB（README："Miri does **not catch every violation of the Rust specification** in your program, not least because there is no such specification."）；TB 比 SB 更实验，今天通过不等于将来通过。

## 报错现场

### 一、rustc 自己就能拦下的两类

第一类是从 `&T` 造出可写指针。最小复现（存成 `src/main.rs`）：

```rust,compile_fail
fn main() {
    let x = 0u8;
    let p = &x as *const u8 as *mut u8;
    unsafe { *p = 1 };
    println!("{x}");
}
```

以 Rust 1.98 为例，完整输出：

```text
error: assigning to `&T` is undefined behavior, consider using an `UnsafeCell`
 --> src/main.rs:4:14
  |
3 |     let p = &x as *const u8 as *mut u8;
  |             -------------------------- casting happened here
4 |     unsafe { *p = 1 };
  |              ^^^^^^
  |
  = note: for more information, visit <https://doc.rust-lang.org/book/ch15-05-interior-mutability.html>
  = note: `#[deny(invalid_reference_casting)]` on by default

error: aborting due to 1 previous error
```

第二类是直接把 `&T` 变形成 `&mut T`（`sb_fails.rs` 里那个用例在 stable 上就是这么被挡住的）：

```rust,compile_fail
fn main() {
    let x = 0usize;
    let m: &mut usize = unsafe { std::mem::transmute::<&usize, &mut usize>(&x) };
    println!("{m}");
}
```

```text
error: transmuting &T to &mut T is undefined behavior, even if the reference is unused, consider instead using an UnsafeCell
 --> src/main.rs:3:34
  |
3 |     let m: &mut usize = unsafe { std::mem::transmute::<&usize, &mut usize>(&x) };
  |                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[deny(mutable_transmutes)]` on by default

error: aborting due to 1 previous error
```

定位路径：两条都没有错误编号（它们是 lint，不是 `E····`），找它们靠搜 lint 名，或读输出里 `#[deny(...)]` 那一行。它们守的正是别名规则的底线：`&T` 不能造出可写指针，也不能自己变成 `&mut T`。

### 二、rustc 放行之后：只有模型工具能说的话

绕开这两条 lint 很容易（比如先把地址转成整数再转回来，见第 24 章），之后诊断的重担就落到 Miri 身上。下面这段**抄自 Miri 仓库的测试输出文件** `tests/fail/stacked_borrows/pass_invalid_mut.stderr`（写书时主分支，提交 `446c2bac99b7`；本机没有 nightly，**未运行**；`<TAG>`、`ALLOC[0x0]`、`LL:CC` 是 Miri 测试套件对标记号与行列所做的归一化写法）。引文是**节选**，末尾原本的 `note: some details are omitted …` 与 `error: aborting …` 两行已略去：

```text
error: Undefined Behavior: trying to retag from <TAG> for SharedReadWrite permission at ALLOC[0x0], but that tag does not exist in the borrow stack for this location
  --> tests/fail/stacked_borrows/pass_invalid_mut.rs:LL:CC
   |
LL |     foo(xref);
   |         ^^^^ this error occurs as part of two-phase retag at ALLOC[0x0..0x4]
   |
   = help: this indicates a potential bug in the program: it performed an invalid operation, but the Stacked Borrows rules it violated are still experimental
   = help: see https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md for further information
help: <TAG> was created by a Unique retag at offsets [0x0..0x4]
   |
LL |     let xref = unsafe { &mut *xraw };
   |                         ^^^^^^^^^^
help: <TAG> was later invalidated at offsets [0x0..0x4] by a read access
   |
LL |     let _val = unsafe { *xraw }; // invalidate xref
   |                         ^^^^^
```

这份报错把模型的三段都摆出来了：**tag 从哪来**（Unique retag）、**什么时候被干掉**（一次读访问）、**怎么干掉**（从借用栈里消失）。最后那条 `help` 里的 "still experimental" 是模型自己的免责声明，不是修辞。

### 三、三条修法

1. **别让派生链被打断**：要地址就用 `&raw const`/`&raw mut`（第 24 章），别先造引用再转。
2. **把「这份数据会被别人改」写进类型**：`UnsafeCell`/`Cell`/原子类型——两套模型都会对内部可变的字节放宽权限。
3. **让所有权与借用表达清楚**：能用安全 Rust 表达的独占就别拿裸指针手工维持；模型检查的是「有没有越线」，不是「必须不必须用 unsafe」。

## 小结

- **三件事分开看**：别名规则（Reference 里 "not determined yet"）、别名模型（跑在 Miri 里的 Stacked/Tree Borrows）、规范承诺（只有 Reference 与 std 文档算数）。
- **SB 是「每个字节一条栈」**：写会弹掉批准项上方的一切，读会把上方的 `Unique` 置 `Disabled`；`FnEntry` 的保护符是 `noalias` 的依据；两处硬伤都写在 UCG 的 SB 文档里。
- **TB 是「每块分配一棵树」**：外来读对**没写过**的 `Reserved` 无影响、对已激活的 `Unique` 才冻成 `Frozen`，外来写一律判死；裸指针重借是空操作；没有子对象来历。
- **同一段代码可能两个结论**：`sb_fails.rs` 那五个用例在 SB 下都是 UB，在 TB 下全部通过。
- **1.98 的现状**：stable 不启用任何模型（Miri 只在 nightly），但编译器早就把 `noalias` 贴到了引用形参上，`std` 也已经为一个具体模型改过代码。
- **provenance 是字节的一部分**：`Init(u8, Option<Provenance>)`；暴露来历需要全局表、只能「尽力而为」，strict 模式直接停机。

## 练习

**1. 解释为什么。** 为什么「读」在两个模型里要处理上方的 `Unique`，而不是干脆不管它？

**2. 预测结论。** 下面这段在本机 stable 1.98 上：编译能过吗？输出是什么？如果交给 Miri，两套模型分别会说什么？

```rust
fn main() {
    let mut v = 1u32;
    let raw = &raw mut v;
    let shared = &v;         // 共享借用插进来
    unsafe { *raw = 2 };     // 再通过裸指针写
    println!("{shared}");
}
```

**3. 找错。** 有人把 `&raw mut x` 换成 `&mut x as *mut u32`，理由是两个都「拿到 `*mut u32`」。从别名模型的角度说，差在哪？在哪个模型下这个替换更危险？

<details><summary>参考答案</summary>

**1.** 因为完全不管理「读」会让优化器丢掉一整类自由：读与读之间本来可以随意重排，但只要有写插进来，重排立刻不成立。模型得记住「这块内存有没有可能被别处读过」，才能决定后面能不能把它当独占的用。

两个模型的取舍不同。SB 粗暴但好实现：读把上方 `Unique` 置为 `Disabled`——「既然有人读过它，独占性就不作数了」；SB 文档自己把它列在「最头疼的设计问题」里（另一处硬伤是裸指针转派生新 tag）。

TB 更精确：外来读只影响**已经激活**的节点——写过的 `Unique` 冻成 `Frozen`（可读、禁写），没写过的 `Reserved` 一点不动。多记一个「激活了没有」的状态，换来读读之间可以自由重排，以及 `pass_invalid_mut` 那类代码不再被误判。

顺带纠一处容易记错的：TB 文档「Imprecisions」列的是它**比 SB 更宽松**、有人主张该收紧的几处（没有子对象来历、`&mut` 一开始不算 writable），不包括「读只冻结」本身。

**2.** 能编译，也会打印 `2`：`invalid_reference_casting` 追不到这条数据流——写是通过 `raw` 发生的，而 `raw` 来自 `&raw mut v`，不是从 `&v` 强转出来的。

但它越线了：违反 Reference 那条原则的第一句——`&T` 必须指向「活跃期间不被改」的内存，而 `shared` 还活着时就被 `raw` 写回去了（第 24 章讲过同一段）。交给 Miri：SB 下这次写会把 `shared` 那一项从栈里弹掉，TB 下把它置为 `Disabled`；两种情况下最后那次读都是 UB。（依据两套模型的规则与 UCG 文档；本机没有 nightly，未实跑。）

关键收获还是第 23 章那条：**stable 全绿、输出也对，不能证明这段代码是对的**。

**3.** 差在「造不造引用」。`&raw mut x` 是裸借用运算符：操作数按 place 求值，直接产出 `*mut u32`，中间**不经过**引用（第 24 章）。`&mut x as *mut u32` 先造出 `&mut x` 再转成裸指针——这一步在别名模型里是一次 retag，带着「我是从这条引用派生出来的」这层含义，会按当时的规则去动借用栈或树。

SB 下更危险在于顺序敏感：`&mut x` 一旦被后来的借用顶掉，从它派生的裸指针也跟着作废（再用就是 UB）——这正是 SB 文档列为「最让人意外」的那一类（`addr_of_mut!(local)` 被对 `local` 的直接写失效）。TB 反而温和：它把裸指针重借当空操作。结论：**要地址就用 `&raw`，别让临时引用成为必经之路。**

</details>

## 延伸阅读

- 别名规则的现行措辞：<https://doc.rust-lang.org/reference/behavior-considered-undefined.html>
- Nomicon《Aliasing》：<https://doc.rust-lang.org/nomicon/aliasing.html>
- Unsafe Code Guidelines（含仍在维护的词汇表）：<https://rust-lang.github.io/unsafe-code-guidelines/>
- UCG 的两份模型文档（都非规范）：<https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md>、<https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/tree-borrows.md>
- 两篇论文：<https://plv.mpi-sws.org/rustbelt/stacked-borrows/>、<https://plf.inf.ethz.ch/research/pldi25-tree-borrows.html>
- t-opsem 的 FCP 清单：<https://github.com/rust-lang/opsem-team/blob/main/fcps.md>
- Miri：<https://github.com/rust-lang/miri>
- `std::ptr` 的 provenance 一节：<https://doc.rust-lang.org/std/ptr/index.html#provenance>
