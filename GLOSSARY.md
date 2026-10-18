# 《Rust 指南》（暂名）的术语约定

本书是中文 Rust 指南：正文简体中文，术语首现写「中文（English）」，之后按本表执行。本表既是写作一致性依据，也是将来书末术语附录的来源；新增或改动术语时先查社区《Rust 语言术语中英文对照表》（CC0）并采纳主流用法，分歧记在这里（依据：`research/01-zh-rust-resources.md` §3）。

## 语言与运行时

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

**引用计数（reference counting）**：
多个所有者各持一份计数、最后一份离开时释放的模式（`Rc`/`Arc`）。
_Avoid_: 计数引用

**强引用 / 弱引用（strong / weak reference）**：
`Rc`/`Arc` 的两档引用：强引用保持存活；弱引用（`Weak`）不阻止释放，用前须 `upgrade`。
_Avoid_: 强指针 / 弱指针

**写时克隆（clone-on-write）**：
`Cow` 的模式：只读时借用，要改时克隆一份再改。译名用「写时克隆」（它走 `Clone` 不走 `Copy`）。
_Avoid_: 写时复制

**互斥锁 / 读写锁（mutex / read-write lock）**：
`Mutex` 与 `RwLock`；首现给中文名，正文通常直接写类型名。
_Avoid_: 互斥体

**锁中毒（poisoning）**：
持锁线程 panic 后锁被标记为脏：之后 `lock` 返回 `Err`（`PoisonError`），可用 `into_inner` 取回。
_Avoid_: 中毒状态

**通道（channel）**：
线程间的消息传递设施（`std::sync::mpsc`）。
_Avoid_: 信道

**交会（rendezvous）**：
容量为 0 的 `sync_channel`：发送方必须等到接收方取走。
_Avoid_: 会合

**原子类型（atomic type）**：
以原子操作访问的类型（`AtomicUsize` 等），不靠锁保证无数据竞争。
_Avoid_: 原子量

**读-改-写（read-modify-write，RMW）**：
`fetch_add`/`compare_exchange` 这类一步完成读与写、对外原子可见的操作。
_Avoid_: 原子的读改

**CAS（compare-and-swap）**：
比较并交换：值符合预期才写入，失败时告诉你当前值。**保留英文缩写**。
_Avoid_: 比较交换（首现可括注英文）

**先行发生（happens-before）**：
跨线程可见性顺序的关系：A 先行发生于 B，B 就一定能看到 A 的结果。
_Avoid_: 发生在前

**顺序一致性（sequential consistency）**：
`SeqCst` 提供的全局单一次序；代价最高、直觉最接近「按程序顺序执行」。
_Avoid_: 强一致

**伪失败（spurious failure）**：
CAS 的失败之一：值其实没变，只是这次没成功；`compare_exchange_weak` 允许它出现。
_Avoid_: 假失败

**无锁（lock-free）**：
进度保证：任一时刻至少一个线程能推进（不是「不用锁」的字面意思）。
_Avoid_: 免锁

**无等待（wait-free）**：
更强的进度保证：每个线程都能在有限步内完成。
_Avoid_: 免等待

**存储缓冲（store buffer）**：
CPU 为写操作排队的缓冲；它的乱序可见性是 `Relaxed` 生效场景的来源之一。
_Avoid_: 写缓冲（可混用，全书统一「存储缓冲」）

**自旋锁（spinlock）**：
拿不到锁就循环重试的锁；适合临界区极短的场合。
_Avoid_: 忙等锁

**ABA 问题（ABA problem）**：
CAS 循环的经典陷阱：值从 A 变 B 又变回 A，旧比较会误判「没变过」。
_Avoid_: AB 问题

**完整屏障（memory barrier）**：
`SeqCst` 语境要求的全序屏障。
_Avoid_: 内存栅栏

**执行器（executor）**：
驱动 future 前进的组件：负责 poll，并在 `Waker` 被调用后重新调度。
_Avoid_: 调度器（那是 OS 概念）

**任务（task）**：
交给执行器的一份已装箱的 future（`spawn` 的单位）。
_Avoid_: 作业

**运行时（runtime）**：
驱动 async 程序的整套环境（执行器 + I/O 支持）。
_Avoid_: 运行环境

**唤醒（wake）与唤醒句柄（`Waker`）**：
future 通知执行器「可以再 poll 我」的机制；`Waker` 可跨线程调用。
_Avoid_: 通知器

**状态机（state machine）**：
`async` 块展开后的形态：每个 `.await` 是一个状态。
_Avoid_: 自动机

**取消（cancellation）**：
把 future drop 掉——不再被 poll，等待点上的状态随之析构。Rust 的取消就是 drop。
_Avoid_: 中止

