# 27 第 16 章 生命周期进阶：variance、HRTB 与 drop check

Type: task
Status: open

## Question

写第 16 章：**生命周期的形式化直觉** —— 子类型从哪来、`for<'a>` 怎么用、drop 时借用怎么查。

- **范围**：variance（协变 / 逆变 / 不变：`&'a T`、`&'a mut T`、`fn(T)`、`PhantomData` 各自；为什么需要不变）；HRTB（`for<'a> Fn(&'a str)` 语法与真实用例，如把闭包传给需要任意生命周期的函数）；drop check 与 drop 顺序（为什么「析构里还会用借用」要单独检查；`#[may_dangle]` 只提名字与用途）；`PhantomData` 的三重用法（占位、调方差、影响 drop check）；生命周期省略规则温故（第 4 章的直觉 → 精确规则表）。
- **素材**：`research/02`（I6）；nomicon 的 subtyping / variance / dropck / PhantomData 节；reference 的 lifetime elision 页。
- **边界**：智能指针 → 17；别名模型与形式化（Stacked/Tree Borrows）→ 35；`unsafe` 与 UB → 23。
- 篇幅 4500–6000 汉字（本章偏难，允许偏长）；八节模板；练习 2–3 题带答案；nightly-only 只标注。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch16-lifetimes-advanced.md`）。
