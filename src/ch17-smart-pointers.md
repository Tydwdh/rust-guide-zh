# 第 17 章 智能指针与内部可变性

## 这一章解决什么问题

第 4 章的所有权规则与第 16 章的生命周期共享同一个前提：**同一时刻，一个值要么被任意多个 `&T` 共享，要么被一个 `&mut T` 独占**。它是 Rust 能在编译期排掉大部分内存错误的原因，但现实里有三类需求正好撞在它上面：

- 一个值要**同时被多处拥有**：图里的节点、事件监听器表、缓存里共享的结果。值只能有一个所有者，怎么办？
- 要在**只有 `&T` 的地方改内容**：命中计数、惰性缓存、日志句柄。编译器不给 `&mut`，怎么办？
- 一个类型**要包含自己**：链表、树、AST。这种类型的大小在编译期算不出来，怎么办？

三个问题的答案分别是 `Rc`/`Arc`、`Cell`/`RefCell`/`Mutex`、`Box`。它们都属于智能指针（smart pointer）：带所有权语义与额外行为的指针类型。

读完这一章，你能：

- 在「共享还是独占 × 单线程还是跨线程」这张四格表里为具体场景挑出正确类型，并说清 `Rc` 与 `Arc` 的差别、`Rc` 为什么不能跨线程；
- 用 `Weak` 剪断引用环，并解释不剪为什么会泄漏；
- 说清 `RefCell` 的借用检查在运行期是怎么做的、越界时 panic 长什么样、什么时候该改用 `try_borrow`；
- 用 `Cell` 的 `get`/`set`/`take`/`replace` 写一个「只有 `&self` 也能改」的类型；
- 判断 `Deref` 解引用强转会不会发生、`*x` 到底得到什么，并看懂 `Send`/`Sync` 在这些类型上的分布。

本章只讲这些类型的用法与语义：`thread`、`Send`/`Sync` 的系统讲解与 channel 在第 19 章，原子与内存序在第 20 章，别名模型在第 35 章。

## 为什么这样设计

### 谁负责回收：三套答案

内存回收只有三种主流答案：

| 方案 | 谁决定何时释放 | 代价 |
|---|---|---|
| 垃圾回收（Java、Go、Python） | 运行时（GC） | 停顿、额外内存、你无法预知释放时机 |
| 手动管理（C、C++） | 你 | 忘释放就泄漏；释放两次或提前释放即未定义行为 |
| 所有权（Rust 默认） | 编译器插入的 `Drop` | 类型系统上的约束 |

Rust 默认方案的好处是析构点由**静态的**所有权规则唯一确定，不需要运行时记账；代价是规则太硬——「两个节点互相当父子」在类型层面根本写不出来。本章的类型都是在静态规则的边界上开一个**受控**口子：`Rc`/`Arc` 把计数放到堆上，让多个句柄共享同一份数据；`Cell`/`RefCell`/`Mutex` 把「可变必须独占」的检查挪到运行期或锁上；`Box` 不开任何口子，只是把值搬到堆上。

### 「智能指针」在 Rust 里就是 Deref + Drop

The Book 的判据是：智能指针通常用结构体实现，并且实现 `Deref` 与 `Drop`。这两个 trait 各管一半：

- `Drop`（第 4 章）让它像所有者：最后一个所有者消失时，堆上那块数据被释放；
- `Deref` 让它像引用：`&Box<T>` 能当 `&T` 用，`rc.len()` 能直接调到 `String::len`。

顺带一个值得留意的细节：`Rc` 的引用计数本身就建在内部可变性上。`Clone::clone` 的签名是 `fn clone(&self) -> Self`，拿不到 `&mut self` 却必须把计数加一；std 的 `std::cell` 文档就拿它当例子——`Rc<T>` 的计数存在一个 `Cell<T>` 里。

### 静态检查与运行期检查的分界线

编译器在你能写 `let mut x`、`&mut x` 的地方已掌握全部信息，所以别名与可变性它能静态判定；它证明不了的是**跨函数、跨共享句柄的不变式**。设想一个共享缓存：

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn main() {
    // 两个句柄指向同一份缓存，谁都没有 &mut
    let cache: Rc<RefCell<HashMap<String, i32>>> = Rc::new(RefCell::new(HashMap::new()));
    let alias = Rc::clone(&cache);

    cache.borrow_mut().insert("answer".to_string(), 42);
    println!("通过另一个句柄读到：{:?}", alias.borrow().get("answer"));
}
```

```text
通过另一个句柄读到：Some(42)
```

两个句柄谁也没有 `&mut`，内容却真的被改了：编译器无法知道「第一个 `borrow_mut()` 拿到的 guard 是否还活着」，那是运行期的事实。`RefCell` 把它变成一对**运行期计数**——借出加一、guard 被 drop 减一，冲突即 panic。

`std::cell` 文档对这种做法的态度很直接：interior mutability is something of a last resort（内部可变性是最后一招）。选型顺序因此是：能用 `&mut` 就用；不能，再考虑内部可变性；要跨线程，才上锁或上原子类型。

## 机制与规则

### 先记住四个格子

| 你要的能力 | 单线程 | 跨线程 |
|---|---|---|
| 多人共享所有权 | `Rc<T>` | `Arc<T>` |
| 共享的同时还要能改 | `Cell<T>` / `RefCell<T>` | `Mutex<T>` / `RwLock<T>` |
| 只是把值放到堆上 | `Box<T>` | `Box<T>`（是否可用取决于内层的 `Send`/`Sync`） |

这张表摊平了就是一张十字图：横轴问「跨不跨线程」，纵轴问「要不要改」。

<svg viewBox="0 0 640 254" width="100%" role="img" aria-label="四个格子：按要不要改与跨不跨线程把本章类型分成 Rc、Arc、Cell 与 RefCell、Mutex 与 RwLock 四格，横向换列就是换件清单" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">四个格子：先问要不要改，再问跨不跨线程</text>
<text x="245" y="52" font-size="12" fill="currentColor" text-anchor="middle">单线程</text>
<text x="495" y="52" font-size="12" fill="currentColor" text-anchor="middle">跨线程</text>
<text x="110" y="100" font-size="12" fill="currentColor" text-anchor="end">只要共享</text>
<text x="110" y="178" font-size="12" fill="currentColor" text-anchor="end">还要能改</text>
<rect x="120" y="60" width="250" height="70" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="245" y="92" font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">Rc&lt;T&gt;</text>
<text x="245" y="114" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">计数非原子</text>
<rect x="370" y="60" width="250" height="70" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="495" y="92" font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">Arc&lt;T&gt;</text>
<text x="495" y="114" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">计数原子</text>
<rect x="120" y="138" width="250" height="70" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="245" y="170" font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">Cell&lt;T&gt; / RefCell&lt;T&gt;</text>
<text x="245" y="192" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">检查挪到运行期</text>
<rect x="370" y="138" width="250" height="70" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="495" y="170" font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">Mutex&lt;T&gt; / RwLock&lt;T&gt;</text>
<text x="495" y="192" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">检查挪到锁上</text>
<text x="0" y="232" font-size="12" fill="currentColor" opacity="0.7">横向换列就是换件清单：Rc&lt;RefCell&lt;T&gt;&gt; → Arc&lt;Mutex&lt;T&gt;&gt;</text>
<text x="0" y="250" font-size="12" fill="currentColor" opacity="0.7">Box&lt;T&gt; 不进这张表：它只决定值放在哪儿，能不能跨线程看内层</text>
</svg>

`std::cell` 文档把中间一行写成现成句子：`RefCell<T>` 的 `Sync` 版本是 `RwLock<T>`，`OnceCell<T>` 的 `Sync` 版本是 `OnceLock<T>`。它没说 `Cell<T>` 的版本是 `Mutex<T>`——原因见 `Cell` 一节。

### Box\<T\>：堆上的独占所有权

`Box<T>` 是最朴素的智能指针：它拥有堆上的一块 `T`，出了作用域就释放。它没有引用计数、没有共享，只是在「值放在哪儿」这件事上做了个选择。

**递归类型离不开它。** 下面这个链表如果不加 `Box`，编译器会直接报「递归类型大小无限」：

```rust
enum List {
    Cons(i32, Box<List>),
    Nil,
}

