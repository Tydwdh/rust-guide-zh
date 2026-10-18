# 第 13 章 dyn 兼容性与 trait 对象

## 这一章解决什么问题

第 9 章留了一个尾巴。那里对比过两条多态路线：`fn show<T: Display>(v: T)` 让编译器为每个用到的类型各生成一份代码（单态化），`Box<dyn Display>` 只有一份代码、每次调用经一张表跳一次。当时只说了「这是个真取舍，第 13 章展开」。

现在展开。真正逼你走第二条路的，通常不是性能，而是**类型在编译期列不出来**：

- 一个 `Vec` 里要同时装圆、正方形、文本标注，它们之间没有任何关系，只是「都能算面积」；
- 一张 `HashMap<String, Box<dyn Handler>>` 注册表，装什么由运行时的配置决定；
- 一个函数要能返回「某种错误」——文件系统的 `io::Error`、解析用的 `ParseIntError`、你自己写的错误都行；
- 库的公开 API 不想让类型参数传染下去：调用方传进来的 `Logger<FileSink<BufWriter<TcpStream>>>` 不该出现在你的函数签名里。

这些场景的共同点是：**你要的是「行为」，不是「类型」**。`dyn Trait` 就是把具体类型擦掉、只留下行为契约和一张分发表的东西，这种类型叫 **trait 对象（trait object）**。

读完这一章，你能：

- 判断一个 trait 能不能变成 `dyn Trait`（**dyn 兼容性（dyn compatibility）**规则），不能时知道两种修法；
- 说清 `&dyn`、`Box<dyn>`、`Arc<dyn>` 各自适合什么场景，以及它们的共同内存形状；
- 说清 vtable 是什么，动态分发的代价具体落在哪条机器指令上；
- 用向上转型（upcasting）在父 trait（supertrait）之间移动 trait 对象；
- 在两个惯用例上直接用起来：`Box<dyn Error>` 与 `&mut dyn Iterator`；
- 用一份清单在「泛型 / `impl Trait` / `dyn`」之间做选择。

> [!NOTE]
> 这个概念的旧名字是「对象安全（object safety）」。Rust Reference 的现行文本叫 dyn 兼容性，并在注里写明「This concept was formerly known as *object safety*.」；1.98 的编译器报错也用新名字。本书统一用 dyn 兼容性。

## 为什么这样设计

### 两条多态路线的账

第 9 章已经量过泛型那条路：三个调用点，机器码里三份 `show`，各自直接调用。`dyn` 那条路只有一份函数体，但它拿到的指针是「两只手」：

<svg viewBox="0 0 760 224" width="100%" role="img" aria-label="泛型与 dyn 两条代码路径对照：单态化按类型生成 show 的三份实例、调用点直接跳转且可内联；dyn 只有一份 show_dyn，收到 data 与 vtable 两个指针，每次调用经 vtable 取槽后间接跳转，挡住内联" xmlns="http://www.w3.org/2000/svg">
<line x1="368" y1="28" x2="368" y2="216" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="0" y="16" font-size="14" fill="currentColor">泛型（单态化）：每个类型一份代码</text>
<text x="384" y="16" font-size="14" fill="currentColor">dyn（动态分发）：只有一份代码</text>
<g fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5">
<rect x="0" y="40" width="210" height="30" rx="5" />
<rect x="0" y="93" width="210" height="30" rx="5" />
<rect x="0" y="146" width="210" height="30" rx="5" />
</g>
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="10" y="60">show::&lt;u32&gt;</text>
<text x="10" y="113">show::&lt;&amp;str&gt;</text>
<text x="10" y="166">show::&lt;f64&gt;</text>
</g>
<path d="M220,40 h6 v136 h-6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.55" />
<text x="234" y="102" font-size="12" fill="var(--links)">3 份机器码</text>
<text x="234" y="120" font-size="12" fill="currentColor" opacity="0.7">每个类型一份</text>
<rect x="384" y="40" width="210" height="30" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="394" y="60" font-size="12" font-family="monospace" fill="currentColor">show_dyn(&amp;dyn Display)</text>
<line x1="489" y1="70" x2="489" y2="80" stroke="currentColor" stroke-width="1.5" />
<polygon points="489,88 484,78 494,78" fill="currentColor" />
<rect x="384" y="88" width="210" height="40" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<line x1="384" y1="108" x2="594" y2="108" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="394" y="104">data 指针 → 实际值</text>
<text x="394" y="124">vtable 指针 → 表</text>
</g>
<line x1="489" y1="128" x2="489" y2="138" stroke="currentColor" stroke-width="1.5" />
<polygon points="489,146 484,136 494,136" fill="currentColor" />
<rect x="384" y="146" width="210" height="30" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="394" y="166" font-size="12" font-family="monospace" fill="currentColor">取槽 → 实际实现</text>
<text x="602" y="166" font-size="12" fill="currentColor" opacity="0.7">间接调用</text>
<text x="0" y="196" font-size="12" fill="var(--links)">✓ 直接调用：跳转目标编译期定死，可内联</text>
<text x="0" y="212" font-size="12" fill="currentColor" opacity="0.7">代价：代码体积与编译时间随用到的类型数增长</text>
<text x="384" y="196" font-size="12" fill="var(--warning-border)">✗ vtable 取址 + 间接跳转，挡住内联</text>
<text x="384" y="212" font-size="12" fill="currentColor" opacity="0.7">代价：每次调用多一次取址</text>
</svg>

两条路的账不一样：泛型把「哪个类型」在编译期填死，代价落到代码体积与编译时间上；`dyn` 把「哪个类型」留到运行时，代价落到每次调用上。没有零代价的第三条路。

### 为什么必须有一套规则

动态调用要求编译期就把调用点的形状固定下来。trait 对象上的调用在编译期只能写成「取 vtable 的第 N 个槽，调用它」——N 是编译期确定的槽位号。凡是塞不进这种固定槽位的东西，都不能出现在能变 `dyn` 的 trait 上：

