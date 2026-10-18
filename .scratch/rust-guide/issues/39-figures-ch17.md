# 39 进阶篇图示：第 17 章（智能指针与内部可变性）

Type: task
Status: claimed

## Question

给 `src/ch17-smart-pointers.md` 补内嵌 SVG（规范 = `WRITING.md` §9）。需求出处：票 28 的「图示需求」（1–4 必做；5 可选）。

- **图清单**：
  1. **`Rc`/`Arc` 堆布局与两档计数**（替换现有 1 个 `text` ASCII 图）：两个句柄 → 右侧共享分配（`strong`/`weak` 两格 + 值），`Weak` 指向同一分配但不占 `strong`。建议宽 ~700。
  2. **两层检查并存**（`Rc<RefCell<T>>`）：`Rc` 的计数（编译期不可见）与 `RefCell` 的借用计数（运行期）两个并排计数块。
  3. **`Ref`/`RefMut` guard 借用区间**：`borrow()`/`borrow_mut()` 到 guard drop 之间的区间条，冲突处标 panic（对应报错现场二）。
  4. **四格矩阵**（共享×线程）：`Rc`/`Arc` × `Cell`/`RefCell`/`Mutex`/`RwLock` 的 2×2 定位，与章末 `Send`/`Sync` 总表互为表里。
  5. （可选）`Box<[T]>`/`Box<str>`/`Rc<str>` 的「瘦指针 vs 胖指针（地址+长度）」对比条。
- **放置**：插在对应小节内；每图前一句话引出；不写图注编号。ASCII 图若被 1 号新图**完整覆盖**可删（逐条论证）；否则保留。
- **边界**：只加图与引出句；正文文字/代码块/表格不动。
- **DoD**：§8 协议 —— 独占 `src/ch17-smart-pointers.md` + 本票；禁 `git`/禁 build；加图后自验 `mdbook test -c ch17-smart-pointers.md` 绿；SVG 自查项同票 34；栅格化目视 `.scratch/tmp-39/`（收尾删）；票尾 `## Comments` 逐图记录。

## Comments

### 收尾记录（图 1–5 全部落地）

独占文件：`src/ch17-smart-pointers.md`（仅加 SVG 与引出句）+ 本票。未动代码块/正文/表格；未碰 `SUMMARY.md`/`book.toml`/其他票。

**图 1 · `Rc`/`Arc` 堆布局与两档计数** — 现 L136–163（`### Rc\<T\> 与 Arc\<T\>` 小节内）
- 引出：沿用本段原有冒号句「它的模型很简单：」（未新增文字）。
- `aria-label`：「Rc 与 Arc 的堆布局：a、b、w 三个句柄各占 8 字节指向同一块堆分配；分配里有 strong 与 weak 两档计数和值，Weak 不计入 strong」。
- 内容：左三个句柄（`Rc<String> a` / `Rc<String> b` 实线、`Weak<String> w` 虚线）→ 右「堆上同一块分配」；分配内三格 `strong = 2`（--links）/ `weak = 1`（--quote-border）/ `"共享的字符串"`；末尾两行注释点出「Weak 不强留内容、不计入 strong」与「Rc 与 Arc 布局相同，只差计数怎么写」。
- **ASCII 删除论证**：原 `text` 块（5 行）只表达「三句柄 → 同一分配（strong/weak + 值），Weak 不计入 strong」；新图逐项覆盖同一关系，且补上句柄 8 字节、Arc 的计数差异两点 → **被完整覆盖，故删除**。新图插在「它的模型很简单：」与「图里三个句柄各占 8 字节……」之间，紧邻那句指代「图」的正文，指代关系不破；全章现无残留 ASCII 图（`grep 'Rc<String> a'` = 0）。

**图 2 · 两层检查并存（`Rc<RefCell<T>>`）** — L465–483（`### RefCell\<T\>` 内，`Rc<RefCell<T>>` 段之后、`[!WARNING]` 之前）
- 引出句（新增）：「两层检查叠在同一个类型上，各管一件事：」。
- `aria-label`：「Rc 与 RefCell 的两层检查：Rc 的强引用计数在堆上、编译期不可见，RefCell 的借用计数在运行期决定这次借出放不放行」。
- 内容：左 `Rc<RefCell<T>>` 句柄 → 右「堆上同一块分配」；并排两块 `strong = 2`（--links，注「编译期不可见」）与 `borrow = 1`（--warning-border，注「运行期决定放行」），下方 `T 的值`；页脚点明两层分工与「冲突只在第二层、只在运行期爆发」。

