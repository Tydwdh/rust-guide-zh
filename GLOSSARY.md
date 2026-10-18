# 《Rust 指南》（暂名）的术语约定

本书是中文 Rust 指南：正文简体中文，术语首现写「中文（English）」，之后按本表执行。本表既是写作一致性依据，也是将来书末术语附录的来源；新增或改动术语时先查社区《Rust 语言术语中英文对照表》（CC0）并采纳主流用法，分歧记在这里（依据：`research/01-zh-rust-resources.md` §3）。

## Language

**所有权（ownership）**：
值与其存储的归属关系：谁拥有、何时转移、何时释放。
_Avoid_: 拥有权、归属权

**借用（borrow）**：
不转移所有权地访问值；`&T` 共享借用，`&mut T` 可变借用（同一时刻只允许一个）。
_Avoid_: 租借；「引用」只指具体类型 `&T`/`&mut T`，不与「借用」混用

**生命周期（lifetime）**：
借用的有效区间，写作 `'a`。
_Avoid_: 生存时间、寿命

**借用检查器（borrow checker）**：
编译期验证借用规则的组件；本书「诊断训练」里最常打交道的对象。
_Avoid_: 借用检查

**切片（slice）**：
对序列中一段连续元素的借用视图（`&[T]`、`&str`）。
_Avoid_: 分片、片段

**智能指针（smart pointer）**：
带所有权语义与额外行为的指针类型（`Box`/`Rc`/`Arc`…）。
_Avoid_: 智能引用

**内部可变性（interior mutability）**：
通过共享引用修改数据的模式（`Cell`/`RefCell`/`Mutex`…）。
_Avoid_: 内部可变

**trait**：
行为契约。**保留英文不译**（首现时用一句话说明；「特征/特质」在中文里歧义大、社区不统一）。
_Avoid_: 特征、特质

**泛型（generics）**：
编译期参数化的类型与函数（配合单态化理解）。
_Avoid_: 模板（那是 C++ 的概念）

**模式匹配（pattern matching）**：
用模式解构并分支（`match`/`if let`/`let else`）。
_Avoid_: 模式配对

**迭代器 / 闭包（iterator / closure）**：
一次产出序列元素的对象；捕获环境的匿名函数。
_Avoid_: 迭代子、匿名函数（后者只在与其他语言对照时用）

**宏（macro）**：
`macro_rules!` 声明宏；过程宏单列为「过程宏（proc macro）」。
_Avoid_: 宏定义（过泛）

**panic**：
不可恢复错误的展开路径，与可恢复的 `Result` 相对。
_Avoid_: 崩溃

**unsafe**：
开启四个超能力（解引用裸指针、调用 `unsafe fn`、访问可变静态、实现 `unsafe trait`）的记号；不是「关闭检查」的开关。
_Avoid_: 不安全代码（作名词时写「unsafe 代码」）

**未定义行为（UB, undefined behavior）**：
编译器不保证任何结果的程序状态；用 Miri 检测。
_Avoid_: 未定义（省略「行为」）

**内存序（memory ordering）**：
原子操作可见性的规则（`Ordering` 五值）。
_Avoid_: 内存顺序（可用但不混用，全书统一「内存序」）

**数据竞争（data race）**：
并发的未同步读写同一内存；Rust 用 `Send`/`Sync` 在安全代码里排除它。
_Avoid_: 竞态（更宽的概念，不混用）

**零成本抽象（zero-cost abstraction）**：
不引入运行时开销的抽象。
_Avoid_: 无开销抽象

**crate / 包（package）**：
crate = 编译单元；包 = 一个或多个 crate 的发布单位（`Cargo.toml`）。
_Avoid_: 箱子；「库」仅指 library crate

---

## 维护规则

- 新术语先进本表再进正文；写作中发现新术语，**在同一张写作票里**补进本表。
- 与社区术语表冲突时以「主流实际用法」为准（01 findings §3 已列三方分歧），分歧本身记进本表。
- 将来书末附录由本表派生（去掉写作向的 `_Avoid_` 备注或保留，届时定）。