- **泛型方法**：`fn put<T>(&self, v: T)` 对每组类型实参都是一份新签名，槽位号没法固定。
- **返回 `Self`**：调用点不知道 `Self` 有多大，连返回值的栈空间都开不出来。
- **关联常量**：vtable 里没有常量这种东西的位置。
- **没有接收者的关联函数**：调用点手上只有一个数据指针，凭什么找到实现？
- **返回 `impl Trait` / `async fn`**：返回类型是隐藏的，同样没法固定返回值形状。

一句话：**dyn 兼容的意思，是「这个 trait 的全部可调用方法，都能靠一根数据指针加一张固定槽位的表表达」**。

这也解释了本章的核心手法。给方法加 `where Self: Sized`，不是在「绕过规则」，而是明确告诉编译器：这个方法不参与动态分发，请把它从 vtable 里剔出去。于是整个 trait 重新变成 dyn 兼容，而那些方法仍能对具体类型使用。标准库的 `Iterator` 就是这么设计的：`next` 在表里，`map`、`filter` 这些要求 `Self: Sized` 的适配器不在表里。

### 语言对照角

**Java：接口调用天生是「动态」的。** Java 的所有引用类型都带运行期类型信息，任何实现接口的引用都能直接当接口用，语言层面不需要「dyn 兼容性」这类规则。JVM 规范里，接口方法调用是一条专门的指令：

> *invokeinterface* invokes an interface method, searching the methods implemented by the particular run-time object to find the appropriate method.
>
> —— *The Java Virtual Machine Specification* §2.11.8

Rust 的差别在于：Rust 有单态化，泛型与 `dyn` 是两条真实的、代价不同的路线；Java 泛型会被擦除，接口调用一律走动态分派，所以「选泛型还是选 dyn」这个问题在 Java 里基本不存在。

**C++：`virtual` 声明 + 继承层次。** C++ 的虚函数必须在类里用 `virtual` 首次声明（或覆盖基类的虚函数），标准草案 `[class.virtual]` 一节的定义是：

> A non-static member function is a virtual function if it is first declared with the keyword virtual or if it overrides a virtual member function declared in a base class (see below).

调用语义也写在标准里（同节注 3，引文末尾的交叉引用标记略去）：

> The interpretation of the call of a virtual function depends on the type of the object for which it is called (the dynamic type), whereas the interpretation of a call of a non-virtual member function depends only on the type of the pointer or reference denoting that object (the static type).

Rust 的 `dyn` 有三处不同：trait 实现写在类型声明之外，类型不必属于某条继承链；trait 对象比普通指针多带一个 vtable 指针，但**被指向的对象本体不会因此变大**（下面有实测）；能不能向上转型由父 trait 关系决定，而不是由继承树决定。

## 机制与规则

### dyn 兼容性规则

Reference 一句话概括：能当 trait 对象基 trait 的，才是 dyn 兼容的。逐条展开是这样：

| 规则 | 反例 | 1.98 报错里的关键说法 |
| --- | --- | --- |
| 所有父 trait 也必须 dyn 兼容 | `trait B: A`，而 `A` 有泛型方法 | 诊断会指到父 trait 里出问题的那一行 |
| 不能要求 `Self: Sized`（不能以 `Sized` 为父 trait） | `trait NeedsSized: Sized` | ``...because it requires `Self: Sized` `` |
| 不能有关联常量 | `const VERSION: u32` | ``...because it contains associated const `VERSION` `` |
| 不能带泛型参数的关联类型（GAT） | `type Item<'a> where Self: 'a;` | ``...because it contains generic associated type `Item` `` |
| 关联函数要么可分发，要么显式 `where Self: Sized` | `fn put<T>(&self, v: T)` / `fn default_name() -> String` | ``...because method `put` has generic type parameters`` / ``...because associated function `default_name` has no `self` parameter`` |
| 可分发方法不能有泛型参数、不能用 `Self` 做接收者以外的类型 | `fn duplicate(&self) -> Self` | ``...because method `duplicate` references the `Self` type in its return type`` |
| 可分发方法不能有隐藏的返回类型 | `async fn fetch(&self)` / `fn next(&mut self) -> impl Iterator<Item = u32>` | `` ...because method `fetch` is `async` `` / ``...references an `impl Trait` type in its return type`` |

「可分发」的接收者只有这几种（Reference 的清单）：`&Self`、`&mut Self`、`Box<Self>`、`Rc<Self>`、`Arc<Self>`，以及 `Pin<P>`（`P` 是前面几种之一）。方法可以带生命周期参数，只是不能带类型参数。

```rust
use std::sync::Arc;

trait Consume {
    fn label(&self) -> String;
    fn into_label(self: Box<Self>) -> String; // 接收者是 Box<Self>，可分发
    fn shared_label(self: Arc<Self>) -> String; // Arc<Self> 也行
}

struct Item(&'static str);

impl Consume for Item {
    fn label(&self) -> String {
        self.0.to_string()
    }
    fn into_label(self: Box<Self>) -> String {
        format!("box:{}", self.0)
    }
    fn shared_label(self: Arc<Self>) -> String {
        format!("arc:{}", self.0)
    }
}

fn main() {
    let a: Box<dyn Consume> = Box::new(Item("a"));
    let b: Arc<dyn Consume> = Arc::new(Item("b"));
    println!("{} {} {}", a.label(), a.into_label(), b.shared_label());
}
```

（`Pin` 那一档是异步里的常客，第 21、22 章再碰。）

还有一条容易误会的：`dyn Trait` 自己**已经**实现了 `Trait` 和它的全部父 trait，这是编译器内建的。Reference 的原话是「Trait objects implement the base trait, its auto traits, and any supertraits of the base trait.」你不用、也不能手写这个实现——写 `impl Shape for dyn Shape` 会得到 E0371：``the object type `(dyn Shape + 'static)` automatically implements the trait `Shape` ``。

### 关联类型要写实

