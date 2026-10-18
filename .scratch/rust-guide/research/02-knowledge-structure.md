# 02 知识结构基线：Rust 1.98 / edition 2024 下的进阶主题全集、依赖序与现状校验

> 对应票：`issues/02-knowledge-structure-baseline.md`（research）。研究日：2026-10-18。
> 用途：作为「05 内容版图与章节结构」的输入。
>
> **事实 / 推断标注约定**
> - 「核实」= 有一手来源（官方文档、release notes、std 文档、编译器实测）。
> - `[推断]` = 本文件的判断（分层仲裁、依赖边、章节批次建议），不是官方结论。
> - 所有版本号、日期均为核实值；未标注的日期未做核实。

---

## 0. 校验环境与来源快照

核实（本机实测）：

| 项 | 值 | 来源 |
|---|---|---|
| rustc | `1.98.0 (88d9e12ae 2026-08-18)` | `rustc --version --verbose` |
| cargo | `1.98.0 (797e8a9bc 2026-08-05)` | `cargo --version` |
| LLVM | 22.1.8 | `rustc --version --verbose` |
| host | `x86_64-pc-windows-msvc` | 同上 |
| mdbook | 0.5.4（地图给定） | 地图 |

**关键便利条件**：本机 rustup 组件自带与 1.98 **完全同步**的官方文档快照：
`C:\Users\tyd27\.rustup\toolchains\1.98.0-x86_64-pc-windows-msvc\share\doc\rust\html\`

其中包含 `book/`、`reference/`、`nomicon/`、`std/`、`rust-by-example/`、`edition-guide/`、`cargo/`、`rustc/`、`rustdoc/`、`clippy/`、`style-guide/`、`unstable-book/`、`embedded-book/`、`releases.md`。
因此下文所有「官方文档覆盖范围」「X 于 1.x 稳定」的结论，都可在**与 1.98 版本一致的**一手快照上复核，不必担心 `stable` 在线文档随版本漂移（在线等价地址：`https://doc.rust-lang.org/1.98.0/...`、`https://doc.rust-lang.org/stable/releases.html`）。

---

## 1. 官方资料覆盖面盘点（谁讲什么 · 空白在哪）

核实（各文档目录树 + 章节标题）：下列文档集随 1.98 一同分发，版本一致。

| 文档 | 性质 | 进阶覆盖（与本书相关的部分） | 对本书而言的空白 |
|---|---|---|---|
| **The Book**（`book/`） | 教程 | 22 章 + 7 附录。ch17 是完整 async 章（Future/async 语法、`join!`、`Stream`、**ch17.5 专讲 `Future`/`Pin`/`Unpin`/`Stream`**）；ch20 进阶章：unsafe（含 Miri 一节）、高级 trait（关联类型/默认泛型参数/supertrait/newtype）、高级类型（newtype/别名/`!`/DST）、宏（`macro_rules!` + 三种 proc-macro） | 无 variance/dropck/PhantomData；trait 无 dyn-compatibility、upcasting、GAT、sealed、RPITIT/`use<>`；无 const 泛型、无内存序、无 FFI/ABI/数据布局、无性能剖析、无 Cargo 深化、无错误处理 API 设计、无 no_std |
| **Rust Reference**（`reference/`） | 规范 | 21 章：type system（variance、coercions、DST、layout、interior mutability）、unsafety（`unsafe`、UB 清单）、const eval、patterns、macros（by example / proc）、inline assembly、ABI、linkage、memory model、destructors、lifetime elision | 只描述「是什么」，不教工程取舍与设计 |
| **Rustonomicon**（`nomicon/`） | unsafe 深水书 | Data Layout/reprs、Ownership（aliasing、variance、dropck、PhantomData、HRTB、borrow splitting）、Type Conversions（coercions/casts/transmutes）、Uninit（checked/unchecked、drop flags）、OBRM（构造/析构/leak）、Unwinding、Concurrency（races、Send/Sync、**atomics**）、Implementing `Vec`、Implementing `Arc`+`Mutex`、FFI、Beneath `std` | 无 async/Pin、无宏、无工程实践 |
| **std 文档**（`std/`） | API | `std::pin`、`std::future`、`std::sync::atomic::Ordering`、`std::ffi`、`std::error::Error`、`std::marker::{Send,Sync}`、`std::task`、`std::cell`、`std::hint` | 逐条 API 参考，无教学结构 |
| **Rust by Example**（`rust-by-example/`） | 示例集 | 24 节，含 macros、error handling、unsafe、FFI、async 之外的 std 用法 | 深度浅，止步于演示 |
| **Cargo Book**（`cargo/`） | 工具规范 | Workspaces、Features、Profiles、Build Scripts、SemVer Compatibility、**Optimizing Build Performance**、Continuous Integration、Testing、Environment Variables、Registries、Lints 表、report timings、`cargo miri` 手册页 | 无「怎么组织一个中大型 crate」的取舍 |
| **rustc Book**（`rustc/`） | 编译器规范 | Codegen Options、Lints（allow/warn/deny 全表）、**Profile-guided Optimization**、**Instrumentation-based Code Coverage**、Linker-plugin LTO、check-cfg、Exploit Mitigations、Symbol Mangling v0、Platform Support/Tier 策略 | 无「何时值得用」的判断 |
| **Edition Guide**（`edition-guide/`） | 迁移指南 | 2015→2024 全部迁移项；**Rust 2024 一节有 16 个语言页 + 3 个 std 页 + 3 个 Cargo 页 + 2 个 rustdoc 页 + 4 个 rustfmt 页**；另有「进阶迁移」一章 | 只讲「改了什么」，不讲「新代码该长什么样」 |
| **Rust Style Guide**（`style-guide/`） | 风格 | Items/Statements/Expressions/Types & Bounds/Cargo.toml 约定/**Style editions**/夜间语法 | 只管排版风格 |
| **Unstable Book**（`unstable-book/`） | 未稳定特性清单 | 4 大类：Compiler env vars、Compiler flags（数百个）、**Language features**、Library features | 需要人工判断「值不值得写」 |
| **Embedded Book**（`embedded-book/`） | no_std/嵌入 | `no_std`、内存映射寄存器、singletons、typestate、HAL 设计、与 C 互操作 | 偏嵌入式专用 |
| 其他 | — | `rustdoc/`（文档与 doctest）、`clippy/`（lint 全表）、`error-index.html`、`guide-*.html`（第 1 版书籍遗留页，**已过时**） | — |

团外一手来源（本日在线核实可达性）：

| 来源 | 状态 | 核实 |
|---|---|---|
| Rust API Guidelines `https://rust-lang.github.io/api-guidelines/` | 可达，标题 `About - Rust API Guidelines`；仓库 `rust-lang/api-guidelines` 最近 push `2025-07-08` | ✅ |
| Unsafe Code Guidelines `https://rust-lang.github.io/unsafe-code-guidelines/` | 可达；仓库最近 push `2026-05-05` | ✅ |
| Miri `https://github.com/rust-lang/miri` | 活跃（最近 push `2026-10-18`）；官方安装/用法入口（`doc.rust-lang.org/nightly/miri/` **已 404**，勿引用） | ✅ |
| Async Book `https://rust-lang.github.io/async-book/` | 可达；仓库最近 push `2026-05-24`（更新慢，见 5.3） | ✅ |

`[推断]` **结论 1**：本书的差异化空间不在「覆盖更多 API」，而在三个官方资料**系统性不覆盖**的交叉地带：
(a) **取舍与判断**（何时该用 `dyn` 而非泛型、何时该 unsafe、如何组织错误类型、如何压性能）；
(b) **跨主题的因果链**（variance → 借用检查错误信息 → API 形状；`Pin` → 自引用 → async 状态机）；
(c) **2024 edition 之后的正确写法**（官方书籍更新滞后于 edition 迁移文档，见 §4）。

