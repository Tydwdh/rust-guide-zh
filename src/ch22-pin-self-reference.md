# 第 22 章 Pin 与自引用

## 这一章解决什么问题

第 21 章的 `Future::poll` 签名里出现过 `self: Pin<&mut Self>`，却没有展开它为什么不是 `&mut self`；`async fn` 编译出的 future 是 `!Unpin` 这件事，也留到这一章补上。本章把这两件事连起来讲完。

先看 `async fn` 编译成的状态机。跨 `.await` 存活的局部变量不会放在独立的栈槽里，而是成为状态机自己的字段；如果代码里出现「先借出一个局部变量、再把这个借用存回状态机」，状态机内部就会有一根指向自己某个字段的指针。这种类型叫**自引用（self-referential）**类型。它本身没有问题，问题在于 Rust 允许值自由移动：一旦整个状态机被搬到内存里的另一个位置，那根内部指针还指着旧地址，解引用它就是访问已经不属于这个值的内存。而编译器在移动值的时候**不会通知被移动的值**，所以「搬完以后自己把指针修一遍」这条路根本走不通。

`Pin` 是绕开这个死结的工具：它把一个「被指向的值不再移动」的承诺写进类型里，规定谁有资格创建这个承诺、谁必须兑现它。读完之后，你应该能做到：

- 说清「移动」在 Rust 里到底发生了什么，以及地址不稳定是设计常态而不是缺陷；
- 读懂 `Pin<Ptr>` 的契约：被 pin（钉住、固定就位）的是 `Ptr` 指向的那个值，而不是 `Ptr` 本身；
- 分清 `Pin::new`、`pin!`、`Box::pin` 三个入口各自的要求与适用场景；
- 解释 `Unpin` 为什么「几乎所有的类型都实现了」，以及 `PhantomPinned` 怎么把类型标成地址敏感；
- 列出 `Pin` API 里哪些操作必须 `unsafe`，以及各自要求调用者保证什么。

> [!NOTE]
> 本章所有「实测」输出来自本机 `rustc 1.98.0 (88d9e12ae 2026-08-18)`、edition 2024、x86_64 Windows；`std` 文档的措辞与签名核对自同一版本的本机快照。nightly-only 的地方单独标注，报错按「最小复现 + 关键行节选」呈现。

## 为什么这样设计

### 「移动」在机器上只是逐字节复制

Rust 文档在 `std::pin` 模块开头给了一句总结：

> All values in Rust are trivially *moveable*. This means that the address at which a value is located is not necessarily stable in between borrows.

译文：Rust 里所有值都可以随意移动；这意味着一个值所在的地址，在两次借用之间不保证保持稳定。

后半句紧接着解释了原因：**"The compiler is allowed to move a value to a new address without running any code to notify that value that its address has changed."**（大意：编译器可以把值搬到新地址，而不运行任何代码去通知这个值「你换地方了」。）写个程序就看得见：

```rust
struct AddrTracker {
    moves_seen: usize,
}

fn main() {
    let tracker = AddrTracker { moves_seen: 0 };
    println!("栈上:   {:#x}", &tracker as *const AddrTracker as usize);

    let mut v = Vec::new();
    v.push(tracker); // 移动进堆
    println!("Vec 里: {:#x}", &v[0] as *const AddrTracker as usize);

    let boxed = Box::new(v.remove(0)); // 再移动一次
    println!("Box 里: {:#x}", &*boxed as *const AddrTracker as usize);
    println!("moves_seen = {}", boxed.moves_seen);
}
```

实测输出（地址每次运行都不同）：

```text
栈上:   0x210a6ffbe0
Vec 里: 0x19b6a918500
Box 里: 0x19b6a911410
moves_seen = 0
```

三次打印出三个地址，而这个值从头到尾没被执行过任何一行「我搬家了」的代码。**移动在语义上是所有权的移动，在机器上就是一次按位复制**，和 `Copy` 的差别只在前者会让原绑定失效。理解这一点，才能理解为什么自引用类型需要额外的机器。

顺带破一个常见直觉：把值放进指针后面并不等于地址稳定。`Box<T>` 允许把 `T` 移出来，`&mut T` 允许 `mem::replace` 换掉里面的值——所以 `Pin` 也不会把「不移动」寄托在指针类型上。

### 两条出路，为什么只有一条能用

一个类型如果存在「对地址敏感（address-sensitive）」的阶段，实现者只有两种办法保证内部指针有效：

1. 值被移动时自己察觉，并把所有指向自己的指针更新一遍；
2. 保证这个值在一段时间内不会被移动、存储也不会被挪作他用。

第一条已经被上文的编译器自由否掉了。第二条在实践中还有两种实现方式：要么整个接口都是 `unsafe`，让每个调用者自己守规矩；要么每个类型自己把数据塞进 `Box`、小心维护内部访问。前者会让 `unsafe` 像病毒一样扩散到所有调用方；后者的问题是每个类型各自为政，组合两个对地址敏感的类型就至少要加一层指针间接（实际就是一次堆分配）。

异步任务恰好是组合的重灾区：一个任务里可能叠着好几层 future 和 `async fn` 状态机，每一层都加一次分配不可接受。`Pin` 属于第三条路——**建立一套共享的契约语言**：`Pin<Ptr>` 是「被指向的值不会移动」的凭证，任何类型都可以把保证交给它，而不必各自造一套机制。

### 保证由谁承担

这是本章最该记住的一节。`Pin` 不改变编译器的行为，文档的说法是：

> Pinning does not require nor make use of any compiler “magic”, only a specific contract between the unsafe parts of a library API and its users.

译文：pin 不需要、也不使用任何编译器的「魔法」，它只是「库 API 的 unsafe 部分」与「用户」之间的一份契约。

契约有两面。

