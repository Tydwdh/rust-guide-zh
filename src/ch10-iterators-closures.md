# 第 10 章 迭代器与闭包

## 这一章解决什么问题

上一章的关键词是「契约」：泛型让一份代码适配多种类型，trait 描述类型必须提供的行为。这一章把这两件工具用到最常见的两个场景上。

第一个场景是**遍历**。`for i in 0..v.len()` 这种写法在 Rust 里不只是啰嗦：筛选、变换、求和本来可以各自描述、再组合起来，手写循环却把它们压成一坨共享状态。迭代器（iterator）把「逐个取元素」定成一条协议，标准库在这条协议上提供了几十个方法（`map`、`filter`、`sum`…），于是「做什么」和「怎么循环」分了家。

第二个场景是**把一段行为当参数传**。很多 API 要的不是数据，而是「拿到数据之后怎么做」：`Vec::sort_by` 要比较规则，`Option::unwrap_or_else` 要兜底逻辑，`thread::spawn` 要任务入口。闭包（closure）是能捕获外部变量的匿名函数，它让这类参数写起来像表达式。

读完这一章，你能做到：

- 写出闭包，并说清它捕获了什么、是借用还是搬走了所有权；
- 看到 `F: Fn` / `FnMut` / `FnOnce` 知道该写哪个、为什么；
- 用迭代器链替换手写循环，并解释为什么链不产生中间集合、也不比循环慢；
- 判断什么时候该退回显式循环。

## 为什么这样设计

### 迭代器是「拉」模型

迭代器自己不会跑。它只是一份「可以一个个取元素」的协议：`next()` 返回 `Option<Item>`，给出 `None` 就表示取完了。谁拿着迭代器，谁决定什么时候要下一个 —— 这是拉（pull）模型。

好处是两头都松：数据源可以昂贵（读文件、走网络）也可以无限（事件流），下游只取需要的那部分；步骤之间随意拼装，因为每一步都只是「把上游再包一层」。换成推（push）模型 —— 框架决定何时调你的回调 —— 就没法说「我只要前 3 个」。

### 闭包 = 函数 + 捕获的环境

`fn` 只能看到参数。闭包额外看到调用点周围的局部变量，于是语言必须先回答：闭包**怎么拿到**这些变量？答案有三档，按闭包对变量的权限从弱到强：共享借用、可变借用、直接拿走所有权。这三档对应三个 trait —— `Fn`、`FnMut`、`FnOnce` —— 它们描述的不是「闭包是什么」，而是「调用者需要什么权限才能调用它」。（C++ / Java / Python 的捕获规则，以及它们的迭代器与流的惰性，见章末「语言对照角」。）

### 零成本从哪来

`map`、`filter` 背后是普通泛型结构体（`std::iter::Map<I, F>` 之类）：适配器把上游迭代器存进自己的字段，`next()` 只转发一次，整条链就是一层套一层的套娃类型。第 9 章讲过单态化：每个具体类型生成一份专门代码，`next()` 是能内联的小函数 —— 没有虚调用，也没有中间集合。链的性能直觉就是「它编译成你手写的那圈循环」（第 29 章讲怎么测）。

## 机制与规则

### 闭包语法

```rust
fn main() {
    let add = |a: i32, b: i32| a + b;      // 参数类型可以标注
    let double = |x| x * 2;                // 有上下文时可以全省
    let announce = || println!("开始");    // 没有参数也要写 ||
    let answer = || -> i32 { 42 };         // 要写返回类型时必须加花括号

    println!("{} {} {}", add(1, 2), double(3), answer());
    announce();
}
```

输出（Rust 1.98 实测）：

```text
3 6 42
开始
```

每个闭包都有编译器生成的独有匿名类型，两个长得一样的闭包属于不同类型，也不能互相赋值。参数与返回类型通常能从上下文推断出来：传给泛型函数、赋值给带类型的变量都算上下文；推断不出来时补标注，要写返回类型时必须同时写花括号。

### 三种捕获

闭包怎么拿到外部变量，由**闭包体怎么用它**决定，编译器只取最小的那一档：

```rust
fn main() {
    // 1) 只读：共享借用捕获
    let name = String::from("rust");
    let len = || name.len();
    println!("借用捕获 len = {}，name 仍可用：{name}", len());

    // 2) 要改：可变借用捕获
    let mut count = 0;
    let mut bump = || {
        count += 1;
        count
    };
    bump();
    println!("可变借用捕获 bump() = {}", bump());
    println!("外部看到 count = {count}");

    // 3) 要搬走：move
    let data = vec![1, 2, 3];
    let take = move || data.len();
    println!("move 捕获 = {}", take());
    // println!("{data:?}"); // 取消注释会报错：data 已经被搬进闭包
}
```

输出：

```text
借用捕获 len = 4，name 仍可用：rust
可变借用捕获 bump() = 2
外部看到 count = 2
move 捕获 = 3
```

三点细节：

- 第 2 段里闭包持有 `count` 的可变借用，所以闭包变量本身要写 `let mut bump`；调用结束后 `count` 从 2（`bump()` 调了两次）被外部读到，说明闭包改的就是它捕获的那个变量。
- 第 3 段里 `move` 把 `data` 的所有权搬进闭包，此后原来的 `data` 不能再用 —— 这也是编译器在借用检查阶段拦下的，不是运行时的事。
- `move` 常被误解成「让闭包变成 `FnOnce`」。它只决定**怎么拿**，不决定**能调几次**：`move || s.len()` 只是读它搬进来的 `s`，仍然是 `Fn`，可以反复调用；如果 `move` 捕获的值都是 `Copy` 的，闭包本身也是 `Copy`。