**图 3 · `Ref`/`RefMut` guard 借用区间** — L427–459（`### RefCell\<T\>` 的「规则要点」列表之后、`Rc<RefCell<T>>` 段之前）
- 引出句（新增）：「把 guard 的生命周期画成区间，冲突就一目了然：」。
- `aria-label`：「Ref 与 RefMut guard 的借用区间：两个共享 guard 的区间可以重叠、可变 guard 另起一段；若共享借用落在可变 guard 的区间内，就在运行期 panic」。
- 内容（沿用第 4 章区间图口径）：图例「共享 guard（Ref）」/「可变 guard（RefMut）」；① 两条 `Ref` 区间重叠且同点结束（虚线标「最后一次使用」）、`RefMut` 另起一段 → 运行期放行；② `RefMut（活到函数末尾）` 的区间内含 `Ref（拒绝）` 的区间 → 红色虚线底注「✗ 共享借用落在可变 guard 的区间内 → panic：RefCell already mutably borrowed」，正对应「报错现场二」。

**图 4 · 四格矩阵（共享 × 线程）** — L85–105（`### 先记住四个格子` 的表之后、`std::cell` 段之前）
- 引出句（新增）：「这张表摊平了就是一张十字图：横轴问「跨不跨线程」，纵轴问「要不要改」。」。
- `aria-label`：「四个格子：按要不要改与跨不跨线程把本章类型分成 Rc、Arc、Cell 与 RefCell、Mutex 与 RwLock 四格，横向换列就是换件清单」。
- 内容：列头「单线程/跨线程」、行标「只要共享/还要能改」；四格 `Rc<T>`（计数非原子）、`Arc<T>`（计数原子）、`Cell<T> / RefCell<T>`（检查挪到运行期）、`Mutex<T> / RwLock<T>`（检查挪到锁上）；页脚给出换件清单 `Rc<RefCell<T>> → Arc<Mutex<T>>`，并注明 `Box<T>` 不进这张表——与章末 `Send`/`Sync` 分布表表里相扣（分布表原文未动）。
- 前置的四行表格**保留**：该表按「能力 × 线程」列类型、还含 `Box` 那一行，图只做十字定位，未完整覆盖该表 → 按票中「否则保留」处理。

**图 5（可选）· 瘦指针 vs 胖指针** — L156–170（`### Box\<T\>` 的 `size_of` 表与 `Box<dyn Trait>` 段之后）
- 引出句（新增）：「上表的 8 与 16 差在哪儿，摊开就是下图：」。
- `aria-label`：「瘦指针与胖指针对比：瘦指针只有一个地址机器字，胖指针多带一个长度或 vtable 指针，因此是 16 字节」。
- 内容：上条一格「地址」= 8 字节（`Box<u64>`）；下条两格「地址」+「长度 / vtable 指针」= 16 字节；底注列出「带长度的：`Box<[u64]>`、`Box<str>`、`&str`、`&[T]`」与「带 vtable 的：`Box<dyn Trait>`、`&dyn Trait`」，并点明指针自身大小与堆上数据大小无关。**本图已做，未略过。**

### 自验

- `mdbook test -c ch17-smart-pointers.md` → `INFO Testing chapter '第 17 章 智能指针与内部可变性': "ch17-smart-pointers.md"`，`EXIT=0`（插图前后各跑一次均绿；SVG 不参与代码测试，只确认栅栏与 `{{#include}}` 未被碰坏）。
- SVG 结构自查（脚本，同票 34 口径）5/5 全过：块内无空行；无 `<script>`/`src`/`href`；`viewBox` + `width="100%"` 且无 `height`；`role="img"` + `aria-label`；颜色仅 `currentColor` 与 `var(--links)`/`var(--warning-border)`/`var(--quote-border)`；`&`/`<`/`>` 全部实体化（零裸 `&`）。块定位：L85-105、L156-170、L178-205、L427-459、L465-483。
- 栅格化目视：`.scratch/tmp-39/`（已按协议删除）。发现并修掉一处：图 3 图例原在 x=430/560，第二条「可变 guard（RefMut）」在 660 宽处出框 → 左移到 x=310/480 后复核正常。其余四图目视无越界、无文字压线、箭头落点正确（图 1 三条箭头分别落在 strong/weak 行；图 2 单箭头落在分配框）。
- 无 `xml:space="preserve"` 需求：本批图未复刻带列对齐的工具输出（代码行各为独立 `<text x="0">`），无连续空格会被折叠的栏位。

### 遗留/衔接

- HTML 实际渲染（真实主题下 `var(--…)` 的观感）留给协调者收尾 build。
- 未新增术语；未改 `SUMMARY.md`。
- `.scratch/tmp-39/` 已删除；本批其他章节图票各自用 `tmp-<票号>` 目录，无冲突。