**对 safe 代码这一面**，`Pin` 通过收窄访问通道来保证：构造入口里 `Pin::new` 要求被指向的类型 `Unpin`（`pin!` 与 `Box::pin` 则各自满足 pin 的不变量）；而想从这个 `Pin<Ptr>` 里拿回能移动被指向值的 `&mut T`，只能走 `Pin::get_mut`、`Pin::into_inner` 或 `DerefMut`，它们同样要求 `Unpin`。只要不使用 `unsafe`，就无法构造出「假 pin」或者把被 pin 的值偷偷搬走。文档的承诺很干脆：

> Note that as long as you don’t use `unsafe`, it’s impossible to create or misuse a pinned value in a way that is unsound.

译文：只要你不碰 `unsafe`，就不可能以不安全的方式创建或误用被 pin 的值。

**对 `unsafe` 代码那一面**，责任反转了：谁调用 `Pin::new_unchecked`，谁就得保证这个指针指向的值从此不再移动。库作者在 `unsafe` 块里兑现这个承诺，调用者才能继续用 safe API 享受它。第 23 章会系统讲 `unsafe` 的五个能力与 UB 清单；本章只负责让你知道，在 pin 这套东西里，`unsafe` 到底在担保什么。

## 机制与规则

### 22.1 自引用的最小构造

先造一个最小的自引用类型，看它是怎么坏的。这个例子不写任何 `unsafe` 解引用，只比较指针的值：

```rust
struct SelfRef {
    data: String,
    ptr: *const String,
}

impl SelfRef {
    fn new(data: &str) -> Self {
        Self { data: data.to_owned(), ptr: std::ptr::null() }
    }

    /// 地址确定之后，再记录指向自己的指针
    fn init(&mut self) {
        self.ptr = &self.data;
    }

    fn points_to_self(&self) -> bool {
        std::ptr::eq(self.ptr, &self.data)
    }
}

fn main() {
    let mut s = SelfRef::new("hello");
    s.init();
    println!("原地初始化后：{}", s.points_to_self());

    let boxed = Box::new(s); // 一次普通移动
    println!("移进 Box 之后：{}", boxed.points_to_self());
    println!("data = {}", boxed.data);
}
```

实测输出：

```text
原地初始化后：true
移进 Box 之后：false
data = hello
```

两步的差别就是全部问题所在。`init` 在线性代码里工作得很好——值在原地待着，`ptr` 记下的就是它的真实地址；`Box::new(s)` 之后 `data` 被复制到堆上，`ptr` 仍然指着栈上那个旧位置。此时若解引用 `ptr`，读到的就是别的值（甚至可能是已经被复用的内存），这正是**未定义行为（UB, undefined behavior）**。本章所有示例都只比较指针值、不解引用，所以是安全代码；真实的自引用集合必须在拿到第一个指针时就确保地址不再变化。

还有一个更隐蔽的坑：**用构造函数返回自引用值是行不通的**。`fn new() -> SelfRef` 里在局部变量上记下地址，返回给调用者时这个值必然被移动一次（就是本节第一小节的那种复制），指针当场失效。所以下文的类型都把「初始化指针」拆成单独一步，并要求它只接受 `Pin<&mut Self>`。

### 22.2 `Pin<Ptr>` 的契约

`Pin` 的定义简单得看不出门道：

```rust,ignore
pub struct Pin<Ptr> { /* private fields */ }
```

它是一个单字段的包装类型（`#[fundamental]` 的包装，第 12 章讲过这个属性），字段私有，所以只能通过官方入口构造。要抓住的关键是：**`Pin` 包住的不是要被 pin 的值，而是指向它的指针**。`Pin<&mut T>` 是把一个 `&mut T` 包起来，承诺它指向的那个 `T` 不再移动；`Pin<Box<T>>` 同理。文档对此的说法是「`Pin<Ptr>` 不 pin `Ptr`，它 pin 的是指针的**被指向值（pointee）**」。

下图把这三层摊开，并用 22.1 那次移动对照说明 pin 契约要挡住什么（右格即实测的 `true → false`）：

<svg viewBox="0 0 660 300" width="100%" role="img" aria-label="Pin&lt;&amp;mut T&gt; 的三层结构：Pin 包装层包住 &amp;mut T 指针层，指针指向 T 的值层；值内部的 ptr 指向自己的 data 字段。移动前值在地址 A，ptr 指向自身、有效（true）；值被搬到新地址 B 后，Pin 与 &amp;mut T 都跟着指向 B，ptr 却仍指向 A，于是失效（false）" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">Pin&lt;&amp;mut T&gt; 的三层结构，与移动前后的对照</text>
<text x="20" y="40" font-size="12" fill="var(--links)">移动前：原地初始化 → true</text>
<text x="360" y="40" font-size="12" fill="var(--links)">移动后：值被搬走 → false</text>
<rect x="90" y="52" width="140" height="32" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="160" y="73" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">Pin&lt;&amp;mut T&gt;</text>
<line x1="160" y1="84" x2="160" y2="92" stroke="currentColor" stroke-width="1.5" />
<polyline points="156,92 160,100 164,92" fill="none" stroke="currentColor" stroke-width="1.5" />
<rect x="90" y="100" width="140" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="160" y="120" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">&amp;mut T</text>
<line x1="160" y1="130" x2="160" y2="138" stroke="currentColor" stroke-width="1.5" />
<polyline points="156,138 160,146 164,138" fill="none" stroke="currentColor" stroke-width="1.5" />
<rect x="20" y="146" width="280" height="98" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="32" y="166" font-size="12" fill="currentColor" opacity="0.7">T 的值（地址 A）</text>
<rect x="36" y="176" width="112" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="92" y="195" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">data = "hello"</text>
<rect x="176" y="176" width="108" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="230" y="195" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">ptr</text>
<line x1="176" y1="191" x2="152" y2="191" stroke="var(--links)" stroke-width="1.5" />
<polyline points="158,187 150,191 158,195" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="160" y="228" font-size="12" fill="var(--links)" text-anchor="middle">ptr 指向自身字段 ✓</text>
<rect x="460" y="52" width="140" height="32" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="530" y="73" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">Pin&lt;&amp;mut T&gt;</text>
<line x1="530" y1="84" x2="530" y2="92" stroke="currentColor" stroke-width="1.5" />
<polyline points="526,92 530,100 534,92" fill="none" stroke="currentColor" stroke-width="1.5" />
<rect x="460" y="100" width="140" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="530" y="120" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">&amp;mut T</text>
<line x1="530" y1="130" x2="530" y2="138" stroke="currentColor" stroke-width="1.5" />
<polyline points="526,138 530,146 534,138" fill="none" stroke="currentColor" stroke-width="1.5" />
<rect x="440" y="146" width="200" height="98" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="452" y="166" font-size="12" fill="currentColor" opacity="0.7">T 的值（新地址 B）</text>
<rect x="456" y="176" width="100" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="506" y="195" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">data = "hello"</text>
<rect x="566" y="176" width="62" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="597" y="195" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">ptr</text>
<rect x="340" y="176" width="88" height="64" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="384" y="200" font-size="12" fill="var(--warning-border)" text-anchor="middle">旧地址 A</text>
<text x="384" y="220" font-size="12" fill="var(--warning-border)" text-anchor="middle">已被复用</text>
<polyline points="628,191 648,191 648,258 384,258 384,246" fill="none" stroke="var(--warning-border)" stroke-width="1.5" />
<polyline points="380,246 384,240 388,246" fill="none" stroke="var(--warning-border)" stroke-width="1.5" />
<text x="500" y="276" font-size="12" fill="var(--warning-border)" text-anchor="middle">ptr 仍指旧地址 ✗</text>
<text x="0" y="292" font-size="12" fill="currentColor" opacity="0.7">移动只复制字节：Pin 与 &amp;mut T 跟着值走，值内部的 ptr 没人通知</text>
</svg>