### `Fn` / `FnMut` / `FnOnce`：为什么是三个

| trait | 调用时对闭包做什么 | 什么样的闭包实现它 |
|---|---|---|
| `FnOnce` | 拿走闭包（按值），调用后闭包不可再用 | 所有闭包 |
| `FnMut` | 可变借用闭包，调用后还能用 | 不会搬走捕获值的闭包 |
| `Fn` | 共享借用闭包，调用后还能用 | 只读使用捕获值的闭包 |

三者的关系是父 trait（标准库里的定义，省去不稳定约束与属性）：

```rust,ignore
pub trait FnOnce<Args> { type Output; fn call_once(self, args: Args) -> Self::Output; }
pub trait FnMut<Args>: FnOnce<Args> { fn call_mut(&mut self, args: Args) -> Self::Output; }
pub trait Fn<Args>: FnMut<Args> { fn call(&self, args: Args) -> Self::Output; }
```

`Fn` 是 `FnMut` 的子 trait、`FnMut` 是 `FnOnce` 的子 trait，所以「实现 `Fn` 的闭包也实现 `FnMut` 和 `FnOnce`」。方向决定了签名的接收面：

- 写 `F: FnOnce()` 接收面最宽：`Fn`、`FnMut`、`FnOnce` 闭包和函数指针都能传；
- 写 `F: Fn()` 最窄：只有只读闭包（和函数指针）能传；
- 边界同时是你对调用者的承诺：`F: Fn` 意味着「我不会改它，也不会消耗它」。

下图把这三个 trait 的包含关系和「边界写在哪一层」并在一起：

<svg viewBox="0 0 640 210" width="100%" role="img" aria-label="Fn、FnMut、FnOnce 三个 trait 的包含关系：内层闭包集合更小，边界写在外层时接收面最宽" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">三个 trait 的包含关系：内层集合小、外层集合大</text>
<ellipse cx="158" cy="113" rx="150" ry="86" fill="none" stroke="var(--quote-border)" stroke-width="1.5" />
<ellipse cx="158" cy="113" rx="106" ry="60" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<ellipse cx="158" cy="113" rx="62" ry="32" fill="none" stroke="var(--links)" stroke-width="2" />
<g text-anchor="middle" fill="currentColor">
<text x="158" y="42" font-size="13">FnOnce</text>
<text x="158" y="66" font-size="13">FnMut</text>
<text x="158" y="103" font-size="13">Fn</text>
<text x="158" y="125" font-size="12" opacity="0.75">只读使用捕获值</text>
</g>
<g fill="currentColor">
<text x="336" y="40" font-size="13">边界写在哪一层</text>
<text x="336" y="64" font-size="12" font-family="monospace">F: FnOnce()</text>
<text x="432" y="64" font-size="12">最宽 —— 什么闭包都能传</text>
<text x="336" y="86" font-size="12" font-family="monospace">F: FnMut()</text>
<text x="432" y="86" font-size="12">居中 —— 能改环境的能传</text>
<text x="336" y="108" font-size="12" font-family="monospace">F: Fn()</text>
<text x="432" y="108" font-size="12">最窄 —— 只有只读闭包能传</text>
<text x="336" y="144" font-size="12" opacity="0.85">内层的闭包一定也在外层里，</text>
<text x="336" y="164" font-size="12" opacity="0.85">所以边界越靠内、接收面越窄。</text>
<text x="336" y="188" font-size="12" opacity="0.6">F: Fn 的承诺 —— 不改它，也不消耗它</text>
</g>
</svg>

这就是「为什么是三个」的答案：**最小权限**。需求方只要求自己真正需要的能力，供给方保留的能力越多（能改、能消耗），能通过边界的闭包就越多。一个 `FnMut` 边界加上一次调用，就等于 `FnOnce`；一个 `Fn` 边界加上两次调用，就是在说「这段代码不修改也不消耗环境」。

三个 trait 的调用方法 `call` / `call_mut` / `call_once` 在 stable 上不能直接调用（`` error[E0658]: use of unstable library feature `fn_traits` ``）；日常一律写调用语法 `f(x)`，编译器替你选对是哪一个。

谁实现哪个 trait，由闭包体决定，而不是由你想怎么调决定。同一件事可以写出三种闭包：

```rust
fn call_many_times<F: Fn(u32) -> u32>(f: F, n: u32) -> u32 {
    f(n) + f(n + 1) // 需要反复按共享方式调用
}

fn call_mut_twice<F: FnMut() -> u32>(mut f: F) -> (u32, u32) {
    (f(), f()) // 需要可变地调用两次
}

fn call_once<F: FnOnce() -> String>(f: F) -> String {
    f() // 只会调用一次，于是允许它搬走捕获值
}

fn main() {
    let bias = 10;
    println!("Fn：{}", call_many_times(|x| x + bias, 1));

    let mut counter = 0;
    let (a, b) = call_mut_twice(|| {
        counter += 1;
        counter
    });
    println!("FnMut：{a}, {b}");

    let owned = String::from("闭包搬走了我");
    let msg = call_once(move || owned);
    println!("FnOnce：{msg}");
}
```

输出：

```text
Fn：23
FnMut：1, 2
FnOnce：闭包搬走了我
```

三个函数分别要求三种能力。`call_many_times` 调用两次但不改环境，所以 `|x| x + bias`（共享借用捕获）合格；`call_mut_twice` 要改状态，`|| { counter += 1; ... }`（可变借用捕获）刚好够；`call_once` 只调一次，于是 `move || owned` 这种会把 `String` 搬出闭包的写法也能进 —— 把边界换成 `F: Fn` 它就会被拒（本章「报错现场」第 3 节是同一家族）。