**线程池（thread pool）**：
固定数量的工作线程从共享队列取任务执行的并发结构。
_Avoid_: 线程队

**工作线程（worker）**：
线程池里执行任务的线程。
_Avoid_: 工人线程

**任务队列（task queue）**：
存放待执行任务的队列（线程池的工作来源）。
_Avoid_: 作业队列

**句柄（handle）**：
代表某个资源或后台任务的轻量标识（如 `TaskHandle`、文件句柄）。
_Avoid_: 收据

**背压（backpressure）**：
有界队列满时阻塞生产端，让生产与消费的速率匹配。
_Avoid_: 反压

**队头阻塞（head-of-line blocking）**：
慢任务把排在它后面的任务一起卡住；轮询派发的代价。
_Avoid_: 头部阻塞

**FFI（Foreign Function Interface）**：
跨语言调用接口。**保留英文缩写**。
_Avoid_: 外部接口

**ABI（application binary interface）**：
调用约定 + 符号命名 + 布局的二进制层面约定。**保留英文缩写**。
_Avoid_: 二进制接口

**调用约定（calling convention）**：
参数/返回值的传递与栈清理规则。
_Avoid_: 调用规范

**符号修饰（symbol mangling）**：
编译器把函数名变成唯一符号名的过程。
_Avoid_: 名字修饰

**空指针优化（nullable pointer optimization）**：
`Option<&T>`/`Option<fn>` 等用 null 表示 `None` 的布局保证。
_Avoid_: 空指针技巧

**不透明句柄（opaque handle）**：
C 侧只见类型名不见字段的指针；Rust 侧用 `into_raw`/`from_raw` 交接所有权。
_Avoid_: 黑盒句柄

**契约（contract）**：
编译器无法检查、由人兑现的前提；`# Safety` 段与 `// SAFETY:` 注释是它的落点。
_Avoid_: 约定（过泛）

**健全的 / 不健全的（sound / unsound）**：
对任意安全客户端都不会触发 UB，称健全；否则不健全。
_Avoid_: 安全（混用）

**裸指针（raw pointer）**：
`*const T` / `*mut T`，无借用检查、可空、可悬垂。
_Avoid_: 原始指针

**可变静态（mutable static）**：
`static mut`；读、写、取引用都要 `unsafe`（2024 起取引用默认 deny）。
_Avoid_: 可变全局变量

**未初始化（uninitialized）**：
内存尚未写入有效值；`MaybeUninit` 处理的那段时间。
_Avoid_: 未赋值

**裸借用运算符（raw borrow operators）**：
`&raw const` / `&raw mut`（Reference 用语）。
_Avoid_: 原始借用

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
`macro_rules!` 声明宏；过程宏单列为「过程宏（procedural macro）」。
_Avoid_: 宏定义（过泛）

**卫生性（hygiene）**：
宏展开中名字解析的边界：展开出来的名字按定义处还是调用处的规则解析。
_Avoid_: 卫生度

**片段说明符（fragment specifier）**：
`$name:spec` 里 `spec` 那一半（`expr`/`ty`/`tt`…），决定这条规则能匹配什么。
_Avoid_: 片段类型

**元变量（metavariable）**：
匹配器里 `$name` 绑定的片段；转写器靠它拼展开结果。
_Avoid_: 宏变量

**跟随集合（follow set）**：
片段之后允许出现的 token 白名单（`expr` 后不能直接跟 `=>` 等）。
_Avoid_: 后随集合

**TT muncher**：
靠 `tt` 递归「吃」token 的递归宏形态（`$head:tt $($rest:tt)*`）。**保留英文**。
_Avoid_: token 咀嚼器

**过程宏（procedural macro）**：
在编译期运行、以代码生成代码的宏；必须是独立的 proc-macro crate，分 derive / 属性 / 函数式三种形态（第 28 章）。
_Avoid_: proc macro（正文用全称）

**derive 宏 / 属性宏 / 函数式宏（derive / attribute / function-like macro）**：
过程宏的三种形态：在项后追加实现、替换被标项、像函数调用一样展开。
_Avoid_: 派生宏

**token 流 / token 树（token stream / token tree）**：
过程宏的输入输出形态：一串 token（流），括号包住的成组 token（树）。
_Avoid_: 记号流

**辅助属性（helper attribute）**：
derive 宏可以声明的惰性（inert）属性，写在被标项上、随输入传给宏。
_Avoid_: 伴随属性

**不卫生（unhygienic）**：
过程宏默认展开后名字与调用处共享作用域（与 `macro_rules!` 的卫生性相对）。
_Avoid_: 无卫生

**语言方言（edition）**：
同一编译器支持的语言版本方言（2015/2018/2021/2024）；由 crate 级 `edition` 键选择，不是编译器版本。
_Avoid_: 版本（单独用时歧义）