访问被 pin 的值，走的是 `Deref`：

| 通道 | 要求 | 拿到的 |
|---|---|---|
| `Deref` | `Ptr: Deref` | `&<Ptr as Deref>::Target` |
| `DerefMut` | `Ptr: DerefMut` 且 `Ptr::Target: Unpin` | `&mut <Ptr as Deref>::Target` |
| `Pin::get_ref` | `Pin<&'a T>` | `&'a T` |
| `Pin::get_mut` | `Pin<&'a mut T>`，`T: Unpin` | `&'a mut T` |

共享访问完全不设防：`&T` 本来就不能把值搬走。可变访问则是分水岭——`DerefMut` 和 `get_mut` 都要求 `T: Unpin`（`Pin` 的可变访问是通过内部 helper 实现的，语义上就是这两条约束），因为一旦把 `&mut T` 交出去，`mem::replace`、`*p = other` 这些操作立刻就能移动或替换被 pin 的值。

几个常被问到的方法，签名与语义如下（签名节选，核对自 1.98 快照）：

| 方法 | 签名节选 | 说明 |
|---|---|---|
| `Pin::new` | `pub const fn new(pointer: Ptr) -> Pin<Ptr>`，要求 `Ptr: Deref`、`<Ptr as Deref>::Target: Unpin` | safe 构造入口之一；`Unpin` 约束在这里把风险挡掉 |
| `Pin::into_inner` | `pub const fn into_inner(pin: Pin<Ptr>) -> Ptr` | 拆回指针，同样要求 pointee `Unpin` |
| `Pin::new_unchecked` | `pub const unsafe fn new_unchecked(pointer: Ptr) -> Pin<Ptr>` | 无约束，责任全在调用者 |
| `Pin::into_inner_unchecked` | `pub const unsafe fn into_inner_unchecked(pin: Pin<Ptr>) -> Ptr` | 拆回指针，之后仍要当作 pinned 对待 |
| `Pin::as_ref` / `as_mut` | `fn as_ref(&self) -> Pin<&<Ptr as Deref>::Target>`；`as_mut` 要求 `Ptr: DerefMut` | `Pin<Box<T>>` → `Pin<&mut T>` 这类降级；`as_mut` 只是重借用，不放松任何限制 |
| `Pin::get_unchecked_mut` | `pub const unsafe fn get_unchecked_mut(self) -> &'a mut T` | 拿到 `&mut T`，但不得把值移出去 |
| `Pin::map_unchecked` / `map_unchecked_mut` | `pub unsafe fn map_unchecked_mut<U, F>(self, func: F) -> Pin<&'a mut U>` | 投影（pin projection）：把整个值的 pin 传给某个字段 |
| `Pin::set` | `pub fn set(&mut self, value: <Ptr as Deref>::Target)` | 原地替换，**不要求** `Unpin`（见下） |
| `Pin::static_ref` / `static_mut` | `pub const fn static_mut(r: &'static mut T) -> Pin<&'static mut T>` | `'static` 借用永远不会结束，所以天然安全 |

`set` 值得单独说：它看起来违反了「不能动被 pin 的值」，其实没有。它先在原地运行旧值的析构函数，再把新值写进同一个位置，地址自始至终没变，新值又是同类型的合法值。实测（`pin!` 见下一小节）：

```rust
use std::marker::PhantomPinned;
use std::pin::pin;

struct Slot {
    name: String,
    _pin: PhantomPinned,
}

impl Slot {
    fn new(name: &str) -> Self {
        Slot { name: name.to_owned(), _pin: PhantomPinned }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

fn main() {
    let slot = Slot::new("first");
    let mut pinned = pin!(slot);
    let addr = &*pinned as *const Slot as usize;

    pinned.set(Slot::new("second")); // !Unpin 也能原地换值

    println!("现在 {}", pinned.name);
    println!("地址不变 {}", &*pinned as *const Slot as usize == addr);
}
```

```text
drop first
现在 second
地址不变 true
drop second
```