### 参数里怎么写：`impl Fn`、泛型参数与函数指针

`fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32` 和 `fn apply(f: impl Fn(i32) -> i32, x: i32) -> i32` 是同一件事的两种写法（第 9 章讲过 `impl Trait` 在参数位置的展开）。签名读作：「`f` 是任何能当 `fn(i32) -> i32` 调用的东西」。

函数指针 `fn(i32) -> i32` 是它的具体版本：一个不带环境的调用入口。两者关系：

- 普通函数（如 `fn double(x: i32) -> i32`）本来就实现 `Fn`，可以直接传给 `F: Fn(i32) -> i32` 的位置；
- 不捕获任何环境的闭包可以强转成 `fn` 指针；捕获了环境的就不行 —— 环境没地方放；
- 只有当「同一变量或字段要存不同的可调用对象」时（`Vec<fn(i32) -> i32>`、结构体字段），才需要 `fn` 指针或 `dyn Fn`（第 13 章）；泛型位置能表达时写 `impl Fn`，零开销。

```rust
fn double(x: i32) -> i32 {
    x * 2
}

fn apply(f: impl Fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

fn main() {
    println!("函数名当参数：{}", apply(double, 21));
    let f: fn(i32) -> i32 = |x| x + 1; // 不捕获环境 → 可以强转
    println!("fn 指针：{}", f(41));
}
```

```text
函数名当参数：42
fn 指针：42
```

### `move` 什么时候是必需的

一条规则：**闭包要活过被捕获变量的作用域，就必须把所有权带在身边** —— 借用不能悬空。典型场景是从函数返回闭包，以及把闭包交给别的线程或异步任务（第 19、21 章）。

```rust
fn make_counter() -> impl FnMut() -> u32 {
    let mut n = 0;
    move || {
        n += 1;
        n
    }
}

fn main() {
    let mut c = make_counter();
    println!("{} {} {}", c(), c(), c());
}
```

```text
1 2 3
```

`n` 是 `make_counter` 的局部变量，函数一返回它的借用就该失效；`move` 让闭包把它搬进自己的环境，闭包活多久都安全。去掉 `move`（以 Rust 1.98 为例）：

```rust,compile_fail
fn make_counter() -> impl FnMut() -> u32 {
    let mut n = 0;
    || {
        n += 1;
        n
    }
}
```

```text
error[E0373]: closure may outlive the current function, but it borrows `n`, which is owned by the current function
  |
3 |     || {
  |     ^^ may outlive borrowed value `n`
4 |         n += 1;
  |         - `n` is borrowed here
  |
help: to force the closure to take ownership of `n` (and any other referenced variables), use the `move` keyword
  |
3 |     move || {
  |     ++++
```

反过来，借用能活够久时不必搬：参数里的借用可以直接带走（返回 `impl Iterator` 见第 14 章）。

```rust
fn positives(v: &[i32]) -> impl Iterator<Item = i32> + '_ {
    v.iter().copied().filter(|x| *x > 0)
}

fn main() {
    let v = [-1, 2, -3, 4];
    println!("{:?}", positives(&v).collect::<Vec<_>>());
}
```

```text
[2, 4]
```

### 迭代器：一个必须方法

标准库里的定义只有两样必填的东西：

```rust,ignore
pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;

    // 其余几十个方法都有默认实现：map / filter / sum / collect …
}
```

想让自己写的东西被 `for` 遍历，就实现 `next` —— 这里涉及的关联类型见第 12 章，在函数签名里返回 `impl Iterator` 见第 14 章。`for` 本身是语法糖：

```rust
fn main() {
    let v = vec![1, 2, 3];
    for x in v {
        println!("{x}"); // x 是每次取出的元素
    }

    // 大致等价于：
    let v = vec![1, 2, 3];
    let mut it = IntoIterator::into_iter(v);
    while let Some(x) = it.next() {
        println!("{x}");
    }
}
```

`IntoIterator` 描述「能被 `for` 吃掉」。`Vec<T>`、`&Vec<T>`、`&mut Vec<T>` 都实现了它，所以三种 `for` 取到的东西不同：

```rust
fn main() {
    let mut v = vec![1, 2, 3];

    let mut seen = Vec::new();
    for x in &v {
        seen.push(*x); // x: &i32
    }
    for x in &mut v {
        *x *= 10; // x: &mut i32
    }
    for x in v {
        println!("按值遍历：{x:?}"); // x: i32，v 被搬走
    }
    println!("借用遍历：{seen:?}");
}
```

```text
按值遍历：10
按值遍历：20
按值遍历：30
借用遍历：[1, 2, 3]
```

（`iter` / `into_iter` / `iter_mut` 与这三种写法的对应关系在第 6 章。）

### 适配器与消费者

`Iterator` 上的方法分两类：

- **适配器（adapter）**：接收一个迭代器，返回**另一个迭代器**。它不取元素，只是把上游包起来。`map`、`filter`、`take`、`skip`、`chain`、`zip`、`enumerate`、`flat_map`、`rev`、`copied`、`cloned`、`filter_map`、`step_by` 都是。
- **消费者（consumer）**：真的取出元素，返回一个值（或产生副作用）。`collect`、`sum`、`product`、`count`、`fold`、`reduce`、`any`、`all`、`find`、`position`、`for_each`、`min`、`max`、`last` 都是。

关键在惰性：**适配器什么都不做**，直到有人调用消费者或 `next()`。所以「一条链」是一份待执行的计划，不是数据。