---

## 2. 主题全集（层级 + 前置依赖 + 官方链接）

层级定义（`[推断]`，供 05 复用）：

- **B 基础**：The Book 已完整覆盖；本书**不重讲**，只按需给「基础补充盒」。
- **I 进阶**：The Book/官方资料只触及片段，本书的**主干内容**。
- **D 深水**：需要 unsafe/形式化/编译器内部知识，或官方资料只有规范没有教程；作为选读章或独立深挖票。

### 2.1 基础层（B）——只做衔接，不设章

| ID | 主题 | 官方链接 | 备注 |
|---|---|---|---|
| B1 | 所有权 / 借用 / 切片 | book ch4 | 全书默认前置 |
| B2 | 泛型与 trait 入门 | book ch10 | `[推断]` 必须在一切 trait 深化之前 |
| B3 | `Result`/`?`/panic 基础 | book ch9 | |
| B4 | 迭代器与闭包 | book ch13 | |
| B5 | `dyn Trait`/trait object 入门 | book ch18.2 | |
| B6 | 包/模块/`use` | book ch7 | |

### 2.2 进阶层（I）——本书主干

| ID | 主题 | 前置（硬） | 前置（软/叙述便利） | 官方一手链接 |
|---|---|---|---|---|
| I1 | **trait 设计**：关联类型 vs 泛型参数、supertrait、sealed trait、扩展 trait、coherence/orphan rule、`#[diagnostic::on_unimplemented]` | B2 | B5 | reference/items/traits.html、reference/items/associated-items.html、reference/attributes/diagnostics.html |
| I2 | **`dyn` 兼容性与对象安全**：dyn-compatibility 规则、`where Self: Sized`、trait 对象 vtables 与 upcasting | I1 | — | reference/items/traits.html#dyn-compatibility、reference/types/trait-object.html |
| I3 | **`impl Trait` 与 opaque type**：参数位置 vs 返回位置、RPIT 生命周期捕获（2024 规则）、precise capturing `use<..>`、RPITIT/AFIT | I1 | — | reference/types/impl-trait.html、edition-guide/rust-2024/rpit-lifetime-capture.html |
| I4 | **GAT（generic associated types）** | I1, I3 | — | reference/items/associated-items.html（GAT 稳定于 1.65） |
| I5 | **const 泛型**：`const N: usize`、默认值、`_` 推断、能力边界（`adt_const_params`/`generic_const_exprs` 仍不稳定） | B2 | I3 | reference/items/generics.html、unstable-book（`adt_const_params`、`generic_const_exprs`） |
| I6 | **生命周期进阶**：variance/subtyping、HRTB `for<'a>`、drop check、`PhantomData`、省略规则边界 | B1, B2 | I1 | nomicon/subtyping.html、nomicon/hrtb.html、nomicon/dropck.html、nomicon/phantom-data.html、reference/lifetime-elision.html |
| I7 | **智能指针与内部可变性**：`Box`/`Rc`/`Arc`/`Cell`/`RefCell`/`Mutex`/`RwLock`/`Cow`；`OnceLock`/`LazyLock`/`LazyCell`；`Deref` 强转陷阱 | B1 | — | book ch15、std/cell/index.html、std/sync/index.html、std/borrow/enum.Cow.html |
| I8 | **错误处理与 API 设计**：`Error`/`source()` 链、`#[non_exhaustive]`、builder、typestate、`From` 转换策略、SemVer 与 breaking change | B3, I1 | I2 | std/error/trait.Error.html、cargo/reference/semver.html、rust-lang.github.io/api-guidelines/ |
| I9 | **并发基础**：`thread`/`scope`、`Send`/`Sync`（auto trait 与 `unsafe impl`）、`Arc<Mutex<T>>`、poisoning、channel、`thread_local!` | B1, I7 | — | book ch16、std/marker/trait.Send.html、nomicon/send-and-sync.html、nomicon/poisoning.html |
| I10 | **内存序与无锁**：`Ordering` 五值语义、happens-before、CAS 循环、`UnsafeCell`、`loom` 测试 | I9, I13 | D2 | std/sync/atomic/enum.Ordering.html、nomicon/atomics.html、nomicon/races.html |
| I11 | **async 基础→进阶**：`Future`、executor 概念、`async`/`.await`、`join!`/`select!`、`!Send` future、`Send` 边界、取消与 drop、运行时选型 | B1, I3, I7, I9 | I2 | book ch17、std/future/index.html、std/task/index.html |
| I12 | **`Pin` 与自引用**：`Pin<P>`、`Unpin`、`pin!`/`Box::pin`、`PhantomPinned`、何时才真的需要 unsafe | I11, I6 | I13 | std/pin/index.html、std/pin/struct.Pin.html、book ch17.5 |
| I13 | **unsafe 语义与 UB**：`unsafe` 四个超能力、`unsafe fn`/`unsafe trait`、UB 清单与「非 UB 但危险」、Miri | B1, I7 | I6 | reference/unsafety.html、reference/behavior-considered-undefined.html、nomicon/meet-safe-and-unsafe.html、book ch20.1 |
| I14 | **指针、provenance 与未初始化内存**：`NonNull`、strict provenance、`MaybeUninit`、`ManuallyDrop`、`&raw`、别名模型（Stacked/Tree Borrows） | I13 | D2 | nomicon/aliasing.html、nomicon/checked-uninit.html、nomicon/unchecked-uninit.html、std/ptr/ |
| I15 | **数据布局与 `repr`**：`repr(Rust)` 无保证、`repr(C)`、`repr(transparent)`、niche 优化、DST/fat pointer | I13 | I14 | reference/type-layout.html、nomicon/repr-rust.html、nomicon/other-reprs.html、nomicon/exotic-sizes.html |
| I16 | **FFI 与 ABI**：`extern "C"`（2024 起 `unsafe extern`）、C 字符串、`#[no_mangle]`→`#[unsafe(no_mangle)]`、回调、`#[repr(C)]` 契约、`c_variadic` 状态 | I15, I13 | I8 | nomicon/ffi.html、reference/items/external-blocks.html、reference/abi.html、std/ffi/ |
| I17 | **宏（`macro_rules!`）**：片段说明符与 2024 变化、卫生性、递归、`$crate`、调试 | B2 | I3 | reference/macros-by-example.html、book ch20.5、rust-by-example/macros.html |
| I18 | **过程宏**：derive/attribute/function-like、`proc_macro` API、`syn`/`quote`、`Span`/错误报告 | I17, I1 | I3 | reference/procedural-macros.html、book ch20.5、std/proc_macro/ |
| I19 | **性能与剖析**：profile 与优化等级、`--timings`、基准测试（criterion；`#[bench]` 已废）、`perf`/flamegraph、PGO、覆盖率、`std::hint`、分配/克隆成本 | B4, I7 | I10 | cargo/guide/build-performance.html、cargo/reference/profiles.html、rustc/profile-guided-optimization.html、rustc/instrument-coverage.html、std/hint/index.html |
| I20 | **Cargo/workspace/工程实践**：features 与可选依赖、workspace 继承（`workspace.dependencies`/`[workspace.lints]`）、`[lints]` 表、`rust-version`/MSRV、resolver v3、build script、CI、发布与 yank | B6 | — | cargo/reference/features.html、cargo/reference/workspaces.html、cargo/reference/build-scripts.html、cargo/reference/lints.html、cargo/guide/continuous-integration.html |
| I21 | **测试进阶**：单元/集成/doctest 分工、`compile_fail`、参数化、属性测试（proptest）、快照、`cargo nextest`、Miri/loom 在 CI 里的位置 | B3, I20 | I13, I10 | book ch11、rustdoc/write-documentation/documentation-tests.html、cargo/guide/tests.html、cargo/commands/cargo-miri.html |
| I22 | **文档与 lint 工程**：rustdoc、intra-doc link、`#[doc(cfg)]`（不稳定）/`cfg_attr` 技巧、clippy 分组、`#[expect]`、禁止 `deny(warnings)` 的理由 | I20 | — | rustdoc/、clippy/、rustc/lints/index.html |
| I23 | **Edition 与迁移**：2024 全部改动、`cargo fix --edition`、lint 组 `rust-2024-compatibility` | B6 | I20 | edition-guide/rust-2024/ |
| I24 | **`no_std` 与嵌入式**（可选） | I13, I15 | I10 | embedded-book/、std/（对照 `core`/`alloc`） |

