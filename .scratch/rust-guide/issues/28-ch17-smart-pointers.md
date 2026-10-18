# 28 第 17 章 智能指针与内部可变性

Type: task
Status: open

## Question

写第 17 章：**智能指针与内部可变性** —— 共享、可变、跨线程三者的工具箱。

- **范围**：`Box`（堆分配、递归类型、与 `Box<dyn>` 的第 13 章衔接）；`Rc`/`Arc`（共享所有权、`Weak` 防环、`Rc` 不是 `Send`/`Sync`）；`Cell`/`RefCell`（运行时借用检查、`RefCell` 越界借用会 panic 的语义、`Cell` 的 `get/set/take/replace`）；`Mutex`/`RwLock`（跨线程内部可变性、锁中毒一句、与第 19 章的衔接）；`Cow`（写时克隆的典型场景）；`Deref`/`DerefMut` 的陷阱（自动强转的边界、`*` 的行为）；一张「`Send`/`Sync` 在这些类型上的分布」总表。
- **素材**：`research/02`（I7 + §5.2 里 `Rc`/`RefCell` 跨线程那条）；std 对应类型页 + book ch15；`reference` 的 interior-mutability 相关页。
- **边界**：并发基础（thread/Send/Sync 系统讲）→ 19；原子 → 20；别名模型 → 35。
- 篇幅 4500–6000 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注；可选「语言对照角」（C++ shared_ptr/unique_ptr、Java 引用）。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch17-smart-pointers.md`）。