trait 有非泛型的关联类型不影响 dyn 兼容，但**在写 `dyn` 时必须把值填上**：

```rust
fn total(it: &mut dyn Iterator<Item = u32>) -> u32 {
    it.sum()
}

fn main() {
    println!("{}", total(&mut (1..=4)));
}
```

只写 `&mut dyn Iterator` 会得到 E0191「must be specified」（完整报错见「报错现场」）。理由和泛型方法相反：关联类型是**类型的一部分**，不同的 `Item` 是不同的 trait 对象类型，编译器必须知道是哪一种。带泛型的关联类型（GAT）则更进一步，直接让 trait 失去 dyn 兼容性（第 15 章展开）。

### `where Self: Sized`：把方法请出 vtable

标准库的 `Iterator` 是最好的老师。它的必需方法 `fn next(&mut self) -> Option<Self::Item>` 可分发；而 `map`、`filter`、`sum` 这些适配器的声明都带着 `where Self: Sized`（本机 1.98 的 core 源码里，`fn map<B, F>(self, f: F) -> Map<Self, F>` 下面的 `where` 子句第一行就是 `Self: Sized`）。所以：

```rust
fn sum_all(items: &mut dyn Iterator<Item = u32>) -> u32 {
    items.sum() // 接收者是 &mut dyn Iterator，它自己是 Sized —— 可以！
}
```

注意这里的细节：`items` 的类型是 `&mut dyn Iterator<Item = u32>`，而 `&mut I` 本身实现了 `Iterator`（core 源码里就是 `impl<I: Iterator + ?Sized> Iterator for &mut I`），并且 `&mut I` 是 Sized 的。所以 `Self: Sized` 限制落在 `&mut I` 这一层，方法照样能调。真正被挡住的是直接以 trait 对象为接收者的情况：

```rust,compile_fail
fn mapped() -> Vec<u32> {
    let it: Box<dyn Iterator<Item = u32>> = Box::new(1..=3);
    let _ = (*it).map(|x| x * 2); // *it 的类型是 dyn Iterator，不是 Sized
    Vec::new()
}

fn main() {
    println!("{:?}", mapped());
}
```

这个错误没有错误码，但信息很直白（完整输出见「报错现场」第三例）：`the `map` method cannot be invoked on a trait object`。

同样的道理让 `Box<dyn Iterator<Item = u32>>` 上可以直接写 `.map(...)`：标准库也为 `Box<I>`（`I: Iterator + ?Sized`）实现了 `Iterator`（源码里的完整签名是 `impl<I: Iterator + ?Sized, A: Allocator> Iterator for Box<I, A>`，`A` 是分配器参数，日常写 `Box<I>` 用的就是它的默认值），而 `Box` 这个外壳是 Sized 的。**「dyn 不满足 `Self: Sized`」说的是 trait 对象本身，不包括指向它的那层指针。**

### trait 对象的边界怎么拼

一个 trait 对象的类型由三部分拼成（Reference）：恰好一个 dyn 兼容的基 trait、任意多个 auto trait、至多一个生命周期。所以下面这些都合法：

```text
dyn Trait
dyn Trait + Send
dyn Trait + Send + Sync
dyn Trait + 'static
dyn Trait + Send + 'static
```

不允许的：两个非 auto trait（`dyn A + B` 不行——想要就得定义一个同时是两者父 trait 的 trait）、`?Sized` 这类 opt-out 边界、`use<..>`。auto trait 的顺序不重要，`dyn Trait + Send + UnwindSafe` 与 `dyn Trait + UnwindSafe + Send` 是同一个类型。

生命周期有个好用的默认规则（Reference 的「default trait object lifetimes」）：**类型位置**上，没有生命周期边界的 trait 对象默认是 `'static`，所以 `Box<dyn Foo>` 与 `Box<dyn Foo + 'static>` 是同一个类型；而在 `&'a T` 这种要求 `T: 'a` 的位置，`&'a dyn Foo` 等价于 `&'a (dyn Foo + 'a)`。写函数签名时通常不用手写；但如果你要把 trait 对象存进结构体、或对它做 `downcast_ref`（要求 `T: 'static`），这层默认就会浮出水面。

`dyn Trait + Send + Sync` 是跨线程共享的通行证：`Send`/`Sync` 是 auto trait，能不能加上由具体实现类型决定，细节留给第 19 章。

> [!WARNING]
> 想让 `dyn Trait` 支持 `async fn`，需要 `async_fn_in_dyn_trait` 这个特性门；以 Rust 1.98 为准它**仍不稳定**。含 `async fn` 的 trait 在 stable 上不是 dyn 兼容的（1.98 实测报 E0038，`...because method `fetch` is `async`）。stable 上需要动态分发的异步接口，做法是让方法返回 `Pin<Box<dyn Future<Output = T> + Send + '_>>` 这类显式类型（第 21、22 章展开）。

### vtable 与三种持有形式

trait 对象是**动态大小类型（DST）**：编译器不知道被擦掉的类型有多大，所以它永远得待在某种指针后面。这种指针是**胖指针**——两个机器字，一个指向数据，一个指向 vtable：

