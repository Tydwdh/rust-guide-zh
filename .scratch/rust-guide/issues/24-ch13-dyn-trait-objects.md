# 24 第 13 章 dyn 兼容性与 trait 对象

Type: task
Status: open

## Question

写第 13 章：**`dyn` 与 trait 对象** —— 动态分发的规则、代价与选型。

- **范围**：对象安全（哪些 trait 能变成 `dyn`，哪些不能；`Self: Sized` 手法；方法与接收者的限制）；`dyn Trait` 的三种持有形式（`&dyn`/`Box<dyn>`/`Arc<dyn>`）与各自场景；vtable 是什么（对照第 9 章的单态化：动态分发的代价在哪）；upcasting（`dyn Sub → dyn Super`，按 1.98 事实写）；`dyn` vs 泛型 vs `impl Trait` 的选型清单；`Box<dyn Error>`/`&dyn Iterator` 的实用例。
- **素材**：`research/02`（I2 条目与 §5）；`reference/items/trait-objects.html`；nomicon 的 vtable 节。
- **边界**：RPIT/AFIT → 14；API 设计 → 18；`unsafe` 里 vtable 布局细节 → 25/26。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注；可选「语言对照角」（Java 接口 / C++ 虚函数）。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch13-dyn-trait-objects.md`）。