impl List {
    fn sum(&self) -> i32 {
        match self {
            List::Cons(v, rest) => v + rest.sum(),
            List::Nil => 0,
        }
    }
}

fn main() {
    let list = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))));
    println!("sum = {}", list.sum());
    println!("size_of::<List>() = {}", std::mem::size_of::<List>());
}
```

```text
sum = 3
size_of::<List>() = 16
```

`List` 的大小是 16 字节：`i32` 占 4 字节，加 4 字节填充，再加 8 字节的 `Box` 指针——`Nil` 折进 `Box` 的空指针 niche，没有独立的判别字段（实测 `Option<Box<u64>>` 是 8 字节；`enum { T(u64, Box<u64>), N }` 若用独立判别字段会占 24 字节，实测 16）。如果没有 `Box`，`Cons` 的第二个字段就是 `List` 自身，大小会无限展开。

**`Box<T>` 的指针开销就是一个机器字**（1.98、x86_64 实测）：

| 类型 | `size_of` |
|---|---|
| `Box<u64>` | 8 |
| `Box<[u64]>` | 16（胖指针：地址 + 长度） |
| `Box<str>` | 16 |

`Box<dyn Trait>` 同样是胖指针，它会带一个 vtable 指针（第 13 章）。另外，`Box<T>` 是**唯一**能从解引用里把值「搬出来」的智能指针，这一点在 `Deref` 一节细说。

上表的 8 与 16 差在哪儿，摊开就是下图：

<svg viewBox="0 0 660 200" width="100%" role="img" aria-label="瘦指针与胖指针对比：瘦指针只有一个地址机器字，胖指针多带一个长度或 vtable 指针，因此是 16 字节" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">瘦指针与胖指针：差的那一个机器字</text>
<text x="0" y="64" font-size="12" fill="currentColor">瘦指针</text>
<rect x="110" y="44" width="180" height="32" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="200" y="65" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">地址</text>
<text x="310" y="65" font-size="12" fill="currentColor" opacity="0.7">8 字节，如 Box&lt;u64&gt;</text>
<text x="0" y="128" font-size="12" fill="currentColor">胖指针</text>
<rect x="110" y="108" width="180" height="32" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="200" y="129" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">地址</text>
<rect x="290" y="108" width="180" height="32" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="380" y="129" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">长度 / vtable 指针</text>
<text x="482" y="129" font-size="12" fill="currentColor" opacity="0.7">16 字节</text>
<text x="0" y="170" font-size="12" fill="currentColor" opacity="0.7">带长度的：Box&lt;[u64]&gt;、Box&lt;str&gt;、&amp;str、&amp;[T]　带 vtable 的：Box&lt;dyn Trait&gt;、&amp;dyn Trait</text>
<text x="0" y="190" font-size="12" fill="currentColor" opacity="0.7">指针自身的大小取决于要不要带长度信息，与堆上数据有多大无关</text>
</svg>

何时用 `Box`：递归类型；值很大而想避免整块拷贝（移动 `Box` 只移动指针）；`Box<dyn Trait>` 做类型擦除（第 13 章）；`Box<[T]>`/`Box<str>` 这类长度固定的堆数组。第 22 章的 `Box::pin` 留给自引用类型。

### Rc\<T\> 与 Arc\<T\>：共享所有权

`Rc<T>` 自己是这么自我介绍的：A single-threaded reference-counting pointer. `Rc` stands for "Reference Counted"（单线程引用计数指针）。它的模型很简单：

<svg viewBox="0 0 700 244" width="100%" role="img" aria-label="Rc 与 Arc 的堆布局：a、b、w 三个句柄各占 8 字节指向同一块堆分配；分配里有 strong 与 weak 两档计数和值，Weak 不计入 strong" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">Rc/Arc 的布局：句柄在栈上，计数与值在同一块堆分配里</text>
<text x="0" y="40" font-size="12" fill="var(--links)">三个句柄各占 8 字节（实测 size_of::&lt;Rc&lt;String&gt;&gt;() == 8）</text>
<rect x="20" y="60" width="150" height="34" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="95" y="82" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Rc&lt;String&gt; a</text>
<rect x="20" y="110" width="150" height="34" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="95" y="132" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Rc&lt;String&gt; b</text>
<rect x="20" y="160" width="150" height="34" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="95" y="182" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Weak&lt;String&gt; w</text>
<line x1="170" y1="77" x2="398" y2="94" stroke="var(--links)" stroke-width="2" />
<polyline points="398,89 406,94 398,99" fill="none" stroke="var(--links)" stroke-width="2" />
<line x1="170" y1="127" x2="398" y2="112" stroke="var(--links)" stroke-width="2" />
<polyline points="398,107 406,112 398,117" fill="none" stroke="var(--links)" stroke-width="2" />
<line x1="170" y1="177" x2="398" y2="139" stroke="var(--quote-border)" stroke-width="2" stroke-dasharray="5 3" />
<polyline points="398,134 406,139 398,144" fill="none" stroke="var(--quote-border)" stroke-width="2" />
<rect x="406" y="52" width="274" height="164" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="543" y="72" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">堆上同一块分配</text>
<rect x="418" y="82" width="250" height="32" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="430" y="103" font-size="12" font-family="monospace" fill="var(--links)">strong = 2</text>
<text x="656" y="103" font-size="12" fill="var(--links)" text-anchor="end">强引用计数</text>
<rect x="418" y="120" width="250" height="32" rx="4" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<text x="430" y="141" font-size="12" font-family="monospace" fill="currentColor">weak = 1</text>
<text x="656" y="141" font-size="12" fill="currentColor" opacity="0.7" text-anchor="end">弱引用计数</text>
<rect x="418" y="160" width="250" height="44" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="543" y="187" font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">"共享的字符串"</text>
<text x="0" y="212" font-size="12" fill="currentColor" opacity="0.7">Weak 只保证分配不被回收，不强留内容，也不计入 strong</text>
<text x="0" y="234" font-size="12" fill="currentColor" opacity="0.7">Rc 与 Arc 布局相同，只差计数怎么写：一个普通整数、一个原子操作</text>
</svg>

图里三个句柄各占 8 字节（实测 `size_of::<Rc<String>>() == 8`），共享的是右边那块带计数的分配。实测一遍计数怎么变：

```rust
use std::rc::{Rc, Weak};