**元变量表达式（meta-variable expression）**：
`macro_rules!` 里在转写期求值的内建表达式（`${count(…)}`/`${index()}`/`${len(…)}` 等）；截至 1.98 仍是 nightly（第 39 章）。
_Avoid_: 元变量宏

**调用点（call site）**：
宏被使用的位置（相对「定义处」）；卫生性与诊断归属都围着它与定义处的区别转。
_Avoid_: 调用处

**panic**：
不可恢复错误的展开路径，与可恢复的 `Result` 相对。
_Avoid_: 崩溃

**unsafe**：
开启五类超能力（解引用裸指针、调用 `unsafe fn`、访问可变静态、实现 `unsafe trait`、访问联合体字段）的记号；不是「关闭检查」的开关（Reference 拆得更细；第 23 章展开）。
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

## 类型与建模

**绑定（binding）**：
用 `let` 给值起的名字。
_Avoid_: 变量声明（Rust 语境下不精确）

**可变绑定（mutable binding）**：
`let mut` 起的、允许重新赋值的绑定。
_Avoid_: 可变变量

**遮蔽（shadowing）**：
同名新绑定遮蔽旧绑定（旧值仍在，只是名字不再可达）。
_Avoid_: 覆盖（那是 trait 实现与一致性（coherence）语境的说法：类型作为另一个类型的参数出现，如 `Vec<T>` 里的 `T` 被「覆盖（covered）」）

**结构体（struct）**：
命名字段的复合类型。
_Avoid_: 记录、record

**元组结构体（tuple struct）**：
按位置访问字段的结构体（`struct Point(i32, i32);`）。
_Avoid_: 位置结构体

**单元结构体（unit struct）**：
没有字段的结构体（`struct Marker;`）。
_Avoid_: 空结构体

**枚举（enum）**：
取值为若干「变体」之一的类型。
_Avoid_: 枚举类型（重言）

**变体（variant）**：
枚举的一个可选形态，可各自携带不同的数据。
_Avoid_: 成员、枚举项

**字段（field）**：
结构体变体里带名字的数据槽。
_Avoid_: 属性（attribute 才是「属性」）

**方法（method）**：
带 `self` 接收者的函数（`fn f(&self)`）。
_Avoid_: 成员函数（C++ 说法）

**关联函数（associated function）**：
不带 `self` 的 `impl` 函数（如 `Vec::new`）。
_Avoid_: 静态方法（Java 说法）

**解构（destructuring）**：
用模式把值拆成各部分（`let Point { x, y } = p;`）。
_Avoid_: 拆包（那是元组/参数展开的说法）

**代数数据类型（ADT, algebraic data type）**：
「且」（结构体）与「或」（枚举）组合出的类型。
_Avoid_: 代数类型（省略「数据」）

**无值类型（uninhabited type）**：
没有任何值的类型（`!`、`enum Never {}`）；匹配它不需要分支。
_Avoid_: 空类型（与单元类型歧义）

**typestate**：
用类型表示值的状态的模式：状态推进表现为类型变化，把误用挡在编译期。**保留英文**。
_Avoid_: 类型状态（首现可括注英文）

**自引用（self-referential）**：
类型内部持有指向自己某部分的指针。
_Avoid_: 自指

**地址敏感（address-sensitive）**：
值的状态要求地址不变：一旦移动即失效（`std` 文档用语）。
_Avoid_: 地址相关

**pin**：
动词：钉住、固定就位——承诺在其析构前不再移动。**保留英文**。
_Avoid_: 固定（单用）

**pin 投影（pin projection）**：
把 `Pin` 保证按字段拆开的写法（手写需要 `unsafe`；第 38 章展开）。
_Avoid_: 拆 pin

**结构化 pinning（structural pinning）**：
「pin 住外层即 pin 住该字段」的字段级契约。
_Avoid_: 结构 pinning

**对齐（alignment）**：
类型地址必须满足的字节边界。
_Avoid_: 对齐度

**填充（padding）**：
为满足对齐插在字段之间的空字节。
_Avoid_: 空隙（歧义）

**判别式（discriminant）**：
枚举用哪个变体的编号/标志（可独立存在，也可折进 niche）。
_Avoid_: 标签（歧义）

**空位（niche）**：
类型中不可能出现的位模式（如空指针）；枚举的空变体可折进去省空间。
_Avoid_: 空洞

**穷尽性（exhaustiveness）**：
`match` 必须覆盖全部可能取值，编译器负责检查。
_Avoid_: 完备性

**匹配守卫（match guard）**：
`match` 分支上附加的 `if` 条件。
_Avoid_: 守卫（单用太泛）

