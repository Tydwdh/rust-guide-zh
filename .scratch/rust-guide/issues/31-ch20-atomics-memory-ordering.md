# 31 第 20 章 原子与内存序

Type: task
Status: open

## Question

写第三篇第二章：**原子与内存序**（依赖 I9 的 `Send`/`Sync`/`UnsafeCell` 语义）。

- **范围**：`Ordering` 五值的语义与选用（`Relaxed`/`Acquire`/`Release`/`AcqRel`/`SeqCst`）；happens-before 的直觉与「没有它会发生什么」（两线程 flag/消息对演示）；`compare_exchange` 的两种失败序与 CAS 循环（`fetch_*` 家族概览）；`UnsafeCell` 与「原子类型为什么需要内部可变性」；无锁入门的判断标准（何时不值得手写 —— 回指第 17 章的锁）。
- **素材**：`research/02-knowledge-structure.md` 的 I10 行（依赖 I9/I13；`std/sync/atomic/enum.Ordering.html`、Nomicon atomics / races）；本机 1.98 快照核对 `Atomic*` API 与文档原文。
- **边界**：别名模型/形式化语义 → 第 35 章；性能与剖析 → 第 29 章；`loom` → 第 31 章（一句）；`Send`/`Sync` 基础 → 第 19 章（回指不重讲）。
- **篇幅**：4000–5500 汉字（不含代码块）；八节模板；练习 2–3 题带答案；内存序演示必须真跑（可跑两种序对照并给真实输出）；nightly-only 只标注。
- **DoD**：同 §8 协议 —— 独占 `src/ch20-atomics-memory-ordering.md` + `snippets/ch20/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch20-atomics-memory-ordering.md`；探针 `.scratch/tmp-31/`；收尾 `claimed` + `## Comments`（同 §8 清单：文件 / 自验命令与结果 / 实测输出清单 / 遗留不确定 / 新增术语 / 衔接提示 / 图示需求）。

## Comments