### 2.3 深水层（D）——选读 / 深挖票候选

| ID | 主题 | 前置（硬） | 官方一手链接 |
|---|---|---|---|
| D1 | 手写集合与智能指针：`RawVec`、`Drain`、ZST、`Arc`+`Mutex` 实现 | I13, I14, I15 | nomicon/vec.html、nomicon/arc-and-mutex.html |
| D2 | 别名模型与形式化：Stacked Borrows / Tree Borrows、UCG、provenance 细节 | I14 | rust-lang.github.io/unsafe-code-guidelines/、nomicon/aliasing.html |
| D3 | 高级类型系统：coherence 细节、negative impls（不稳定）、specialization（不稳定）、`adt_const_params`（不稳定） | I1, I2, I5 | unstable-book（`specialization`、`generic_const_exprs`、`adt_const_params`） |
| D4 | 编译器与代码生成：MIR、单态化、`-C` 选项、inline asm、naked functions、PGO/LTO 原理 | I19 | rustc/codegen-options/index.html、reference/inline-assembly.html、rustc/linker-plugin-lto.html |
| D5 | 自己实现 async 运行时：`Waker`/`RawWaker`、手写状态机、pin 投影 | I11, I12, I13 | std/task/trait.Wake.html、std/future/trait.Future.html |
| D6 | 宏系统极限：`macro_metavar_expr`、属性宏组合、`#[track_caller]`、错误诊断设计 | I17, I18 | reference/procedural-macros.html、unstable-book |
| D7 | 生产工程：panic 策略（abort/unwind）、动态库与 ABI 稳定、交叉编译、`build-std`、供应链与 registry | I15, I16, I20 | rustc/exploit-mitigations.html、cargo/reference/registries.html、unstable-book（`build-std`） |

---

## 3. 依赖序（可直接支撑章节排序）

### 3.1 硬依赖边（不满足会讲不通）

核实依据（每条只列一手事实）：
- `dyn` 兼容性、RPITIT、GAT 都以 trait 的语法与关联项为前提 → Reference 的 traits/associated-items 章节。
- variance/HRTB/dropck 的讨论对象是引用与泛型生命周期 → Nomicon ch3 各节标题。
- async 需要 `Future`（std）、`Pin`（`std::pin` 说 Pin 是「让 `!Unpin` 数据能安全地被固定」的机制）、以及 `Send`+任务模型（`std::task`）。
- `Ordering` 的语义在 std 文档中明确指向 nomicon 的 atomics 章；无锁结构建立在 `UnsafeCell`+原子之上。
- unsafe 与 UB 清单（Reference §17）是布局、FFI、未初始化内存、别名模型的共同前提。
- `macro_rules!` 的片段说明符是 proc-macro 之前的最低门槛（Book ch20.5 的编排顺序亦如此）。

```
B1,B2,B3,B4,B5,B6
   ↓
I1 ──┬─→ I2 ─→ I3 ─→ I4
     │         └─→ I5
     ├─→ I8 ─→ I16
     ├─→ I17 ─→ I18 ─→ D6
     └─→ I6 ─┬─→ I12 ─→ D5
             └─→ I13 ─┬─→ I14 ─→ D2 ──┐
                      ├─→ I15 ─┬─→ I16 │
                      │        └─→ D1  │
                      └─→ D3 ──────────┘
I7 ─┬─→ I9 ─→ I10 ─→ D5
    ├─→ I19
    └─→ I11 ─→ I12
I20 ─┬─→ I21
     ├─→ I22
     └─→ I23
I11 ─→ D5 ;   I16 ─→ D7 ;   I19 ─→ D4
```

### 3.2 `[推断]` 章节批次建议（拓扑顺序）

| 批次 | 内容 | 为什么必须在这批 | 可并行度 |
|---|---|---|---|
| **批 0** | I23（edition 2024 + 工具链约定）+ 基础补充盒 B1–B6 | 2024 edition 改了 RPIT 捕获、临时值作用域、`unsafe extern` 等**跨章语法前提**；先用一章把「本书所有代码的 edition/编译器版本约定」钉死，后续章节才不必反复解释 | 单章 |
| **批 1（概念主干）** | I1 → I2 → I3 → I4/I5 → I6 → I7 → I8 | trait 是全书最大公约数：`dyn` 兼容性（I2）、RPIT/`use<>`（I3）、GAT（I4）都直接读 I1；I6 的 variance/dropck 是 I12 与 I13 的解释框架；I8 需要 I1+I2 才能谈「API 形状」 | 章内线性，可跨章并行写草稿 |
| **批 2（并发与异步）** | I9 → I10；I11 → I12 | I10 需要 `Send`/`Sync` 与 `UnsafeCell` 的准确语义；I12 必须在 I11 之后（先有「为什么需要固定」，再讲 Pin 的 API 契约） | I9/I10 与 I11 可并行 |
| **批 3（系统层）** | I15 → I16；I13 → I14 → I15 | unsafe/UB（I13）是布局与指针两章的共同前提；FFI（I16）需要布局契约（I15）与错误映射（I8） | I13 后可并行三支 |
| **批 4（工程实践，可与批 1–3 并行）** | I19, I20, I21, I22, I17→I18 | 工程章只需基础层；宏（I17/I18）与主线弱耦合，可随时插入 | 高度并行 |
| **批 5（深水选读）** | D1–D7 | 每项都依赖批次 1–4 的对应基础 | 各自独立 |

**排序上不可妥协的两条边（`[推断]` 但依据充分）**：
1. **I1（trait 设计）必须在 I2/I3/I4 之前**：因为 `dyn` 兼容性、RPITIT、GAT 都是「trait 的语法与关联项」的推论，反序会让读者把三章都读成「魔法规则」。
2. **I11（async 概念）必须在 I12（Pin）之前**：`std::pin` 与 Book ch17.5 都是「先 `Future`，再 `Pin`/`Unpin`」的编排；先讲 Pin 会失去全部动机。
3. **I13（unsafe/UB）必须在 I14（指针/provenance）、I15（布局）、I16（FFI）之前**：UB 清单是这三章的判据来源。

---

## 4. 现状校验（截至 1.98，2026-08）

### 4.1 edition 2024 要点（1.85.0 起稳定，RFC 3501）

核实：Edition Guide 的 `rust-2024/` 目录页（本机快照 1.98；在线 `https://doc.rust-lang.org/edition-guide/rust-2024/index.html`，页面标注 **RFC #3501 / Release version 1.85.0**）。以下为**近期原页摘要**（每页正文已逐页核对）：

