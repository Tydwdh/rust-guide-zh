# 第 5 章 结构体、枚举与模式匹配

## 这一章解决什么问题

前四章你有了基础类型、函数、控制流，也有了对所有权与借用的心智模型。但你能用的类型都是别人定义好的：`u32`、`String`、`bool`。想表示「一个任务有编号、有标题、有状态」，你只能开三个变量，或者塞进元组 `(u32, String, bool, u8)`——过两天你就忘了第三个位置到底是什么意思。

这一章给你两个定义类型的工具：结构体（struct）把几个值绑成一条记录，枚举（enum）表达「只可能是这几种之一」。再给你一个把值拆开的工具：模式匹配（pattern matching）。

读完这一章你能：

- 定义 struct 与 enum，写 `impl` 方法，用 `#[derive]` 按需加上常用能力；
- 用枚举建模状态，让「非法状态」在类型层面根本构造不出来；
- 用 `match` 把值拆开，并明白编译器为什么逼你写全所有分支；
- 在只关心一种情况时，用 `if let`、`while let`、`let else`、`matches!` 写出更短的代码。

> [!NOTE]
> 本章代码与报错输出都在 rustc 1.98.0（stable，edition 2024）上跑过；沿用 edition 2024 新语法的两处（let chains、模式绑定模式的变化）在正文里单独标注。

## 为什么这样设计

把类型想成「一组可能的值」：`bool` 有两个值，`u8` 有 256 个。你定义的 struct 是「每个字段各自取值，拼在一起」——这是 AND；enum 是「从这几种形状里选一个」——这是 OR。用 AND 与 OR 把类型拼起来，这套做法叫代数数据类型（algebraic data type，ADT）。剩下的问题是怎么把值拆开：`match` 负责这件事，顺便让编译器替你检查「是否每种情况都处理了」。

对比其他语言对同一个问题的处理：

- C 表示「任务状态」通常是结构体里放一个 `int status` 加几个字段，用 `switch` 分派。编译器不检查你有没有漏掉某个状态，也不阻止「失败的任务带着完成路径」这种组合。
- Java/Python 常见的是继承加 `null`、`Optional` 或异常。`null` 的真正问题在于类型系统对「可能没有」失灵：签名写着返回 `Task`，实际可能给你 `null`，运行时才知道。
- Rust 的选择是把状态和它携带的数据合成一个类型，用 `match` 取出：每个状态只能看到自己那份数据，而且编译器强制你处理全部状态。C++ 的 `std::variant`、F# 的 discriminated union 是近亲（C# 只有提案）。

本章的主线是**状态建模**，语法只是手段。当「任务已经完成但进度还是 42%」这种组合无法被构造出来时，一整类 bug 就消失了。下面是三种做法的对照：

```text
bool/字符串「拼」状态（可以构造出非法组合）      用 enum 建模（非法组合不存在）

struct Job {                                    enum Status {
    is_downloading: bool,                           Pending,
    progress: u8,                                   Downloading { progress: u8 },
    done_path: String,                              Done { path: String },
    failed_reason: String,                          Failed { reason: String },
}                                               }
```

左边可以同时 `is_downloading == true` 且 `done_path != ""`；右边不可能同时是 `Downloading` 和 `Done`。规则也不多：一个值在任何时刻只属于一个变体（variant）。

下图把「3 个 bool 的 8 种组合」与「enum 的 4 个变体」摆在一起：