这份「计划」在类型上是一层套一层 —— 以 `v.iter().map(|x| x * 2).filter(|x| x % 2 == 0)` 为例：

<svg viewBox="0 0 640 240" width="100%" role="img" aria-label="迭代器链的套娃类型：Filter 包着 Map，Map 包着源迭代器，next 由外向内逐层穿透" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">一条链的类型：Filter&lt;Map&lt;Iter&lt;'_, i32&gt;, F&gt;, P&gt;</text>
<rect x="14" y="28" width="310" height="204" rx="6" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="26" y="50" font-size="13" font-family="monospace" fill="currentColor">Filter&lt;I, P&gt;</text>
<text x="26" y="70" font-size="12" font-family="monospace" fill="currentColor" opacity="0.75">iter: I</text>
<text x="26" y="90" font-size="12" font-family="monospace" fill="currentColor" opacity="0.75">predicate: P</text>
<rect x="42" y="98" width="254" height="126" rx="6" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" />
<text x="54" y="120" font-size="13" font-family="monospace" fill="currentColor">Map&lt;I, F&gt;</text>
<text x="54" y="140" font-size="12" font-family="monospace" fill="currentColor" opacity="0.75">iter: I</text>
<text x="54" y="160" font-size="12" font-family="monospace" fill="currentColor" opacity="0.75">f: F</text>
<rect x="70" y="168" width="198" height="48" rx="6" fill="none" stroke="var(--links)" stroke-width="2" />
<text x="82" y="188" font-size="13" font-family="monospace" fill="currentColor">Iter&lt;'a, T&gt;</text>
<text x="82" y="208" font-size="12" fill="currentColor" opacity="0.75">真正持有数据来源</text>
<g fill="currentColor">
<text x="340" y="50" font-size="13">next() 由外向内穿透</text>
<text x="352" y="74" font-size="12" font-family="monospace">Filter::next()</text>
<text x="352" y="94" font-size="12" font-family="monospace">→ Map::next()</text>
<text x="352" y="114" font-size="12" font-family="monospace">→ Iter::next()</text>
<text x="368" y="134" font-size="12">只有它真的取元素</text>
<text x="340" y="164" font-size="12" opacity="0.85">每层只存上游 + 自己的闭包，</text>
<text x="340" y="184" font-size="12" opacity="0.85">谁都不建中间集合。</text>
<text x="340" y="214" font-size="12" opacity="0.6">泛型 I / F / P 由单态化落地成具体类型</text>
</g>
</svg>

```rust
fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];

    // 适配器：都返回新的迭代器，原来的 vec 不动
    let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
    let even: Vec<i32> = v.iter().copied().filter(|x| x % 2 == 0).collect();
    let head: Vec<i32> = v.iter().copied().take(3).collect();
    let tail: Vec<i32> = v.iter().copied().skip(3).collect();
    let mixed: Vec<i32> = v.iter().copied().take(2).chain(v.iter().copied().skip(4)).collect();
    let zipped: Vec<(i32, char)> = v.iter().copied().zip("abc".chars()).collect();
    let pairs: Vec<(usize, i32)> = v.iter().copied().enumerate().take(2).collect();
    let chars: Vec<char> = ["hello world", "rust guide"]
        .iter()
        .flat_map(|s| s.chars())
        .collect();

    // 消费者：把迭代器吃掉，得到一个值
    let sum: i32 = v.iter().sum();
    let product: i32 = v.iter().fold(1, |acc, x| acc * x);
    let any_big = v.iter().any(|x| *x > 5);
    let all_positive = v.iter().all(|x| *x > 0);
    let first_even = v.iter().find(|x| *x % 2 == 0);

    println!("map      {doubled:?}");
    println!("filter   {even:?}");
    println!("take     {head:?}");
    println!("skip     {tail:?}");
    println!("chain    {mixed:?}");
    println!("zip      {zipped:?}");
    println!("enumerate {pairs:?}");
    println!("flat_map {}", chars.iter().collect::<String>());
    println!("sum {sum} / fold {product}");
    println!("any(>5) {any_big} / all(>0) {all_positive} / find(even) {first_even:?}");
}
```

```text
map      [2, 4, 6, 8, 10, 12]
filter   [2, 4, 6]
take     [1, 2, 3]
skip     [4, 5, 6]
chain    [1, 2, 5, 6]
zip      [(1, 'a'), (2, 'b'), (3, 'c')]
enumerate [(0, 1), (1, 2)]
flat_map hello worldrust guide
sum 21 / fold 720
any(>5) true / all(>0) true / find(even) Some(2)
```

容易踩的点：

- `Vec` 本身没有 `map`（`v.map(...)` 报 E0599），要先 `v.iter()` 或 `into_iter()` 拿到迭代器。
- `v.iter()` 的元素是 `&i32`，`filter` / `find` 的闭包又收到 `&&i32`，直接做算术会报 E0369。三种处理：手工解引用（`|x| **x % 2 == 0`）、模式匹配掉一层（`|&x| x % 2 == 0`）、或先 `copied()` / `cloned()`；本章示例多用第三种。另外 `zip` 以较短的迭代器为准，`flat_map` 是「先 `map` 再摊平一层」。

### `collect` 与类型推断

`collect` 的签名（标准库）：

```rust,ignore
fn collect<B: FromIterator<Self::Item>>(self) -> B
```

它自己不知道要收集成什么，目标类型由上下文决定。两种等价写法：