**语言（14 页 / 14 项，Edition Guide 目录逐条核对）**
1. **RPIT 生命周期捕获规则**（RFC 3498）：2024 起，`impl Trait` 无 `use<..>` 时**隐式捕获所有在作用域内的泛型参数（含生命周期）**；`Captures<..>` hack 失效，改用 `use<..>`。
2. **`if let` 临时值作用域**：scrutinee 产生的临时值在进入 `else` 前就 drop。
3. **`if`/`while` 中的 let chains**：允许 `&&` 串接 `let`；`||` 不行。
4. **尾表达式临时值作用域**：函数/闭包/块尾表达式的临时值现在**可能早于局部变量** drop，某些情况不再延长到外层作用域。
5. **Match ergonomics 保留**：默认绑定模式非 `move` 时，禁止在绑定上写 `mut`/`ref`/`ref mut`；`&`/`&mut` 引用模式也不能出现在「非全显式」模式里。
6. **`unsafe extern`**：`extern` 块必须写 `unsafe`（1.82 起各 edition 都支持新语法）。
7. **unsafe 属性**：`export_name`/`link_section`/`no_mangle` 必须写成 `#[unsafe(...)]`。
8. **`unsafe_op_in_unsafe_fn` 默认 warn**：`unsafe fn` 内的 unsafe 操作需显式 `unsafe {}`。
9. **禁止 `static mut` 引用**：`static_mut_refs` lint 在 2024 为 deny。
10. **never type fallback 改变**：never-to-any 强制转换回退到 `!` 而非 `()`；`never_type_fallback_flowing_into_unsafe` 为 deny。
11. **宏片段说明符**：`expr` 现在也接受 const 块与 `_`；新增 `expr_2021` 供兼容。
12. **缺失片段说明符**：`missing_fragment_specifier` 成硬错误（注：1.89 起**所有 edition** 均为硬错误）。
13. **`gen` 成为保留关键字**（RFC 3513，为未来 `gen` block 让路）。
14. **保留语法**：无前缀的 `#"foo"#` 与 ≥2 个 `#` 被保留。
（prelude 变更归在标准库页，见下）

上述 14 项与 Edition Guide 的 14 个语言页一一对应：`rpit-lifetime-capture`、`temporary-if-let-scope`、`let-chains`、`temporary-tail-expr-scope`、`match-ergonomics`、`unsafe-extern`、`unsafe-attributes`、`unsafe-op-in-unsafe-fn`、`static-mut-references`、`never-type-fallback`、`macro-fragment-specifiers`、`missing-macro-fragment-specifiers`、`gen-keyword`、`reserved-syntax`。

**标准库（3 页）**：prelude 变更（同上）；**`Box<[T]>: IntoIterator`（1.80 起所有 edition 生效，但 `.into_iter()` 的语义随 edition 变：2024 按值，之前按引用，迁移 lint `boxed_slice_into_iter`）**；**若干函数新增 `unsafe`：`std::env::set_var`、`std::env::remove_var`、`std::os::unix::process::CommandExt::before_exec`**。

**Cargo（3 项）**：**MSRV-aware resolver**（=`resolver = "3"`）；**表/键名一致性规则**；**继承依赖里未使用的 `default-features` 现在报错**。

**Rustdoc / Rustfmt**：rustdoc 合并 doctest、嵌套 `include!` 语义变化；rustfmt 引入 **style edition**、若干格式修正、raw identifier 与版本号排序。

**实测交叉验证（本机 rustc 1.98.0，edition 对比）**：

| 代码 | edition 2021 | edition 2024 |
|---|---|---|
| `#[no_mangle] pub extern "C" fn f(){}` | 通过 | **错误：`unsafe attribute used without unsafe`** |
| `extern "C" { fn abs(x:i32)->i32; }` | 通过 | **错误：`extern blocks must be unsafe`** |
| `unsafe fn f(p:*const u8)->u8 { *p }` | 无警告 | **warn[E0133]：dereference of raw pointer is unsafe and requires unsafe block** |
| `static mut V:u32=0; ... &mut V` | warning（`creating a mutable reference to mutable static`） | **错误（deny）** |
| `if let Some(x)=a && x>0 {}` | **错误：`let chains are only allowed in Rust 2024 or later`** | 通过 |
| `Box<[u8]>` 的 `for x in b` | 通过 | 通过（1.80 起所有 edition 支持 for 循环按值迭代） |

### 4.2 至 1.98 已稳定的、与本书主题直接相关的特性（按版本）

核实来源：本机 `releases.md`（= 在线 `https://doc.rust-lang.org/stable/releases.html`）逐条 + std 页面上的 “Stable since Rust version X” 徽标 + 编译器实测。