<svg viewBox="0 0 640 220" width="100%" role="img" aria-label="同一条状态：3 个 bool 有 8 种组合、其中 4 种自相矛盾，用一个四变体 enum 则非法组合构造不出来" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">同一条状态：3 个 bool 有 2³ = 8 种组合，enum Status 只有 4 个变体</text>
<text x="0" y="40" font-size="12" fill="currentColor">3 个 bool，位序依次是 downloading、done、failed</text>
<g font-family="monospace" font-size="13" text-anchor="middle" fill="currentColor">
<rect x="0" y="52" width="46" height="22" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/><text x="23" y="67">000</text>
<rect x="0" y="82" width="46" height="22" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/><text x="23" y="97">100</text>
<rect x="0" y="112" width="46" height="22" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/><text x="23" y="127">010</text>
<rect x="0" y="142" width="46" height="22" rx="4" fill="none" stroke="var(--links)" stroke-width="1.5"/><text x="23" y="157">001</text>
<rect x="150" y="52" width="46" height="22" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="4 3"/><text x="173" y="67">110</text>
<rect x="150" y="82" width="46" height="22" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="4 3"/><text x="173" y="97">101</text>
<rect x="150" y="112" width="46" height="22" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="4 3"/><text x="173" y="127">011</text>
<rect x="150" y="142" width="46" height="22" rx="4" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="4 3"/><text x="173" y="157">111</text>
</g>
<g font-size="12" fill="currentColor">
<text x="56" y="67">Pending</text>
<text x="56" y="97">Downloading</text>
<text x="56" y="127">Done</text>
<text x="56" y="157">Failed</text>
<text x="206" y="67">下载中 + 完成</text>
<text x="206" y="97">下载中 + 失败</text>
<text x="206" y="127">完成 + 失败</text>
<text x="206" y="157">三种同时为真</text>
</g>
<line x1="316" y1="34" x2="316" y2="170" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3"/>
<text x="336" y="40" font-size="12" fill="currentColor">enum Status：4 个变体，每个只带自己那份数据</text>
<g font-family="monospace" font-size="13" fill="currentColor">
<rect x="336" y="50" width="296" height="26" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="348" y="68">Pending</text>
<rect x="336" y="80" width="296" height="26" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="348" y="98">Downloading { progress: u8 }</text>
<rect x="336" y="110" width="296" height="26" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="348" y="128">Done { path: String }</text>
<rect x="336" y="140" width="296" height="26" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="348" y="158">Failed { reason: String }</text>
</g>
<line x1="0" y1="182" x2="14" y2="182" stroke="var(--links)" stroke-width="2"/>
<text x="20" y="186" font-size="12" fill="currentColor">合法：能构造出来</text>
<line x1="170" y1="182" x2="184" y2="182" stroke="var(--warning-border)" stroke-width="2" stroke-dasharray="4 3"/>
<text x="190" y="186" font-size="12" fill="currentColor">非法：同一个状态里两个都为真</text>
<text x="0" y="210" font-size="12" fill="currentColor" opacity="0.7">左边 8 种组合里只有 4 种合法；右边的 4 个变体之外没有别的值。</text>
</svg>

## 机制与规则

### 5.1 结构体（struct）：三种形态

```rust
struct Job {              // 命名字段：最常用
    id: u32,
    title: String,
}

struct Point(f64, f64);   // 元组结构体（tuple struct）：字段没有名字，用 .0/.1 访问
struct Marker;            // 单元结构体（unit struct）：不携带数据

let job = Job { id: 7, title: String::from("第 5 章草稿") };
println!("{} {}", job.id, job.title);

let p = Point(1.0, 2.0);
let _m = Marker;
println!("{}", p.0);
```

输出是 `7 第 5 章草稿` 和 `1`。三种形态没有本质差别，只是字段命名方式不同：有名字的读起来清楚，元组结构体适合「新类型」（比如 `struct Meters(f64)` 让单位进类型），单元结构体用于只需要一个类型名、不需要数据的场合——第 9 章会看到它作为 trait 实现的标记。

创建实例时会写 `Job { id: 7, title: ... }`；如果局部变量名和字段名恰好相同，可以简写：

```rust
struct Job {
    id: u32,
    title: String,
}

let id = 7;
let title = String::from("第 5 章草稿");
let job = Job { id, title };   // 等价于 Job { id: id, title: title }
println!("{} {}", job.id, job.title);
```

### 5.2 impl：方法与关联函数

`impl` 块把行为挂到类型上，第一个参数决定你能怎么调用它：

```rust
struct Counter {
    count: u32,
}

impl Counter {
    fn new() -> Self {           // 关联函数：没有 self，用 Counter::new() 调用
        Counter { count: 0 }
    }
    fn count(&self) -> u32 {     // &self：只读，借用
        self.count
    }
    fn bump(&mut self) {         // &mut self：要改，独占可变借用
        self.count += 1;
    }
    fn into_count(self) -> u32 { // self：拿走所有权
        self.count
    }
}

let mut c = Counter::new();
c.bump();
c.bump();
println!("{}", c.count());   // 2
let n = c.into_count();      // c 在这里被移走
println!("{n}");             // 2
```

三点说明：

- 没有 `self` 的函数叫关联函数（associated function），用 `类型::函数()` 调用；用来构造的关联函数按惯例叫 `new`。这不是语言规定，只是标准库与生态的默认约定。
- `&self` / `&mut self` / `self` 的取舍就是第 4 章的借用规则投影到方法上：只读用 `&self`；要改字段用 `&mut self`，所以接收者变量必须声明成 `let mut c`；要消耗掉这个值（比如转成另一种类型）就用 `self`。
- `Self` 是「当前类型」的别名，`-> Self` 和 `-> Counter` 等价。

调用 `c.into_count()`（或把 `c` 传给别的函数）之后，`c` 已被移走，再用就是第 4 章的移动报错。

下图把三种接收者对原值做了什么摊开：