**或模式（or-pattern）**：
用 `|` 组合的多个模式（`Some(1) | Some(2)`）。
_Avoid_: 选择模式

**部分移动（partial move）**：
只移走结构体/枚举中部分字段，其余仍可访问。
_Avoid_: 局部移动

**匹配人体工程学（match ergonomics）**：
让 `match`/`let` 在引用上自动安排绑定模式的规则。
_Avoid_: 匹配人体工学（错字）

**derive（`#[derive]`）**：
让编译器生成 trait 实现的属性。**保留英文**。
_Avoid_: 派生（中译歧义，全书用 derive）

**单元类型（unit type）**：
写作 `()`，唯一值也是它。
_Avoid_: 空类型、void（对照其他语言时才提）

**尾表达式（tail expression）**：
块末尾不带 `;` 的表达式，其值即块的值。
_Avoid_: 返回表达式

**类型标注与推断（annotation / inference）**：
显式写出的类型（`let x: u32`）与编译器补出的类型。
_Avoid_: 类型声明（只看字面）

**整数溢出（integer overflow）**：
超出类型范围的计算；debug 检查并 panic，release 环绕（wrapping）。
_Avoid_: 溢出（不加限定）

## 所有权与借用

**移动（move）**：
所有权从一处转移到另一处，原绑定失效。
_Avoid_: 转移（可用但不统一，全书用「移动」）

**克隆（clone）**：
`Clone` 的显式复制（深浅由实现决定）。`Copy` 是隐式的按位复制，**保留英文**。
_Avoid_: 拷贝（与 `Copy` 混用）

**析构（drop）**：
值离开作用域时的清理。`Drop` 是相应的 trait，**保留英文**。
_Avoid_: 销毁、释放（释放指针另说）

**析构检查（drop check，dropck）**：
保证值析构时其借用仍然有效的编译期检查（比普通借用检查更保守）。
_Avoid_: 析构检查器

**析构作用域（drop scope）**：
值真正运行析构代码的位置（与作用域及绑定书写顺序相关）。
_Avoid_: 消亡域

**健全的泛型析构（sound generic drop）**：
泛型容器允许元素先于容器被释放所需的析构约束；`#[may_dangle]` 是放宽它的手段。
_Avoid_: 安全析构

**动态大小类型（DST, dynamically sized type）**：
大小在编译期未知、只能经由指针使用的类型（`str`、`[T]`、`dyn Trait`）。
_Avoid_: 不定长类型（社区另一说法，全书统一 DST）

**胖指针（fat pointer）**：
除地址外还带元数据的指针（`&str`/`&[T]` 带长度，`&dyn T` 带 vtable）。
_Avoid_: 宽指针

**瘦指针（thin pointer）**：
只含地址的指针（`&u32`、`&String`）。
_Avoid_: 细指针

**解引用强转（deref coercion）**：
`&String` → `&str` 这类自动强转（经 `Deref` 链）。
_Avoid_: 解引用强制转换（同义；全书统一用「强转」）

**自动解引用（autoderef）**：
方法解析时自动沿 `Deref` 链找方法的规则；与「解引用强转」是两个概念。
_Avoid_: 自动 deref

**借用区间（borrow region）**：
借用实际生效的代码区间（NLL 下由最后一次使用决定）。
_Avoid_: 生命周期区间（两个概念不混用）

**别名规则（aliasing rules）**：
同一时刻：共享借用可以有多个，可变借用只能有一个。
_Avoid_: 借用规则（那还包含作用域等更多内容）

**来历（provenance）**：
指针携带的「这块地址属于哪块分配、由哪条引用派生」的信息。
_Avoid_: 出处、来源（易与「引用的来源」混淆）

**严格来历 / 暴露来历（strict / exposed provenance）**：
std 对两组 provenance API 的分类标签；严格来历可推理，暴露来历只能「尽力而为」。
_Avoid_: 严格来源

**别名模型（aliasing model）**：
在 Miri 里实现的候选别名规则（Stacked Borrows / Tree Borrows）——「别名规则」是要求，模型是用来做检查的形式化。
_Avoid_: 别名理论

**重借（reborrow）**：
不转移原指针/引用、重新借出一次的行为（`&mut *p`、传给函数时）；别名模型里的关键动作。
_Avoid_: 再借用

**借用栈（borrow stack）**：
Stacked Borrows 为每个位置维护的「许可栈」：重借压栈，非法访问使上方条目失效。
_Avoid_: —

**许可树（permission tree）**：
Tree Borrows 的树状许可结构：子借用挂成树枝，兄弟分支互不干扰。
_Avoid_: —

**保护符（protector）**：
在函数参数位置上给借用的临时标记：调用期间对应的许可不得被外部访问砸掉。
_Avoid_: 保护者

