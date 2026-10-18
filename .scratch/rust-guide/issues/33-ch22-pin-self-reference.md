# 33 第 22 章 Pin 与自引用

Type: task
Status: claimed

## Question

写第三篇第四章：**Pin 与自引用**（编排上必须在第 21 章「先有 Future」之后讲，已满足）。

- **范围**：自引用为什么是问题（值被移动后内部指针失效的最小构造）；`Pin<P>` 的契约（「不移动」的保证到底由谁承担）；`Unpin` 的含义与「几乎所有的类型都 Unpin」；`pin!` 与 `Box::pin` 的用法与差别；`PhantomPinned` 制造 `!Unpin`；「什么时候真的需要 `unsafe`」的边界清单（投影规则预告 → 第 38 章）。
- **素材**：`research/02-knowledge-structure.md` 的 I12 行（依赖 I11/I6；`std/pin/index.html`、`std/pin/struct.Pin.html`、书 ch17.5）；本机 1.98 快照核对 `Pin` 的方法签名与 `Unpin` 实现清单。
- **边界**：unsafe 四能力与 UB 清单 → 第 23 章；手写 pin 投影 / 自引用容器 → 第 38 章（第 34 章可回指）；`Pin` 在 async 里的角色在第 21 章末尾已引出（回指不重讲）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；所有示例在 1.98 + 2024 上真跑；nightly-only 只标注。
- **DoD**：同 §8 协议 —— 独占 `src/ch22-pin-self-reference.md` + `snippets/ch22/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch22-pin-self-reference.md`；探针 `.scratch/tmp-33/`；收尾 `claimed` + `## Comments`（同 §8 清单）。

## Comments

### 文件

- 写了 `src/ch22-pin-self-reference.md`（八节齐全：这一章解决什么问题 / 为什么这样设计 / 机制与规则 / 动手看看 / 报错现场 / 小结 / 练习 / 延伸阅读；正文 5246 汉字，不含代码块）。
- 新建 `snippets/ch22/selfref.rs`（完整可编译程序：`PhantomPinned` 自引用类型 + `pin!` 就地 pin + `spawn()` 返回 `Pin<Box<SelfRef>>`）。
- 未碰 `SUMMARY.md` / `book.toml` / `WRITING.md` / `GLOSSARY.md` / 其他票与其他章文件。

### 自验命令与结果

- `mdbook test -c ch22-pin-self-reference.md` → **exit 0**（15 个测试：11 个真跑、3 个 `compile_fail`、2 个 `rust,ignore` 的 trait/struct 声明片段；`动手看看` 的 `{{#rustdoc_include}}` 被预处理展开后同样编译并运行）。
- 注入反证（已还原）：把 `snippets/ch22/selfref.rs` 里 `assert!(pinned.points_to_self())` 改成 `assert!(!...)` → 同一命令 **exit 101**，rustdoc 报 `test ch22-...::动手看看 (line 445) ... FAILED`，`test result: FAILED. 5 passed; 1 failed`；还原后复跑 exit 0。门禁不是假绿。
- `rustc --edition 2024 snippets/ch22/selfref.rs -o selfref.exe && .\selfref.exe` → 输出 `pin!     stack -> true` / `Box::pin heap -> true`。
- 所有行内示例均用同样命令单文件真跑（探针在 `.scratch/tmp-33/`，已删）。

### 实测过的输出清单（全部 1.98 / edition 2024 / x86_64 Windows）

| 示例位置 | 实测输出 |
|---|---|
| 为什么这样设计：地址三次变化 | `栈上: 0x210a6ffbe0` / `Vec 里: 0x19b6a918500` / `Box 里: 0x19b6a911410` / `moves_seen = 0` |
| 22.1 自引用被移动后失效 | `原地初始化后：true` / `移进 Box 之后：false` / `data = hello` |
| 22.2 `Pin::set` 原地换值 | `drop first` / `现在 second` / `地址不变 true` / `drop second` |
| 22.3 `Box::pin` 跨作用域 | `job-1 -> true` / `job-1 -> true` |
| 22.4 `Unpin` 断言（通过集） | `这些都是 Unpin` |
| 22.4 手动 `impl Unpin` | `v` |
| 22.4 `!Unpin` 断言失败（compile_fail） | `error[E0277]: PhantomPinned cannot be unpinned` 两条（第二条含 `within HasPin`） |
| 动手看看 | `pin!     stack -> true` / `Box::pin heap -> true` |
| 现场一 E0277 | `PhantomPinned cannot be unpinned`；`note: required by a bound in Pin::<Ptr>::new` → `impl<Ptr: Deref<Target: Unpin>> Pin<Ptr>` |
| 现场二 E0596 | `cannot borrow data in dereference of Pin<Box<Task>> as mutable`，help 指向 `DerefMut` 未实现 |
| 现场三 E0716 | `temporary value dropped while borrowed`（`pin!` 的值带出块） |
| 练习 3 代码 | 真跑通过（`Waker::noop()` + 手动 `Future::poll`，两个断言都过） |
| 额外核对 | `impl !Unpin for Foo {}` → `error[E0658]: negative impls are experimental`；`super let` 单独写同样 E0658（`pin!` 内部用 `allow_internal_unstable`，正文按此标注）；`Box::pin(PhantomPinned)` 上 `*p = PhantomPinned` → **E0594**（cannot assign to data in dereference），方法调用才是 E0596（练习 1 答案按实测区分了这两个码） |