<svg viewBox="0 0 640 228" width="100%" role="img" aria-label="self、&amp;self、&amp;mut self 三种接收者对原值分别做了什么：移动所有权、共享借用、独占可变借用" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">self 三形态：第 4 章的所有权与借用规则投影到方法上</text>
<g stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" fill="none">
<rect x="0" y="32" width="176" height="44" rx="5"/>
<rect x="0" y="90" width="176" height="44" rx="5"/>
<rect x="0" y="148" width="176" height="44" rx="5"/>
<rect x="298" y="32" width="342" height="44" rx="5"/>
<rect x="298" y="90" width="342" height="44" rx="5"/>
<rect x="298" y="148" width="342" height="44" rx="5"/>
</g>
<g font-family="monospace" font-size="13" text-anchor="middle" fill="currentColor">
<text x="88" y="59">fn into_count(self)</text>
<text x="88" y="117">fn count(&amp;self)</text>
<text x="88" y="175">fn bump(&amp;mut self)</text>
</g>
<g stroke="currentColor" stroke-width="1.5">
<line x1="182" y1="54" x2="292" y2="54"/>
<line x1="182" y1="112" x2="292" y2="112"/>
<line x1="182" y1="170" x2="292" y2="170"/>
</g>
<polygon points="294,54 286,49 286,59" fill="currentColor"/>
<polygon points="294,112 286,107 286,117" fill="currentColor"/>
<polygon points="294,170 286,165 286,175" fill="currentColor"/>
<g font-size="12" fill="currentColor" opacity="0.7" text-anchor="middle">
<text x="237" y="46">调用</text>
<text x="237" y="104">调用</text>
<text x="237" y="162">调用</text>
</g>
<g fill="var(--links)" font-size="12">
<text x="310" y="50">移动所有权：c 被移走</text>
<text x="310" y="108">共享借用 &amp;T：c 仍可用，只读</text>
<text x="310" y="166">独占可变借用 &amp;mut T：c 仍可用，而且能改</text>
</g>
<g fill="currentColor" font-size="12">
<text x="310" y="68">之后再用 c 就是 E0382</text>
<text x="310" y="126">可以同时有多个 &amp;c</text>
<text x="310" y="184">需 let mut c；调用期间不能有别的借用</text>
</g>
<text x="0" y="216" font-size="12" fill="currentColor" opacity="0.7">三种接收者就是三种访问方式；Self 是「当前类型」的别名，-&gt; Self 与 -&gt; Counter 等价。</text>
</svg>

### 5.3 derive：按需索取常用能力

Rust 不会给你的类型自动加「能打印」「能比较」这类能力，要什么就 `#[derive]` 什么：

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