fn main() {
    let a = Rc::new(String::from("共享的字符串"));
    let b = Rc::clone(&a);
    let w: Weak<String> = Rc::downgrade(&a);
    println!("strong = {}, weak = {}", Rc::strong_count(&a), Rc::weak_count(&a));
    println!("b 看到的内容：{b}");
    drop(a);
    drop(b);
    println!("强引用清零后 upgrade -> {:?}", w.upgrade());
}
```

```text
strong = 2, weak = 1
b 看到的内容：共享的字符串
强引用清零后 upgrade -> None
```

规则有三条：

- **`Rc::clone` 加计数，不复制内容**。习惯写 `Rc::clone(&a)` 而不是 `a.clone()`：两者等价，但前者一眼看出这是次便宜的计数递增。（`Rc` 文档也提醒：它的固有方法全是关联函数，得写 `Rc::get_mut(&mut v)`——只有如此才能避开与内层 `T` 的方法重名。）
- **计数清零时释放**：上面 `drop(a); drop(b);` 之后 `w.upgrade()` 变成 `None`。
- **`Weak` 不参与放行**：只有强引用归零，内容才被 drop。

`Arc<T>` 是 `Rc<T>` 的原子版本（Arc = Atomically Reference Counted），唯一差别很关键：`Rc` 的计数是普通整数，`Arc` 的是原子操作。std 的 `Send` 文档把理由写得很明确——两个线程同时 clone 同一份数据的 `Rc`，计数更新就撞车，那是未定义行为；`Arc` 用原子操作（有点开销），所以可以 `Send`。Nomicon 更短：Rc isn't Send or Sync (because the refcount is shared and unsynchronized)。

**结论**：单线程用 `Rc`，跨线程才付 `Arc` 的原子代价；C++ 的 `shared_ptr` 没这个选择，计数天然原子（见语言对照角）。

### Weak\<T\>：不增加强引用的那条边

引用计数有个众所周知的死角：**成环就永远不归零**。亲手看一遍：

```rust
use std::cell::RefCell;
use std::rc::Rc;