顺带记住这里的析构顺序：旧值在写入前析构，新值在作用域结束时析构——这正是 pin 的 **Drop guarantee**：值一旦被 pin，它的存储就不能在 `drop` 返回之前被复用或失效，所以覆盖旧值必须先跑完它的析构。第 16 章的析构检查（dropck）只在更窄的一点上与本章相关：`Drop::drop` 拿到的是 `&mut self`，如果析构代码要去读那根自引用指针，那些被指向的数据就必须活到析构完成。

### 22.3 三个构造入口

safe 代码想造出一个 `Pin`，只有三条路：

| 入口 | 产物 | 对被 pin 类型的要求 | 典型场景 |
|---|---|---|---|
| `Pin::new(&mut value)` | `Pin<&mut T>` | `T: Unpin` | 值本来就不在乎地址，只是要凑一个 `Pin` 参数 |
| `pin!(value)` | `Pin<&mut T>` | 无 | 就地 pin：不分配，值留在当前作用域 |
| `Box::pin(value)` | `Pin<Box<T>>` | 无 | 需要跨作用域保存、返回，一次堆分配 |

（还有一个 `Box::into_pin(boxed)`，把已有的 `Box<T>` 原地变成 `Pin<Box<T>>`，不额外分配。）

三者的差别不是「高级写法」，而是**存放位置决定了能活多久**。

`pin!(value)` 把值移动进一个宏内部的绑定，并对它调用 `Pin::new_unchecked`——宏自己承担了 `unsafe` 的责任（它内部还用了尚未稳定的 `super let` 语法，以 1.98 为准 `super let` 单独写会报 E0658，这里只是 std 内部的特权）。因此：

- 原绑定被移走，之后再用它就是 E0382，而不是什么「pin 之后不能碰」的玄学；
- 拿到的 `Pin<&mut T>` 借的是宏展开处的本地存储，**不能带出作用域**；
- 好处是零分配，而且不依赖分配器，`no_std` 环境里也能用。

`Box::pin(value)` 把值放到堆上，返回的 `Pin<Box<T>>` 打包的是「堆地址 + 保证」。堆上的值不会因为指针自己搬家而移动，所以这个 `Pin<Box<T>>` 可以随意移动、返回、塞进结构体：

```rust
use std::marker::PhantomPinned;
use std::pin::Pin;

struct Task {
    name: String,
    ptr: *const String,
    _pin: PhantomPinned,
}

impl Task {
    fn new(name: &str) -> Self {
        Self { name: name.to_owned(), ptr: std::ptr::null(), _pin: PhantomPinned }
    }

    fn init(self: Pin<&mut Self>) {
        // SAFETY: 只写字段，不移出 self
        let this = unsafe { self.get_unchecked_mut() };
        this.ptr = &this.name;
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn ptr_ok(&self) -> bool {
        std::ptr::eq(self.ptr, &self.name)
    }
}

fn spawn(name: &str) -> Pin<Box<Task>> {
    let mut task = Box::pin(Task::new(name));
    task.as_mut().init();
    task // 返回的是指针，堆上的值没动过
}

fn main() {
    let task = spawn("job-1");
    println!("{} -> {}", task.name(), task.ptr_ok());

    let task2 = task; // 移动 Pin<Box<Task>>：动的是指针
    println!("{} -> {}", task2.name(), task2.ptr_ok());
}
```

```text
job-1 -> true
job-1 -> true
```

`task2` 这一句是关键：移动 `Pin<Box<Task>>` 完全合法，因为被 pin 的是堆上那个 `Task`，而不是 `Box` 本身。代价是一次分配，换来「可以放进任何地方」。

至于 `Pin::new`，它其实什么也没 pin：它是给 `T: Unpin` 的类型准备的「形式上的 `Pin`」。文档特意用了「facade（门面）」这个词——把 `&mut u8` 包成 `Pin<&mut u8>`，`Pin` 的不变量被 `Unpin` 直接豁免，`Pin<&mut T>` 的行为与 `&mut T` 一致（`pinned.push('!')`、`*pinned = ...` 都能写）。它的价值在于让普通类型也能调用那些 `Pin` 参数的 API。

### 22.4 `Unpin`：绝大多数类型都无关

`Unpin` 是一个 **auto trait**（由编译器自动推导实现与否的 trait），声明空得很：

```rust,ignore
pub auto trait Unpin { }
```

它的语义不是「可以被移动」，而是「不依赖任何 pin 保证」。文档原话：

> Implementing the `Unpin` trait for `T` expresses the fact that `T` is pinning-agnostic: it shall not expose nor rely on any pinning guarantees.

译文：为 `T` 实现 `Unpin` 表达的是「`T` 对 pin 无所谓」：它既不公开、也不依赖任何 pin 保证。

所以 `Unpin` 的推导是「字段全 `Unpin`，则类型 `Unpin`」，而绝大多数类型都没有地址敏感状态：

```rust
use std::marker::PhantomPinned;

fn assert_unpin<T: Unpin>() {}

fn main() {
    assert_unpin::<i32>();
    assert_unpin::<String>();
    assert_unpin::<Vec<u8>>();
    assert_unpin::<Box<PhantomPinned>>();   // Box<T> 恒为 Unpin
    assert_unpin::<&mut PhantomPinned>();   // &mut T 恒为 Unpin
    assert_unpin::<*const PhantomPinned>(); // 裸指针同理
    println!("这些都是 Unpin");
}
```

```text
这些都是 Unpin
```

这里藏着本章最值得记的一条规则：**`Pin<Ptr>` 的限制看的是 pointee，不是 `Ptr`**。`Box<T>`、`&mut T`、裸指针永远 `Unpin`（它们本身只是一小段地址，搬走无所谓），所以 `Pin<Box<T>>` 里那个 `Box` 是不是 `Unpin` 完全不影响结果；决定 `Pin<Box<T>>` 松紧的是 `<Box<T> as Deref>::Target = T`。`Unpin` 的文档页把这份清单列得很清楚：基本类型（`bool`、`i32`、`f32`…）、引用与裸指针、以及 `Box`/`Rc`/`Arc`/`String` 这些标准库类型都在里面。文档对此的说法是 **"This trait is automatically implemented for almost every type."**（这个 trait 被自动实现在几乎所有的类型上。）