let p = Point { x: 1, y: 2 };
let q = p;            // Copy：按位复制，p 之后仍然可用
let r = p.clone();    // 显式克隆（对 Copy 类型其实不必要）
println!("{p:?} == {q:?} -> {}", p == q);
println!("{r:?}");
```

```text
Point { x: 1, y: 2 } == Point { x: 1, y: 2 } -> true
Point { x: 1, y: 2 }
```

- `Debug` 提供 `{:?}`（单行）与 `{:#?}`（多行）调试打印；`{}` 需要的是 `Display`，它不能 derive（第 9 章讲 trait 时说清为什么）。
- `Clone` 提供 `.clone()`，是**可能分配内存的显式拷贝**。
- `Copy` 表示「赋值与传参按位复制，而不是移动」。**能不能 Copy 由字段决定**：所有字段都 Copy，而且不含 `String`、`Vec`、`Box` 这类拥有堆内存的字段才行。这是第 4 章移动规则的直接推论。
- `PartialEq` 提供 `==` 与 `!=`；`Eq` 用于需要「完全等价关系」的场合（比如当 `HashMap` 的键，但键还要求 `Hash`，第 6 章展开）。

`#[derive]` 不是魔法：它展开成对这个类型的 `impl`（第 9 章会看到 trait 与手写 `impl`）。把上面的 `Point` 换成带 `String` 的类型，赋值就从复制变回移动：

```rust
#[derive(Debug, PartialEq)]
struct Job {
    title: String,
}

let a = Job { title: String::from("第 5 章") };
let b = a;              // 没有 Copy：这里是移动
println!("{b:?}");
// println!("{a:?}");   // 取消注释：E0382，第 4 章那个现场
```

输出 `Job { title: "第 5 章" }`。

### 5.4 枚举（enum）：一个值，多种形状

```rust
enum Shape {
    Circle(f64),                // 元组变体
    Rect { w: f64, h: f64 },    // 命名字段变体
}

fn area(s: &Shape) -> f64 {
    match s {
        Shape::Circle(r) => std::f64::consts::PI * r * r,
        Shape::Rect { w, h } => w * h,
    }
}

println!("{:.2}", area(&Shape::Circle(1.0)));              // 3.14
println!("{:.2}", area(&Shape::Rect { w: 2.0, h: 3.0 }));  // 6.00
```

每个变体携带的数据形状可以完全不同：`Circle` 是一个数，`Rect` 是两个数。使用时必须写明是哪个变体（`Shape::Circle`），不存在「忘了判断类型」这件事。

现在把它用在状态上。一个下载任务的状态：

```rust
enum Status {
    Pending,
    Downloading { progress: u8 },
    Done { path: String },
    Failed { reason: String },
}
```

`Status` 的任何值都只是这四个之一，且只带自己那份数据：`Done` 才有 `path`，`Failed` 才有 `reason`。改状态就是整个替换，配上 `match` 就能每种状态给出各自的说法：

```rust
enum Status {
    Pending,
    Downloading { progress: u8 },
    Done { path: String },
    Failed { reason: String },
}

fn label(status: &Status) -> &'static str {
    match status {                              // 四个变体一个都不能漏
        Status::Pending => "等待中",
        Status::Downloading { .. } => "下载中", // .. 表示其余字段不关心
        Status::Done { .. } => "完成",
        Status::Failed { .. } => "失败",
    }
}

let mut status = Status::Pending;
status = Status::Downloading { progress: 42 };
println!("{}", label(&status));                 // 下载中
status = Status::Done { path: String::from("ch05.md") };
println!("{}", label(&status));                 // 完成
```

注意 `status` 必须声明成 `mut`（要整体替换），以及 `label` 里的四个分支：这不是啰嗦，是编译器在替你确认「每种状态都有对应的处理」。

枚举同样可以挂方法和关联函数（`impl Status { ... }`）。写成不带数据的 `enum Color { Red, Green, Blue }` 也合法，那只是「变体都不携带数据」的退化形式。

> [!NOTE]
> 枚举适合「变体集合在编写时就封闭」的建模。如果需要的是「任何实现了某个 trait 的类型都能放进同一个集合」，那要用 trait 对象（`dyn`，第 13 章）；两种建模手法的取舍在那一章展开。

### 5.5 Option&lt;T&gt;：没有 null

「可能没有」怎么表达？Rust 的答案不是关键字，而是标准库里的一个枚举：

```text
// std 里的定义（省去若干 derive 与标注）
pub enum Option<T> {
    None,
    Some(T),
}
```

于是「查找可能失败」「字段可能不存在」都变成类型的一部分——返回 `Option<T>` 的函数，调用方必须处理 `None`：

```rust
let line = "path=ch05.md";
let path = line.strip_prefix("path=");
println!("{path:?}");                            // Some("ch05.md")
println!("{}", path.unwrap_or("（未指定）"));     // ch05.md
println!("{:?}", line.strip_prefix("size="));     // None
println!("{}", path.map(str::len).unwrap_or(0));  // 7
```

三种处理方式先认识一下（细节见第 7 章）：

- `match`：`Some(p)` 与 `None` 都写出来，最完整。
- `if let`：只关心 `Some` 的分支（见 5.8）。
- 组合子：`unwrap_or`（给默认值）、`map`（对 `Some` 里的值做变换）、`unwrap_or_else`、`is_some`，把常见模式压缩成一行。`unwrap()` / `expect()` 在 `None` 时 panic，只在「逻辑上不可能失败」时用。

**没有 null 的直接后果**：`Option<&str>` 不能当 `&str` 用，你必须先处理 `None`——这个「麻烦」正是它值钱的地方（第 7 章的 `?` 会让流程短很多）。

### 5.6 match：穷尽性与「匹配是表达式」

`match` 有两条要记住的规则：

- **它是表达式**：每个分支产出的值类型必须一致，`match` 整体就是这些值之一。所以它能直接写进 `let`，也能给函数当返回值。
- **它要求穷尽（exhaustive）**：所有可能的值都要有分支覆盖。编译器知道每个 enum 的完整变体列表（外部 crate 里标了 `#[non_exhaustive]` 的枚举除外），漏一个就是编译错误。

```rust
for n in [0, 7, 42, -1] {
    match n {
        0 => println!("{n} 是零"),                 // 字面量
        p @ 1..=9 => println!("{n} 是一位数 {p}"), // @：命中后把值绑给 p
        _ if n < 0 => println!("{n} 是负数"),      // 守卫（guard）：额外条件
        _ => println!("{n} 是别的"),               // _ 兜底
    }
}
```

```text
0 是零
7 是一位数 7
42 是别的
-1 是负数
```

分支自上而下尝试，第一个命中的生效；所以 `_ if n < 0` 必须写在 `_` 前面。守卫不是模式，是模式后面附加的 `if` 条件（1.95 起守卫里也能写 `if let`）。

`_` 兜底能少写很多分支，但它也是「静音键」：以后给 enum 加变体，编译器不会再提醒你这里的 `_` 吃掉了新情况。建模状态时优先把变体写全；只在确实「其余都一样」的地方用 `_`。

下图把 `match` 的两件事摊开：分支自上而下怎么试、穷尽性检查在查什么。

<svg viewBox="0 0 640 252" width="100%" role="img" aria-label="match 自上而下尝试分支、首个命中生效，以及编译器按变体清单做穷尽性检查、缺分支就报 E0004" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">match 的两件事：自上而下尝试（首个命中生效）与穷尽性检查</text>
<text x="0" y="40" font-size="12" fill="currentColor">① 自上而下：从第一个分支开始试，命中之后就停</text>
<rect x="0" y="46" width="68" height="26" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/>
<text x="34" y="64" font-size="13" font-family="monospace" text-anchor="middle" fill="currentColor">n = 7</text>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<line x1="68" y1="59" x2="104" y2="59"/>
<line x1="104" y1="59" x2="104" y2="137"/>
</g>
<polygon points="112,59 105,55 105,63" fill="currentColor"/>
<polygon points="112,85 105,81 105,89" fill="currentColor"/>
<polygon points="112,111 105,107 105,115" fill="currentColor"/>
<polygon points="112,137 105,133 105,141" fill="currentColor"/>
<g font-family="monospace" font-size="12" fill="currentColor">
<rect x="112" y="48" width="190" height="22" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="122" y="63">0 =&gt;</text>
<rect x="112" y="74" width="190" height="22" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="122" y="89">p @ 1..=9 =&gt;</text>
<rect x="112" y="100" width="190" height="22" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="122" y="115">_ if n &lt; 0 =&gt;</text>
<rect x="112" y="126" width="190" height="22" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="122" y="141">_ =&gt;</text>
</g>
<g font-size="12" fill="currentColor" opacity="0.6">
<text x="312" y="63">未命中，继续往下</text>
<text x="312" y="115">不再尝试</text>
<text x="312" y="141">不再尝试</text>
</g>
<text x="312" y="89" font-size="12" fill="var(--links)">✓ 生效：后面的分支不再尝试</text>
<line x1="0" y1="164" x2="640" y2="164" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3"/>
<text x="0" y="186" font-size="12" fill="currentColor">② 穷尽性检查：编译器知道 enum 的全部变体，每个变体都要有分支接住</text>
<g font-family="monospace" font-size="12" fill="currentColor">
<rect x="0" y="196" width="148" height="26" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="12" y="214">Pending</text>
<rect x="158" y="196" width="148" height="26" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="170" y="214">Downloading</text>
<rect x="474" y="196" width="148" height="26" rx="5" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5"/><text x="486" y="214">Failed</text>
</g>
<g font-size="12" fill="currentColor" opacity="0.6" text-anchor="end">
<text x="136" y="214">✓</text>
<text x="294" y="214">✓</text>
<text x="610" y="214">✓</text>
</g>
<rect x="316" y="196" width="148" height="26" rx="5" fill="none" stroke="var(--warning-border)" stroke-width="1.5" stroke-dasharray="4 3"/>
<text x="328" y="214" font-family="monospace" font-size="12" fill="var(--warning-border)">Done</text>
<text x="452" y="214" font-size="12" fill="var(--warning-border)" text-anchor="end">✗</text>
<text x="316" y="242" font-size="12" fill="var(--warning-border)">缺一个变体 → error[E0004]: non-exhaustive patterns</text>
</svg>

### 5.7 模式（pattern）能写什么

| 形式 | 写法 | 说明 |
|---|---|---|
| 字面量 | `0`、`"q"` | 相等才命中 |
| 范围 | `1..=9` | 闭区间 |
| 或模式（or-pattern） | `'q' \| 'Q'` | 任一命中即可 |
| 绑定 | `p`、`title` | 把值绑定到名字；是移动还是复制由类型决定 |
| 通配 | `_` | 匹配任何值且不绑定 |
| 剩余 | `..` | 忽略剩余字段或元素 |
| 解构（destructuring） | `Job { id, title }`、`Point(x, y)`、`Some((head, tail))` | 拆开 struct / 元组 / enum，可嵌套 |
| `@` | `p @ 1..=9` | 命中就绑定整个值 |

模式不只在 `match` 里出现：`let`、函数参数、`for` 的循环变量都是模式（`let (a, b) = pair;` 就是一次解构）。几个例子：

```rust
struct Job {
    id: u32,
    title: String,
    retries: u8,
}

let job = Job {
    id: 7,
    title: String::from("第 5 章草稿"),
    retries: 2,
};
match job {
    Job { id: 0, .. } => println!("占位任务"),        // 字面量 + 忽略其余字段
    Job { id, title, .. } => println!("任务 {id}：{title}"),
}
```

输出 `任务 7：第 5 章草稿`。嵌套与元组：

```rust
let pair = ((1, 2), (3, 4));
let ((a, b), (c, d)) = pair;    // 嵌套解构
println!("{a} {b} {c} {d}");    // 1 2 3 4

let (first, ..) = (10, 20, 30);
println!("{first}");            // 10

struct Point(f64, f64);
let p = Point(1.0, 2.0);
let Point(x, y) = p;            // 元组结构体同样能解构
println!("{x} {y}");            // 1 2
println!("{}", p.0);            // 1：p 仍然可用
```

最后一行顺便说明 `p.0` 仍可访问：`f64` 是 Copy，解构只是复制，`p` 没被移走。

当模式匹配的对象是引用时，绑定会自动变成引用（这套规则叫「匹配人体工程学」，match ergonomics）：

```rust
struct Job {
    id: u32,
    title: String,
}

let job = Job { id: 7, title: String::from("草稿") };
let r = &job;                       // r: &Job
match r {
    Job { id, .. } => println!("{id}"),  // id: &u32，不是 u32
}
```

> [!NOTE]
> edition 2024 起，这种「隐式借用」的模式里不能再手写 `ref` / `mut`：`Job { ref id, .. }` 会得到 `cannot explicitly borrow within an implicitly-borrowing pattern`，按提示去掉 `ref` 即可。2021 及更早的 edition 允许这么写，想提前发现可开默认 `allow` 的 `rust-2024-incompatible-pat` lint。细节见 Reference 的 pattern binding modes 一节。

### 5.8 更短的匹配：if let / while let / let else / matches!

`match` 写全所有分支有时是浪费。四种更轻的写法：

```rust
let line = "timeout=30";

if let Some((key, value)) = line.split_once('=') {
    println!("{key} = {value}");     // timeout = 30
}

// edition 2024 起：多个 let 可以用 && 串起来（let chains）
let mut timeout = None;
if let Some((_, value)) = line.split_once('=') && let Ok(n) = value.parse::<u32>() {
    timeout = Some(n);
}
println!("{timeout:?}");             // Some(30)
```

`if let` 只处理一种情况，不做穷尽性检查——代价是「剩下的情况」被静默忽略，所以它适合「其余情况就是不做事」的场合。`while let` 是循环版：模式不匹配就退出循环。

```rust
let mut rest: &[i32] = &[1, 2, 3];
while let Some((head, tail)) = rest.split_first() {
    println!("{head}");   // 依次打印 1、2、3
    rest = tail;
}
```

`let else` 用在函数开头：拿不到就立刻离开当前作用域（`return` / `break` / `continue` / `panic!`），拿到则继续往下走，不用把后续代码缩进一层：

```rust
fn first_word(text: &str) -> &str {
    let Some(i) = text.find(' ') else {
        return text;      // 不匹配必须发散
    };
    &text[..i]
}

println!("{}", first_word("rust guide"));  // rust
println!("{}", first_word("rust"));        // rust
```

`matches!` 只需要一个布尔值，不需要把数据取出来：

```rust
#[derive(Debug)]
enum Status {
    Pending,
    Done { path: String },
}

let done = Status::Done { path: String::from("ch05.md") };
println!("完成？{}", matches!(done, Status::Done { .. }));  // true
```

选择表：

| 场景 | 用什么 |
|---|---|
| 每种情况都要处理，而且要把数据取出来 | `match`（带穷尽性检查） |
| 只关心一种情况，其余不做处理 | `if let`（2024 起可用 `&&` 串多个） |
| 反复从一个序列里取，直到取不到 | `while let` |
| 函数开头拿不到就退出 | `let else` |
| 只要「是不是这种形状」的布尔值 | `matches!` |

## 动手看看

把上面的零件装成一个能跑的程序：`snippets/ch05/task_state.rs`。它用 `Status` 建模下载任务的四种状态，用 `impl Job` 收拢状态转换，用 `match` 生成描述，用 `Option` 表达「完成才有产出路径」。

```rust
{{#rustdoc_include ../snippets/ch05/task_state.rs}}
```

在仓库根编译并运行（下面按 Windows 写：`rustc -o` 不会自动补 `.exe`，运行要带 `.\` 前缀；其他平台编译成 `task_state`、运行 `./task_state`）：

```console
$ rustc --edition 2024 snippets/ch05/task_state.rs -o task_state.exe
$ .\task_state.exe
```

```text
[1] 第 5 章草稿 等待中
[1] 第 5 章草稿 下载中 42%
[1] 第 5 章草稿 完成 -> ch05.md
saved_path = Some("ch05.md")
素材下载 没有产出文件
失败了吗？true
```

值得留意的几处：

- 状态转换只写在 `impl Job` 里（`start` / `set_progress` / `finish`），外面拿不到「半个状态」；`set_progress` 用 `if let ... = &mut self.status` 写，不在下载中时调用它什么也不做。
- `saved_path` 返回 `Option<&str>`，而不是用空字符串之类的哨兵值（sentinel value）表示「没有产出」：类型本身就把这件事说清楚了。
- `describe` 的 `match` 写全了四个变体。给 `Status` 加一个 `Cancelled { at: u32 }` 变体，编译器会立刻在这里报错，告诉你哪些地方还没处理新状态——这就是穷尽性检查的价值。
- `main` 结尾的 `matches!(failed.status, Status::Failed { .. })` 是只取布尔值的写法，不需要把 `reason` 拿出来。

## 报错现场

### 现场 1：漏掉一个 match 分支（E0004）

最小复现：

```rust,compile_fail
enum Status {
    Pending,
    Done,
}

fn label(status: &Status) -> &'static str {
    match status {
        Status::Pending => "等待中",
    }
}