### 遗留不确定

- 引擎本机跑出来的地址每次不同，正文已注明「地址每次运行都不同」；`Pin::set` 的例子打印的是地址是否相等（`true`），不是具体数值。
- `UnsafePinned<T>`（1.98 已在 `std::pin` 里，但需 `#![feature(unsafe_pinned)]`）只在术语层面被考虑，正文未展开（属第 23 章的别名规则范围）。
- 素材 `research/02-knowledge-structure.md` 实际在 `.scratch/rust-guide/research/02-knowledge-structure.md`（我最初按仓库根找，误报为不存在，**此处更正**）。补核 I12 行原文：「**`Pin` 与自引用**：`Pin<P>`、`Unpin`、`pin!`/`Box::pin`、`PhantomPinned`、何时才真的需要 unsafe | I11, I6 | I13 | std/pin/index.html、std/pin/struct.Pin.html、book ch17.5」——五项均在正文落地（依次 22.2 / 22.4 / 22.3 / 22.5 / 22.6），上游依赖 I11（第 21 章已引出）与 I6（第 16 章 variance/dropck 已讲，本章 22.2 回指），下游 I13（第 23 章）与 D5（第 38 章 pin 投影）已在 22.6/22.7 划界；本机 1.98 快照另用于逐条核对签名、`where` 约束与英文引文。
- `DerefMut for Pin<Ptr>` 在 1.98 走内部 helper（`PinDerefMutHelper`，源码里是 `Ptr::Target: Unpin`），正文按语义写成「`Ptr: DerefMut` 且 `Ptr::Target: Unpin`」，并给了 E0596 实测佐证。

### 新增术语（请协调者并表）

- **自引用（self-referential）**：类型内部持有指向自己某部分的指针。首现给了中英对照。
- **地址敏感（address-sensitive）**：值的某段状态要求地址不变，移动即失效（`std` 文档用语）。
- **pin**：动词用法保留英文（「pin 住」「被 pin 的值」「就地 pin」），首现解释为「钉住、固定就位」；`Pin`/`Unpin`/`PhantomPinned`/`pin!`/`Box::pin` 一律代码体保留。
- **pin 投影（pin projection）** 与 **结构化 pinning（structural pinning）**：正文首现给中英，预告第 38 章。
- `_Avoid_` 自检：未用「转移 / 特征 / 箱子 / 崩溃 / 未定义（省「行为」）」；统一用「移动」「trait」「crate」「panic」「UB」。

### 衔接提示

- 正文对第 21 章的引用不依赖其细节：只说 `Future::poll` 的接收者是 `self: Pin<&mut Self>`、`async fn` 的 future 是 `!Unpin`。若第 21 章末尾已引出 `Pin`，请在该处回指本章 22.3（三个构造入口）。
- 第 14 章 950–962 行、第 13 章 220 行都写了「第 21、22 章把这条链路讲全」（`Pin<Box<dyn Future + Send + '_>>` 的形状）——本章 22.3 与练习 3 覆盖了 `Box::pin` 一侧，`Future`/`poll` 一侧仍需第 21 章落地。
- 第 12 章已讲 `Pin` 是 fundamental type constructor，本章 22.2 回指该处。
- 第 16 章的 dropck 已讲；本章 22.2 末尾只把「pin 的保证延续到析构跑完」与它对接，未展开 dropck 细节。
- 第 23 章（unsafe 四能力与 UB）与第 34/38 章（手写容器 / 手写 pin 投影）的边界已在 22.6、22.7 明确划出。

### 图示需求（本章未画图）