| 版本 | 日期 | 项 | 来源 |
|---|---|---|---|
| 1.63 | 2022-08-11 | `thread::scope`（scoped threads） | releases.md 1.63 稳定 API 列表 |
| 1.65 | 2022-11-03 | `let else`；**GAT**；labeled block break value | releases.md 1.65；特性探针 `generic_associated_types`→stable 1.65.0、`let_else`→stable 1.65.0 |
| 1.68 | 2023-03-09 | `{core,std}::pin::pin!` | releases.md 1.68 |
| 1.70 | 2023-06-01 | `std::sync::OnceLock` | releases.md 1.70 |
| 1.75 | 2023-12-28 | **`async fn` + RPITIT in traits** | releases.md 1.75；探针 `return_position_impl_trait_in_trait`→stable 1.75.0 |
| 1.77 | 2024-03-21 | C 字符串字面量 `c"..."`；`offset_of!` | releases.md 1.77；探针 `offset_of`→stable 1.77.0 |
| 1.78 | 2024-05-02 | `#[diagnostic::on_unimplemented]` | releases.md 1.78 |
| 1.79 | 2024-06-13 | 内联 `const {}`；关联类型约束（associated type bounds） | releases.md 1.79；探针 `inline_const`→1.79.0 |
| 1.80 | 2024-07-25 | `LazyCell`/`LazyLock`；`Option::take_if`；`Box<[T]>: IntoIterator` | releases.md 1.80；std 徽标 1.80.0 |
| 1.81 | 2024-09-05 | `#[expect]`；`core::error::Error`（no_std 可用 Error） | releases.md 1.81；探针 `error_in_core`→1.81.0 |
| 1.82 | 2024-10-17 | **precise capturing `use<..>`（RFC 3617）**；**`&raw const/mut`（RFC 2582）**；**unsafe extern blocks（RFC 3484）**；unsafe 属性选择加入；`offset_of!` 嵌套字段；inline asm 中的 const 操作数；`repeat_n` | releases.md 1.82；探针 `precise_capturing`→1.82.0、`unsafe_extern_blocks`→1.82.0、`raw_ref_op`→1.82.0、`iter_repeat_n`→1.82.0 |
| 1.83 | 2024-11-28 | `expr_2021` 片段说明符（所有 edition）；const 中的 `&mut`/`&Cell` | releases.md 1.83 |
| 1.84 | 2025-01-09 | **strict provenance 系列**（`ptr::with_exposed_provenance` 等）；Cargo **resolver v3**（MSRV-aware） | releases.md 1.84 |
| 1.85 | 2025-02-20 | **Edition 2024 稳定**；**async 闭包** | releases.md 1.85；探针 `async_closure`→1.85.0 |
| 1.86 | 2025-04-03 | **trait object 向上转型（upcasting）** | releases.md 1.86 |
| 1.87 | 2025-05-15 | `use<..>` 在 trait 内 RPIT 上可用（`precise_capturing_in_traits`）；`asm_goto`；`Vec::extract_if` | releases.md 1.87；std 徽标 `Vec::extract_if`=1.87.0 |
| 1.88 | 2025-06-26 | **let chains（2024 edition）**；`naked_functions`；`cfg_boolean_literals`；`HashMap/Set::extract_if`；`Cell::update`；`[T]::as_chunks`；`#[bench]` **彻底废置**；`--nocapture` 弃用 | releases.md 1.88；std 徽标 1.88.0 |
| 1.89 | 2025-08-07 | **显式推断的 const 实参 `_`**（`generic_arg_infer`）；`File::lock` 系列；`NonNull` provenance 方法；可在变量中保存 `format_args!()` | releases.md 1.89；编译实测通过 |
| 1.90 | 2025-09-18 | x86_64-linux 默认 `lld`；多包发布；`u*::*_sub_signed` | releases.md 1.90 |
| 1.91 | 2025-10-30 | 语言：**模式绑定的 drop 顺序改为按书写顺序/主绑定顺序**；C 风格可变参数**声明**扩展到 sysv64/win64/efiapi/aapcs；`integer_to_ptr_transmutes`、`dangling_pointers_from_locals` 等 lint。库：`str::floor_char_boundary`、`Path::file_prefix`、`BTreeMap/Set::extract_if`、`strict_*` 算术、`PathBuf::add_extension`、`core::array::repeat`、`Cell::as_array_of_cells`、`Path/PathBuf` 的 `PartialEq<str/String>` | releases.md 1.91；std 徽标 `floor_char_boundary`/`file_prefix`=1.91.0 |
| 1.92 | 2025-12-11 | `RwLockWriteGuard::downgrade`；`Box/Rc/Arc::new_zeroed*`；never type 两个 lint 转 deny | releases.md 1.92 |
| 1.93 | 2026-01-22 | `Vec/String::into_raw_parts`；`<[T]>::as_array`；`VecDeque::pop_*_if`；`std::fmt::from_fn`；`MaybeUninit<[T]>` 系列；`cargo clean --workspace` | releases.md 1.93；std 徽标 `into_raw_parts`=1.93.0 |
| 1.94 | 2026-03-05 | `LazyCell/LazyLock::get`/`get_mut`/`force_mut`（**`this:` 接收者，见 5.2**）；`[T]::array_windows`；`Peekable::next_if_map`；Cargo `include` 配置、TOML v1.1、`pubtime` | releases.md 1.94 |
| 1.95 | 2026-04-16 | **`if let` guards on match arms**；`cfg_select!`；`core::range`；`Atomic*::update`/`try_update`；`Vec::push_mut` 等 `*_mut` 插入 API；`Layout::repeat*` | releases.md 1.95；探针 `if_let_guard`→stable 1.95.0 |
| 1.96 | 2026-05-28 | `assert_matches!`/`debug_assert_matches!`（**需显式 `use std::assert_matches;`**）；`core::range::{Range,RangeFrom,...}` | releases.md 1.96 |
| 1.97 | 2026-07-09 | **v0 symbol mangling 默认**；`pin!` 修掉 `&mut T` 的 deref coercion；Cargo `build.warnings`、`resolver.lockfile-path`、`-m` 简写；rustdoc `--emit`/`--remap-path-prefix` | releases.md 1.97 |
| 1.98 | 2026-08-20 | `{core,std}::derive` 明确稳定（MSRV 1.96）；`String::from_utf16le/be`；**具体原子类型新增 `from_mut`/`from_mut_slice`/`get_mut_slice`**（`AtomicU32` 等实为泛型 `Atomic<u32>` 的特化，但泛型写法 `Atomic<u32>` 本身仍不稳定，见 5.2#17）；`Cell<&'long mut T>` 可缩短并 unsize-coerce；**`repr(transparent)` 对「trivial 字段」判定更严格** | releases.md 1.98；编译器诊断引用 `library/core/src/sync/atomic.rs:2768`（注意：**版本文档日期为 2026-08-20，本机 rustc commit 日期 2026-08-18**） |

> 注：本表是按 `releases.md`（在线等价 `https://doc.rust-lang.org/stable/releases.html`）逐段摘出的**索引**，不是逐条转写；写作票引用具体 API 时请按版本号回查该段原文，避免二次转写误差。1.98 段落起始于本机 `releases.md` 第 3 行、1.91 段落起始于第 782 行。

### 4.3 仍未稳定（写书时必须标注 nightly，或明确不进正文）

核实：**特性门探针**（`rustc --edition 2024`，stable 1.98；见附录 A）与 Unstable Book 目录（`unstable-book/` 的 “Language features” 101+ 项）。