**子对象来历（subobject provenance）**：
指针携带的「来自哪个子对象」的信息（字段/元素级）；Tree Borrows 的已知不精确处之一。
_Avoid_: 子对象溯源

**retag**：
编译器/Miri 在重借时给指针重新贴标记的动作（指针上的标记即 tag）。**保留英文**。
_Avoid_: —

**FCP（final comment period）**：
RFC/变更走完流程的收尾公示期；别名模型的规则演进也会经过它。**保留英文**。
_Avoid_: —

**守卫（guard）**：
离开作用域时负责收尾的 RAII 小对象（`Drain`、`MutexGuard`…）。与「匹配守卫」（`match` 臂上的 `if`）不同物。
_Avoid_: 卫语句

**悬垂引用（dangling reference）**：
指向已被释放数据的引用；安全 Rust 编译期即拒。
_Avoid_: 野指针（那是裸指针语境）

**重复释放（double free）**：
同一块内存被释放两次；Rust 用所有权与 `Drop` 在安全代码里排除。
_Avoid_: 二次释放

## 集合与字符串

**集合（collection）**：
标准库的序列/映射容器（`Vec`/`String`/`HashMap`…）。
_Avoid_: 容器（C++ 说法，正文不用）

**元素（element）**：
集合/序列里的单个值。
_Avoid_: 项、成员

**容量（capacity）**：
已分配存储能容纳的元素数（`len ≤ capacity`）。
_Avoid_: 大小（那是 length）

**重新分配（reallocation）**：
容量不足时申请更大块并搬移元素。
_Avoid_: 扩容（口语可用，正式叙述用「重新分配」）

**哈希映射（hash map）**：
`HashMap` 的中文名；正文通常直接写类型名 `HashMap`。
_Avoid_: 字典、映射表

**entry API**：
`HashMap::entry` 那套「一次查找、就地插入或修改」的接口。**保留英文**。
_Avoid_: 条目 API

**Unicode 标量值（Unicode scalar value）**：
一个码点（不含代理对区间的 `u32`），Rust 的 `char` 即此。
_Avoid_: 码点（严格说两者不同；只在语言对照时用）

**字素簇（grapheme cluster）**：
用户感知的一个「字符」（可能由多个标量值组成，如 `é`）。
_Avoid_: 字形

## 错误处理

**诊断（diagnostic）**：
编译器输出的一条 error/warning 报文。
_Avoid_: 错误信息（过泛）

**错误码（error code）**：
`E0308` 这类编号，可在错误索引里查。
_Avoid_: 编号（单用）

**最小复现（minimal reproduction）**：
能触发同一问题的最少代码。
_Avoid_: 最小例子（可用，正式叙述用「最小复现」）

**连带错误（cascading errors）**：
由同一根因引出的后续错误。
_Avoid_: 级联错误（可用但不统一）

**span**：
报错里由 `^^^^` 标出的源码范围。**保留英文**。
_Avoid_: 跨度

**lint**：
编译器的静态检查项（`#[warn(...)]` 控制）。**保留英文**。
_Avoid_: 检查项（过泛）

**展开（unwind）**：
panic 时沿调用栈回退并运行析构；与「宏展开」区分。
_Avoid_: 解绑（灌篮？不，那是别的词）

**错误链（error chain）**：
`Error::source()` 串起来的原因序列。
_Avoid_: 错误栈

**错误传播（error propagation）**：
用 `?` 把错误交还给调用者。
_Avoid_: 错误透传（可用但不统一）

**父 trait（supertrait）**：
`trait A: B` 里的 B（A 的实现者必须同时实现 B）。
_Avoid_: 超 trait（直译生硬）

## 模块与工程

**模块（module）**：
crate 内的命名空间单元（`mod`）。
_Avoid_: 包（与 package 冲突）

**可见性（visibility）**：
项能否被别的模块/包访问（`pub` 系列）。
_Avoid_: 访问权限

**命名空间（namespace）**：
名字的可见作用域。
_Avoid_: 名字空间

**重导出（re-export）**：
用 `pub use` 把别处的项搬进自己的对外接口。
_Avoid_: 转发导出

**依赖（dependency）**：
`[dependencies]` 里声明的外部 crate。
_Avoid_: 库（单一 crate 时可用）

**SemVer / 语义化版本（semantic versioning）**：
`主.次.补丁` 的版本约定；`0.x` 期间 minor 版本可以带破坏性变更。
_Avoid_: 语义版本（省略「化」）

**破坏性变更（breaking change）**：
让既有代码编译失败或行为改变的改动；SemVer 里对应 major（`0.x` 期间为 minor）。
_Avoid_: 不兼容变更