建议后续补一张内嵌 SVG：`Pin<&mut T>` 的三层结构（`T` 的值 / `&mut T` 指针 / `Pin` 包装）与「移动前后」对照——左侧 `ptr` 指向自身字段成立，右侧值被搬到新地址后 `ptr` 仍指旧地址。正好承载 22.1 那段 `true → false` 的实测。另有候选图：`Unpin` 的字段传播（一个 `PhantomPinned` 字段关掉整个类型的 `Unpin`）。

### 别处观察（未改）

- 跑 `mdbook test -c ch22-pin-self-reference.md` 时，mdbook 预处理整个 book，报了**第 20 章**的 `{{#rustdoc_include ../snippets/ch20/litmus.rs}}` 读不到文件（`snippets/ch20/litmus.rs` 目前不存在）。这与本章无关，但会污染并行期间的日志；等第 20 章票落地即消失。若非并行期仍复现，请检查该票。

### 交叉审校（RvCh22）修改记录

协调者转来 13 × minor（0 blocker / 0 major），已逐条处理，只动了本章文件与本票：

1. 现场一栏位宽度按真实输出统一（该 error 含 4 位行号 `1159`，rustc 会把同级栏位补到 4 位宽）：分隔行改为 5 空格 + `|`，`  4 |` 改为 `   4 |`。现场一/二/三与 22.4 四处输出都用 `rustc --edition 2024 src/main.rs` 重新生成后逐行 diff 过。
2. 现场一补回被略去的行：`     |                  |` + `required by a bound introduced by this call`、`--> src/main.rs:4:8`、rustc 自带的 `...` 与 `1181 | …` 两行；并在「报错现场」总说明里写明省略规则（路径前缀压成 `…/`、末尾 `error: aborting…` 与 `For more information…` 略去、`...` 为省略标记）。
3. 22.4 第二条错误补回 `= note: consider using …` 两行，末尾以 `...` 收尾（第一条的约束出处保留，便于对照）。
4. `get_unchecked_mut` 英文引文改为完整句（`… you receive when you call this function, so that the invariants on the Pin type can be upheld.`）并配译文，不再截断后改成句号。
5. 练习 3 参考答案里的 `run` → `poll_once_local`（与练习代码一致）。
6. 「容器」4 处全部改口径：22.1「真实的自引用集合」、22.5「手写集合里」、22.7「手写集合时的组合细节」、练习 2「真实的自引用集合……一个这样的集合」；`grep -c 容器` 现为 0。
7. 「所有权转移」2 处（为什么这样设计 + 小结）改为「所有权的移动」。
8. 术语首现格式统一：「这一章解决什么问题」第 2 条给动词 pin 的释义（被 pin（钉住、固定就位）的是…）；方法表 `map_unchecked*` 行给「投影（pin projection）」；`pointee（被指向值）` 改为「被指向值（pointee）」；22.7 标题改「pin 投影与结构化 pinning（预告）」，正文给「结构化 pinning（structural pinning）」（原为逗号并列）。
9. 22.6 引言改为「**下面这些地方要么要你自己写 `unsafe`，要么要你额外承担 pin 的契约**（第 5 项属于后者——`Drop` 的实现本身是 safe 语法，只是写法必须按 pin 处理）」；第 5 项标题与正文同步点明 `unsafe` 只出现在 `Pin::new_unchecked` 一步。
10. 22.2 末段改写：pin 的 **Drop guarantee**（值一旦被 pin，存储到 `drop` 返回前不得复用或失效）与第 16 章的 dropck 分开叙述，不再说成同一机制。
11. 开篇改为「第 21 章的 `Future::poll` 签名里出现过 `self: Pin<&mut Self>`，却没有展开它为什么不是 `&mut self`；`async fn` 编译出的 future 是 `!Unpin` 这件事，也留到这一章补上」，不再声称第 21 章「留下了两处」。
12. safe 面表述更正：构造侧 `Pin::new` 要求 pointee `Unpin`；取回 `&mut T` 走 `Pin::get_mut` / `Pin::into_inner` / `DerefMut`（不再把 `Pin::new` 混进「取回 `&mut T` 的通道」）。

复跑与复检：`mdbook test -c ch22-pin-self-reference.md` → **exit 0**。四处粘贴输出的逐行 diff：现场二、现场三、22.4 逐字一致；现场一仅「工具链路径前缀」按书中约定缩写为 `…/library/core/src/pin.rs:1159:25`，其余逐字一致。正文 5495 汉字（修改过程中曾到 5527，压缩三处引导句后回到 4000–5500 内）。探针目录 `.scratch/tmp-33f/` 及本票产生的临时文件已删。
