# 62 第 34 章 手写集合与智能指针

Type: task
Status: open

## Question

写第六篇第一章：**手写集合与智能指针**（素材 = `research/02` 的 D1 行：Nomicon `vec.html`、`arc-and-mutex.html`）。

- **范围**：从能跑的 mini `Vec` 出发——RawVec 式增长与容量台阶（用实测台阶证明）、`Drain` 守卫与「守卫 drop 时收尾」的安全写法、ZST 特例（`cap = usize::MAX` 惯例）；然后简化版 `Arc`（原子计数 + `Acquire`/`Release` 配对理由 + `Weak` 不阻断释放）与 `Mutex`（`UnsafeCell` + park 或自旋）。unsafe 契约写法与 ch23 对齐（`// SAFETY:` 注释、`# Safety` 文档）。
- **要求**：全部 stable 真跑（`snippets/ch34/` 真编译 + 测试；关键结构用 `size_of`/字节转储对照 ch25 结论）；**Miri 无 nightly，不得给 Miri 输出**（只文档化命令并标注）。
- **边界**：智能指针基础 → ch17 回指；布局/niche → ch25；unsafe/UB → ch23；`loom` 只提名字并明说未跑。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 需要处内嵌 1–2 张 SVG（如 RawVec 增长/Drain 生命周期）。
- **DoD**：§8 —— 独占 `src/ch34-handwritten-collections.md` + `snippets/ch34/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch34-handwritten-collections.md` exit 0 + `snippets/ch34/` 的 `cargo test`（私有 target）全绿；探针 `.scratch/tmp-62/`；收尾 `Status: claimed` + `## Comments`。

## Comments