struct Node {
    name: &'static str,
    neighbor: RefCell<Option<Rc<Node>>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

fn main() {
    {
        let a = Rc::new(Node { name: "a", neighbor: RefCell::new(None) });
        let b = Rc::new(Node { name: "b", neighbor: RefCell::new(None) });
        *a.neighbor.borrow_mut() = Some(Rc::clone(&b));
        *b.neighbor.borrow_mut() = Some(Rc::clone(&a));
        println!("互相强引用：strong(a) = {}, strong(b) = {}", Rc::strong_count(&a), Rc::strong_count(&b));
    }
    println!("作用域结束：上面没有 drop 输出");
}
```

```text
互相强引用：strong(a) = 2, strong(b) = 2
作用域结束：上面没有 drop 输出
```

两个节点各被对方持有一个强引用，作用域结束时计数都还剩 1，`Drop` 一次都没跑：这是**内存泄漏**（安全，但内存回不来）。把其中一条边换成 `Weak` 即可。`std::rc::Weak` 文档说得很清楚：`Weak` 不计入所有权，不会阻止分配里的值被 drop，只保证**分配本身**不被回收。

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    name: &'static str,
    neighbor: RefCell<Weak<Node>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

fn main() {
    {
        let a = Rc::new(Node { name: "a", neighbor: RefCell::new(Weak::new()) });
        let b = Rc::new(Node { name: "b", neighbor: RefCell::new(Weak::new()) });
        *a.neighbor.borrow_mut() = Rc::downgrade(&b);
        *b.neighbor.borrow_mut() = Rc::downgrade(&a);
        println!("weak(a) = {}, weak(b) = {}", Rc::weak_count(&a), Rc::weak_count(&b));
        println!("a 能看到 b 吗：{:?}", a.neighbor.borrow().upgrade().map(|n| n.name));
    }
    println!("作用域结束");
}
```

```text
weak(a) = 1, weak(b) = 1
a 能看到 b 吗：Some("b")
drop b
drop a
作用域结束
```

要点：

- 用 `Rc::downgrade(&rc)` 拿到 `Weak`；想用先 `upgrade()`，它返回 `Option<Rc<T>>`——**弱引用随时可能失效**，这个 `Option` 就是处理它的地方。
- 谁该是弱边？**看方向**：谁「天然活得更久」谁持强引用。典型是父指针：父目录强引用子目录，子目录只留 `Weak`（「动手看看」的目录树就这么写）。
- `Weak::new()` 造的是「不指向任何分配」的弱引用，`upgrade()` 永远返回 `None`，适合当占位值。

`Arc` 也有对应的 `std::sync::Weak`，语义一致，只是计数是原子的。

### Cell\<T\>：值进，值出

`Cell<T>` 解决的是最窄的问题：**我只有 `&self`，但要换掉里面的值**。做法是彻底回避引用——你永远拿不到 `&T`，只能把值搬进搬出。std 文档的定位是：适合复制/移动成本低的简单类型（如数字），能用它就别用 `RefCell`。

```rust
use std::cell::Cell;

struct Counter {
    hits: Cell<u32>,
}

impl Counter {
    fn hit(&self) {
        self.hits.set(self.hits.get() + 1);
    }
}

fn main() {
    let c = Cell::new(5);
    c.set(7);
    let old = c.replace(9);
    println!("replace(9) 返回 {old}，现在是 {}", c.get());

    let d = Cell::new(String::from("值"));
    let taken = d.take();
    println!("take() 拿走 {taken:?}，留下 {:?}", d.into_inner());

    let k = Cell::new(1);
    k.update(|v| v * 10);
    println!("update 后 {}", k.get());

    let counter = Counter { hits: Cell::new(0) };
    counter.hit();
    counter.hit();
    println!("hits = {}", counter.hits.get());
}
```

```text
replace(9) 返回 7，现在是 9
take() 拿走 "值"，留下 ""
update 后 10
hits = 2
```

各方法的分工：

| 方法 | 需要内层满足 | 做什么 |
|---|---|---|
| `get()` | `T: Copy` | 复制一份当前值给你 |
| `set(v)` | — | 换上新值，旧值被 drop |
| `replace(v)` | — | 换上新值，**返回**旧值 |
| `take()` | `T: Default` | 把值拿走，留下 `T::default()` |
| `update(f)` | — | 把当前值交给 `f`，用返回值替换（`FnOnce(T) -> T`；1.88 起） |
| `into_inner()` | — | 消耗 `Cell`，把值取出来 |

`get()` 要求 `Copy` 不是刁难：它必须复制一份出来，否则同一时刻就有两个所有者。所以 `Cell<String>` 上写 `.get()` 会编译失败（报错现场三），该用 `take()` 或 `replace()`。

`Counter::hit(&self)` 是这个类型存在的意义：`hits` 是私有字段，外面只能通过 `&self` 调 `hit`，却真的改到了内容；这样的 `Counter` 因此能放进 `Rc` 共享（但只适合单线程）。

`Cell<T>` 的大小就等于 `T` 的大小（实测 `Cell<u64>` 是 8 字节）——它没有额外标志位，因为 `Cell` 压根不让你借出引用；`RefCell<T>` 必须存一个运行期借用计数，所以 `RefCell<u64>` 是 16 字节。

### RefCell\<T\>：把借用检查搬到运行期

`RefCell<T>` 比 `Cell` 宽松：它允许借出 `&T` 和 `&mut T`，代价是把借用规则挪到运行期。std 文档说：`borrow`/`borrow_mut` 先校验借用规则——共享借用可以有任意多个，可变借用只能有一个，两者不共存；一旦违反，the thread will panic（原文如此）。

```rust
use std::cell::RefCell;

fn main() {
    let c = RefCell::new(5);
    {
        let r1 = c.borrow();
        let r2 = c.borrow();
        println!("两个共享借用：{r1} 和 {r2}");
    }
    {
        let mut m = c.borrow_mut();
        *m += 10;
    }
    println!("写入后 {}", c.borrow());

    let held = RefCell::new(1);
    let _guard = held.borrow_mut();
    println!("try_borrow     -> {:?}", held.try_borrow().map(|_| ()));
    println!("try_borrow_mut -> {:?}", held.try_borrow_mut().map(|_| ()));
}
```

```text
两个共享借用：5 和 5
写入后 15
try_borrow     -> Err(BorrowError)
try_borrow_mut -> Err(BorrowMutError)
```

规则要点：

- `borrow()` 返回 `Ref<'_, T>`，`borrow_mut()` 返回 `RefMut<'_, T>`。**这两个 guard 才是「借用」本身**——进作用域时计数加一，drop 掉才减一。所以上面那两对花括号不是装饰，它们决定了借用何时结束。
- `borrow()` 在**已经被可变借用**时 panic；`borrow_mut()` 在**已经被任何借用**时 panic。这就是运行期版的「别名规则」。
- 不想 panic 就用 `try_borrow()` / `try_borrow_mut()`，返回 `Result<_, BorrowError>` / `Result<_, BorrowMutError>`；std 的方法文档也这么建议（For a non-panicking variant, use `try_borrow`）。「失败可以被处理」的场合该选它。
- `RefCell<T>` 不会让「两个 `&mut` 同时存在」变成 UB，它只是 panic。代价是多一个借用计数：实测 `RefCell<u64>` 是 16 字节，`RefCell<String>` 是 32 字节。

把 guard 的生命周期画成区间，冲突就一目了然：

<svg viewBox="0 0 660 264" width="100%" role="img" aria-label="Ref 与 RefMut guard 的借用区间：两个共享 guard 的区间可以重叠、可变 guard 另起一段；若共享借用落在可变 guard 的区间内，就在运行期 panic" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">guard 的作用域就是借用的区间：创建时计数加一，drop 时减一</text>
<rect x="310" y="30" width="26" height="9" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="342" y="39" font-size="12" fill="var(--links)">共享 guard（Ref）</text>
<rect x="480" y="30" width="26" height="9" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="512" y="39" font-size="12" fill="currentColor">可变 guard（RefMut）</text>
<text x="0" y="58" font-size="12" fill="var(--links)">① 两个共享 guard 重叠、可变 guard 另起一段：运行期放行</text>
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="0" y="80">let r1 = c.borrow();</text>
<text x="0" y="102">let r2 = c.borrow();</text>
<text x="0" y="124">println!(“{r1} {r2}”);</text>
<text x="0" y="146">let m = c.borrow_mut();</text>
</g>
<rect x="250" y="72" width="230" height="16" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="365" y="84" font-size="12" fill="var(--links)" text-anchor="middle">Ref</text>
<rect x="300" y="94" width="180" height="16" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="390" y="106" font-size="12" fill="var(--links)" text-anchor="middle">Ref</text>
<rect x="520" y="138" width="120" height="16" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="580" y="150" font-size="12" fill="currentColor" text-anchor="middle">RefMut</text>
<line x1="480" y1="68" x2="480" y2="160" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="486" y="116" font-size="12" fill="currentColor" opacity="0.7">最后一次使用</text>
<text x="0" y="184" font-size="12" fill="var(--warning-border)">② 可变 guard 还没 drop 就又借一次：区间重叠（运行期 panic）</text>
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="0" y="206">let m = c.borrow_mut();</text>
<text x="0" y="228">let b = c.borrow();</text>
</g>
<rect x="250" y="198" width="330" height="16" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="415" y="210" font-size="12" fill="currentColor" text-anchor="middle">RefMut（活到函数末尾）</text>
<rect x="400" y="220" width="180" height="16" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="5 3" />
<text x="490" y="232" font-size="12" fill="var(--warning-border)" text-anchor="middle">Ref（拒绝）</text>
<line x1="480" y1="194" x2="480" y2="242" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="0" y="258" font-size="12" fill="var(--warning-border)">✗ 共享借用落在可变 guard 的区间内 → panic：RefCell already mutably borrowed</text>
</svg>

`Rc<RefCell<T>>` 是单线程里最常见的组合：`Rc` 提供共享，`RefCell` 提供可变。std 文档给的正是这个用法，并警告：上一个借用没先离开作用域，下一次借用就会造成 dynamic thread panic——用 `RefCell` 的主要危险。

两层检查叠在同一个类型上，各管一件事：

<svg viewBox="0 0 700 236" width="100%" role="img" aria-label="Rc 与 RefCell 的两层检查：Rc 的强引用计数在堆上、编译期不可见，RefCell 的借用计数在运行期决定这次借出放不放行" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">Rc&lt;RefCell&lt;T&gt;&gt; 的两层检查：一层计持有者，一层计借用者</text>
<text x="0" y="40" font-size="12" fill="var(--links)">两层的数字都不在类型里，只有运行期才知道</text>
<rect x="20" y="94" width="150" height="40" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="95" y="119" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">Rc&lt;RefCell&lt;T&gt;&gt;</text>
<line x1="170" y1="114" x2="246" y2="114" stroke="var(--links)" stroke-width="2" />
<polyline points="246,109 254,114 246,119" fill="none" stroke="var(--links)" stroke-width="2" />
<rect x="254" y="52" width="426" height="160" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="467" y="72" font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">堆上同一块分配：里面是一个 RefCell&lt;T&gt;</text>
<rect x="268" y="82" width="192" height="56" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="278" y="104" font-size="12" font-family="monospace" fill="var(--links)">strong = 2</text>
<text x="278" y="126" font-size="12" fill="currentColor" opacity="0.7">编译期不可见</text>
<rect x="474" y="82" width="192" height="56" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" />
<text x="484" y="104" font-size="12" font-family="monospace" fill="var(--warning-border)">borrow = 1</text>
<text x="484" y="126" font-size="12" fill="currentColor" opacity="0.7">运行期决定放行</text>
<rect x="268" y="148" width="398" height="50" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="467" y="178" font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">T 的值</text>
<text x="0" y="230" font-size="12" fill="currentColor" opacity="0.7">Rc 管谁还持有、RefCell 管此刻谁在借；冲突只会发生在第二层，也只在运行期爆发</text>
</svg>

> [!WARNING]
> `RefCell` 的 panic 是**运行期**的：编译器不会替你挡住它，测试没跑到那条路径就看不出来。看到 `Rc<RefCell<T>>` 时先问：所有借用的作用域能一眼看清吗？看不清就别用它。

### Mutex\<T\> 与 RwLock\<T\>：跨线程的内部可变性

把 `RefCell` 换到多线程，就是 `Mutex`（互斥锁）。对应关系一句话：`Mutex<T>` 是「跨线程的 `RefCell`」，`RwLock<T>` 是「支持多读单写的 `RefCell`」。

```rust
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

fn main() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                *c.lock().unwrap() += 1;
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    println!("counter = {}", *counter.lock().unwrap());

    let table = RwLock::new(1);
    {
        let r1 = table.read().unwrap();
        let r2 = table.read().unwrap();
        println!("两个读锁同时持有：{} 和 {}", *r1, *r2);
    }
    *table.write().unwrap() += 41;
    println!("写后 {}", *table.read().unwrap());
}
```

```text
counter = 4000
两个读锁同时持有：1 和 1
写后 42
```

四个要点：

- **`Arc<Mutex<T>>` 是共享可变状态的默认答案**（第 19 章展开）：`Arc` 管「多个人持有同一份」，`Mutex` 管「同一时刻只有一个人在改」。持锁期间给的是独占访问，所以 `Mutex<T>: Sync` 只要求 `T: Send`，**不要求 `T: Sync`**。
- **guard 就是锁的生命周期**。`lock()` 返回 `MutexGuard<'_, T>`，它实现 `Deref`/`DerefMut`，所以能直接 `*guard += 1`；guard 被 drop 时自动解锁，不需要（也无法）手工解锁。
- **中毒只值一句**：持锁期间 panic 会把锁标记为「中毒」，之后 `lock()` 返回 `Err(PoisonError)`；中毒只是建议性的，`PoisonError::into_inner()` 能把 guard 取回来。所以绝大多数代码里 `.unwrap()` 是对的——它把「某个线程在临界区崩了」传出去，而不是悄悄放过。
- **`RwLock` 的读锁可并存**，因此读多写少才划算；它的 `Sync` 要求 `T: Send + Sync`，比 `Mutex` 严（多个线程会同时拿着 `&T`）。1.92 起 `RwLockWriteGuard::downgrade` 能把写锁降级为读锁（写成 `RwLockWriteGuard::downgrade(guard)`）。

> [!NOTE]
> 这里只到「知道该用哪个、为什么」。锁的公平性、`try_lock` 的非阻塞语义、死锁与自锁（同一线程重复 `lock()` 的行为是未指定的）留给第 19 章。

### OnceLock 与 LazyLock：只初始化一次的全局

配置、正则、静态查找表这类「只写一次」的值，标准库自带支持：`OnceCell`/`LazyCell` 是单线程版，`OnceLock`（1.70）/`LazyLock`（1.80）带锁、可放进 `static`；`LazyLock` 实现了 `Deref`，`*CONFIG` 就能触发初始化。

```rust
use std::sync::LazyLock;

static CONFIG: LazyLock<String> = LazyLock::new(|| {
    println!("（只算这一次）");
    String::from("loaded.toml")
});

fn main() {
    println!("第一次读：{}", *CONFIG);
    println!("第二次读：{}", *CONFIG);
    println!("UFCS 取引用：{}", LazyLock::force(&CONFIG));
    println!("主动查询：{:?}", LazyLock::get(&CONFIG));
}
```

```text
（只算这一次）
第一次读：loaded.toml
第二次读：loaded.toml
UFCS 取引用：loaded.toml
主动查询：Some("loaded.toml")
```

`LazyLock` 实现了 `Deref`，所以 `*CONFIG` 就足以触发初始化。

> [!WARNING]
> `force`/`get`/`get_mut`/`force_mut` 不能用方法语法调用：它们的接收者是显式写的 `this: &LazyLock<T, F>`（`force`/`get`）或 `this: &mut LazyLock<T, F>`（`get_mut`/`force_mut`），而带名字的 `this` 接收者在 stable 上只是关联函数——`lazy.force()` 报 E0599（this is an associated function, not a method），得写 `LazyLock::force(&CONFIG)` 或走 `Deref`（`*CONFIG`）。版本：`force` 从 1.80 起就是这样（不是新坑），`get`/`get_mut`/`force_mut` 1.94 才稳定。

### Cow：写时克隆

`Cow`（clone-on-write）回答一个很具体的问题：「返回一段文本，但只有约 10% 的情况需要改动——不改的时候能不能一个字节都不复制？」std 的自我介绍是：Cow is a smart pointer providing clone-on-write functionality: it can enclose and provide immutable access to borrowed data, and clone the data lazily when mutation or ownership is required.

```rust
use std::borrow::Cow;

fn normalize(input: &str) -> Cow<'_, str> {
    if input.bytes().all(|b| b != b' ' && !b.is_ascii_uppercase()) {
        return Cow::Borrowed(input); // 不用改，一个字节都不复制
    }
    let mut owned = input.replace(' ', "");
    owned.make_ascii_lowercase();
    Cow::Owned(owned)
}

fn main() {
    let a = normalize("abc123");
    let b = normalize("Hello World");
    println!("a = {a:?}，是借用吗 {}", matches!(a, Cow::Borrowed(_)));
    println!("b = {b:?}，是借用吗 {}", matches!(b, Cow::Borrowed(_)));

    let mut c: Cow<'_, str> = Cow::Borrowed("abc123");
    c.to_mut().push('!');
    println!("c = {c:?}，是借用吗 {}", matches!(c, Cow::Borrowed(_)));
}
```

```text
a = "abc123"，是借用吗 true
b = "helloworld"，是借用吗 false
c = "abc123!"，是借用吗 false
```

三条规则：

- `Cow<'a, B>` 只有两个变体：`Borrowed(&'a B)` 和 `Owned(<B as ToOwned>::Owned)`；它实现了 `Deref<Target = B>`，所以读的时候像 `&str` 一样用。
- `to_mut()` 是分水岭：当前是 `Borrowed` 就先 **clone** 成 `Owned` 再返回 `&mut`，已经是 `Owned` 就直接返回。`into_owned()` 类似，要的是所有权。
- 值得用的场合是**「大多数调用不需要改」**——函数返回值、结构体字段、配置项；每次都要改就不如直接要 `String`（`Cow` 只是把一次 clone 换成一个枚举分支）。

想要 `Rc`/`Arc` 版的写时克隆，标准库给了 `Rc::make_mut` 和 `Arc::make_mut`。

### Deref 与 DerefMut：解引用强转的来路与边界

前面所有类型「用起来像它包的东西」，靠的都是 `Deref`。这一节讲清它到底做了什么、什么时候不生效。

**`*x` 是什么**。std 文档的原文是：In immutable contexts, `*v` (where `T` is neither a reference nor a raw pointer) is equivalent to `*Deref::deref(&v)`.——即不可变上下文里，当 `T` 既不是引用也不是裸指针时，`*v` 等价于 `*Deref::deref(&v)`；可变上下文走 `DerefMut`。`*` 不是魔法，它只调了一次你自己实现的 `deref`。

**解引用强转（deref coercion）**发生在有明确类型要求的**位置**上。Rust Reference 列出的规则之一是：`&T` 或 `&mut T` 能转成 `&U`，条件是 `T: Deref<Target = U>`。实测一下边界：

```rust
fn takes_str(s: &str) -> usize {
    s.len()
}

fn generic_size<T: ?Sized>(x: &T) -> usize {
    std::mem::size_of_val(x)
}

fn main() {
    let s = String::from("hello");
    println!("&String 传给 &str：{}", takes_str(&s));

    let mut m = String::from("world");
    println!("&mut String 传给 &str：{}", takes_str(&mut m));

    let r = &&s;
    println!("&&String 也能传：{}", takes_str(r));

    println!("泛型处不强转，量到的是 String 本体：{}", generic_size(&s));
    println!("显式切成 &str 后量到：{}", generic_size(s.as_str()));
}
```

```text
&String 传给 &str：5
&mut String 传给 &str：5
&&String 也能传：5
泛型处不强转，量到的是 String 本体：24
显式切成 &str 后量到：5
```

三个值得记住的结论：

1. `&String`、`&&String`、`&mut String` 都能传给要 `&str` 的参数——强转会沿 `Deref` 链走多级。
2. **强转只发生在类型已知的位置**。`generic_size(&s)` 里 `T` 是待推断的，编译器把它推成 `String`，不会「顺便」切成 `str`，于是量到的是 `String` 本体（24 字节）。泛型 API 常常要你显式写 `&*x` 或 `x.as_str()`，原因就在这里。
3. 强转不改变「谁拥有这块内存」，只是把引用重新指向内层。

**方法调用是另一套规则**：编译器先把接收者类型反复解引用，得到候选类型列表，再逐级找。Reference 的例子：接收者是 `Box<[i32; 2]>` 时，候选依次为 `Box<[i32; 2]>`、`&Box<[i32; 2]>`、`&mut Box<[i32; 2]>`、`[i32; 2]`、`&[i32; 2]`、`&mut [i32; 2]`、`[i32]`（数组到切片的 unsize 强转）、`&[i32]`、`&mut [i32]`。**先在近处找，找不到才往外走**——顺序带来一个反直觉的后果：

```rust
use std::ops::Deref;

struct Inner;
impl Inner {
    fn name(&self) -> &'static str {
        "Inner"
    }
}

struct Wrapper(Inner);
impl Deref for Wrapper {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}
impl Wrapper {
    fn name(&self) -> &'static str {
        "Wrapper"
    }
}

fn main() {
    let w = Wrapper(Inner);
    println!("w.name() -> {}", w.name());
    println!("(*w).name() -> {}", (*w).name());
}
```

```text
w.name() -> Wrapper
(*w).name() -> Inner
```

外层自己的方法先被找到，`Inner` 的同名方法**被遮蔽**：`w.name()` 永远调不到它，必须写 `(*w).name()`。std 的 `Deref` 文档把这条列进「不该实现 `Deref`」的三条理由之一（另两条：解引用可能意外失败、承诺公开 API 代价太大）——类型若有和 `Target` 同名的方法、或强转会让人意外，就别实现它；要「换个视角看内部数据」，用 `AsRef`/`Borrow`。

**最后一条边界：`*` 能不能把值搬出来。** `*` 并不总是复制或移动：

```rust
fn main() {
    let b = Box::new(String::from("box"));
    let s: String = *b; // Box 是唯一能这样「搬出内容」的智能指针
    println!("搬出来了：{s}");

    let v = vec![1, 2, 3];
    let bv: Box<[i32]> = v.into_boxed_slice();
    println!("Box<[i32]> 长度 {}", bv.len());
}
```

```text
搬出来了：box
Box<[i32]> 长度 3
```

`Box` 特殊：编译器知道堆上那块内存只有它能到，所以 `*box` 允许把值搬走；`Rc`/`Arc` 的内容是共享的，搬不出来（见报错现场四）。这也解释了 `Box` 为什么是「零开销」的——它就是「把值放到别处」，不带计数。

### Send 与 Sync 在这些类型上的分布

`Send`/`Sync` 系统讲在第 19 章，这里只给本章类型的一张分布表。判据先列清楚——std 的 `Sync` 文档给出的定义原文是：a type `T` is `Sync` if and only if `&T` is `Send`。由此有三条（译自原文）：

- `&T` 是 `Send`，当且仅当 `T` 是 `Sync`；
- `&mut T` 是 `Send`，当且仅当 `T` 是 `Send`；
- `&T` 与 `&mut T` 都是 `Sync`，当且仅当 `T` 是 `Sync`。

下面是本章各类型的实现，每一格取自 1.98 各类型页的 `Send`/`Sync` 实现——多数写在 Trait Implementations 段，`Box`、`Cow`、`UnsafeCell` 的 `Send`、`LazyLock` 的 `Send` 写在 Auto Trait Implementations 段（`T` 指内层类型）：

| 类型 | `Send` | `Sync` | 为什么 |
|---|---|---|---|
| `Box<T>` | `T: Send` | `T: Sync` | 透传内层性质 |
| `Rc<T>`、`rc::Weak<T>` | 否 | 否 | 计数非原子 |
| `Arc<T>`、`sync::Weak<T>` | `T: Send + Sync` | `T: Send + Sync` | 计数原子；但 `&T` 会到别的线程，最后一个句柄也可能在别的线程 drop |
| `Cell<T>` | `T: Send` | 否 | `&Cell<T>` 就能改值，跨线程共享即无同步写 |
| `RefCell<T>` | `T: Send` | 否 | 同上，检查挪到运行期 |
| `UnsafeCell<T>` | `T: Send` | 否 | 内部可变性的底层原语，规则由使用者保证 |
| `Mutex<T>` | `T: Send` | `T: Send` | 拿锁即独占，故 `Sync` **不要求** `T: Sync` |
| `RwLock<T>` | `T: Send` | `T: Send + Sync` | 读锁可并存，还要 `T: Sync` |
| `OnceLock<T>` | `T: Send` | `T: Send + Sync` | 同 `RwLock` |
| `LazyLock<T, F>` | `T: Send`、`F: Send` | `T: Send + Sync`、`F: Send` | 同 `OnceLock`，外加初始化函数要能跨线程 |
| `Cow<'a, B>` | `B::Owned: Send` 且 `B: Sync` | `B::Owned: Sync` 且 `B: Sync` | 两个变体都要过关 |

读表抓住三件事：

1. **`Send` 与 `Sync` 是两个独立的问题**。`Cell<T>` 是「送得出去、共享不出去」的典型：整块搬到别的线程没问题，两个线程同持 `&Cell<T>` 就有问题。
2. **`Arc<T>` 两个约束都要满足 `T: Send + Sync`**，这是最容易记错的一格：`Arc` 生来被多方持有，`&T` 必然出现在别的线程（要 `Sync`），而最后一个句柄可能在任意线程 drop（要 `Send`）。
3. **`Mutex<T>` 的 `Sync` 只要 `T: Send`**：锁保证同一时刻只有一个访问者，内层不必 `Sync`。所以 `Mutex<RefCell<i32>>` 能编译。

这张表最实用的用法是当「换件清单」：报错说某类型 `cannot be sent/shared between threads safely` 时，找它所在的格，换成「跨线程」那一列——`Rc` 换 `Arc`，`Rc<RefCell<T>>` 换 `Arc<Mutex<T>>`。

## 动手看看

下面这个程序把 `Rc`、`Weak`、`RefCell`、`Cell`、`Cow` 用在了一个真实形状上：一棵目录树。

```rust
{{#rustdoc_include ../snippets/ch17/tree.rs}}
```

设计上每个选择都有理由：

- `children: RefCell<Vec<Rc<Dir>>>` —— 目录要能加子目录，但拿到的只有 `&self`，所以用 `RefCell`；子目录由父目录共享持有，所以是 `Rc`。
- `parent: Weak<Dir>` —— 「向上」的边用弱引用，避免父子互持的环。
- `files: Cell<usize>` —— 计数是最典型的 `Cell` 场景：值小、可复制、只需要 `&self`。
- `path()` 返回 `Cow<'_, str>` —— 根目录直接借用一个字面量，其余情况才拼字符串。

在仓库根编译并运行（下面按 Windows 写；其他平台编译成 `tree`、运行 `./tree`）：

```console
$ rustc --edition 2024 snippets/ch17/tree.rs -o tree.exe
$ .\tree.exe
```

```text
util.path()     = /src/util
root.path()     = /
root.total_files() = 11
util 的父目录   = Some("src")
src 下有 5 个文件
docs 下有 5 个文件
src 的强引用数 = 3
释放 root 与 src 之后，util 仍被本地变量持有
  父目录还在吗：None
  这时 path() 退化成 "/"
```

最后三行最该带走。`Rc::strong_count(&src)` 是 3：握着 `src` 的有三处——`root.children` 里的一份、局部变量 `src`、`Rc::clone` 出来的 `same_src`。`root` 与 `src` 都 drop 之后，局部 `util` 还活着（它自己也是一个强引用），但它指向的父目录已释放，于是 `util.parent.upgrade()` 返回 `None`，`path()` 顺着父指针往上走也拿到 `None`，退化成 `"/"`。

**弱引用不会报错，只会告诉你「那头没了」**。所以用 `Weak` 的代码必须把 `upgrade()` 的 `Option` 当正常情况处理——和 `RefCell` 的 panic 相对，这里不会 panic，只有「安静地退化」这种更难查的 bug。

## 报错现场

下面四条最容易撞上。行号列号按「最小复现放进 `src/main.rs`」给出，无关输出已删；编译器 1.98。

### 现场一：`Rc` 进不了线程（E0277）

把单线程的共享类型丢给线程，是最常见的试错：

```rust,compile_fail
use std::rc::Rc;
use std::thread;

fn main() {
    let data = Rc::new(vec![1, 2, 3]);
    let d = Rc::clone(&data);
    thread::spawn(move || {
        println!("{:?}", d.len());
    })
    .join()
    .unwrap();
    println!("{:?}", data.len());
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0277]: `Rc<Vec<i32>>` cannot be sent between threads safely
   --> src/main.rs:7:19
    |
  7 |       thread::spawn(move || {
    |       ------------- ^------
    |  _____|_____________within this `{closure@src/main.rs:7:19: 7:26}`
    = help: within `{closure@src/main.rs:7:19: 7:26}`, the trait `Send` is not implemented for `Rc<Vec<i32>>`
note: required because it's used within this closure
note: required by a bound in `spawn`
    |
128 |     F: Send + 'static,
    |        ^^^^ required by this bound in `spawn`
```

读法：主 span 落在闭包上（「这个闭包不是 `Send`」），`help` 点破根因（`Rc<Vec<i32>>` 没实现 `Send`），`note` 给出约束来源（`spawn` 要求 `F: Send + 'static`）。修法就是把 `Rc` 换成 `Arc`：

```rust
use std::sync::Arc;
use std::thread;

fn main() {
    let data = Arc::new(vec![1, 2, 3]);
    let d = Arc::clone(&data);
    thread::spawn(move || {
        println!("线程里长度 {}", d.len());
    })
    .join()
    .unwrap();
    println!("主线程长度 {}", data.len());
}
```

```text
线程里长度 3
主线程长度 3
```

顺带说明：`Arc<Vec<i32>>` 能用，是因为 `Vec<i32>: Send + Sync` 都满足。内层不是 `Sync`（比如 `RefCell`）时 `Arc` 也救不了——`Arc<RefCell<i32>>` 报同一个 E0277，缺的这次是 `Sync`。

### 现场二：`RefCell` 的运行期 panic

这次编译器帮不了你：代码完全通过编译，运行时才炸：

```rust,should_panic
use std::cell::RefCell;

fn main() {
    let c = RefCell::new(5);
    let m = c.borrow_mut();
    println!("持有可变借用时 m = {m}");
    let b = c.borrow(); // 越界借用：运行时 panic
    println!("{b}");
}
```

1.98 实测输出（首行是带线程号的 panic 头，第二行才是要记的）：

```text
thread 'main' (29588) panicked at src/main.rs:7:15:
RefCell already mutably borrowed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

反向也对称：持有 `borrow()` 时再 `borrow_mut()`，信息是 `RefCell already borrowed`。两次 panic 的根源都是那个借用计数：`m` 活到 `main` 结束，计数没归零，第二次借用自然被判为冲突。

定位思路是「找谁还活着」而不是「找哪一行错」：`m` 的作用域一直到函数末尾。修法有两种：

```rust
use std::cell::RefCell;

fn main() {
    let c = RefCell::new(5);
    {
        let mut m = c.borrow_mut(); // 借用在块内结束
        *m += 10;
    }
    println!("现在 {}", c.borrow()); // 这里已经没有活的 guard 了
}
```

```text
现在 15
```

或者换成 `try_borrow`：

```rust
use std::cell::RefCell;

fn main() {
    let c = RefCell::new(5);
    if let Ok(v) = c.try_borrow() {
        println!("拿到 {v}");
    }
    if c.try_borrow_mut().is_err() {
        println!("有借用没结束，走另一条路");
    }
}
```

```text
拿到 5
```

**用哪个**：借用冲突属于「代码逻辑坏了」时用 `borrow` + panic（测试立刻发现）；属于「外部输入决定的正常分支」时用 `try_borrow` + 处理 `Err`（判据同第 7 章）。

> [!TIP]
> 更隐蔽的变体：`if let Some(&first) = c.borrow().first() { c.borrow_mut().push(first); }` 同样 panic。原因不在 `borrow_mut` 而在 `if let`——`c.borrow()` 的临时值会一直活过整个 `then` 块（edition 2024 对该作用域的调整只影响 `else` 分支）。练习 2 有分析和修法。

### 现场三：`Cell::get` 要求 `Copy`（E0599）

```rust,compile_fail
use std::cell::Cell;

fn main() {
    let c = Cell::new(String::from("hi"));
    let s: String = c.get();
    println!("{s}");
}
```

```text
error[E0599]: the method `get` exists for struct `Cell<String>`, but its trait bounds were not satisfied
   --> src/main.rs:5:23
    |
  5 |     let s: String = c.get();
    |                       ^^^
    |
    = note: the following trait bounds were not satisfied:
            `String: Copy`
```

`get` 必须复制一份给你，所以内层得是 `Copy`。换方法就好：拿走用 `take()`，换新值并取旧值用 `replace()`。

### 现场四：`*rc` 搬不出内容（E0507）

```rust,compile_fail
use std::rc::Rc;

fn main() {
    let b = Box::new(String::from("box"));
    let s: String = *b; // 这一句没问题：Box 能搬出内容
    println!("{s}");

    let r = Rc::new(String::from("rc"));
    let t: String = *r; // 这一句不行
    println!("{t}");
}
```

```text
error[E0507]: cannot move out of an `Rc`
 --> src/main.rs:9:21
  |
9 |     let t: String = *r;
  |                     ^^ move occurs because value has type `String`, which does not implement the `Copy` trait
  |
help: consider cloning the value if the performance cost is acceptable
  |
9 -     let t: String = *r;
9 +     let t: String = r.clone();
```

编译器把两条路都写出来了，取哪条取决于你要「句柄」还是「内容」：`r.clone()` 复制**句柄**（多一个所有者，内容共享），`(*r).clone()` 复制**内容**；确定是最后一个句柄时 `Rc::try_unwrap(r)` 能把内容原样取出。`Box` 是唯一允许 `*b` 直接搬走的智能指针，因为堆上那块内存只有它知道。

那条 `help` 要留心：它建议的 `r.clone()` 换的是**句柄**，照抄会报 E0308（`expected String, found Rc<String>`）——它不是能直接抄的修法；要拿 `String` 得写 `(*r).clone()`。

## 小结

- 三类需求对应三类工具：**共享所有权**用 `Rc`/`Arc`，**在 `&T` 下改内容**用 `Cell`/`RefCell`（单线程）或 `Mutex`/`RwLock`（跨线程），**递归类型与堆上的独占值**用 `Box`。
- 「智能指针」= `Deref` + `Drop`：前者让它像引用，后者让它在最后一个所有者消失时释放堆内存。
- `Rc::clone` 只加计数；`Weak` 不计入强引用，用来剪断引用环，但必须处理 `upgrade() == None`。
- `Cell` 只搬值（`get` 要 `Copy`、`take` 要 `Default`、`replace` 换出旧值）；`RefCell` 把别名规则挪到运行期，冲突即 panic，可预期时用 `try_borrow`——**guard 的作用域就是借用本身**。
- `Mutex<T>` 是跨线程的 `RefCell`，guard 即锁；持锁 panic 会「中毒」，`.unwrap()` 是传播它的正常做法。
- `Deref` 解引用强转只发生在类型明确的位置（泛型参数处不会发生），方法解析「先近后远」故外层同名方法遮蔽内层；`*box` 能搬出内容，`*rc` 不能。
- `Rc` 既非 `Send` 也非 `Sync`；`Cell`/`RefCell` 在 `T: Send` 时是 `Send`，但都不是 `Sync`。跨线程按分布表换件：`Rc<RefCell<T>>` → `Arc<Mutex<T>>`。

## 练习

**1. 预测编译结果。** 下面两段只差一个类型：哪段能编译、哪段不能？不能的那段报的是哪个 trait 的错？

```rust,compile_fail
use std::rc::Rc;
use std::thread;

fn main() {
    let n = Rc::new(1);
    let m = Rc::clone(&n);
    thread::spawn(move || println!("{m}")).join().unwrap();
    println!("{n}");
}
```

```rust
use std::sync::Arc;
use std::thread;

fn main() {
    let n = Arc::new(1);
    let m = Arc::clone(&n);
    thread::spawn(move || println!("{m}")).join().unwrap();
    println!("{n}");
}
```

<details><summary>参考答案</summary>

第一段编译不过，报 `E0277`；第二段能编译，输出两行 `1`。

```text
error[E0277]: `Rc<i32>` cannot be sent between threads safely
   = help: within `{closure@src/main.rs:7:19: 7:26}`, the trait `Send` is not implemented for `Rc<i32>`
note: required because it's used within this closure
note: required by a bound in `spawn`
```

原因在**闭包捕获了什么**：`move ||` 把 `m`（`Rc<i32>`）搬进闭包体，而 `thread::spawn` 要求 `F: Send + 'static`，于是要求 `Rc<i32>: Send`。缺的是 `Send` 而非 `Sync`——正对应 `Rc` 那一格的「否/否」：非原子计数让它连「整块搬过去」都不安全（另一个线程还能 clone）。`Arc<i32>` 能过，因为 `i32: Send + Sync` 都满足。

常见误区：`Rc` 不能跨线程**不是因为「共享」本身**，而是因为那个计数器没有同步；`Arc` 的原子计数就是为此付的价钱。

</details>

**2. 找错并解释。** 下面这段代码编译通过，运行时会 panic：指出直接原因、给出两种修法，并说明为什么 `let first = c.borrow().first().copied();` 是安全的。

```rust,should_panic
use std::cell::RefCell;

fn main() {
    let c = RefCell::new(vec![1, 2, 3]);
    if let Some(&first) = c.borrow().first() {
        c.borrow_mut().push(first);
    }
    println!("{:?}", c.borrow());
}
```

<details><summary>参考答案</summary>

panic 信息是 `RefCell already borrowed`。直接原因是 `if let` 的 scrutinee 里由 `c.borrow()` 产生的临时 `Ref` 会**活过整个 `then` 块**（edition 2024 只把它的终点提前到「进入 `else` 之前」，不影响 `then`），于是 `then` 里的 `borrow_mut()` 撞上同一个 `RefCell` 的借用计数。

两种修法（下面把它们合在一个完整程序里，两种写法都能跑）：

```rust
use std::cell::RefCell;

fn main() {
    let c = RefCell::new(vec![1, 2, 3]);

    // 修法一：让借用收在一条语句里
    let first = c.borrow().first().copied();
    if let Some(first) = first {
        c.borrow_mut().push(first + 10);
    }

    // 修法二：用一个显式块把借用范围画清楚
    let last = {
        let g = c.borrow();
        g.last().copied()
    };
    if let Some(last) = last {
        c.borrow_mut().push(last + 10);
    }

    println!("{:?}", c.borrow());
}
```

```text
[1, 2, 3, 11, 21]
```

两者都可行是同一个原因：`.copied()` 把 `Option<&i32>` 变成 `Option<i32>`，**值被复制出来了**，对 `c` 的借用在那条语句（或那个块）结束时就能结束，scrutinee 里不再有任何 `Ref`，运行期计数归零，后面的 `borrow_mut()` 才拿得到。

这条练习的通用教训：`RefCell` 的问题几乎从来不是「哪一行」，而是「哪个 guard 还活着」。遇到 panic 先画 guard 的生命周期。

</details>

**3. 解释为什么。** `Cell<T>` 与 `RefCell<T>` 在 `T: Send` 时都是 `Send`，却都**不是** `Sync`。为什么必须如此？如果你确实要跨线程共享可变状态，该从本章哪张表里挑什么？

<details><summary>参考答案</summary>

`T: Sync` 的定义是「`&T` 可以跨线程传」（std 原文：a type `T` is `Sync` if and only if `&T` is `Send`）。

而它们的意义就是「拿着 `&Cell<T>` 就能改里面的值」——`Cell::set` 与 `RefCell::borrow_mut` 的接收者都是 `&self`。若实现了 `Sync`，两个线程就能各拿一个 `&Cell<T>` 同时 `set`（或各拿 `RefMut`）而**毫无同步**，那正是数据竞争、即未定义行为，所以必须 `!Sync`。

那为什么还是 `Send`？`Send` 问的是另一个问题：「整块搬给别的线程行不行」。移交之后原线程手上什么都不剩——移动没事，共享才出事。

跨线程时按「四个格子」表取「跨线程」那一列：`RefCell<T>` → `Mutex<T>`（写多）或 `RwLock<T>`（读多写少）；`Cell<T>` → 通常是原子类型（第 20 章）或 `Mutex`/`OnceLock`；外层 `Rc<T>` → `Arc<T>`。所以 `Rc<RefCell<Vec<Task>>>` 的对应物是 `Arc<Mutex<Vec<Task>>>`。`Mutex<RefCell<i32>>` 也能编译，但锁已提供独占访问，再套 `RefCell` 纯属多余。

</details>

## 语言对照角

**C++。** `std::unique_ptr` 与 `Box` 是同一个模型：标准草案要求它「可移动构造、可移动赋值，但不可复制构造、不可复制赋值」（is Cpp17MoveConstructible and Cpp17MoveAssignable, but is not Cpp17CopyConstructible nor Cpp17CopyAssignable）。`std::shared_ptr` 对应 `Arc`，但 C++ 没有 `Rc` 的对应物——引用计数天然原子，单线程也要付原子的钱。环的问题两边一样（`std::weak_ptr` 加 `lock()` 与 `downgrade`/`upgrade` 同构）；**同步性不一样**：C++ 没有 `Send`/`Sync`，把 `shared_ptr<T>` 送到别的线程再两边同改 `*p`，编译器不会拦你。

**Java。** Java 的对象引用不对应本章任何类型：没有所有权，没有确定性析构，回收全交给 GC。最接近 `Weak` 的是 `java.lang.ref.WeakReference`——官方文档说它「不阻止其 referent 被终结和回收」，且「最常用于实现规范化映射」，正是缓存。区别在时机：`Rc`/`Arc` 的强引用归零是**确定**的；Java 弱引用何时被清空由 GC 决定。

## 延伸阅读

- The Book 第 15 章《Smart Pointers》（`Box`/`Deref`/`Drop`/`Rc`/`RefCell` 入门）：<https://doc.rust-lang.org/book/ch15-00-smart-pointers.html>
- `std::cell` 模块文档（四种 cell 的对照、什么时候该用内部可变性）：<https://doc.rust-lang.org/std/cell/index.html>
- `std::rc`（`Rc`/`Weak`、`strong_count`/`downgrade`）：<https://doc.rust-lang.org/std/rc/index.html>
- `std::sync`（`Arc`/`Mutex`/`RwLock`/`OnceLock`/`LazyLock` 全在这页）：<https://doc.rust-lang.org/std/sync/index.html>
- `std::borrow::Cow`（`to_mut`/`into_owned`）：<https://doc.rust-lang.org/std/borrow/enum.Cow.html>
- `std::ops::Deref`（解引用强转的定义，以及「什么时候不该实现 `Deref`」的清单）：<https://doc.rust-lang.org/std/ops/trait.Deref.html>
- `std::marker::Send` 与 `std::marker::Sync`（判据与引用类型的对应关系）：<https://doc.rust-lang.org/std/marker/trait.Send.html>、<https://doc.rust-lang.org/std/marker/trait.Sync.html>
- The Rust Reference：内部可变性（<https://doc.rust-lang.org/reference/interior-mutability.html>）、方法调用表达式里的自动解引用顺序（<https://doc.rust-lang.org/reference/expressions/method-call-expr.html>）、类型强转清单（<https://doc.rust-lang.org/reference/type-coercions.html>）
- The Nomicon：`Send`/`Sync` 与 `unsafe impl`（<https://doc.rust-lang.org/nomicon/send-and-sync.html>）、锁中毒（<https://doc.rust-lang.org/nomicon/poisoning.html>）