<svg viewBox="0 0 700 262" width="100%" role="img" aria-label="胖指针与 vtable 布局：&amp;dyn Shape 的两个机器字分别指向具体值和该类型的 vtable，Circle 与 Square 各有一张表，表头是元数据、后面每个可分发方法一格" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">胖指针 + vtable：两个机器字各司其职，表归类型所有</text>
<text x="0" y="38" font-size="12" font-family="monospace" fill="var(--links)">&amp;dyn Shape（16 字节：两个 8 字节机器字）</text>
<rect x="0" y="44" width="130" height="60" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<line x1="0" y1="74" x2="130" y2="74" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="10" y="66">data</text>
<text x="10" y="96">vtable</text>
</g>
<line x1="130" y1="62" x2="176" y2="62" stroke="currentColor" stroke-width="1.5" />
<polygon points="186,62 176,57 176,67" fill="currentColor" />
<rect x="188" y="48" width="140" height="28" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="258" y="67" font-size="12" font-family="monospace" text-anchor="middle" fill="currentColor">Circle { x, y }</text>
<line x1="130" y1="92" x2="340" y2="92" stroke="currentColor" stroke-width="1.5" />
<polygon points="350,92 340,87 340,97" fill="currentColor" />
<rect x="352" y="40" width="348" height="76" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="364" y="60" font-size="12" font-family="monospace" fill="var(--links)">头部元数据：size / align / drop（1.98 实测 24 字节）</text>
<line x1="352" y1="70" x2="700" y2="70" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="364" y="88">area → Circle::area</text>
<text x="364" y="106">name → Circle::name</text>
</g>
<text x="0" y="150" font-size="12" fill="currentColor" opacity="0.75">换个具体类型：指针形状一个字节不变，换的是指向哪张表</text>
<rect x="0" y="156" width="130" height="60" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<line x1="0" y1="186" x2="130" y2="186" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="10" y="178">data</text>
<text x="10" y="208">vtable</text>
</g>
<line x1="130" y1="174" x2="176" y2="174" stroke="currentColor" stroke-width="1.5" />
<polygon points="186,174 176,169 176,179" fill="currentColor" />
<rect x="188" y="160" width="140" height="28" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="258" y="179" font-size="12" font-family="monospace" text-anchor="middle" fill="currentColor">Square { side }</text>
<line x1="130" y1="204" x2="340" y2="204" stroke="currentColor" stroke-width="1.5" />
<polygon points="350,204 340,199 340,209" fill="currentColor" />
<rect x="352" y="152" width="348" height="76" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="364" y="172" font-size="12" font-family="monospace" fill="var(--links)">头部元数据：size / align / drop</text>
<line x1="352" y1="182" x2="700" y2="182" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="364" y="200">area → Square::area</text>
<text x="364" y="218">name → Square::name</text>
</g>
<text x="0" y="252" font-size="12" fill="currentColor" opacity="0.7">表归类型所有：同一个类型的所有实例共用一张；表里一格对应一个可分发方法</text>
</svg>

vtable 里除了每个可分发方法的函数指针，还放着运行时需要的元数据——`size_of_val` 能从 trait 对象上读出对象大小，靠的就是它（不依赖具体类型是不可行的）。头部里各项的排布是编译器实现细节，不是稳定 ABI；`unsafe` 视角的布局在第 25 章，本章只需记住「每个方法占一格」。

选哪种持有形式，取决于谁拥有这个值：

| 形式 | 所有权 | 典型场景 |
| --- | --- | --- |
| `&dyn Trait` / `&mut dyn Trait` | 借来的 | 函数参数：只借一次实现用用，不负责生命周期 |
| `Box<dyn Trait>` | 你独占 | 异构集合、工厂返回值、从注册表里取出一个实现 |
| `Arc<dyn Trait>` | 共享 | 跨线程共享的回调 / 插件（需要 `dyn Trait + Send + Sync`，第 19 章） |
| `Rc<dyn Trait>` | 共享（单线程） | 观察者、UI 树；需要 `dyn Trait`，但不是 `Send` |
| `Pin<Box<dyn Future<…>>>` | 你独占 | 保存一个异步任务（第 21、22 章） |

`Box<dyn Trait>` 自己满足 `Sized`，所以能进 `Vec`、能当结构体字段；`&dyn Trait` 不用分配，最便宜，但生命周期受借用约束。三种形式共用同一套 vtable 机制，转换只改「谁拥有」这一层。

### 动态分发的代价落在哪条指令上

光看文字容易滑过去，直接看编译产物。下面这个程序对同一件事写了两遍：一份收 `&dyn Op`，一份收泛型 `&O`：

```rust
{{#rustdoc_include ../snippets/ch13/dispatch.rs}}
```

用 `--emit=llvm-ir` 输出 LLVM IR，再挑出两个函数：

```console
$ rustc --edition 2024 -C opt-level=0 --emit=llvm-ir dispatch.rs
$ grep -n 'vtable\.1 =\|run_dyn\|run_generic' dispatch.ll
10:@vtable.1 = private unnamed_addr constant <{ [24 x i8], ptr }> <{ [24 x i8] c"\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\00\00\00\00", ptr @_RNvXCsghNZpGUrMD6_8dispatchNtB2_6AddOneNtB2_2Op5apply }>, align 8
19:define internal i32 @_RINvCsghNZpGUrMD6_8dispatch11run_genericNtB2_6AddOneEB2_(ptr %op, i32 %x) unnamed_addr #0 {
28:define internal i32 @_RINvCsghNZpGUrMD6_8dispatch11run_genericNtB2_8TimesTwoEB2_(ptr %op, i32 %x) unnamed_addr #0 {
162:define internal i32 @_RNvCsghNZpGUrMD6_8dispatch7run_dyn(ptr %op.0, ptr align 8 %op.1, i32 %x) unnamed_addr #0 {
```

先看两处调用点，它们的实参列表差别就说明了一切（同一份输出里的相邻两组，空行未录）：

```text
; call dispatch::run_dyn
  %0 = call i32 @_RNvCsghNZpGUrMD6_8dispatch7run_dyn(ptr %_1, ptr align 8 @vtable.1, i32 1) #9
; call dispatch::run_generic::<dispatch::AddOne>
  %1 = call i32 @_RINvCsghNZpGUrMD6_8dispatch11run_genericNtB2_6AddOneEB2_(ptr %_1, i32 1) #9
```

`run_dyn` 多收了一个 `@vtable.1`——胖指针的两个机器字被拆成两个实参传下去，而且 vtable 是**按具体类型各生成一份**的常量（`AddOne` 一份、`TimesTwo` 一份）。`run_generic` 只收一个指针，并且被单态化成两份，函数名里带着 `AddOne` / `TimesTwo`——正是第 9 章数过的那种「每个类型一份代码」。