**builder（构建器）**：
把「多个可选参数」收集成链式调用的构造模式（`Foo::builder().a(1).build()`）。**保留英文**。
_Avoid_: 构建器模式（首现可括注英文）

**工作区（workspace）**：
多个包共享一个 `Cargo.lock` 与 target 目录的顶层布局。
_Avoid_: 工作空间（较旧译法）

**features**：
Cargo 的条件编译开关。**保留英文**。
_Avoid_: 特性（与 trait 旧译冲突）

**profile（构建配置）**：
cargo 的编译档位（`dev`/`release`），控制优化与调试信息。
_Avoid_: 配置文件（`config` 是另一回事）

**文档注释（doc comment）**：
`///`（给下一项）与 `//!`（给所在模块）的注释，进 rustdoc。
_Avoid_: 文档串

**退出码（exit code）**：
进程返回给 shell 的整数；本书 CLI 约定 `2` 表示用法错误。
_Avoid_: 返回码

**二进制目标（bin target）**：
产出可执行文件的 crate 目标（`src/main.rs`）。
_Avoid_: 可执行文件（谈 Cargo 语境时）

**build script（构建脚本）**：
Cargo 在编译 crate 前运行的构建时程序（`build.rs`），用来探测环境、生成代码或传链接参数。**保留英文**。
_Avoid_: 构建脚本（正文用英文）

**虚拟清单（virtual manifest）**：
只含 `[workspace]`、自身不是包的根清单。
_Avoid_: 纯工作区清单

**加法性（additive）**：
feature 只应增添能力、不应削减能力的设计要求。
_Avoid_: 可加性

**统一（unification）**：
同一依赖被多条路径带上不同 feature 时，按包取并集解析。
_Avoid_: 特征合并

**MSRV（最低支持 Rust 版本）**：
crate 声明并承诺支持的最低 Rust 版本（`rust-version` 键）。
_Avoid_: 最低版本

**yank**：
在 crates.io 撤下一个版本（阻止新项目选中，不删代码）。**保留英文**。
_Avoid_: 撤回

**rustdoc**：
Rust 官方文档工具；也指它生成的文档与诊断。**保留英文**。
_Avoid_: 文档工具（泛称时可用）

**clippy**：
Rust 官方 lint 工具。**保留英文**。
_Avoid_: —

**intra-doc link（文档内链接）**：
文档注释里用项路径写的相对链接（`[`Type`]`、`[text](path)`）。
_Avoid_: 内部链接

**隐藏行（hidden line）**：
doctest/代码块里以 `#` 开头、渲染时收起但参与编译的行。
_Avoid_: 隐藏代码

**剖析（profiling）**：
测「时间花在哪」的通用方法（采样或插桩两类路子）。
_Avoid_: 性能分析（口语可用）

**插桩（instrumentation）**：
往程序里插入记录/计数以取证的手段（覆盖率与 PGO 都用它）。
_Avoid_: 埋点

**中间表示（IR, intermediate representation）**：
编译器内部的程序形式；rustc 先后用 HIR 与 MIR，再交给 LLVM IR。
_Avoid_: 中间代码

**基本块（basic block）**：
一串直进直出的指令（末尾一条终结指令），控制流图的节点。
_Avoid_: 区块

**终结指令（terminator）**：
结束基本块的指令（`br`/`switchInt`/`return`/`call`…）。
_Avoid_: 终止符

**代码生成单元（codegen unit, CGU）**：
给自己发一份 LLVM 模块的编译批次；`-C codegen-units` 调它的大小与并发度。
_Avoid_: 编译单元（与 crate 的「编译单元」义混）

**值槽（MIR local）**：
MIR 里编号的存储位（`_0`、`_1`…）：局部变量、临时值与返回值都放这里。
_Avoid_: 寄存器（MIR 还没分寄存器）

**交叉编译（cross-compilation）**：
在一种平台（host）上产出另一种平台（target）的可执行文件/库。
_Avoid_: 跨平台编译

**目标三元组（target triple）**：
`<arch>-<vendor>-<os>-<abi>` 形式的平台标识（`thumbv7m-none-eabi`）；`--target` 收它。
_Avoid_: 目标字符串

## 泛型与 trait

**单态化（monomorphization）**：
编译期为每个具体类型生成一份泛型代码的实例（零成本抽象的来源）。
_Avoid_: 单态（不完整）

**trait 约束（trait bound）**：
限制泛型参数必须实现的 trait（`T: Trait` / `where`）。
_Avoid_: trait 边界、限定

**关联类型（associated type）**：
trait 里由实现者填写的类型占位（`trait Iterator { type Item; }`）。
_Avoid_: 关联种类

**关联项（associated item）**：
trait（或 impl）里与类型绑定的一组项：类型、常量与函数。
_Avoid_: 关联成员