反过来，一旦类型里有 `!Unpin` 的字段，整个类型就不是 `Unpin`。本机实测，给这类类型做 `Unpin` 断言会失败：

```rust,compile_fail
use std::marker::PhantomPinned;

fn assert_unpin<T: Unpin>() {}

struct HasPin {
    data: String,
    _pin: PhantomPinned,
}

fn main() {
    assert_unpin::<PhantomPinned>();
    assert_unpin::<HasPin>();
}
```

以 Rust 1.98 为例，两条错误各取关键几行（每条末尾重复的约束出处从略，第二条以 `...` 收尾）：

```text
error[E0277]: `PhantomPinned` cannot be unpinned
  --> src/main.rs:11:20
   |
11 |     assert_unpin::<PhantomPinned>();
   |                    ^^^^^^^^^^^^^ the trait `Unpin` is not implemented for `PhantomPinned`
   |
   = note: consider using the `pin!` macro
           consider using `Box::pin` if you need to access the pinned value outside of the current scope
note: required by a bound in `assert_unpin`
  --> src/main.rs:3:20
   |
 3 | fn assert_unpin<T: Unpin>() {}
   |                    ^^^^^ required by this bound in `assert_unpin`

error[E0277]: `PhantomPinned` cannot be unpinned
  --> src/main.rs:12:20
   |
12 |     assert_unpin::<HasPin>();
   |                    ^^^^^^ within `HasPin`, the trait `Unpin` is not implemented for `PhantomPinned`
   |
   = note: consider using the `pin!` macro
           consider using `Box::pin` if you need to access the pinned value outside of the current scope
note: required because it appears within the type `HasPin`
  --> src/main.rs:5:8
   |
 5 | struct HasPin {
   |        ^^^^^^
...
```

下图把字段到类型的这条传播路径摊开：

<svg viewBox="0 0 660 250" width="100%" role="img" aria-label="Unpin 的字段传播：HasPin 的两个字段中，data: String 是 Unpin（✓），_pin: PhantomPinned 不是 Unpin（✗）；只要有一个字段不是 Unpin，整个类型 HasPin 就不是 Unpin" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">Unpin 按字段传播：一个字段说「不」，整个类型就不是 Unpin</text>
<text x="190" y="40" font-size="12" fill="currentColor" opacity="0.7">类型</text>
<rect x="190" y="48" width="280" height="56" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="330" y="74" font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">HasPin</text>
<text x="330" y="92" font-size="12" fill="var(--warning-border)" text-anchor="middle">不是 Unpin ✗</text>
<text x="90" y="130" font-size="12" fill="currentColor" opacity="0.7">字段</text>
<rect x="90" y="136" width="480" height="76" rx="5" fill="none" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<rect x="110" y="150" width="190" height="48" rx="4" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="205" y="175" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">data: String</text>
<text x="205" y="191" font-size="12" fill="var(--links)" text-anchor="middle">Unpin ✓</text>
<rect x="360" y="150" width="190" height="48" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" />
<text x="455" y="175" font-size="12" font-family="monospace" fill="currentColor" text-anchor="middle">_pin: PhantomPinned</text>
<text x="455" y="191" font-size="12" fill="var(--warning-border)" text-anchor="middle">不是 Unpin ✗</text>
<line x1="205" y1="150" x2="205" y2="106" stroke="currentColor" stroke-width="1.5" />
<polyline points="201,112 205,104 209,112" fill="none" stroke="currentColor" stroke-width="1.5" />
<line x1="455" y1="150" x2="455" y2="106" stroke="var(--warning-border)" stroke-width="1.5" />
<polyline points="451,112 455,104 459,112" fill="none" stroke="var(--warning-border)" stroke-width="1.5" />
<text x="0" y="234" font-size="12" fill="currentColor" opacity="0.7">字段全 Unpin，类型才 Unpin；有一个字段不是，自动推导就在这里停下</text>
</svg>

第一条的错误标题就很有信息量：`` `PhantomPinned` cannot be unpinned``——它把 `Unpin` 的缺席描述成一种「无法解除 pin」的属性。第二条的 `within HasPin` 则是字段传播的直接证据。

`Unpin` 是 safe trait，所以也可以手动实现。当类型里的「地址敏感」状态其实与某个字段无关时，可以显式把它标回 `Unpin`：

```rust
use std::marker::PhantomPinned;

struct Wrapper {
    value: String,
    _pin: PhantomPinned,
}

// 显式声明：Wrapper 不依赖任何 pin 保证
impl Unpin for Wrapper {}

fn assert_unpin<T: Unpin>() {}

fn main() {
    assert_unpin::<Wrapper>();
    let w = Wrapper { value: String::from("v"), _pin: PhantomPinned };
    println!("{}", w.value);
}
```

```text
v
```

> [!WARNING]
> `impl Unpin for X {}` 是 safe 写、safe 编译，但它是一句**语义承诺**：从此 `X` 声称自己不在乎地址。这个承诺写错，别人用完全 safe 的代码就能触发 UB——它是本书里少见的「safe 语法、unsafe 后果」。写它之前先问：这个类型的内部有没有指向自己的指针？有没有别的地方依赖它的地址？
>
> 反向的 `impl !Unpin for X {}` 在以 1.98 为准的 stable 上不可用：报 `error[E0658]: negative impls are experimental`。stable 上的标准做法就是像上面那样加一个 `PhantomPinned` 字段，让推导自然得出 `!Unpin`。

### 22.5 `PhantomPinned`：把类型标成地址敏感

`PhantomPinned` 是标准库给出的「开关」，它自己的文档说明只有两句：

> A marker type which does not implement `Unpin`. If a type contains a `PhantomPinned`, it will not implement `Unpin` by default.

译文：一个不实现 `Unpin` 的标记类型。如果某个类型含有 `PhantomPinned`，它默认就不会实现 `Unpin`。