```rust
fn main() {
    let nums = [1, 2, 3, 4];
    let a: Vec<i32> = nums.iter().map(|x| x * 2).collect(); // 靠变量类型
    let b = nums.iter().map(|x| x * 2).collect::<Vec<_>>(); // 靠 turbofish
    let s: String = ['r', 'u', 's', 't'].iter().collect();
    let parsed: Result<Vec<i32>, _> = ["1", "2", "x"].iter().map(|t| t.parse::<i32>()).collect();
    println!("{a:?} {b:?} {s} {parsed:?}");
}
```

```text
[2, 4, 6, 8] [2, 4, 6, 8] rust Err(ParseIntError { kind: InvalidDigit })
```

`collect::<Vec<_>>()` 里的 `_` 是「元素类型你接着推断」，它只管收成 `Vec`，元素仍可由上下文定下来。目标类型能有很多种：`Vec`、`String`、`HashMap`（从 `(键, 值)` 对收集，但遍历顺序不保证，别写依赖顺序的代码）、`BTreeMap`、`VecDeque`。

`Result` 与 `Option` 的收集会**短路**：上面 `parsed` 的类型是 `Result<Vec<i32>, _>`，一遇失败就停 —— 这正是「一串可能失败的操作」的惯用写法（第 7 章讲过 `?`）。要保留每个元素的结果，就收集成 `Vec<Result<T, E>>`：两者只差你标的类型。

## 动手看看

### 1. 惰性：不消费，什么都不发生

```rust
fn main() {
    let v = vec![1, 2, 3];
    let mut it = v.iter().map(|x| {
        println!("map 执行：{x}");
        x * 2
    });
    println!("--- 到这里还没消费，map 一次都没跑 ---");
    println!("next() = {:?}", it.next());
    println!("--- 消费了一次 ---");
    let rest: Vec<i32> = it.collect();
    println!("剩下的：{rest:?}");
}
```

```text
--- 到这里还没消费，map 一次都没跑 ---
map 执行：1
next() = Some(2)
--- 消费了一次 ---
map 执行：2
map 执行：3
剩下的：[4, 6]
```

`next()` 只推进一格，`collect` 才把剩下的取空。在 `map` 里打印只是为了把「惰性」看见；真要产生副作用，用 `for_each` 更清楚。

忘了消费时编译器会提醒（Rust 1.98 实测）：

```rust
fn main() {
    let v = vec![1, 2, 3];
    v.iter().map(|x| println!("{x}"));
}
```

```text
warning: unused `Map` that must be used
 --> src/main.rs:3:5
  |
3 |     v.iter().map(|x| println!("{x}"));
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: iterators are lazy and do nothing unless consumed
  ...
warning: `Iterator::map` call that discard the iterator's values
 --> src/main.rs:3:14
  |
3 |     v.iter().map(|x| println!("{x}"));
  |              ^^^^-------------------^
  |              this function returns `()`, which is likely not what you wanted
  ...
help: you might have meant to use `Iterator::for_each`
  |
3 |     v.iter().for_each(|x| println!("{x}"));
  |             ~~~~~~~~
  |
```

### 2. 一条链是「每个元素流过全程」

```rust
fn main() {
    let v = vec![1, 2, 3, 4];
    let out: Vec<i32> = v
        .iter()
        .map(|x| {
            println!("  map({x})");
            x * 2
        })
        .filter(|x| {
            println!("  filter({x})");
            x % 4 == 0
        })
        .collect();
    println!("结果 {out:?}");
}
```

```text
  map(1)
  filter(2)
  map(2)
  filter(4)
  map(3)
  filter(6)
  map(4)
  filter(8)
结果 [4, 8]
```

下图把「逐个穿过」和「每步建中间 `Vec`」两种执行方式并排：

<svg viewBox="0 0 640 210" width="100%" role="img" aria-label="对比：迭代器链让每个元素逐个穿过全部步骤、只建一个结果集合，而每步先建中间 Vec 的写法要分配两条临时 Vec" xmlns="http://www.w3.org/2000/svg">
<line x1="318" y1="4" x2="318" y2="200" stroke="var(--quote-border)" stroke-width="1.5" stroke-dasharray="4 3" />
<text x="0" y="16" font-size="14" fill="currentColor">迭代器链：逐个穿过</text>
<text x="336" y="16" font-size="14" fill="currentColor">每步建中间 Vec：整批进出</text>
<g font-size="13" font-family="monospace" fill="currentColor" text-anchor="middle">
<rect x="4" y="40" width="80" height="38" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" /><text x="44" y="64">map</text>
<rect x="106" y="40" width="80" height="38" rx="4" fill="none" stroke="currentColor" stroke-opacity="0.55" stroke-width="1.5" /><text x="146" y="64">filter</text>
<rect x="208" y="40" width="80" height="38" rx="4" fill="none" stroke="var(--links)" stroke-width="2" /><text x="248" y="64">collect</text>
</g>
<g font-size="14" fill="currentColor" text-anchor="middle" opacity="0.7">
<text x="95" y="67">→</text>
<text x="197" y="67">→</text>
</g>
<g font-size="12" fill="currentColor">
<text x="0" y="102">元素 1 走完全程，元素 2 才进来</text>
<text x="0" y="124">next() 一次只推进一格</text>
<text x="0" y="146">结果集合只有 collect 建的那一个</text>
<text x="0" y="174" opacity="0.7">适配器自身 0 次分配</text>
</g>
<g fill="currentColor" stroke="currentColor">
<rect x="336" y="36" width="280" height="32" rx="4" fill="none" stroke-opacity="0.55" stroke-width="1.5" />
<rect x="336" y="92" width="280" height="32" rx="4" fill="none" stroke-opacity="0.55" stroke-width="1.5" />
<rect x="336" y="148" width="280" height="32" rx="4" fill="none" stroke="var(--links)" stroke-width="2" />
</g>
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="348" y="57">v</text><text x="420" y="57">[1, 2, 3, 4]</text>
<text x="348" y="113">tmp1</text><text x="420" y="113">[2, 4, 6, 8]</text>
<text x="348" y="169">结果</text><text x="420" y="169">[4, 8]</text>
</g>
<g font-size="14" fill="currentColor">
<text x="348" y="85">↓</text>
<text x="348" y="141">↓</text>
</g>
<g font-size="12" font-family="monospace" fill="currentColor">
<text x="366" y="85">map</text>
<text x="366" y="141">filter</text>
</g>
<g font-size="12" fill="currentColor" opacity="0.75">
<text x="430" y="85">新分配一个 Vec</text>
<text x="430" y="141">又一条临时 Vec</text>
</g>
<text x="336" y="198" font-size="12" fill="currentColor" opacity="0.7">两条临时 Vec 建好就丢，白跑两遍内存</text>
</svg>