再看两个函数体：

```text
; dispatch::run_dyn
define internal i32 @_RNvCsghNZpGUrMD6_8dispatch7run_dyn(ptr %op.0, ptr align 8 %op.1, i32 %x) unnamed_addr #0 {
start:
  %0 = getelementptr inbounds i8, ptr %op.1, i64 24
  %1 = load ptr, ptr %0, align 8, !invariant.load !5, !nonnull !5
  %_0 = call i32 %1(ptr %op.0, i32 %x) #8
  ret i32 %_0
}

; dispatch::run_generic::<dispatch::AddOne>
define internal i32 @_RINvCsghNZpGUrMD6_8dispatch11run_genericNtB2_6AddOneEB2_(ptr %op, i32 %x) unnamed_addr #0 {
start:
; call <dispatch::AddOne as dispatch::Op>::apply
  %_0 = call i32 @_RNvXCsghNZpGUrMD6_8dispatchNtB2_6AddOneNtB2_2Op5apply(ptr %op, i32 %x)
  ret i32 %_0
}
```

差别一眼可见。`run_dyn` 干三件事：把 vtable 指针偏移 24 字节（`i64 24`，即跳过头部、到第一个方法槽）、把槽里的函数地址 `load` 出来、通过这个地址**间接调用**（`call i32 %1`）。`run_generic` 直接点名 `AddOne::apply`，是**直接调用**——而且是 `internal` 符号，优化开启后可以内联进调用方，`run_dyn` 里那条间接调用则天然挡住内联（除非编译器能证明实际类型只有一种，做去虚化）。

这就是动态分发的全部代价：**一次额外取址 + 一次无法内联的间接跳转**。单次调用的成本很小，但它在热循环里会挡住所有跨调用的优化（内联、常量传播、向量化）。所以选型清单里那条老规矩成立：内层热点用泛型，边界上用 `dyn`。

### 向上转型（1.86 起）

当 `trait Pet: Animal` 时，`dyn Pet` 的 trait 对象可以在保持同一个数据指针的前提下，换成 `dyn Animal` 的视图——这就是向上转型。Reference 把它写成一条内建强转规则：

> `dyn T` to `dyn U`, when `U` is one of `T`'s supertraits.

下图把这次转型摊开：两处 `data` 指向同一个值，换掉的只是 vtable 视图。

<svg viewBox="0 0 700 258" width="100%" role="img" aria-label="向上转型的两步：dyn Pet 与 dyn Animal 共用同一个数据指针，只把 vtable 换成父 trait 的视图；横向转到无关 trait 不合法，报 E0605" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">向上转型：数据指针一个字节不动，只换 vtable 视图</text>
<text x="0" y="38" font-size="12" font-family="monospace" fill="var(--links)">&amp;dyn Pet</text>
<rect x="0" y="44" width="140" height="56" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<line x1="0" y1="72" x2="140" y2="72" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="10" y="64">data</text>
<text x="10" y="92">vtable</text>
</g>
<line x1="140" y1="60" x2="186" y2="60" stroke="var(--links)" stroke-width="1.5" />
<polygon points="196,60 186,55 186,65" fill="var(--links)" />
<rect x="200" y="46" width="110" height="28" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="255" y="65" font-size="12" font-family="monospace" text-anchor="middle" fill="currentColor">Dog</text>
<line x1="140" y1="88" x2="336" y2="88" stroke="currentColor" stroke-width="1.5" />
<polygon points="346,88 336,83 336,93" fill="currentColor" />
<rect x="348" y="42" width="352" height="64" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="360" y="60" font-size="12" font-family="monospace" fill="var(--links)">Pet 视图：name + owner</text>
<line x1="348" y1="68" x2="700" y2="68" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="360" y="84">name → Dog::name</text>
<text x="360" y="102">owner → Dog::owner</text>
</g>
<text x="142" y="118" font-size="12" fill="var(--links)">两处 data 指向同一个值：指针逐位不变</text>
<text x="0" y="146" font-size="12" font-family="monospace" fill="var(--links)">&amp;dyn Animal</text>
<rect x="0" y="152" width="140" height="56" rx="5" fill="none" stroke="currentColor" stroke-width="1.5" />
<line x1="0" y1="180" x2="140" y2="180" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="10" y="172">data</text>
<text x="10" y="200">vtable</text>
</g>
<line x1="140" y1="168" x2="186" y2="168" stroke="var(--links)" stroke-width="1.5" />
<polygon points="196,168 186,163 186,173" fill="var(--links)" />
<rect x="200" y="154" width="110" height="28" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="255" y="173" font-size="12" font-family="monospace" text-anchor="middle" fill="currentColor">Dog</text>
<line x1="140" y1="196" x2="336" y2="196" stroke="currentColor" stroke-width="1.5" />
<polygon points="346,196 336,191 336,201" fill="currentColor" />
<rect x="348" y="150" width="352" height="64" rx="5" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="360" y="168" font-size="12" font-family="monospace" fill="var(--links)">Animal 视图：只有 name</text>
<line x1="348" y1="176" x2="700" y2="176" stroke="currentColor" stroke-width="1.5" stroke-opacity="0.35" />
<text x="360" y="194" font-size="12" font-family="monospace" fill="currentColor">name → Dog::name</text>
<text x="360" y="212" font-size="12" fill="currentColor" opacity="0.7">owner 不在这个视图里</text>
<text x="0" y="234" font-size="12" fill="var(--links)">✓ 只沿父 trait 方向：dyn Pet → dyn Animal（1.86 起）</text>
<text x="0" y="252" font-size="12" fill="var(--warning-border)">✗ 横向不合法：&amp;dyn Shape as &amp;dyn Named → E0605</text>
</svg>

这条能力在 **Rust 1.86（2025-04-03）** 稳定；1.98 上引用、`Box`、`Arc`、`Rc` 形式全部可用：

