# 39 进阶篇图示：第 17 章（智能指针与内部可变性）

Type: task
Status: open

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