fn main() {
    println!("{}", label(&Status::Done));
}
```

报错关键行（以 Rust 1.98 为例）：

```text
error[E0004]: non-exhaustive patterns: `&Status::Done` not covered
   |
 7 |     match status {
   |           ^^^^^^ pattern `&Status::Done` not covered
   |
note: `Status` defined here
   |
 3 |     Done,
   |     ---- not covered
   |
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
   |
 8 ~         Status::Pending => "等待中",
 9 ~         &Status::Done => todo!(),
   |
```

定位路径：

- 报错里的 `&Status::Done` 带了个 `&`，因为 `status` 是 `&Status`，匹配的是引用（5.7 的「匹配人体工程学」）。读报错先看被匹配值的类型。
- 编译器能写出具体缺哪个变体，是因为它知道 `Status` 的全部变体：所以建模状态时，**写全分支**比 `_` 兜底更有价值。
- 修法：补一句 `Status::Done => "完成",`。或者临时用 `_ => todo!()` 兜底让编译通过，但要清楚它掩盖了后续新增的变体。

### 现场 2：if let 搬走了字段（E0382）

「只关心一种情况」的 `if let` 同样遵守移动规则。复现：

```rust,compile_fail
enum Status {
    Pending,
    Failed { reason: String },
}

struct Job {
    title: String,
    status: Status,
}

fn consume(job: Job) {
    println!("收下 {}", job.title);
}

fn main() {
    let job = Job {
        title: String::from("素材下载"),
        status: Status::Failed {
            reason: String::from("连接超时"),
        },
    };

    if let Status::Failed { reason } = job.status {
        println!("失败原因：{reason}");
    }

    println!("{}", job.title);
    consume(job);
}
```

```text
error[E0382]: use of partially moved value: `job`
   |
23 |     if let Status::Failed { reason } = job.status {
   |                             ------ value partially moved here
...
28 |     consume(job);
   |             ^^^ value used here after partial move
   |
   = note: partial move occurs because value has type `String`, which does not implement the `Copy` trait
help: borrow this binding in the pattern to avoid moving the value
   |
23 |     if let Status::Failed { ref reason } = job.status {
   |                             +++
```

定位路径：

- 关键新词是**部分移动（partial move）**：`job.status` 里的 `String` 被搬走，`job` 的其他字段还完好——所以 `job.title` 照样能打印，但把整个 `job` 交给 `consume` 就不行。第 4 章的移动规则在字段粒度上同样成立。
- 最省事的修法是借过来匹配：`if let Status::Failed { reason } = &job.status`，此时 `reason` 是 `&String`，什么都没被搬走。
- 想在匹配后随手改一改再交出去，就明确写 `job.status.clone()`：类型没实现 `Copy` 时，克隆从来不会自动发生。
- 编译器提示的 `ref reason` 是「绑定成引用」的另一种写法，等价于 `&job.status` 那行——但它只在**没有**隐式借用的模式里能写（5.7 的 NOTE）。

### 现场 3：字段名打错（E0609）

```rust,compile_fail
struct Job {
    id: u32,
    title: String,
}

fn main() {
    let job = Job {
        id: 1,
        title: String::from("第 5 章草稿"),
    };
    println!("{}", job.titel);
}
```

```text
error[E0609]: no field `titel` on type `Job`
   |
11 |     println!("{}", job.titel);
   |                        ^^^^^ unknown field
   |
help: a field with a similar name exists
   |
11 -     println!("{}", job.titel);
11 +     println!("{}", job.title);
   |
```

字段访问是编译期查表，没有运行时「找不到字段」这回事；这也是 Rust 没有动态属性（`obj.任意名字`）的原因。跨模块访问私有字段是另一种错误（E0616），第 8 章会碰到。修法就是按 `help` 提示改名字——这类「猜错字段名」的报错在编辑器里通常连修都不用自己动手。

## 小结

1. struct 是 AND：把几个值拼成一条记录，有三种形态（命名字段 / 元组 / 单元）；enum 是 OR：一个值只可能是若干变体之一，每个变体携带自己的数据。
2. `impl` 把行为挂到类型上；`self` / `&self` / `&mut self` 就是第 4 章所有权规则在方法上的投影；构造用关联函数 `new`（约定，不是语法）。
3. `#[derive(Debug, Clone, Copy, PartialEq)]` 是按需索取的能力；能不能 `Copy` 由字段决定，含 `String`/`Vec`/`Box` 就不行。
4. `Option<T>` 把「可能没有」写进类型，替代 `null`：`match` 最完整，`if let` 最省事，`unwrap_or`/`map` 这些组合子把常见模式压成一行。
5. `match` 是表达式且要求穷尽，这是状态建模能落地的另一半；`_` 兜底会让新增变体不再报错，对状态机要慎用。
6. 只想处理一种情况时用 `if let` / `while let` / `let else` / `matches!`，各自的适用场合见 5.8 的选择表。

## 练习

**1. 预测编译结果。** 下面这个类型加上 `#[derive(Copy, Clone)]` 会发生什么？

```rust,compile_fail
#[derive(Copy, Clone)]
struct Order {
    id: u32,
    items: Vec<String>,
}
```

**2. 找错。** 下面这段代码想改任务的标题，为什么编译不过？给出两种修法。

```rust,compile_fail
struct Job {
    title: String,
}

fn relabel(job: &Job) {
    job.title = String::from("新标题");
}
```

**3. 建模。** 用 enum 表示登录状态：未登录、已登录（携带用户名）、锁定（携带失败次数）。写出 `can_retry(&self) -> bool`，并说明为什么不用 `bool + String` 两个字段。

<details><summary>参考答案</summary>

**1.** 报错，因为 `Copy` 要求**所有字段**都实现 `Copy`，而 `Vec<String>` 拥有堆内存、不是 `Copy`（每个值都要负责释放自己的缓冲区，位复制会变成双重释放）。1.98 的报错是 E0204，关键行是 ``the trait `Copy` cannot be implemented for this type``，并指出 `items: Vec<String>` 这个字段没有实现 `Copy`。去掉 `Copy`、保留 `Clone` 即可：`.clone()` 会真的复制一份 `Vec`，这是可以被察觉的开销。

**2.** `&Job` 是共享借用，只能读。`job.title = ...` 试图通过共享借用写入，报 E0594，原话是 ``cannot assign to `job.title`, which is behind a `&` reference``，`help` 会建议把参数改成 `&mut Job`。

- 修法 A：签名改成 `fn relabel(job: &mut Job)`，调用方也要能拿到可变借用（`relabel(&mut job)`，且 `job` 本身是 `let mut`）。
- 修法 B：不要原地修改，返回一个新值：`fn relabeled(job: &Job) -> Job { Job { title: String::from("新标题") } }`。想避免把 `title` 搬走又需要沿用原值时，用 `job.title.clone()`（第 6 章会看到更省的做法）。

**3.**

```rust
enum Session {
    LoggedOut,
    LoggedIn { user: String },
    Locked { failures: u32 },
}

impl Session {
    fn can_retry(&self) -> bool {
        match self {
            Session::LoggedOut => true,
            Session::LoggedIn { .. } => false,
            Session::Locked { failures } => *failures < 5,
        }
    }
}

let s = Session::Locked { failures: 3 };
println!("{}", s.can_retry());   // true
```

关键在于「每个状态只带自己那份数据」：`user` 只可能出现在已登录状态，`failures` 只可能出现在锁定状态，不存在「未登录却有用户名」这种组合。若用 `logged_in: bool` 加 `user: String`，就得靠约定和注释保证「未登录时 `user` 是空串」，任何一处忘记同步就出现非法状态；而且加第三、第四种状态时，`bool` 再也表达不了。`match self` 里 `Session::Locked { failures }` 绑到的是 `&u32`，所以比较时要写 `*failures`。

</details>

## 延伸阅读

- The Book：[第 5 章 Structs](https://doc.rust-lang.org/book/ch05-00-structs.html)、[第 6 章 Enums and Pattern Matching](https://doc.rust-lang.org/book/ch06-00-enums.html)——口径参照，不逐段对应。
- Reference：[Patterns](https://doc.rust-lang.org/reference/patterns.html)（模式全集与绑定模式）、[Enumerations](https://doc.rust-lang.org/reference/items/enumerations.html)、[Match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)。
- 标准库：[`std::option::Option`](https://doc.rust-lang.org/std/option/enum.Option.html)——注意它自己就 derive 了一长串能力。
- Edition Guide：[let chains（2024）](https://doc.rust-lang.org/edition-guide/rust-2024/let-chains.html)、[match ergonomics 的 edition 2024 变更](https://doc.rust-lang.org/edition-guide/rust-2024/match-ergonomics.html)。