这个类型本身是空结构体，文档页上明晃晃写着 `impl !Unpin for PhantomPinned`。它的用法就是当一个「必须存在的字段」：只要类型的数据里有自引用指针（或者别的地址敏感状态），就放一个 `_pin: PhantomPinned` 进去，让 `Unpin` 的自动推导在这里停下。本章的 `SelfRef`、`Task`、`Slot` 都用了这一招。

第 16 章为了「声明型变」用过 `PhantomData`，两者分工不同：`PhantomData<T>` 声明的是「类型参数/生命周期如何被使用」，`PhantomPinned` 声明的是「这个类型必须被 pin 住才安全」。它们经常一起出现在手写集合里，符号长得也像，别混。

### 22.6 什么时候真的需要 `unsafe`

把这章的边界收成一张清单：**下面这些地方要么要你自己写 `unsafe`，要么要你额外承担 pin 的契约**（第 5 项属于后者——`Drop` 的实现本身是 safe 语法，只是写法必须按 pin 处理）。每一项要保证的东西都不同：

1. **`Pin::new_unchecked`**：你保证指针指向的值从现在起不再移动、存储也不会被复用。文档把违规后果说得很重：**"If the caller that is constructing this `Pin<Ptr>` does not ensure that the data `Ptr` points to is pinned, that is a violation of the API contract and may lead to undefined behavior in later (even safe) operations."**（大意：调用者若没保证 pinned，就违反了 API 契约，之后的**安全代码**都可能触发 UB。）它还附带一条常被忘掉的义务：`Ptr` 的 `Deref`/`DerefMut`/`Drop` 实现不能把 `self` 里的值移出去，否则 `Pin::as_ref`/`as_mut` 会跟着出错。
2. **`Pin::get_unchecked_mut`**：拿到 `&mut T` 之后不许把值移走、换掉。文档的要求是 **"You must guarantee that you will never move the data out of the mutable reference you receive when you call this function, so that the invariants on the Pin type can be upheld."**（译文：你必须保证，永远不会把数据移出你调用该函数时收到的那个可变引用，好让 `Pin` 的不变量得以维持。）
3. **`Pin::map_unchecked` / `map_unchecked_mut`**：把整个值的 pin 投影到字段上，要保证「父值不动，字段也不动」。这正是 22.7 的话题。
4. **`Pin::into_inner_unchecked`**：拆回指针之后，仍要把它当作 pinned 的一直对待到析构结束。
5. **手写 `Drop`（实现本身是 safe 语法，但写法要按 pin 处理）**：为地址敏感类型实现 `Drop` 时，要把它当成接收 `self: Pin<&mut Self>`（官方推荐的写法是让 `drop` 转调一个 `fn inner_drop(this: Pin<&mut Self>)`，`unsafe` 只出现在其中 `Pin::new_unchecked` 那一步）。另外 `#[repr(packed)]` 的类型与 pin 不兼容——编译器为了析构可能会移动字段。

再把本章的主线重复一遍：**不使用 `unsafe` 时，你在 pin 的世界里不可能写出不健全的代码**。需要 `unsafe` 的地方，恰好就是「你在替别人兑现承诺」的地方。

### 22.7 pin 投影与结构化 pinning（预告）

一个自引用结构体往往还要往外暴露字段。签名写起来就有分歧：`fn field(self: Pin<&mut Self>)` 应该返回 `&mut Field` 还是 `Pin<&mut Field>`？标准库的答案是「作者自己定」，但必须前后一致，而且这是 unsafe 代码正确性的一部分：

- 决定**不**把 pin 传给字段：字段当作普通字段，可以返回 `&mut Field`（内部还是 `unsafe { &mut self.get_unchecked_mut().field }`），代价是以后任何地方都不许再假设这个字段是 pinned 的；
- 决定**传给**字段（这叫**结构化 pinning（structural pinning）**，即 pin 跟着类型结构往下传）：用 `self.map_unchecked_mut(|s| &mut s.field)` 返回 `Pin<&mut Field>`，则「`Struct: Unpin` 只在所有结构化 pin 的字段都 `Unpin` 时才成立」这条约束必须由你守住——写一个 `impl<T> Unpin for Struct<T> {}` 再暴露 `Pin<&mut T>` 投影，就是 UB。

完整规则、以及基于它手写 `Pin` 投影（含 `pin-project` 那类库解决的问题）放在第 38 章；手写集合时的组合细节在第 34 章。现在只要记住：**投影是 `unsafe` 的入口之一，不是随手写的 getter**。

## 动手看看

把前面的碎片拼成一个完整程序：一个自引用类型，栈上就地 pin 一份、堆上 pin 一份。

```rust
{{#rustdoc_include ../snippets/ch22/selfref.rs}}
```

在仓库根编译运行（Windows 写法；其他平台把 `selfref.exe` 换成 `selfref`、运行 `./selfref`）：

```console
$ rustc --edition 2024 snippets/ch22/selfref.rs -o selfref.exe
$ .\selfref.exe
```

```text
pin!     stack -> true
Box::pin heap -> true
```

两个 `true` 就是这一章的成果：`ptr` 记下的地址与 `data` 当前地址一致。两条路径的差别也体现在代码里——`pin!` 那条把 `SelfRef` 留在 `main` 的栈帧里，`pinned` 借用了它；`spawn` 那条把值放到堆上，返回的 `Pin<Box<SelfRef>>` 可以传给别人、塞进集合，堆上的值始终没动。两次 `assert!` 会在任何路径出错时立刻炸掉，所以这不是「看起来对」的演示。

## 报错现场

在 pin 上撞到的错误信息质量普遍很高，`note` 里直接给了修法。三条最常见，行号列号按「最小复现放进 `src/main.rs`」给出，编译器 1.98。三处输出都照真实栏位宽度摘录（行号宽度会带动整块缩进），并统一略去末尾的 `error: aborting…` 与 `For more information…`；块内的 `...` 是省略标记。