**扩展 trait（extension trait）**：
为外部类型补方法而定义的本地 trait（`impl VecExt for Vec<i32>`）。
_Avoid_: 扩展方法群

**密封 trait（sealed trait）**：
用私有父 trait 挡住外部实现的 trait：只能由本 crate 实现。
_Avoid_: 封闭 trait

**一致性（coherence）**：
同一 trait 对同一类型最多只能有一个实现的全局保证；孤儿规则是它的一部分。
_Avoid_: 连贯性

**覆盖 / 未覆盖（covered / uncovered）**：
一致性语境：类型参数作为另一个类型的参数出现即「被覆盖」（`Vec<T>` 里的 `T`）；裸 `T` 是「未覆盖」，孤儿规则对它设限。
_Avoid_: 遮蔽（那是变量层面的，见「遮蔽」条）

**固有方法（inherent method）**：
直接写在类型的 `impl` 块里、不来自任何 trait 的方法。
_Avoid_: 内置方法

**blanket impl**：
为一大类类型给出的通用实现（`impl<T: Display> ToString for T`）。**保留英文**。
_Avoid_: 毯式实现

**基础类型构造器（fundamental type constructor）**：
对孤儿规则「透明」的构造器（`&T`/`&mut T`/`Box<T>`/`Pin<T>`）：`Box<本地类型>` 仍算本地类型；它们不能覆盖内部参数。
_Avoid_: 基本构造子

**重叠（overlap）**：
两条 impl 可能匹配同一类型而被禁止的情形（E0119）；「不相交」判定是编译器一致性检查的核心。
_Avoid_: 重叠实现（作名词时写作「重叠」）

**负实现（negative impls）**：
`impl !Trait for T`：声明「绝对不实现」。截至 1.98 仍是 nightly（`negative_impls`，第 36 章）。
_Avoid_: 否定实现

**特化（specialization）**：
在 blanket impl 之上为更具体类型提供更专门实现的能力；nightly（`specialization`；标准库内部只开 `min_specialization`）。
_Avoid_: 专门化

**dyn**：
trait 对象类型的前缀（`dyn Trait` = 「某个实现了 `Trait` 的类型」）。**保留英文**。
_Avoid_: 动态类型

**trait 对象（trait object）**：
`dyn Trait` 类型的值：大小不定，经胖指针（数据 + vtable）使用。
_Avoid_: 动态对象

**dyn 兼容性（dyn compatibility）**：
一个 trait 能否安全转成 trait 对象的规则；旧称「对象安全（object safety）」。
_Avoid_: 对象安全（旧名，仅历史语境）

**向下转型（downcast）**：
从 trait 对象取回具体类型（`dyn Any` 的 `downcast_ref`/`downcast_mut`）。
_Avoid_: 降型

**vtable**：
trait 对象背后的方法地址表（胖指针的第二个机器字）。**保留英文**。
_Avoid_: 虚函数表

**auto trait**：
由编译器自动推导实现与否的 trait（`Send`/`Sync`/`Unpin` 等）。**保留英文**。
_Avoid_: 自动 trait（首现解释时可括注英文）

**孤儿规则（orphan rule）**：
实现 trait 时，trait 与类型至少有一个属于本 crate。
_Avoid_: 孤立规则

**newtype**：
单字段包装类型（`struct Meters(u32);`），用于绕过孤儿规则或加语义。**保留英文**。
_Avoid_: 新类型

**不透明类型（opaque type）**：
只暴露 trait 而不暴露具体类型的返回类型（`impl Trait`）。
_Avoid_: 隐藏类型（那是它背后的具体类型，是另一个概念，见下条）

**隐藏类型（hidden type）**：
不透明类型背后的具体类型（Reference 用语 "hidden type"）。
_Avoid_: 底层类型

**RPIT（return-position impl Trait）**：
返回位置的 `impl Trait`。**保留英文缩写**。
_Avoid_: 返回 trait（不准确）

**向上转型（upcasting）**：
把 trait 对象转成其父 trait 的 trait 对象（`&dyn Sub → &dyn Super`，`Box`/`Arc`/`Rc` 同理；1.86 起稳定）。
_Avoid_: 上转型

**型变（variance）**：
生命周期/类型参数在子类型关系里的方向（协变/逆变/不变）。
_Avoid_: 变型（早期草稿用过，已废弃）

**协变 / 逆变 / 不变（covariant / contravariant / invariant）**：
`&'a T` 协变（长的能当短的用）、`fn(&'a T)` 逆变、`&'a mut T` 与 `Cell<&'a T>` 不变。
_Avoid_: 共变

**HRTB（higher-ranked trait bound）**：
对「所有生命周期都成立」的约束（`for<'a> Fn(&'a str)`）。**保留英文缩写**，首现给全称。
_Avoid_: 高阶生命周期约束