看顺序：不是「先跑完全部 `map` 再跑 `filter`」，而是每个元素依次穿完整条链。所以链**不需要中间集合** —— 没有临时 `Vec` 被建出来又丢掉，`collect` 只建一个结果集合。这也是它和手写循环能编译成同一件事的原因。

### 3. 组合实战：日志过滤，两种写法对照

需求：给一段日志，数出 `ERROR` 行的条数，并列出它们的时间戳。先手写循环，再用迭代器链改写，两边共用同一个解析函数，末尾用 `assert_eq!` 钉住结果一致。完整程序在 `snippets/ch10/log_stats.rs`：

```rust
{{#rustdoc_include ../snippets/ch10/log_stats.rs}}
```

在仓库根目录跑：

```console
$ rustc --edition 2024 snippets/ch10/log_stats.rs -o log_stats
$ ./log_stats
```

输出：

```text
ERROR 2 条，时间：09:02:03、09:02:04
```

对照着看：`imperative` 用可变绑定加 `push` 累积，`chained` 把同样的判断写成 `.map(parse).filter(…).collect()`，两边共用 `parse` —— 改写换掉的只是「控制流怎么表达」。`chained` 里 `errors` 是 `Vec<(&str, &str)>`，第二次遍历只取时间戳；元素多、字段大时可以用一次 `fold` 同时算出条数和时间戳，省掉中间 `Vec`（值不值得，第 29 章讲怎么量）。`parse` 里的 `unwrap` 成立是因为示例数据格式有保证；真实程序应当返回 `Option` / `Result`（第 7 章）。

### 4. 零成本回顾

单态化之后，「迭代器链 vs 手写循环」多数时候不是性能取舍，而是表达方式取舍：链把「做什么」按顺序排开，循环把「怎么做」摊平成状态。

两个别吹过头的地方：

- 链本身不引入开销，链里的操作会有：`collect` 会真的分配；`Box<dyn Iterator>` 引入间接调用（第 13 章）；闭包要**持有** `Box` / `Rc` 时，创建它们的那一次分配照样发生（捕获本身不分配）。
- 想确认某条链和手写循环一样快，别猜：`--release` 编译后用基准工具量（第 29 章会给方法：criterion、`std::hint::black_box`、看汇编）。

### 5. 什么时候用链、什么时候用循环

用链，当任务是「一批元素的流水线」（筛选 → 变换 → 汇总），每一步都能用现成的适配器 / 消费者表达（`map` / `filter` / `sum` / `max_by_key`…），且控制流简单 —— 不需要 `break` / `continue`、多层嵌套或跨步骤的中间状态。

用显式循环，当需要 `break` / `continue` 或多个出口；循环体里有大量中间状态（多个累积变量、还要维护索引）；要边遍历边改同一个结构（借用规则会拦下，先收集要改的位置往往更清楚）；或者链要套很多层闭包，比一个五行循环还难读。

判断标准只有一条：**读者能不能一眼看出这段代码在做什么。** 链是为了可读性，不是为了短。

## 报错现场

下面几条都是真实编译结果，以 Rust 1.98 为例（只保留关键行，路径已略去）；`rust,compile_fail` 的代码块正是「必须编译失败」的最小复现。

### 1. 闭包捕获与借用冲突：E0502 / E0499

```rust,compile_fail
fn main() {
    let mut v = vec![1, 2, 3];
    let first = || v[0];
    v.push(4);
    println!("{}", first());
}
```

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
  |
3 |     let first = || v[0];
  |                 -- - first borrow occurs due to use of `v` in closure
  |                 |
  |                 immutable borrow occurs here
4 |     v.push(4);
  |     ^^^^^^^^^ mutable borrow occurs here
5 |     println!("{}", first());
  |                    ----- immutable borrow later used here
```

`|| v[0]` 捕获了 `v` 的共享借用，这个借用从闭包创建处一直活到它最后一次被调用；中间的 `v.push(4)` 需要可变借用，于是撞上第 4 章的规则（同一时刻要么多个共享借用，要么一个可变借用）。编译器把三处都标了出来：借用起点、冲突点、最后一次使用。

两个修法：

- **让借用提前结束**：先用掉闭包（`println!("先取 {}", first());`）再 `v.push(4)`；闭包不再被使用，借用在 NLL 下就结束了。
- **别让闭包捕获**：`let zero = v[0];` 先把值存下来（`i32` 是 `Copy`），或者改成显式参数的函数式写法 `|v: &Vec<i32>| v[0]`。

同一家族的另一位是 E0499：两个闭包同时要可变借用同一个变量。

```rust,compile_fail
fn main() {
    let mut v = vec![1, 2, 3];
    let mut a = || v.push(1);
    let mut b = || v.push(2);
    a();
    b();
}
```

```text
error[E0499]: cannot borrow `v` as mutable more than once at a time
  |