### 现场一：`Pin::new` 撞上 `!Unpin`（E0277）

```rust,compile_fail
use std::marker::PhantomPinned;
use std::pin::Pin;

struct Task {
    name: String,
    _pin: PhantomPinned,
}

fn main() {
    let mut task = Task { name: String::from("job-1"), _pin: PhantomPinned };
    let pinned = Pin::new(&mut task);
    let _ = pinned;
}
```

关键几行（工具链路径前缀压成 `…/`；栏位宽度照真实输出，含 4 位行号 `1159`）：

```text
error[E0277]: `PhantomPinned` cannot be unpinned
    --> src/main.rs:11:27
     |
  11 |     let pinned = Pin::new(&mut task);
     |                  -------- ^^^^^^^^^ within `Task`, the trait `Unpin` is not implemented for `PhantomPinned`
     |                  |
     |                  required by a bound introduced by this call
     |
     = note: consider using the `pin!` macro
             consider using `Box::pin` if you need to access the pinned value outside of the current scope
note: required because it appears within the type `Task`
    --> src/main.rs:4:8
     |
   4 | struct Task {
     |        ^^^^
note: required by a bound in `Pin::<Ptr>::new`
    --> …/library/core/src/pin.rs:1159:25
     |
1159 | impl<Ptr: Deref<Target: Unpin>> Pin<Ptr> {
     |                         ^^^^^ required by this bound in `Pin::<Ptr>::new`
...
1181 |     pub const fn new(pointer: Ptr) -> Pin<Ptr> {
     |                  --- required by a bound in this associated function
```

读法：主 span 落在 `&mut task` 上，`within Task` 指出 `Unpin` 缺在哪个字段，最后一行直接给出约束的出处 `impl<Ptr: Deref<Target: Unpin>> Pin<Ptr>`。修法就写在 `note` 里——改用 `pin!(task)`（值留在当前作用域）或 `Box::pin(task)`（值放到堆上）。只有在类型本来就 `Unpin` 时才该用 `Pin::new`。

### 现场二：`!Unpin` 的值拿不到 `&mut`（E0596）

```rust,compile_fail
use std::marker::PhantomPinned;

struct Task {
    name: String,
    _pin: PhantomPinned,
}

fn main() {
    let task = Box::pin(Task { name: String::from("job-1"), _pin: PhantomPinned });
    println!("{}", task.name);
    task.name.push('!');
}
```

```text
error[E0596]: cannot borrow data in dereference of `Pin<Box<Task>>` as mutable
  --> src/main.rs:11:5
   |
11 |     task.name.push('!');
   |     ^^^^^^^^^ cannot borrow as mutable
   |
   = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `Pin<Box<Task>>`
```

注意第 10 行的 `println!("{}", task.name)` 是**合法**的：共享访问不受限制，`Pin<Box<Task>>` 照样能 `Deref` 成 `&Task`。限制只发生在可变访问上。修法是正视需求：如果你的类型真的地址敏感，就不能走这条路——要么把修改包进一个接收 `Pin<&mut Self>` 的方法里（内部用 `get_unchecked_mut`，像 `init` 那样），要么重新考虑这个类型是否真的需要 pin。

### 现场三：`pin!` 的值带不出作用域（E0716）

```rust,compile_fail
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::pin::pin;

struct Task {
    name: String,
    _pin: PhantomPinned,
}

fn main() {
    let escaped: Pin<&mut Task> = {
        let pinned = pin!(Task { name: String::from("job-1"), _pin: PhantomPinned });
        pinned
    };
    let _ = escaped;
}
```

```text
error[E0716]: temporary value dropped while borrowed
  --> src/main.rs:12:22
   |
11 |     let escaped: Pin<&mut Task> = {
   |         ------- borrow later stored here
12 |         let pinned = pin!(Task { name: String::from("job-1"), _pin: PhantomPinned });
   |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ creates a temporary value which is freed while still in use
13 |         pinned
14 |     };
   |     - temporary value is freed at the end of this statement
   |
   = note: consider using a `let` binding to create a longer lived value
```

`pin!` pin 住的是**当前作用域里的那块存储**，块一结束值就没了，`Pin<&mut Task>` 立刻变成悬垂，借用检查器如实拦下。这不是缺陷而是设计：`pin!` 的定位就是「临时用一下」。要把 pinned 值传出去，用 `Box::pin`。

## 小结

- 「移动」在语义上是所有权的移动，在机器上是逐字节复制，编译器**不会通知**被移动的值；地址稳定从来不是 Rust 的默认保证。
- `Pin<Ptr>` 不 pin `Ptr`，它 pin 的是 `Ptr` 指向的值；限制的松紧由 **pointee 是否 `Unpin`** 决定，与指针类型无关。
- pin 是一份**库契约**，不是编译器魔法：safe 代码无法欺骗它（`Pin::new`、`get_mut`、`DerefMut` 都要 `Unpin`），`unsafe` 代码则必须替它兑现承诺。
- `Unpin` 是 auto trait，语义是「不依赖任何 pin 保证」，几乎对所有类型自动成立；一个 `PhantomPinned` 字段就能关掉这个推导。
- 三个 safe 入口分工明确：`Pin::new` 只服务 `Unpin` 类型，`pin!` 就地 pin 且不能带出作用域，`Box::pin` 可以跨作用域但有一次分配。
- `unsafe` 只在固定几处出现（`new_unchecked`、`get_unchecked_mut`、`map_unchecked(_mut)`、`into_inner_unchecked`、手写 `Drop`），每一处要保证的都不是「别写错」，而是「这个值从此不移动」。

## 练习

### 练习 1：为什么 `Box<T>` 总是 `Unpin`

`Box<PhantomPinned>` 是 `Unpin` 的，但 `Pin<Box<PhantomPinned>>` 依然不放行可变访问。请解释这两件事为什么同时成立，并说明 `Pin<Box<T>>` 的「松紧」到底由谁决定。

