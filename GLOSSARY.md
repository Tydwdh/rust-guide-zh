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
对孤儿规则「透明」的构造器（`&T`/`&mut T`/`Box<T>`）：`Box<本地类型>` 仍算本地类型。
_Avoid_: 基本构造子

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

---

## 维护规则

- 新术语先进本表再进正文；写作中发现新术语，**在同一张写作票里**补进本表（并行批次期间由写作者回报、协调者统一并表，见 `WRITING.md` §8）。
- 与社区术语表冲突时以「主流实际用法」为准（01 findings §3 已列三方分歧），分歧本身记进本表。
- 将来书末附录由本表派生（去掉写作向的 `_Avoid_` 备注或保留，届时定）。