| 主题 | 门名 | 1.98 状态 |
|---|---|---|
| 特化 | `specialization` / `min_specialization` | 不稳定（探针 + 实测 `impl<T> T for T { default fn ... }` 报 E0658） |
| const 泛型算术 | `generic_const_exprs`、`min_generic_const_args`、`generic_const_args` | 不稳定 |
| 非整数 const 参数 | `adt_const_params` | 不稳定 |
| `gen` 块 / 协程 | `gen_blocks`、`coroutines`、`coroutine_trait` | 不稳定（实测：`gen blocks are experimental`；`gen` 已是保留字） |
| 关联类型位置的 `impl Trait` | `impl_trait_in_assoc_type` | 不稳定（实测 E0658：``impl Trait` in associated types is unstable`） |
| 任意位置的 `!` 类型 | `never_type` | 不稳定（返回值位置可用；`let x: ! = ...` 报 E0658） |
| 任意 self 类型 | `arbitrary_self_types`、`arbitrary_self_types_pointers` | 不稳定 |
| 自定义分配器 | `allocator_api` | 不稳定 |
| `Try` trait | `try_trait_v2`、`try_trait_v2_residual` | 不稳定 |
| 异步迭代 | `async_iterator`、`async_fn_in_dyn_trait`、`async_for_loop` | 不稳定（`dyn` + `async fn` 实测报 E0038，见 5.3） |
| C 可变参数**定义** | `c_variadic` | 不稳定。**声明**是稳定的：实测 `unsafe extern "C" { fn printf(fmt: *const i8, ...) -> i32; }` 在 1.98 编译通过；1.91 把声明能力扩到 sysv64/win64/efiapi/aapcs，1.93 扩到 `system` ABI；release notes 明言「可声明不可定义」 |
| 泛型原子 | `generic_atomic` | 不稳定：`Atomic<u32>` 这种泛型写法实测报 E0658（`core::sync::atomic::Atomic<T>` 文档页明确标 “nightly-only experimental API (generic_atomic #130539)”）。既有具体类型（`AtomicU32` 等）可用；1.98 在具体类型上稳定了 `from_mut`/`from_mut_slice`/`get_mut_slice` |
| trait alias | `trait_alias` | 不稳定 |
| 其他常见候选 | `default_field_values`、`fn_delegation`、`ergonomic_clones`、`deref_patterns`、`exhaustive_patterns`、`decl_macro`、`box_patterns`、`type_alias_impl_trait`、`associated_type_defaults`、`marker_trait_attr`、`tuple_trait`、`ptr_metadata`、`offload`、`explicit_tail_calls`、`frontmatter` | 均不稳定（Unstable Book 语言特性清单） |

`[推断]` **写作策略建议**：正文只写稳定特性；对 `gen` 块、特化、`generic_const_exprs`、`impl Trait in assoc type`、`arbitrary_self_types` 这五项，各给一个 2–4 行的「未来展望盒」（有 RFC/已保留关键字可引），明确标 nightly。

### 4.4 1.94→1.98 的近期行为变化（会「打脸旧稿」的少数项）

核实：`releases.md` 各版 Compatibility Notes。

- **1.94**：标准库宏改为**经 prelude 导入**，不再注入 `#[macro_use]`；同名宏的 glob import 会变歧义错误（`ambiguous_panic_imports` 警告处理 `panic!`）。闭包捕获在模式匹配上更精确（可能改变 `Drop` 时机、触发新借用错误）。
- **1.95**：`if let` guard 上的 `#[cfg]`、`repr(transparent)` 相关 future-incompat 警告（`repr(C)` 类型不再算 trivial）、`#[test]` 放在无意义位置现在报错、`deref_nullptr` 转 deny。
- **1.96**：`#[repr(Int)]` 枚举在 ZST 无值字段边界情况的布局修复；`Pin` 相关强制转换收紧（不能对不实现 `Deref` 的类型 unsize-coerce 成 `Pin<Foo>`）。
- **1.97**：**v0 symbol mangling 默认**（旧调试器/剖析器可能无法 demangle，backtrace 文本也可能变化）；`pin!` 去掉 deref coercion（`pin!(x)`，`x: &mut T` 现在稳定得到 `Pin<&mut &mut T>`）；无布局保证的 enum 编码实现有变；`#[export_name=""]` 报错。
- **1.98**：`repr(transparent)` 更严格；**`derive(PartialOrd)` 在同时 `derive(Ord)` 时走快路径**（PartialOrd/Ord 不一致的 crate 可能出错）；模式常量匹配的结构相等检查漏洞被补上；`&mut` 在 invariant 位置也可缩短生命周期（`Cell<&'long mut T>` → `Cell<&'short mut dyn Send>`）。

---

## 5. 已过时 / 易踩坑说法清单

格式：**说法** → 现状（含一手来源）→ `[推断]` 写作处置。

### 5.1 已过时（旧教程仍在传播）

| # | 过时说法 | 现状（核实） | 来源 |
|---|---|---|---|
| 1 | 「trait 里不能写 `async fn`，得用 `async-trait` crate」 | 1.75 起 `async fn` in trait 稳定。**但仍有两个真限制**：(a) 含 `async fn` 的 trait **dyn 不兼容**（实测 E0038，且 `async_fn_in_dyn_trait` 仍不稳定）；(b) 返回的 future 不保证 `Send` | releases.md 1.75；实测 E0038 + `doc.rust-lang.org/reference/items/traits.html#dyn-compatibility` |
| 2 | 「trait 里不能返回 `impl Trait`」 | 1.75 起 RPITIT 稳定；1.82 加 `use<..>`；1.87 让 `use<..>` 在 trait 内可用 | releases.md 1.75/1.82/1.87；探针 `return_position_impl_trait_in_trait`→1.75.0 |
| 3 | 「async 闭包不支持，只能返回 boxed future」 | 1.85 起 async 闭包稳定 | releases.md 1.85；探针 `async_closure`→1.85.0 |
| 4 | 「`dyn Trait` 不能向上转型到 supertrait」 | 1.86 起 trait upcasting 稳定 | releases.md 1.86 |
| 5 | 「`if let` 不能与 `&&` 连用；let chains 要 nightly」 | 1.88 起在 **2024 edition** 可用。实测：2021 报 `let chains are only allowed in Rust 2024 or later` | releases.md 1.88；edition-guide/rust-2024/let-chains.html；实测 |
| 6 | 「match guard 里不能写 `if let`」 | 1.95 起稳定（`if let` guards） | releases.md 1.95；探针 `if_let_guard`→1.95.0 |
| 7 | 「GAT 还不稳定」 | 1.65 起稳定 | releases.md 1.65；探针 `generic_associated_types`→1.65.0 |
| 8 | 「`let ... else` 要 nightly」 | 1.65 起稳定 | 探针 `let_else`→1.65.0 |
| 9 | 「`extern` 块直接写就行；`#[no_mangle]` 直接标」 | **2024 edition 起**必须写 `unsafe extern` 与 `#[unsafe(no_mangle)]`（新语法 1.82 起各 edition 均可用）。实测：2024 报错，2021 通过 | edition-guide/rust-2024/unsafe-extern.html、unsafe-attributes.html；实测 |
| 10 | 「`static mut` 取引用最多是个警告」 | 2024 edition 起 `static_mut_refs` 为 **deny**。实测：2021 warning / 2024 error。正确做法：`&raw mut`、原子类型、`OnceLock`/`Mutex` | edition-guide/rust-2024/static-mut-references.html；实测 |
| 11 | 「`unsafe fn` 体内自动获得 unsafe 权限」 | 1.82 起 lint 存在；**2024 edition 默认 warn**。实测 2021 无警告、2024 warn[E0133] | edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html；实测 |
| 12 | 「`&raw const x` 是 nightly 的 `addr_of!` 替代」 | 1.82 起 `&raw const/mut` 稳定（`addr_of!` 仍可用） | releases.md 1.82；探针 `raw_ref_op`→1.82.0 |
| 13 | 「const 泛型实参必须全部显式写出」 | 1.89 起可写 `_` 让编译器推断 | releases.md 1.89；实测通过 |
| 14 | 「const 泛型里能做算术/依赖表达式」 | 仍不稳定（`generic_const_exprs`） | 探针；Unstable Book |
| 15 | 「`lazy_static!`/`once_cell` 是唯一选择」 | 标准库有 `OnceLock`（1.70）、`LazyLock`/`LazyCell`（1.80） | releases.md 1.70/1.80；std 徽标 |
| 16 | 「`#[bench]` 用来写基准测试」 | 1.88 起 `#[bench]` **彻底 destabilize，成为硬错误** | releases.md 1.88 |
| 17 | 「`cargo test -- --nocapture`」 | 1.88 起弃用，改为 `--no-capture` | releases.md 1.88 |
| 18 | 「最新 edition 是 2021」 | 2024 于 1.85（2025-02-20）稳定 | edition-guide/rust-2024/index.html（RFC #3501 / Release version 1.85.0） |
| 19 | 「no_std 里不能用 `Error` trait」 | 1.81 起 `core::error::Error` 可用 | 探针 `error_in_core`→1.81.0；releases.md 1.81 |
| 20 | 「lint 只能在源码里写 `#![deny]`」 | Cargo `[lints]` 表（1.74）+ `[workspace.lints]` | cargo/CHANGELOG（1.74：“The `[lints]` table has been stabilized”） |
| 21 | 「Cargo 只有 resolver v1/v2」 | 1.84 稳定 resolver v3（MSRV-aware），2024 edition 默认 | cargo/CHANGELOG 1.84；edition-guide/rust-2024/cargo-resolver.html |
| 22 | 「`cargo publish` 一次只能发一个包」 | 1.90 稳定多包发布（含互相依赖的 workspace 包） | cargo/CHANGELOG 1.90；releases.md 1.90 |
| 23 | 「指针转整数随便是 UB / provenance 无关紧要」 | 1.84 稳定 strict provenance API（`ptr::with_exposed_provenance`、`NonNull::expose_provenance` 等） | releases.md 1.84 |
| 24 | 「`assert_matches!` 和 `assert_eq!` 一样开箱即用」 | 1.96 才稳定，且**不在 prelude**，实测必须 `use std::assert_matches;` | releases.md 1.96；实测错误信息 `consider importing this macro: use std::assert_matches;` |
| 25 | 「`std::sync::Once` / `RwLock` 足够，锁降级不可能」 | 1.92 起 `RwLockWriteGuard::downgrade` 稳定 | releases.md 1.92 |

### 5.2 易踩坑（不是过时，而是**新版 Rust 特有的反直觉点**）

| # | 坑 | 说明（核实） | 来源 |
|---|---|---|---|
| 1 | **`LazyLock`/`LazyCell` 的 `force`/`force_mut`/`get`/`get_mut` 不能用方法语法** | std 源码把它们声明成显式接收者形式：`pub fn force(this: &LazyLock<T, F>) -> &T`、`pub fn get(this: &LazyLock<T, F>) -> Option<&T>`（`LazyCell` 同理）。带名字的 `this: &T` 接收者在 stable 上**只是关联函数、不是方法**（我另写的最小复现 `fn m(this: &S)` 同样无法用 `s.m()` 调用），所以 `lazy.force()` / `lazy.get()` 一律编译失败（E0599：`this is an associated function, not a method`），必须 UFCS：`LazyLock::force(&lazy)`、`LazyCell::get(&lc)`，或走 `Deref`（`*lazy`）。注意 `force` 从 1.80 起就是这种形式，**不是 1.94 引入的新坑**；1.94 新增的是 `get`/`get_mut`/`force_mut` | std 源码（本机 rust-src：`library/std/src/sync/lazy_lock.rs:177,241,283,311`、`library/core/src/cell/lazy.rs:138,179,277,301`）+ std 文档签名 + 1.98 实测报错 |
| 2 | **`pin!` 的 deref coercion 在 1.97 被移除** | 1.88 起一度允许的错误强制转换被撤销：`pin!(x)`（`x: &mut T`）现在稳定得到 `Pin<&mut &mut T>` | releases.md 1.97 |
| 3 | **标准库宏现在走 prelude（1.94）** | 自定义同名宏 + glob import 会在 1.94 起变歧义错误；`panic!` 的歧义走 `ambiguous_panic_imports` 警告 | releases.md 1.94 |
| 4 | **临时值 drop 时机在 2024 edition 变了** | `if let` scrutinee 的临时值在进入 `else` 前 drop；尾表达式的临时值可能早于局部变量 drop。1.91 还把 `pin!`/`format_args!`/`write!` 在 if-let scrutinee 的临时值统一按 2024 规则处理（**该改动与 edition 无关**，所有 edition 生效） | edition-guide/rust-2024/temporary-if-let-scope.html、temporary-tail-expr-scope.html；releases.md 1.91 |
| 5 | **never type fallback 改了，两个 lint 变 deny** | 2024 语义改变 + 1.92 起 `never_type_fallback_flowing_into_unsafe`、`dependency_on_unit_never_type_fallback` 为 deny | edition-guide/rust-2024/never-type-fallback.html；releases.md 1.92 |
| 6 | **`repr(transparent)` 判定更严格（1.98）** | `repr(C)` 类型、含私有字段的类型、`#[non_exhaustive]` 类型不再算「trivial 字段」，可能影响 FFI 包装类型 | releases.md 1.98 |
| 7 | **不要依赖无 `repr` 保证的 enum 布局** | 1.97 改了某些 enum 的编码实现（官方声明「非 breaking」，但已见有人因假设布局算法而受影响）；`repr(C)` 枚举判别值超 `c_int`/`c_uint` 也新增 future-incompat 警告（1.93） | releases.md 1.97、1.93 |
| 8 | **v0 mangling 成为默认（1.97）** | 旧版调试器/剖析器可能无法 demangle；backtrace 文本格式可能变化——性能章与调试章需警示 | releases.md 1.97；rustc/symbol-mangling/v0.html |
| 9 | **`derive(PartialOrd)` + `derive(Ord)` 走快路径（1.98）** | 两者若手工实现且互相不一致，行为会变 | releases.md 1.98 |
| 10 | **`Box<[T]>` 的 `.into_iter()` 语义随 edition 变** | 2024 起按值产出 `T`，此前按引用；迁移 lint `boxed_slice_into_iter` 会改写为 `.iter()` | edition-guide/rust-2024/intoiterator-box-slice.html |
| 11 | **`unsafe extern` 的签名正确性是块作者的责任** | 写错签名直接是 UB 来源；`improper_ctypes*` 只能查一部分 | edition-guide/rust-2024/unsafe-extern.html（RFC 3484） |
| 12 | **AFIT 的 future 不保证 `Send`** | 实测：`trait T { async fn go(&self) -> u32; }` + `async fn use_it<X: T>(x: X) { let f = x.go(); needs_send(f); }`（`fn needs_send<F: Send>(_: F)`）报 **E0277 `impl Future<Output = u32>` cannot be sent between threads safely**；把签名改写成 `fn go(&self) -> impl Future<Output = u32> + Send + '_` 后同一代码编译通过。这是把它与 `tokio::spawn`/`Send` 边界组合时的头号坑 | 1.98 实测（两组对照）；reference/items/traits.html（RPIT 捕获与 dyn-compatibility）；book ch17.5 |
| 13 | **`Pin` 只对 `!Unpin` 提供保证；`get_mut` 需要 `Unpin`** | 常见误用是以为 `Pin<&mut T>` 总能变 `&mut T` | std/pin/index.html、std/pin/struct.Pin.html；book ch17.5 |
| 14 | **`Rc`/`RefCell` 不是 `Send`/`Sync`** | 跨线程必须换 `Arc`/`Mutex`；`unsafe impl Send` 是**安全承诺**不是「解锁开关」 | nomicon/send-and-sync.html；book ch16.4 |
| 15 | **`#[test]` 放在无意义位置现在报错（1.93）** | 以前被静默忽略（如放在 trait 方法/类型上），现在直接报错，可能影响 rustdoc 生成 | releases.md 1.93 |
| 16 | **`cfg` 相关：`cfg(true)` 才刚能写；`missing_fragment_specifier` 已成硬错误（1.89，全 edition）** | 宏里漏写片段说明符（`$e` 而非 `$e:expr`）以前是 lint，1.89 起所有 edition 报错 | edition-guide/rust-2024/missing-macro-fragment-specifiers.html；releases.md 1.88/1.89 |
| 17 | **原子类型的「泛型拼写」与 `*_mut_slice` 的方向** | (a) `AtomicU32` 在 1.98 的编译器视图中就是 `Atomic<u32>`，但直接写 `Atomic<u32>` 仍报 `generic_atomic` 不稳定 —— **只能用具体类型名**。(b) 方向易写反：`AtomicU32::from_mut(&mut u32) -> &AtomicU32`、`from_mut_slice(&mut [u32]) -> &mut [AtomicU32]`，而 `get_mut_slice(&mut [AtomicU32]) -> &mut [u32]`（两个方向我都实测编译验证） | 1.98 实测（`--emit metadata` 全部通过）+ 编译器诊断引用 `library/core/src/sync/atomic.rs:2768` + std 文档 `core/sync/atomic/struct.Atomic.html` |

### 5.3 `[推断]` 需谨慎引用的「权威但滞后」来源

核实（GitHub API，2026-10-18 查询）：

- `rust-lang/async-book`：最近 push **2026-05-24**，未归档。其内容仍以「手写 executor + `futures` 生态」为主线，与本机 1.98 的 Book ch17 存在重叠与潜在的版本滞后 → **不建议作为事实来源，只作为「深入异步内部」的补充读物**，引用前逐条对照 `std::future`/`std::pin`/Book ch17。
- `rust-lang/api-guidelines`：最近 push **2025-07-08**，未归档 → 作为 API 设计章的一手规范可用（但需注意它不涵盖 2024 edition 之后的语法风格）。
- `rust-lang/unsafe-code-guidelines`：最近 push **2026-05-05**，**仍是进行中的非规范性工作**（原文定位为 reference 的补充）→ 引用时必须标「非最终规范」。
- `doc.rust-lang.org/nightly/miri/` **404**（实测）→ Miri 的一手入口是 `https://github.com/rust-lang/miri` 与 Cargo Book 的 `cargo miri` 手册页（`cargo/commands/cargo-miri.html`，其中明确「only available on the nightly channel」）。
- 本机 `html/guide-*.html`（第一版 The Book 遗留页：`guide-macros.html`、`guide-ffi.html`、`guide-unsafe.html` 等）**高度过时**，不得作为来源。

---

## 6. 给「05 内容版图」的直接输入（`[推断]`）

1. **主干只要 13 个进阶主题就能撑起「进阶」定位**：I1（trait 设计）、I2（dyn 兼容性）、I3（impl Trait/RPIT）、I6（生命周期/variance）、I7（内部可变性）、I8（错误处理与 API 设计）、I9（并发）、I11（async）、I12（Pin）、I13（unsafe/UB）、I15（布局）、I16（FFI）、I19（性能）。其余（I4/I5/I10/I17/I18/I20–I24）按「每部分配实战」的需要挂靠。
2. **必须新增、且官方书籍系统性缺失的 4 块**：variance/borrow-checker 心智模型（I6）、dyn 兼容性与 API 形状（I2）、错误处理/API 设计（I8）、性能剖析（I19）。
3. **`[推断]` 依赖序直接给出的章节骨架**：批 0 约定章 → 批 1 类型与 trait 主干 → 批 2 并发/异步 → 批 3 系统层（unsafe/布局/FFI）→ 批 4 工程实践 → 批 5 深水选读。
4. **地图 `Not yet specified` 的 6 个「待 05 定」深挖项，本票给出的现状判断**：async/`Pin`（I11+I12，稳定面足够，可深挖）；宏（I17+I18，稳定面够，proc-macro 需 `syn`/`quote` 外部依赖）；unsafe/FFI（I13–I16，官方有 Nomicon 打底 → 深挖价值在「现代做法 + Miri + provenance」）；并发与内存序（I9+I10，`Ordering` 语义有一手依据可写）；性能剖析（I19，官方有 rustc/cargo 章节但缺「工作流」）；错误处理与 API 设计（I8，官方只有 API Guidelines，缺系统教程）。**六项都够格单独开票**。

---

## 附录 A. 实测方法与可复现产物

环境：`rustc 1.98.0 (88d9e12ae 2026-08-18)`，host `x86_64-pc-windows-msvc`，工作目录 `D:/Rust/.scratch/rust-guide/research/tmp-probe/`（**已在收尾时删除**）。

**A.1 特性门探针**：对每个特性名生成 `#![feature(<name>)] fn main(){}`，以 `rustc --edition 2024 --crate-type bin` 编译并解析诊断。
- 稳定：诊断含 “the feature `X` has been stable since 1.x.0…”
- 不稳定：`error[E0554]: #![feature] may not be used on the stable release channel`
- 已移除：`error[E0557]: feature has been removed`
结果见 §4.2 与 §4.3 的「探针」备注。

**A.2 语义/语法实测**（每项单独文件，`--edition 2021` 与 `2024` 对照）：
- `#[no_mangle]`、`extern "C" {}`、`unsafe fn` 内的裸解引用、`static mut` 取引用、let chains、`Box<[T]>` for 循环 —— 结果见 §4.1 表格。
- 「不稳定」实测：`gen { yield 1; }`、`impl<T> T for T { default fn .. }`、`type F = impl Future`、`let x: ! = ...`、`Atomic<u32>`、`Box<dyn Trait>` where trait has `async fn` —— 结果见 §4.3 与 §5.1/§5.2。
- 「已稳定」正样本批量编译（`--emit metadata`，绕开链接）：`LazyLock`/`OnceLock`/`LazyCell`、`c"..."`、`Vec::extract_if`、`HashMap::extract_if`、`Option::take_if`、`strict_*`、`offset_of!`（含嵌套字段）、`core::error::Error`、`Path::file_prefix`、`Duration::from_hours`、`core::array::repeat`、`as_chunks`、`RwLockWriteGuard::downgrade`、`str::floor_char_boundary`、`into_raw_parts`、`assert_matches!`（验证「需显式导入」这一条）、`if let` guard、`async` 闭包、trait upcasting、`pin!` + `Pin::set`、`thread::scope`、`derive(Default)` 带 `#[default]` 枚举 —— 除 `assert_matches!`（缺 `use`）外全部通过，验证了 §4.2 表与 §5.1 的判定。
- 专项实测（用于 §5.2）：AFIT 的 `Send` 缺失（E0277）与手写 `impl Future + Send` 的对照；`LazyLock/LazyCell` 的 `force`/`get` 方法语法失败（E0599）与 UFCS 通过；`AtomicU32::from_mut`/`from_mut_slice`/`get_mut_slice` 的方向；自定义 `fn m(this: &S)` 确证「带名 `this:` 接收者不是方法」；`unsafe extern "C" { fn printf(...) }` 可变参数声明可编译。

**A.3 std 版本徽标**：解析本机 std HTML 中 `title="Stable since Rust version X"`，用于 §4.2 中标注了「std 徽标」的条目。

**A.4 版本映射**：把本机 `releases.md` 的关键行号映射到最近的 `Version x.y.z` 段（用于 §4.2 的 1.63/1.68/1.70/1.77/1.81/1.84/1.91/1.96 等条目）。

**A.5 std 源码核对**：本机安装了 rust-src，`library/std/src/sync/lazy_lock.rs`、`library/core/src/cell/lazy.rs`、`library/core/src/sync/atomic.rs` 用于确证接收者形式与方法方向。

---

## 附录 B. 不确定 / 未核实项（写作票须回查）

1. **§4.2 是索引而非转写**：表内每行都是对 `releases.md` 相应段落的摘要，个别 API 名可能被概括；写作时以 `doc.rust-lang.org/stable/releases.html` 对应段落为准。
2. **C 风格可变参数「声明」最早在哪个版本稳定**：只核实到 1.91/1.93 扩展到具体 ABI，未找到 C ABI 本身的稳定版本行；写 FFI 章时按「1.98 上可用」表述即可，勿写具体版本。
3. **`AtomicU32` 与 `Atomic<u32>` 的源码关系**：编译器把 `AtomicU32` 展示为 `Atomic<u32>`（`atomic_int!` 宏特化），但二者在 stable 上的**可用拼写**差异已在 §5.2#17 实测记录；若写作票要引用「类型别名」措辞，请回查 `library/core/src/sync/atomic.rs`。
4. **《Rust 1.98.0》博客日期**：`releases.md` 记为 **2026-08-20**，本机 rustc commit 日期为 **2026-08-18**；两者是「发布日期 vs 构建提交日期」，引用时选其一并注明口径。
5. **`gen` 块的最终形态**：`gen_blocks`/`coroutine_trait` 的具体稳定计划未核实（RFC 3513 只核实到「保留关键字」）。
6. **各深水主题的现有优秀第三方资源**（如 `loom`、`criterion`、`proptest` 的官方文档版本状态）未做核实 → 留给对应深挖票。
7. **`rust-lang/async-book` 是否会被官方新 async 章节取代**：未核实（只核实到它仍在维护但节奏慢）。
8. **本文件所有 `[推断]` 结论**（分层、依赖边、批次建议、深挖票判断）需 05 票独立决策，不得当成事实基线。