3 |     let mut a = || v.push(1);
  |                 -- - first borrow occurs due to use of `v` in closure
  |                 |
  |                 first mutable borrow occurs here
4 |     let mut b = || v.push(2);
  |                 ^^ - second borrow occurs due to use of `v` in closure
5 |     a();
  |     - first borrow later used here
```

修法：合并成一个闭包；或者让闭包不捕获同一个变量（改用参数传入）；再不行才上 `RefCell`（第 17 章）—— 能用借用解决的地方别急着上它。

### 2. `collect` 不知道收成什么：E0283

```rust,compile_fail
fn main() {
    let v = vec![1, 2, 3];
    let out = v.iter().map(|x| x * 2).collect();
    println!("{out:?}");
}
```

```text
error[E0283]: type annotations needed
  |
3 |     let out = v.iter().map(|x| x * 2).collect();
  |         ^^^                           ------- type must be known at this point
  |
  = note: the type must implement `FromIterator<i32>`
help: consider giving `out` an explicit type
```

`collect` 的目标类型只能从上下文来；这里 `out` 之后只用于打印，没有约束，编译器就停下来问。两种修法任选：

```rust
fn main() {
    let v = vec![1, 2, 3];
    let a: Vec<i32> = v.iter().map(|x| x * 2).collect();
    let b = v.iter().map(|x| x * 2).collect::<Vec<_>>(); // _ 只留给元素类型
    println!("{a:?} {b:?}");
}
```

同一个错误也会出现在**元素类型**上：`str::parse` 的返回类型取决于上下文，直接当函数传就没有任何线索。

```rust,compile_fail
fn main() {
    let text = "1 2 3";
    let nums = text.split(' ').map(str::parse).collect::<Vec<_>>();
    println!("{nums:?}");
}
```

```text
error[E0283]: type annotations needed
  |
3 |     let nums = text.split(' ').map(str::parse).collect::<Vec<_>>();
  |                                    ^^^^^^^^^^ cannot infer type of the type parameter `F` declared on the method `parse`
  |
  = note: the type must implement `FromStr`
help: consider specifying a concrete type for the type parameter `F`
```

修法是给 `parse` 指定目标类型：写成 `map(str::parse::<i32>)`（元素于是变成 `Result<i32, ParseIntError>`，按上一段的写法再收成 `Result<Vec<i32>, _>`），或者 `map(|t| t.parse::<i32>())`。

### 3. `FnOnce` 闭包被调用两次：E0382

```rust,compile_fail
fn call_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}

fn main() {
    let s = String::from("只能搬走一次");
    let g = move || s;
    println!("{}", call_once(g));
    println!("{}", call_once(g));
}
```

```text
error[E0382]: use of moved value: `g`
  |
8 |     println!("{}", call_once(g));
  |                              - value moved here
9 |     println!("{}", call_once(g));
  |                              ^ value used here after move
  |