**晚绑定生命周期（late-bound lifetime）**：
在函数项类型里被 `for<>` 量化的生命周期（与早绑定相对），决定函数指针能否跨生命周期通用。
_Avoid_: 后绑定

**const 泛型（const generics）**：
以常量值作泛型参数（`[T; N]`、`Buf<const N: usize>`）。
_Avoid_: 常量泛型

**GAT（generic associated type）**：
带泛型参数（通常含生命周期）的关联类型（`type Item<'a>;`）。**保留英文缩写**。
_Avoid_: 泛型关联类型（首现可括注英文）

**借用式迭代器（lending iterator）**：
`next` 借 `self`、产出物也借 `self` 的迭代器（GAT 的典型用途）。
_Avoid_: 借出迭代器

**内联 const（inline const）**：
`const { … }` 块：在表达式位置求值常量（`[0; const { N + 1 }]`）。
_Avoid_: 内联常量

**APIT（argument-position impl Trait）**：
参数位置的 `impl Trait`（类型由调用者选）。**保留英文缩写**。
_Avoid_: 匿名参数类型

**RPITIT（return-position impl Trait in trait）**：
trait 方法里的 RPIT（`fn f(&self) -> impl Trait`）。**保留英文缩写**。
_Avoid_: trait 内 RPIT

**AFIT（async fn in trait）**：
trait 里的 `async fn`（1.75 起的 `-> impl Future` 糖）。**保留英文缩写**。
_Avoid_: 异步 trait 方法

**捕获（capture）**：
`impl Trait` 隐含捕获哪些泛型参数与生命周期；「精确捕获（precise capturing，`use<..>`）」是显式列出的写法。
_Avoid_: 抓取

## 迭代器与闭包

**适配器（adapter）**：
把迭代器变成另一个迭代器的组合子（`map`/`filter`/`take`…）。
_Avoid_: 中间操作（Java Stream 说法）

**消费者（consumer）**：
驱动迭代到底并产出结果的组合子（`collect`/`sum`/`fold`…）。
_Avoid_: 终止操作（同上）

**惰性求值（lazy evaluation）**：
不调用消费者就什么都不发生。
_Avoid_: 延迟计算

**拉/推模型（pull / push）**：
数据由消费者主动取（迭代器 = 拉），还是生产者推给消费者（回调/观察者 = 推）。
_Avoid_: 拉取/推送（口语可用，正式叙述用「拉/推」）

## 测试

**单元测试（unit test）**：
贴着代码写的测试（`#[cfg(test)] mod tests`）。
_Avoid_: 模块测试

**集成测试（integration test）**：
`tests/` 目录下、用公开 API 测整个 crate 的测试。
_Avoid_: 外部测试

**端到端测试（end-to-end test）**：
把程序当黑盒、跑完整流程的测试（CLI 常用）。
_Avoid_: 全链路测试

**断言（assertion）**：
`assert!`/`assert_eq!` 等检查，失败即 panic。
_Avoid_: 校验（歧义）

**测试运行器（test runner）**：
执行测试并汇总结果的工具（libtest / `cargo test`）。
_Avoid_: 测试器

**测试框架（test harness）**：
`#[test]` 背后的收集与调度机制。
_Avoid_: 测试脚手架

**文档测试（doctest）**：
文档代码块被当作测试编译运行（`cargo test` 的 Doc-tests 段）。
_Avoid_: 文档用例

**表驱动（table-driven）**：
把用例排成表、用一个 helper 循环跑的参数化组织方式。
_Avoid_: 参数化（泛指时可用「参数化测试」）

**属性测试（property-based testing）**：
随机生成大量输入、验证性质而非逐例断言的测试方式（proptest 等）。
_Avoid_: 性质测试

**快照测试（snapshot testing）**：
首次运行记录输出为快照、之后对比的测试方式（insta 等）。
_Avoid_: 记录测试

**假绿（false green）**：
测试或门禁没真正覆盖就显示通过的现象（本书特指 rustdoc 不编译 `#[cfg(test)]` 块）。非正式用语。
_Avoid_: 伪绿

**夹具（fixture）**：
测试前搭好、多处复用的输入或环境（rstest 等框架里的术语）。
_Avoid_: 固件

---

## 维护规则

- 新术语先进本表再进正文；写作中发现新术语，**在同一张写作票里**补进本表（并行批次期间由写作者回报、协调者统一并表，见 `WRITING.md` §8）。
- 与社区术语表冲突时以「主流实际用法」为准（01 findings §3 已列三方分歧），分歧本身记进本表。
- 将来书末附录由本表派生（去掉写作向的 `_Avoid_` 备注或保留，届时定）。