```rust
trait Animal {
    fn name(&self) -> String {
        String::from("dog")
    }
}

trait Pet: Animal {
    fn owner(&self) -> String {
        String::from("me")
    }
}

struct Dog;
impl Animal for Dog {}
impl Pet for Dog {}

fn main() {
    let dog = Dog;
    let pet: &dyn Pet = &dog;
    let animal: &dyn Animal = pet; // dyn Pet -> dyn Animal，隐式强转
    println!("{} 由 {} 养", animal.name(), pet.owner());
}
```

`Box`、`Arc`、`&mut` 形式以及「顺手丢掉 auto trait」的写法，都在「动手看看」第四节完整跑过。

三条补充：

- **父 trait 的方法本来就能直接调用。** `pet.name()` 用的是 `Animal` 的方法，而 `pet` 是 `dyn Pet`：`dyn Pet` 的 vtable 里本来就包含父 trait 的方法。向上转型解决的不是「能不能调」，而是「能不能把它当 `dyn Animal` 传出去、存起来」。
- **可以顺手丢掉 auto trait**：`Box<dyn Pet + Send>` → `Box<dyn Animal>` 合法（规则原文里明确写了 "This allows dropping auto traits"）。反过来，只有当 auto trait 是基 trait 的父 trait 时才能加：给定 `trait T: U + Send {}`，`dyn T` → `dyn T + Send` 与 `dyn U + Send` 都允许。
- **只能沿父 trait 方向，不能横向**。两个互不相关的 trait 之间转型会被拒绝。1.98 的最小复现报 E0605：``non-primitive cast: `&dyn Shape` as `&dyn Named` ``，附带一句「an `as` expression can only be used to convert between primitive types or to coerce to a specific trait object」。

### 选型清单

| 你的处境 | 选 | 为什么 |
| --- | --- | --- |
| 类型在调用点已知，热点路径在意性能 | 泛型参数 `T: Trait` | 单态化、可内联，零运行时代价 |
| 只想在参数位置收「某个实现了 trait 的东西」，不需要给它命名 | `impl Trait`（参数位置） | 就是泛型的语法糖，写起来短（第 9 章） |
| 返回「某种实现了 trait 的类型」，调用方不需要、也不该写具体类型 | `impl Trait`（返回位置） | 隐藏具体类型，没有堆分配；生命周期捕获见第 14 章 |
| 异构集合 / 运行时决定实现 / 回调注册表 / 插件 | `Box<dyn Trait>`（或 `&dyn`、`Arc<dyn>`） | 类型擦除是这里唯一的选择 |
| 库的公开 API 想少一些类型参数传染、缩短编译时间 | 边界处用 `dyn` | 一份代码；代价是胖指针与间接调用 |
| 「任意错误」 | `Box<dyn Error>` | 标准库的 `From` 与 `?` 已经铺好路 |
| 函数签名里只借一次序列 | `&mut dyn Iterator<Item = T>` | 调用方不必暴露具体迭代器类型 |
| 类型不多、且必须极快 | 枚举 + `match` | 静态分发版的「多态」，还免费获得穷尽性检查 |

最后一行值得单独说：`enum` 与 `dyn` 是同一问题的两条路。枚举把「可能是什么」在编译期穷举，保持 `Sized`，无 vtable，`match` 有穷尽性检查，还能 derive；`dyn` 允许实现方与调用方各自独立编译、互不知道对方存在。**能被枚举穷举的，优先枚举**；只有当类型集合对编译单元封闭不住时，才用 `dyn`。

## 动手看看

### 一、异构集合、胖指针与向下转型

`Vec<Box<dyn Shape>>` 把三种互不相关的类型装进同一个集合，顺便量一量内存：

```rust
{{#rustdoc_include ../snippets/ch13/shapes.rs}}
```

```console
$ rustc --edition 2024 shapes.rs && .\shapes.exe
圆：面积 3.14
正方形：面积 4.00
点：面积 0.00
&dyn Shape = 16 字节，Box<dyn Shape> = 16 字节
a 指向的对象 8 字节，b 指向的对象 0 字节
是 Circle 吗？true
找回 Circle，半径 2
```

两处值得停一下。第一，`&dyn Shape` 是 16 字节 = 两个 8 字节机器字（数据指针 + vtable 指针），而 `&Circle` 只有 8 字节——胖的这一个字就是动态分发的门票。第二，`a` 和 `b` 的静态类型都是 `&dyn Shape`，但 `size_of_val` 读出 8 和 0 两个不同值：**被指向对象的尺寸只能在运行时从 vtable 里问**。零大小的 `Dot` 也能进这个集合——它不需要堆分配数据，但 `Box<dyn Shape>` 这层指针本身还是有的。

最后那段用 `dyn Any` 做了向下转型（downcast）：`Any` 要求 `T: 'static`，`downcast_ref::<Circle>()` 在运行时比对类型标识。`dyn Error` 的 `downcast_ref` 是同一套机制。

### 二、一个错误类型，装三种错

```rust
{{#rustdoc_include ../snippets/ch13/errors.rs}}
```

```console
$ rustc --edition 2024 errors.rs && .\errors.exe
Display：系统找不到指定的文件。 (os error 2)
原来是 io::Error，kind = NotFound
Send + Sync 版：系统找不到指定的文件。 (os error 2)
装箱后：invalid digit found in string
还能还原成 ParseIntError 吗？true
```

这个例子里有三件事同时发生：

1. 一个签名接住了三种错误：`?` 把 `io::Error` 转成 `Box<dyn Error>`（本次运行走的就是这条路径），`Box::new(MissingKey { … })` 把自定义错误直接塞进去，末尾的 `parsed.into()` 则用 `From` 把 `ParseIntError` 装箱。标准库铺好的这条实现是 `impl<'a, E: Error + 'a> From<E> for Box<dyn Error + 'a>`。
2. 需要跨线程时改签名即可：`Box<dyn Error + Send + Sync>`，对应的实现是 `impl<'a, E: Error + Send + Sync + 'a> From<E> for Box<dyn Error + Send + Sync + 'a>`。
3. 想复原具体类型，就 `downcast_ref`——这正是 trait 对象「擦掉类型」之后留下的唯一线索。