<details><summary>参考答案</summary>

`Pin<Ptr>` 的不变量针对的是 `<Ptr as Deref>::Target`，也就是 pointee。`Box<T>` 本身只是一小段指针，把它搬来搬去不影响它指向的堆上数据，所以 `Box<T>` 无条件 `Unpin`（`impl<T, A> Unpin for Box<T, A> where A: Allocator, T: ?Sized`）。

而 `Pin<Box<T>>` 的 `Deref`/`DerefMut` 针对的是 `Box<T>` 解引用出来的 `T`，所以 `DerefMut` 需要 `T: Unpin`。当 `T = PhantomPinned` 时，指针能移动、被指向的值不能动——这两条并不矛盾，恰好是 pin 想要的：**「搬运凭证」和「地址承诺」是两件事**。可以验证：`Box<PhantomPinned>` 通过 `assert_unpin`，而 `Box::pin(PhantomPinned)` 上写 `*p = PhantomPinned` 报 E0594（cannot assign to data in dereference），调用需要 `&mut` 的方法则报 E0596（现场二那种）。
</details>

### 练习 2：找出这段代码的三个问题

```rust,compile_fail
use std::marker::PhantomPinned;
use std::pin::Pin;

struct Buffer {
    data: Vec<u8>,
    ptr: *const u8,
    _pin: PhantomPinned,
}

impl Buffer {
    fn new(data: Vec<u8>) -> Self {
        let mut b = Buffer { data, ptr: std::ptr::null(), _pin: PhantomPinned };
        b.ptr = b.data.as_ptr(); // ①
        b
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

fn main() {
    let mut b = Buffer::new(vec![1, 2, 3]);
    let mut pinned = Pin::new(&mut b); // ②
    println!("{}", pinned.len());
    let moved = *pinned; // ③
    println!("{}", moved.len());
}
```

<details><summary>参考答案</summary>

**① 构造函数里记录自引用指针，然后按值返回。** 返回本身就是一次移动，`data` 到新地址后 `ptr` 已经失效；而且 `Vec::as_ptr` 记下的是堆缓冲区地址，`data` 这个 `Vec` 头一旦被搬到别处，指针与「哪个 `Vec`」的对应关系也会错乱。正确做法是像本章的 `init` 那一步：**地址确定之后再设置指针**，并让这个操作只接受 `Pin<&mut Self>`。

**② `Pin::new` 要求 `T: Unpin`，而 `Buffer` 因为 `_pin: PhantomPinned` 是 `!Unpin`。** 这行会报 E0277（现场一）。需要改成 `pin!(b)` 或 `Box::pin(b)`。

**③ 不能把 `Pin<&mut Buffer>` 解引用后按值取出 `Buffer`。** 需要 `&mut Buffer` 才谈得上移出，而 `Buffer: !Unpin` 拿不到它；即使拿到了，移出被 pin 的值也直接破坏不变量。把 ② 改成 `pin!(b)` 的话，这里会因为借用检查（移动已被借出的 `b`）先报错——**这正说明 safe 代码里的 pin 是拦得住人的**。

（顺带：`Buffer::new` 改成 `Pin<&mut Self>` 之后，`ptr` 指向的是 `Vec` 的堆缓冲区，而 `Vec` 一旦重新分配，`ptr` 又会失效。真实的自引用集合要把这一点也算进不变量；第 34 章会完整实现一个这样的集合。）
</details>

### 练习 3：`pin!` 与 `Box::pin` 怎么选

下面两种写法都是对的。请说出各自的判断依据。

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

fn poll_once_local() -> u32 {
    let fut = async { 42u32 };
    let mut fut = std::pin::pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    match Future::poll(fut.as_mut(), &mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => 0,
    }
}

fn boxed() -> Pin<Box<dyn Future<Output = u32>>> {
    Box::pin(async { 42u32 })
}

fn main() {
    assert_eq!(poll_once_local(), 42);

    let mut fut = boxed();
    let mut cx = Context::from_waker(Waker::noop());
    let v = match Future::poll(fut.as_mut(), &mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => 0,
    };
    assert_eq!(v, 42);
}
```

<details><summary>参考答案</summary>

判据是**值需要在哪些作用域里存活**，以及**是否需要在类型上隐藏具体类型**。

- 第一种：future 只在 `poll_once_local` 内部被 poll，出栈即弃。`pin!` 就地 pin、零分配，正合适；代价是这个 `Pin<&mut _>` 不能返回（现场三就是这个限制）。它也不依赖分配器，`no_std` 可用。
- 第二种：调用者要拿到 future 的所有权，而且要统一类型（`dyn Future`）。值必须放在一个「不会跟着指针搬家」的地方，堆是标准答案；`Box::pin` 一步完成「装箱 + pin」，返回的 `Pin<Box<dyn Future<...>>>` 自己实现了 `Future`（第 14 章遇到过这个形状）。

一句话版本：**`pin!` 管「就地用完」，`Box::pin` 管「交出去」。** 具体类型能写出来、生命周期够用时优先 `pin!`；需要 trait 对象或跨作用域时用 `Box::pin`。
</details>

## 延伸阅读

- `std::pin` 模块文档（pin 的权威说明，含 Drop 保证与投影规则）：<https://doc.rust-lang.org/std/pin/index.html>
- `Pin` 类型的方法与安全条款：<https://doc.rust-lang.org/std/pin/struct.Pin.html>
- `Unpin`：<https://doc.rust-lang.org/std/marker/trait.Unpin.html>
- `PhantomPinned`：<https://doc.rust-lang.org/std/marker/struct.PhantomPinned.html>
- `pin!`：<https://doc.rust-lang.org/std/pin/macro.pin.html>
- `Box::pin`：<https://doc.rust-lang.org/std/boxed/struct.Box.html#method.pin>
- The Book 第 17.5 节（`Future`、`Pin`、`Unpin` 的入门视角）：<https://doc.rust-lang.org/book/ch17-05-traits-for-async.html>
