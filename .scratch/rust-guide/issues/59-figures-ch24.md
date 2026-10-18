# 59 第四批图示：第 24 章（指针、provenance 与未初始化内存）

Type: task
Status: claimed

## Question

给 `src/ch24-pointers-provenance.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 42 的「图示需求」。

- **图清单**（1 张，必做）：
  1. **四种指针形态 × 地址 / 来历**：`*mut T` / `NonNull<T>` / `usize` / `without_provenance` 四行，两栏「有没有地址、有没有来历、能不能解引用」，把 provenance 的「地址对 ≠ 可用」一眼摊开（与 24.1/24.2 的严格来历叙述一致）。
- **放置**：插在 provenance 小节内（按正文位置就近）；图前一句话引出；不写图注编号。`text` 块仅在「被新图完整覆盖」时可删（论证并记票）。
- **边界**：只加图与引出句；正文一字不动（本章刚定稿、修回后）。
- **DoD**：§8 —— 独占 `src/ch24-pointers-provenance.md` + 本票；禁 `git`/禁 build；`mdbook test -c ch24-pointers-provenance.md` 绿；SVG 自查同票 58 口径；栅格化目视 `.scratch/tmp-59/`（收尾删）；收尾 `Status: claimed` + `## Comments`。

## Comments

- **文件**：`src/ch24-pointers-provenance.md`（只插图与引出句，正文其余一字未动）。插入位置：`### 指针上的 provenance API`（旧第 111 行）小节开头——第 113 行两段导语之后、`**稳定面（1.98 上可用，无需 `#![feature]`）**` 之前。新增内容 = 第 115 行引出句 + 第 117–150 行单块 `<svg>`（文件 542 行 → 579 行，全部增量集中在这一处；临时反证块已还原，`grep -c probe-59 src/ch24-pointers-provenance.md` = 0）。未动其他任何文件；探针目录 `.scratch/tmp-59/` 已删除。
- **引出句**（新第 115 行）：「先把四种「指针形态」摆在一起：地址能不能对上是一回事，能不能拿来用是另一回事。」（图前一句引出；无图注编号）
- **图**：`viewBox="0 0 720 252"`、`width="100%"`、无 `height`、`role="img"`；aria-label = 「四种指针形态的地址与来历对照：\*mut T 与 NonNull&lt;T&gt; 既有地址又有来历（NonNull 保证非空），usize 与 without_provenance 只有地址、没有来历，因此不能解引用——地址对不等于可用」。
- **图内容**：表头三栏「有地址？/ 有来历？/ 能解引用？」，四行 —— `*mut T`（有（可空、可悬垂）/ 有（派生链决定）/ 能（unsafe））、`NonNull<T>`（有（保证非空）/ 有 / 能（unsafe））、`usize`（有（就是地址本身）/ 无 / 不能（不是指针））、`without_provenance(addr)`（有（你给的数值）/ 无 / 不能（非零大小 UB））；底注两行把「地址都能对上 ≠ 能用」与 `with_exposed_provenance` / `without_provenance` 的分歧收口。行内颜色只用 `currentColor`（线/字）、`var(--links)`（有来历）、`var(--warning-border)`（无来历 / 不能解引用）、`var(--quote-border)`（表格线）。
- **事实来源（本机 1.98 快照 `C:\Users\tyd27\.rustup\...\share\doc\rust\html\std\ptr\` 逐条核对）**：
  - `index.html` §Provenance：「A pointer value in Rust semantically contains the following information: The address it points to, which can be represented by a `usize`. The provenance it has, defining the memory it has permission to access. Provenance can be absent, in which case the pointer does not have permission to access any memory.」→ 「有地址 / 有来历」两栏的判据。
  - `index.html` §Pointers Vs Integers：「a `usize` cannot accurately represent a pointer, and converting from a pointer to a `usize` is generally an operation which only extracts the address.」→ `usize` 行「有地址、无来历」。
  - `index.html` §Safety：「For a pointer to be valid for reads/writes, it is necessary, but not always sufficient, that the pointer be dereferenceable. The provenance of the pointer is used to determine which allocation it is derived from…」→ 底注「这次访问得落在来历覆盖的范围内」。
  - `fn.without_provenance.html`：「no-provenance pointer may be used for zero-sized memory accesses (if suitably aligned), but non-zero-sized memory accesses with a no-provenance pointer are UB. No-provenance pointers are little more than a `usize` address in disguise.」→ 第 4 行「无来历 / 不能（非零大小 UB）」。
  - `struct.NonNull.html`：「Unlike `*mut T`, the pointer must always be non-null, even if the pointer is never dereferenced. … However the pointer may still dangle if it isn't dereferenced.」→ 第 2 行「有（保证非空）」。
- **§9 自查（同票 58 口径，逐项脚本过）**：块内无空行 ✓；无 `<script>` / 外链 / `<image>` / mermaid ✓；`viewBox` + `width="100%"` 且无 `height` ✓；颜色仅 4 个主题变量、无写死色 ✓；`role="img"` + `aria-label` ✓；`&` / `<` / `>` 已转义（正文 `NonNull&lt;T&gt;`，aria-label 同）✓；本图无列对齐的工具输出，故未用 `xml:space="preserve"`（不需要，非遗漏）✓；线宽 1.5、无渐变/阴影 ✓；字号 12/13/14 ✓。
- **栅格化目视**：把正文第 117–150 行原样切到 `.scratch/tmp-59/fig59.svg`，`read …:img` 栅格化目视 —— 四行三栏对齐、无重叠；`without_provenance(addr)` 标签宽约 173 px，未越出左栏（216 px）；底注两行（约 456 px / 698 px）都在 720 宽度内。栅格器不解析 `var()`（`--links` / `--warning-border` / `--quote-border` 处按黑渲染，未消失），最终配色以协调者收尾 build 的 HTML 为准。
- **自验命令与结果**：
  - `mdbook test -c ch24-pointers-provenance.md` → **exit 0**（0.5.4 成功时不打印计数，只有 INFO 行）。
  - 反证：临时在 `</svg>` 之后插入 `fn main() { let x: u32 = "probe-59"; }` → **exit 101**，rustdoc 输出 `running 16 tests` / `13 passed; 1 failed; 2 ignored`；15 个既有块逐条列出且全部 ok/ignored（含 SVG 之后的 `line 209 / 269 / 325 / 348 / 431 / 477 / 582 / 650 / 697 / 733 / 772 / 777 / 795`），**证明 SVG 块没有破坏其后的围栏解析**（否则会假绿）；还原后复跑 exit 0 —— 与票 42 基线 `13 passed; 2 ignored` 一致。
- **`text` 块删留论证（票面要求逐条论证）**：可删的前提是「被新图完整覆盖」。本章 `text` 块共 4 处：`Option<NonNull<Node>> = 8 字节 / Option<*mut Node> = 16 字节`（讲大小，与地址/来历无关）、`E0308` 诊断、`E0015` 诊断、`` `ManuallyDrop::<T>::take` is not yet stable as a const fn `` 诊断。新图只覆盖「地址 / 来历 / 能否解引用」的对照，与这四块的主题均不重叠，因此**一块未删**。
- **新增术语**：无。图内只出现「地址、来历（provenance）、解引用、派生链、零大小访问」等本章与 `GLOSSARY.md` 已用的词。
- **衔接提示（给协调者）**：图放在「稳定面」表之前，读者先拿到四种形态的对照再读 API 表；`with_exposed_provenance` 按要求**未单列成行**，只在图内底注与紧随其后的 API 表出现。
- **遗留不确定**：栅格化工具不解析 CSS 变量，深色/浅色主题下的实际配色与 `--warning-border` 的对比度只能在收尾 build 的 HTML 里确认（本票禁 build）。