`Box<dyn Error>` 是 API 边界上最值钱的 `dyn` 用法：调用方不必知道你内部有哪些错误类型。要构造自己的错误层级、控制 `source()` 链，那是第 18 章的事。

### 三、借一次迭代器

```rust
{{#rustdoc_include ../snippets/ch13/iterators.rs}}
```

```console
$ rustc --edition 2024 iterators.rs && .\iterators.exe
sum_all(&mut (1..=4)) = 10
sum_all(&mut v.iter().copied()) = 100
take_n(&mut (100..), 3) = [100, 101, 102]
evens_up_to(10) = [2, 4, 6, 8, 10]
```

`sum_all` 接受任何 `u32` 迭代器——`RangeInclusive`、`Copied<Iter>`，一个签名全收；`evens_up_to` 把 `Filter<RangeInclusive<u32>, …>` 这个没人愿意写的类型藏在 `Box<dyn Iterator<Item = u32>>` 后面返回。这两个签名在真实代码里极常见，也是 dyn 兼容性最直接的红利。

### 四、向上转型

```rust
{{#rustdoc_include ../snippets/ch13/catalogue.rs}}
```

```console
$ rustc --edition 2024 catalogue.rs && .\catalogue.exe
dog 的主人是 小明
dog 属于 犬科
dog
Arc 向上转型后：dog
丢掉 Send 之后：dog
```

五个场景一次过：`dyn Pet` 直接调父 trait 方法、`Box<dyn Pet>` → `Box<dyn Animal>`、引用形式的隐式强转、`Arc` 形式、以及顺手丢掉 `Send`。

## 报错现场

### 一、E0038：泛型方法让 trait 失去 dyn 兼容性

最小复现：

```rust,compile_fail
trait Store {
    fn put<T>(&self, value: T);
}

struct VecStore;

impl Store for VecStore {
    fn put<T>(&self, _value: T) {}
}

fn main() {
    let s: Box<dyn Store> = Box::new(VecStore);
    s.put(1);
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0038]: the trait `Store` is not dyn compatible
  --> e0038.rs:12:20
   |
12 |     let s: Box<dyn Store> = Box::new(VecStore);
   |                    ^^^^^ `Store` is not dyn compatible
   |
note: for a trait to be dyn compatible it needs to allow building a vtable
  --> e0038.rs:2:8
   |
 1 | trait Store {
   |       ----- this trait is not dyn compatible...
 2 |     fn put<T>(&self, value: T);
   |        ^^^ ...because method `put` has generic type parameters
   = help: consider moving `put` to another trait
   = help: only type `VecStore` implements `Store`; consider using it directly instead.
```

（把上面那段代码存成 `e0038.rs` 编译即得这份输出，里面的路径与行号跟它一一对应；`note:` 里指向 Reference 的第二行、以及末尾的汇总行略去，缩进照实。）定位路径：报错先指出**使用处**（`Box<dyn Store>`），再把「哪条规则被违反」指回 trait 定义里的具体方法——看 `...because` 那句就够，不用读整段。

修法有两条，报错里的 `help` 也提示了两条：

1. **把不合规的方法挪到另一个 trait**：让主体 trait 保持 dyn 兼容，泛型方法单放在一个「扩展 trait」里。适合方法本就面向具体类型的情况。
2. **给它加 `where Self: Sized`**：声明「这个方法不参与动态分发」，trait 恢复 dyn 兼容，方法仍能对具体类型使用。代价是它从 vtable 里消失，trait 对象上再也调不到。

```rust
trait Store {
    fn put<T>(&self, value: T)
    where
        Self: Sized; // 不进 vtable，只对具体类型可用
    fn len(&self) -> usize;
}
```

同一个 trait 里没有接收者的关联函数也走这条路——1.98 的报错会直接给出两条建议：给它加 `&self` 变成方法，或加 `where Self: Sized`。

### 二、E0191：关联类型没写实

```rust,compile_fail
fn first(it: &mut dyn Iterator) -> Option<u32> {
    it.next()
}

fn main() {
    println!("{:?}", first(&mut (1..=3)));
}
```

```text
error[E0191]: the value of the associated type `Item` in `Iterator` must be specified
 --> e0191.rs:1:23
  |
1 | fn first(it: &mut dyn Iterator) -> Option<u32> {
  |                       ^^^^^^^^
  |
help: specify the associated type
  |
1 | fn first(it: &mut dyn Iterator<Item = /* Type */>) -> Option<u32> {
  |                               +++++++++++++++++++
```

`help` 已经把改法写出来了：`dyn Iterator<Item = u32>`。

### 三、`Self: Sized` 的方法在 trait 对象上调用

```rust,compile_fail
fn mapped() -> Vec<u32> {
    let it: Box<dyn Iterator<Item = u32>> = Box::new(1..=3);
    let _ = (*it).map(|x| x * 2);
    Vec::new()
}

fn main() {
    println!("{:?}", mapped());
}
```

```text
error: the `map` method cannot be invoked on a trait object
   --> sized_method.rs:3:19
    |
  3 |     let _ = (*it).map(|x| x * 2);
    |                   ^^^
    |
   ::: …\lib/rustlib/src/rust\library\core\src\iter\traits\iterator.rs:833:15
    |
833 |         Self: Sized,
    |               ----- this has a `Sized` requirement
```

注意那个 `:::` 行：编译器把指头直接伸进了标准库源码，告诉你哪个 `Self: Sized` 挡住了调用（路径前缀是本机工具链位置，此处省略）。修法不是给 `Iterator` 打补丁，而是换一层壳：`it.map(...)`（`Box<dyn Iterator>` 自己实现了 `Iterator` 且是 Sized）或把实现拿回具体类型上再适配。

## 小结