note: closure cannot be moved more than once as it is not `Copy` due to moving the variable `s` out of its environment
```

`move || s` 会把 `s` 搬出闭包，这种闭包只实现 `FnOnce`；`call_once(g)` 按值接收 `f`，第一次调用就把 `g` 消耗掉了。两个修法：

- 不需要消耗就让闭包只借用：`let g = || s.clone();` —— 此时捕获的是 `&s`，闭包本身是 `Copy`，可以反复传。
- 确实只调用一次，就别写第二次。

> [!NOTE]
> 「能重复调用」与「把捕获值搬出去」不可能同时满足：给 `F: FnMut` 的位置传 `|| drop(s)` 会报 E0507（`` cannot move out of `s`, a captured variable in an `FnMut` closure ``），提示里直接写着 `Fn` 和 `FnMut` 闭包要求捕获值能被多次消耗。

## 小结

- 闭包 = 匿名函数 + 捕获的环境。捕获方式（共享借用 / 可变借用 / 搬走）由闭包体的用法推断；`move` 只强制第三档，不改变能调用几次。
- `Fn` / `FnMut` / `FnOnce` 描述「调用者需要什么权限才能调用」；关系是 `Fn: FnMut: FnOnce`，所以边界写 `Fn` 最紧、接收面最小，写 `FnOnce` 最宽。
- 参数位置写 `impl Fn(T) -> U` 就是「泛型 + 单态化」；`fn(T) -> U` 是不带环境的具体版本，只在需要同一类型存放时才用。
- 迭代器只需要实现一个 `next`；`for` 是 `IntoIterator` + `while let` 的语法糖。
- 适配器返回新迭代器、不取元素（惰性），消费者返回一个值；不消费就什么都不发生。`collect` 的目标类型由上下文决定，`Result` / `Option` 的收集会短路。
- 迭代器链是零成本抽象，也是可读性工具；控制流一复杂，就退回显式循环。

## 语言对照角

**闭包捕获**

- **C++**：捕获方式要显式写进方括号（`[&]` 按引用、`[=]` 按值、`[i, &v]` 混写），按值捕获要改得加 `mutable`；搬 move-only 对象进去用 C++14 的初始化捕获 `[p = std::move(p)]`。按引用捕获的 lambda 活过被引用对象就是悬垂引用（UB），编译器不拦；不捕获的 lambda 能隐式转成函数指针，这点和 Rust 一致。
- **Java**：lambda 只能捕获 effectively final 的局部变量，捕获的是那份值（对象是引用的副本）；想在里面改外部计数器，得绕道数组或 `AtomicInteger`。
- **Python**：闭包按变量捕获（后期绑定），`[lambda: i for i in range(3)]` 里的三个 lambda 都返回 2；内存回收交给 GC，不存在悬垂。
- **Rust**：不写捕获列表，编译器按闭包体的用法在共享借用 / 可变借用 / 拿走所有权里选最小一档（`move` 是「一律搬走」的开关）；因为捕获就是借用或移动，「闭包活过被捕获变量」是编译错误。

**迭代器与惰性**

- **C++**：惰性、可组合的视图（`std::views::filter` / `transform`）是 C++20 才有的，形态最接近 Rust 适配器；此前的主流写法是 `<algorithm>` 配输出迭代器（`std::transform(begin, end, out)`），每步都要指定输出位置。视图同样不拥有数据，悬垂的坑还在。
- **Java**：流的模型与 Rust 迭代器几乎一致 —— `filter` / `map` 只搭流水线，`collect` / `sum` / `forEach` 才驱动执行；流是一次性的（复用抛 `IllegalStateException`），对象流还有装箱开销，靠 `IntStream` 等特化类型规避。
- **Python**：Python 3 的 `map` / `filter` / 生成器表达式是惰性的，`itertools` 提供 `chain` / `takewhile`；列表推导则直接建出列表。
- **Rust**：和 Java 流一样惰性、一样一次性（被 `collect` 吃掉后不能复用），但数据源可以反复借用（`&v` 每次都能 `.iter()`）。适配器是泛型结构体，单态化后没有虚调用和装箱 —— 惰性不带来运行时代价。

## 练习

### 1. 预测编译结果与输出

```rust
fn main() {
    let nums = vec![1, 2, 3, 4];
    let mut count = 0;
    let mut tally = |x: i32| {
        if x % 2 == 0 {
            count += 1;
        }
    };
    nums.iter().copied().for_each(&mut tally);
    println!("偶数 {count} 个");
}
```

能编译吗？打印什么？把 `&mut tally` 改成 `tally` 又会怎样？

### 2. 把循环改成链

```rust
fn main() {
    let words = vec!["rust", "guide", "中文书"];
    // 求「字符数大于 4」的单词的字符数之和
    let mut total = 0;
    for w in &words {
        let n = w.chars().count();
        if n > 4 {
            total += n;
        }
    }
    println!("{total}");
}
```

用一条迭代器链写出等价代码，并说明为什么这里用 `chars().count()` 而不是 `len()`。

### 3. 找错

```rust,compile_fail
fn apply_twice<F: FnMut()>(mut f: F) {
    f();
    f();
}

fn main() {
    let s = String::from("hi");
    apply_twice(|| drop(s));
}
```

编译会失败在哪？为什么？给出至少一种修法。

<details><summary>参考答案</summary>

**1.** 能编译，打印 `偶数 2 个`。`tally` 的闭包体改了 `count`，所以是 `FnMut`；`for_each` 要 `F: FnMut(Self::Item)`，传 `&mut tally` 也合法（标准库为 `&mut F` 实现了 `FnMut`）。事后读 `count` 也合法：`tally` 最后一次使用就在那一行，NLL 在那里结束了它的可变借用。

改成 `tally`（按值传）同样能编译：所有权交给 `for_each`，借用在调用结束时结束；区别是之后不能再用 `tally`，且编译器会提醒 `mut` 多余。

**2.** 等价写法：

```rust
fn main() {
    let words = vec!["rust", "guide", "中文书"];
    let total: usize = words
        .iter()
        .map(|w| w.chars().count())
        .filter(|n| *n > 4)
        .sum();
    println!("{total}");
}
```

`len()` 返回的是字节数：UTF-8 下一个汉字占 3 字节，`"中文书"` 的 `len()` 是 9，而 `chars().count()` 是 3。要「字符数」就必须逐字符数（第 6 章）。这里的链会得到 5（只有 `"guide"` 的 5 个字符超过 4）。

**3.** 错在 `apply_twice(|| drop(s))`：`` error[E0507]: cannot move out of `s`, a captured variable in an `FnMut` closure ``（以 Rust 1.98 为例）。`apply_twice` 要求 `F: FnMut`，而 `FnMut` 意味着闭包可能被调用多次，所以闭包体不能把捕获的 `String` 消耗掉 —— `drop(s)` 正是消耗。

修法二选一：`apply_twice(|| drop(s.clone()))`（闭包只共享借用 `s`，每次克隆一份再丢弃），或者把接口改成只调一次：`fn apply_once<F: FnOnce()>(f: F) { f() }`。

</details>

## 延伸阅读

- `Iterator` 的所有适配器与消费者：<https://doc.rust-lang.org/std/iter/trait.Iterator.html>
- 迭代器总览与 `IntoIterator`：<https://doc.rust-lang.org/std/iter/index.html>
- `Fn` / `FnMut` / `FnOnce`：<https://doc.rust-lang.org/std/ops/trait.Fn.html>、<https://doc.rust-lang.org/std/ops/trait.FnMut.html>、<https://doc.rust-lang.org/std/ops/trait.FnOnce.html>
- 闭包表达式（Reference）：<https://doc.rust-lang.org/reference/expressions/closure-expr.html>
- The Book 第 13 章「函数式语言特性：迭代器与闭包」：<https://doc.rust-lang.org/book/ch13-00-functional-features.html>