- trait 对象是**擦掉类型、留下行为**的 DST，永远待在胖指针后面：一个机器字指向数据，一个指向 vtable。
- 能不能变 `dyn`，取决于 dyn 兼容性：父 trait 也要兼容、不能要求 `Self: Sized`、不能有关联常量或泛型关联类型、可分发方法不能带类型参数或用 `Self` 做接收者以外的类型、不能有隐藏的返回类型。
- `where Self: Sized` 是「请把方法从 vtable 里剔出去」的声明，标准库的 `Iterator` 靠它同时做到「trait 可以 dyn」和「适配器依然好用」。被剔出去的方法在 trait 对象上调用不到，这是有意的。
- 代价落在具体的指令上：一次 vtable 槽位取址加一次间接调用，且挡住内联等跨调用优化。选型口诀是**热点用泛型，边界用 `dyn`**；类型能被枚举穷举时，优先 `enum`。
- 向上转型（1.86 起）让 `dyn Sub` 变成 `dyn Super`，还能顺手丢掉 auto trait；但只在父 trait 方向上合法。
- `Box<dyn Error>` 与 `&mut dyn Iterator` 是这套机制最日常的两个出口。

## 练习

**1. 下面四个 trait 能不能当 `dyn` 的基 trait？逐个给理由。**

```rust
trait A {
    fn get(&self) -> u32;
}

trait B: A {
    fn tag(&self) -> String;
}

trait C: Sized {
    fn get(&self) -> u32;
}

trait D {
    fn get(&self) -> u32;
    fn make() -> Self;
}
```

**2. 下面哪一行能编译？为什么另一行不能？**

```rust,ignore
let it: Box<dyn Iterator<Item = u32>> = Box::new(1..=3);
let a = it.map(|x| x * 2).collect::<Vec<_>>();      // 第 1 行
let b = (*it).map(|x| x * 2).collect::<Vec<_>>();   // 第 2 行
```

**3. 你写了一个想放进 trait 对象的方法 `fn duplicate(&self) -> Self;`，求解锁的两种办法，并说明各自的代价。**

<details><summary>参考答案</summary>

**1.** 逐个看：

- `A`：可以。只有一个 `&self` 方法，无泛型参数、无关联常量、无 `Self` 返回值，符合全部规则。
- `B`：可以。`A` 自己 dyn 兼容，所以父 trait 条件满足；`tag` 也没有问题。
- `C`：**不行**。`trait C: Sized` 等于要求 `Self: Sized`，1.98 的报错原话是 ``...because it requires `Self: Sized` ``。dyn 兼容性从定义上就排除了「实现者必须已知大小」。
- `D`：**不行**，但不是因为 `make` 返回 `Self` 这一点——`fn make() -> Self` 同时是「没有接收者的关联函数」和「用 `Self` 做返回类型」。它会报 ``...because associated function `make` has no `self` parameter``。给 `make` 加 `where Self: Sized` 之后，`D` 就是 dyn 兼容的（`make` 从 vtable 里消失）。

**2.** 第 1 行能编译，第 2 行不能。

`it` 的类型是 `Box<dyn Iterator<Item = u32>>`。`Box<I>` 对任意 `I: Iterator + ?Sized` 都实现了 `Iterator`（源码实现还带分配器参数 `A: Allocator`），而 `Box<_>` 本身是 Sized 的——`map` 的 `where Self: Sized` 约束落在 `Box<dyn Iterator<…>>` 上，满足。第 2 行 `*it` 解引用到 `dyn Iterator<Item = u32>`，这才是真正的 trait 对象；`map` 不在它的 vtable 里，编译器报 `the `map` method cannot be invoked on a trait object`，并把 `Self: Sized` 那一行指给你看。要点：**「dyn 不满足 `Self: Sized`」针对 trait 对象本身，不针对指向它的指针。**

**3.** 两种办法：

- **加 `where Self: Sized`**：`` fn duplicate(&self) -> Self where Self: Sized; ``。trait 恢复 dyn 兼容，但 `duplicate` 从 vtable 里消失，在 trait 对象上再也调不到。只有在「具体类型上要用、trait 对象上不需要」时才合适。
- **改成关联函数 + 显式边界**：把「复制自己」变成构造新值的操作，比如 `fn duplicate(&self) -> Box<dyn Trait>` 或 `fn duplicate(&self) -> Self where Self: Clone`——后者仍是 `Self: Sized`。更彻底的做法是接受「trait 对象上无法按 `Self` 返回」这个事实，把「复制」留在具体类型一侧（调用方本来就持有具体类型时才能复制）。作为对照，标准库的 `Clone` 就没有给自己留后路：它声明为 `pub trait Clone: Sized`，所以 `dyn Clone` 根本不存在（1.98 实测：`&dyn Clone` 报 ``the trait is not dyn compatible because it requires `Self: Sized` ``）。要克隆 trait 对象，只能另定义一个包装 trait、让它的方法返回 `Box<dyn Trait>`（这属于第 12 章的 trait 设计手法）。

</details>

## 延伸阅读

- dyn 兼容性规则全文：<https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility>
- trait 对象类型（边界怎么拼、默认生命周期、vtable 的定义）：<https://doc.rust-lang.org/reference/types/trait-object.html>
- 内建强转（含向上转型那条）：<https://doc.rust-lang.org/reference/type-coercions.html#unsized-coercions>
- *The Book* 第 18 章第 2 节「Trait Objects」：<https://doc.rust-lang.org/book/ch18-02-trait-objects.html>
- `std::error::Error`（含 `Box<dyn Error>` 的 `From` 实现）：<https://doc.rust-lang.org/std/error/trait.Error.html>
- `std::any`（`dyn Any` 与向下转型）：<https://doc.rust-lang.org/std/any/index.html>
- Nomicon「Exotic Sizes」（DST 与 vtable 指针的角色）：<https://doc.rust-lang.org/nomicon/exotic-sizes.html>
- 版本说明（向上转型在 1.86 稳定）：<https://doc.rust-lang.org/releases.html>
