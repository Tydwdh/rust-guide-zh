# 第 4 章 所有权、借用与切片

## 这一章解决什么问题

前三章你已经能写出跑得起来的程序。但从这里开始，一碰 `String` 就会遇到两种「按理说不该出问题」的情况：

- 把一个变量赋值给另一个变量，原来那个居然不能用了；
- 想往 `Vec` 里加一个元素，编译器却说这个值「已经被借走了」。

在 Python、Java、Go 里这两件事都不会发生：变量名就像贴在对象上的标签，`a = b` 之后两个名字随便用，回收交给垃圾回收器（GC）。Rust 换了一套规则——这套规则就是本书后面所有内容的地基。

读完这一章，你应该能做到：

- 说出所有权的三条规则，并解释 Rust 为什么这样设计（对比 GC 语言与手动管理）；
- 判断一次赋值、传参、返回会不会**移动**值，移动之后哪个变量作废；
- 说清 `&T` 与 `&mut T` 的别名规则，以及「同一时刻」究竟指哪一段时间；
- 画出 `String`、`Vec<T>`、`&str`、`&[T]`、`str`、`[T]` 之间的内存关系；
- 拿到 `E0382`、`E0499`、`E0502`、`E0515` 时知道先看什么、往哪里改。

> [!NOTE]
> 本章所有代码与输出都来自 Rust 1.98（edition 2024，x86_64 Windows）的实测。「1.98 实测」的输出来自本机 `rustc --edition 2024`，文件统一放在 `src/main.rs`。

## 为什么这样设计

内存什么时候释放，主流语言只有两条路可走：

| 方案 | 谁决定释放 | 何时决定 | 运行时开销 | 典型事故 |
|---|---|---|---|---|
| 垃圾回收（Java / Go / Python / JS） | 运行时 GC | 程序运行中 | GC 暂停、内存占用偏高 | 泄漏（对象可达但不用）、延迟抖动 |
| 手动管理（C / C++） | 写代码的人 | 写代码时 | 无 | 忘记释放、重复释放、释放后继续用 |
| 编译期所有权（Rust） | 编译器 | 编译时 | 无 | 编译不过 |

第三条不是「更聪明的 GC」，而是把「谁该释放」从运行时搬进类型系统：让释放点在编译期就唯一确定，于是「释放后还用」和「释放两次」在安全代码里根本表达不出来。

### 三条规则

Rust 用三条规则把这件事说清楚：

1. **每个值在任一时刻只有一个所有者（owner）**；所有者是唯一负责释放它的地方。
2. **所有者离开作用域，值就被释放**（drop，即运行析构代码）。
3. **值可以在所有者之间转移**（这一动作叫**移动（move）**）；不能移动时，只能通过**借用（borrow）**访问，而借用受别名规则约束。

第 1 条解决「谁负责」：释放点唯一，重复释放就不可能发生。第 2 条解决「什么时候」：不管函数从哪条路返回，出作用域就会释放，不需要人记得写释放代码。第 3 条解决「不转移所有权怎么用」：给它一个不夺走所有权的访问方式，代价是访问期间要守规矩。

三条规则合起来，正好卡在 GC 与手动管理中间：**不需要运行时记账，也不需要人工检查**——检查发生在编译期，运行时什么都不用做。

> [!TIP]
> 这一章要记住的**一句话结论**：Rust 把「谁释放这块内存」变成了一条编译期可检查的性质。编译器检查的代价是写代码时要显式表达「是给出去，还是只借一下」；换来的是没有 GC 也没有手动 `free`。

### 语言对照角：C++、Java 与 Rust

C++ 的 RAII 是距离最近的方案：值语义 + 析构函数，对象离开作用域时析构函数自动跑。差别在于两者的默认选择正好相反——C++ 的复制默认**隐式**（`auto b = a;` 就是一次可能很贵的深拷贝），Rust 的移动默认**免费**、想真复制数据必须显式 `.clone()`。GC 语言里 `a = b` 让两个名字指向同一个对象；Rust 里 `a = b` 是**搬走**，`a` 作废，要「两个名字都可用」就得明确选择借用（`&b`）或克隆（`b.clone()`）。

```cpp
// C++：编译通过，运行时是未定义行为（UB）
std::string& dangling() {
    std::string s = "hello";
    return s;   // s 在函数返回时析构，返回的引用立刻悬垂
}
```

同一个错误在 Rust 安全代码里写不出来——本章「引用总是有效」一节会看到编译器怎么拦它。

## 机制与规则

### 4.1 move：赋值、传参、返回都会转移所有权

```rust
fn takes(s: String) -> usize {
    s.len()
}

fn gives() -> String {
    String::from("hello")
}

fn main() {
    let a = String::from("hello");
    let b = a; // 赋值：所有权从 a 移到 b，a 作废
    println!("b = {b}");

    let len = takes(b); // 传参：所有权移进形参 s
    println!("len = {len}");

    let c = gives(); // 返回：所有权从函数内部移出到 c
    println!("c = {c}");
}
```

输出：

```text
b = hello
len = 5
c = hello
```

移动的语义是「**交出释放责任**」。机器层面通常只是复制几个机器字（`String` 的指针、长度、容量），堆上的字节一个也没动；但语义上 `a` 立刻失效，因为释放责任已经不在它身上了。所以「移动」既不是「深拷贝后销毁原值」，也不是「两个变量指向同一份数据」——它是所有权的交接，编译器把它当成零开销的位复制处理。

用错时编译器会说得很清楚：

```rust,compile_fail
fn main() {
    let a = String::from("hello");
    let b = a;
    println!("{a}");
    println!("{b}");
}
```

```text
error[E0382]: borrow of moved value: `a`
 --> src/main.rs:4:16
  |
2 |     let a = String::from("hello");
  |         - move occurs because `a` has type `String`, which does not implement the `Copy` trait
3 |     let b = a;
  |             - value moved here
4 |     println!("{a}");
  |                ^ value borrowed here after move
```

`E0382` 是本章最常见的一个报错码，完整读法见「报错现场」。这里先记住：**报错位置在「用」的那一行，原因（move occurs here）在更早的那一行**。

顺带一个容易忽略的事实：可变借用 `&mut T` 也不是 `Copy`（原因见 4.4 节），把它赋给另一个变量同样是移动。

```rust,compile_fail
fn main() {
    let mut x = 10;
    let r = &mut x;
    let r2 = r; // &mut T 不是 Copy：这里发生了移动
    *r2 += 1;
    println!("{r}"); // 再用 r：报 E0382
}
```

### 4.2 Copy 与 Clone：什么时候不用担心移动

有些类型移来移去也没问题，因为它们实现了 `Copy`，赋值时按位复制，原变量照样能用。

```rust
fn main() {
    // Copy：赋值后两个变量都能用
    let x = 5;
    let y = x;
    println!("x = {x}, y = {y}");

    // 元组里的元素全是 Copy，元组本身也就 Copy
    let p = (1, 2.5);
    let q = p;
    println!("p = {p:?}, q = {q:?}");

    // Clone：显式写出「这里要复制数据」
    let s1 = String::from("hello");
    let s2 = s1.clone();
    println!("s1 = {s1}, s2 = {s2}");
}
```

输出：

```text
x = 5, y = 5
p = (1, 2.5), q = (1, 2.5)
s1 = hello, s2 = hello
```

`Copy` 的边界很硬：**一个类型的所有组成部分都实现 `Copy`，它才能实现 `Copy`**。原因不是技术限制，而是语义——`Copy` 承诺「按位复制就够了」，而「按位复制就够了」的意思就是「这个值没有需要额外处理的资源」。

| 类型 | `Copy`？ | 原因 |
|---|---|---|
| `i32`、`f64`、`bool`、`char` | 是 | 定长、自包含，位复制就是完整语义 |
| `(i32, f64)`、`[i32; 3]` | 是（元素全 `Copy` 才 `Copy`） | 同上 |
| `&T` | 是 | 复制一个地址不影响任何所有权 |
| `&mut T` | **否** | 可变借用是独占的，复制一份等于凭空多出第二条写入通道 |
| `String`、`Vec<T>`、`Box<T>` | 否 | 拥有堆内存，位复制会造成「两处都以为自己该释放」 |
| `Option<i32>`、`Result<i32, bool>` | 是（成员全 `Copy` 才 `Copy`） | 同上 |

试试给 `String` 的包装类型强行 `Copy`：

```rust,compile_fail
#[derive(Copy, Clone)]
struct Bad {
    name: String,
}
```

```text
error[E0204]: the trait `Copy` cannot be implemented for this type
 --> src/main.rs:2:8
  |
1 | #[derive(Copy, Clone)]
  |          ---- in this derive macro expansion
2 | struct Bad {
  |        ^^^
3 |     name: String,
  |     ------------ this field does not implement `Copy`
```

编译器拒绝背后是实打实的危害：如果 `String` 是 `Copy`，两个变量会共用同一块堆缓冲区，函数返回时各释放一次——**重复释放（double free）**。C++ 里这正是拷贝构造、析构、赋值运算符必须一起写清楚的原因。

`Clone` 是另一回事：它表示「我可以被复制，但复制要显式请求，而且可能昂贵」。

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1.clone(); // 分配新缓冲区、逐字节拷贝
    println!("{} {}", s1, s2);

    let n: i32 = 7;
    let m = n.clone(); // 合法但没意义：Copy 类型都实现 Clone
    println!("{n} {m}");
}
```

对 `String` 来说，`.clone()` 是一次真实的堆分配加内存拷贝。所以读代码时的经验法则是：**看到 `.clone()` 就要能说出「这里为什么必须复制」**，说不出来通常是在绕借用检查，而不是真的需要两份数据。

### 4.3 作用域与 drop

规则 2 说「所有者离开作用域，值被释放」。用带析构的类型能直接看到这件事。

```rust
struct PrintOnDrop(&'static str);

impl Drop for PrintOnDrop {
    fn drop(&mut self) {
        println!("drop: {}", self.0);
    }
}

fn consume(_x: PrintOnDrop) {
    println!("进入 consume");
} // 形参在这里出作用域，被 drop

fn main() {
    let _a = PrintOnDrop("a");
    let b = PrintOnDrop("b");
    consume(b); // b 的所有权移进函数
    let _c = PrintOnDrop("c");
    println!("main 结束前");
}
```

输出：

```text
进入 consume
drop: b
main 结束前
drop: c
drop: a
```

三件事一起看：`b` 被移进 `consume`，于是在**函数内部**出作用域时就 drop，`main` 再也管不着它；`main` 结束时剩下的局部变量按**声明逆序**释放（`_c` 先 drop，`_a` 最后）；而 `_a` 没被移动过，释放责任一直在它身上。（`impl Drop` 的语法在第 9 章、第 17 章展开；这里只需知道：实现 `Drop` 的类型能在「被释放」时执行代码。）

赋值也会触发 drop——左边原来的值先被释放：

```rust
struct PrintOnDrop(&'static str);

impl Drop for PrintOnDrop {
    fn drop(&mut self) {
        println!("drop: {}", self.0);
    }
}

fn main() {
    let mut slot = PrintOnDrop("旧值");
    slot = PrintOnDrop("新值"); // 赋值：左值原来的值在这里被 drop
    println!("main 结束前，槽位里是 {}", slot.0);
}
```

输出：

```text
drop: 旧值
main 结束前，槽位里是 新值
drop: 新值
```

（这个程序还会得到一个 `unused_assignments` 警告，因为「旧值」写进去之后没人读过——那是另一条 lint，本章不展开。）

想在作用域结束前就释放，用 `drop` 函数把值传进去：`drop(x);` 会移动 `x` 并立刻析构它，因此 `x` 之后不能再使用。这和第 17 章的智能指针是同一套机制的应用。

### 4.4 借用：`&T` 与 `&mut T`

如果所有函数调用都要转移所有权，写起来会非常痛苦（每次调用前 `clone`）。于是有了**借用**：`&T` 是共享借用，`&mut T` 是可变借用。借用不改变所有权，只是想「用一下」。

```rust
fn main() {
    // 共享借用可以同时存在多个
    let s = String::from("hello");
    let r1 = &s;
    let r2 = &s;
    println!("{r1} {r2}");

    // 可变借用独占，改完再用
    let mut t = String::from("hello");
    let m = &mut t;
    m.push_str(", world");
    println!("{m}");
}
```

输出：

```text
hello hello
hello, world
```

**别名规则（aliasing rules）**只有一句，但要和「时间」一起理解：

> 对同一个值，在**同一时刻**，要么有任意多个 `&T`，要么恰好有一个 `&mut T`，两者不能并存。

配套的两条推论同样重要：

- 只要还存在可用的借用，被借的值就不能被移动——移动会把数据搬走，借用就成了**悬垂引用（dangling reference）**；
- `&mut T` 生效期间，原变量本身也不能用——因为你手里的可变借用已经是对这块数据的唯一访问入口。

至于「同一时刻」到底多长，这就是下一节的内容。

### 4.5 借用的边界：借到「最后一次使用」为止

「同一时刻」不是「同一个作用域」。自 Rust 1.31（edition 2018）引入 NLL（non-lexical lifetimes）起，借用持续到**最后一次使用**为止；这段区间叫**借用区间（borrow region）**。下面的代码能编译，正是因为它成立：

```rust
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    println!("first = {first}"); // first 的最后一次使用
    v.push(4); // 这里借用已经结束，可变借用被允许
    println!("v = {v:?}");
}
```

输出：

```text
first = 1
v = [1, 2, 3, 4]
```

而一旦把顺序改回去，同一段代码就编不过了：

```rust,compile_fail
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    v.push(4);
    println!("first = {first}");
}
```

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
  |
3 |     let first = &v[0];
  |                  - immutable borrow occurs here
4 |     v.push(4);
  |     ^^^^^^^^^ mutable borrow occurs here
5 |     println!("first = {first}");
  |                        ----- immutable borrow later used here
```

两段代码的差别只有一行，读报错的关键也在这里：编译器用 `-` 标出共享借用开始处，用 `^` 标出可变借用处，最后用一行「immutable borrow later used here」说明第一段借用为什么还活着。把它画成时间轴就是：

```text
① first 在后面还要用（E0502）
   let first = &v[0];      ├────────── 共享借用 ──────────┤
   v.push(4);                           ├── 可变借用 ──┤
                                        ↑ 区间重叠：拒绝

② first 用完就不用了（通过）
   let first = &v[0];      ├── 共享借用 ──┤
   println!("{first}");                  ↑ 借用到此结束
   v.push(4);                                  ├── 可变借用 ──┤
                                               ↑ 不重叠：通过
```

同样的规则也解释了「连续两次可变借用」：

```rust,compile_fail
fn main() {
    let mut s = String::from("hello");
    let r1 = &mut s;
    let r2 = &mut s;
    println!("{r1} {r2}");
}
```

```text
error[E0499]: cannot borrow `s` as mutable more than once at a time
 --> src/main.rs:4:14
  |
3 |     let r1 = &mut s;
  |              ------ first mutable borrow occurs here
4 |     let r2 = &mut s;
  |              ^^^^^^ second mutable borrow occurs here
5 |     println!("{r1} {r2}");
  |                -- first borrow later used here
```

而把两个可变借用拆到互不重叠的区间里，就完全合法：

```rust
fn main() {
    let mut s = String::from("hello");
    {
        let m = &mut s;
        m.push('!');
    } // m 在这里结束
    let m2 = &mut s;
    m2.push('?');
    println!("{s}");
}
```

输出：

```text
hello!?
```

> [!NOTE]
> 编译器对你写的**每一个**借用都算一遍区间，但只会抱怨确实重叠的那些。所以修 `E0499` / `E0502` 最常用的两招是：把借用关进更小的作用域（像上面那样加 `{}`），或者把「先取出来用」的数据取成拥有所有权的值（`let first = v[0];` 而不是 `&v[0]`）。

还有两个值得知道的细节：

- **字段级拆分借用**：借不同的字段可以同时成立，因为它们是不同的值。`let (x, y) = (&mut pair.0, &mut pair.1);` 能通过。
- **两阶段借用**：`v.push(v.len())` 能编译——编译器对方法调用的接收者做了特殊处理，参数先算完，可变借用才真正生效。这类放宽规则不需要你记住，碰到时知道它不是玄学即可。

为什么别名规则值得这么严？因为它一次性解决了两个经典问题：**数据竞争（data race）**（两个线程同时一写多读、又没有任何同步）与**引用/迭代器失效**（`v.push` 可能重新分配缓冲区，此时旧的 `&v[0]` 就指向了已释放内存）。规则保证「可变借用期间没有别人能看见这块数据」，所以这两类事故在安全代码里都写不出来。

### 4.6 引用总是有效

Rust 对引用给了一个非常强的不变量：**只要一个引用能用，它指向的数据就是有效的、类型正确的、且没有被移动过的**（内部可变性与 `unsafe` 是这条不变量上仅有的口子，分别在第 17、23 章）。

所以「悬垂引用」在安全代码里不是运行时报错，而是编译不过。想从函数里返回局部变量的引用时，编译器第一句话是：

```rust,compile_fail
fn dangling() -> &String {
    let s = String::from("hello");
    &s
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:18
  |
1 | fn dangling() -> &String {
  |                  ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but there is no value for it to be borrowed from
help: instead, you are more likely to want to return an owned value
  |
1 - fn dangling() -> &String {
1 + fn dangling() -> String {
  |
```

`E0106` 说的是「返回的引用没有可挂靠的生命周期（lifetime）」——生命周期就是借用的有效区间，标注语法在第 16 章展开，而好消息是绝大多数日常代码根本不需要写它。即使把它写成 `fn dangling<'a>() -> &'a String`，编译器也会拦住，理由更直白：

```rust,compile_fail
fn dangling<'a>() -> &'a String {
    let s = String::from("hello");
    &s
}
```

```text
error[E0515]: cannot return reference to local variable `s`
 --> src/main.rs:3:5
  |
3 |     &s
  |     ^^ returns a reference to data owned by the current function
```

对比一下开头的 C++ 例子：同样的逻辑在 C++ 里编译通过、运行时是未定义行为；在 Rust 里它是**编译期的类型错误**。这就是「不需要 GC 也不需要人工检查」的具体形态——借用检查器（borrow checker）把「数据活着」的区间与「引用被使用」的区间对齐：对得上就编译通过，运行时不必再做任何检查；对不上就编译失败。

反过来，从参数借来的引用可以放心返回，因为数据的寿命由调用方保证。这是最常见的一类签名：

```rust
fn first_word(s: &str) -> &str {
    for (i, c) in s.char_indices() {
        if c == ' ' {
            return &s[..i]; // 返回 s 的一段视图，而不是新字符串
        }
    }
    s
}

fn main() {
    let line = String::from("hello world");
    let word = first_word(&line);
    println!("word = {word}");
    println!("line = {line}"); // line 只被借用过，仍然可用
}
```

输出：

```text
word = hello
line = hello world
```

`fn first_word(s: &str) -> &str` 里有两个 `&str` 却没有写生命周期参数，是因为省略规则（lifetime elision）规定：**参数里恰好只有一个生命周期时，输出的生命周期就取它**。完整规则在第 16 章。

### 4.7 切片：借来的一段序列

`&str` 和 `&[T]` 不是「指向某个元素的指针」，而是「**指针 + 长度**」的组合，叫做**切片（slice）**。对应的无长度版本 `str` 和 `[T]` 是**动态大小类型（DST，dynamically sized type）**：它们的大小取决于运行时的数据量，编译器不知道一个 `str` 占多少字节，所以 `str` 只能通过引用或 `Box` 使用，不能按值传递。

```text
                     栈                            堆（或只读数据段）
        ┌────────────────────┐
        │ ptr ───────────────┼──────► h e l l o      ← 5 字节 UTF-8
String  │ len = 5            │
        │ cap = 5            │
        └────────────────────┘
         所有者：3 个机器字

        ┌────────────────────┐
        │ ptr ───────────────┼──────► 指向某个已有序列中的一段
&str    │ len = 5            │
        └────────────────────┘
         借用（胖指针，fat pointer）：2 个机器字

        str / [T]：DST，没有编译期已知大小，只能通过引用或 Box 使用
```

本机实测的各种尺寸（64 位；`String::from("hello")` 的 `cap` 正好是 5）：

| 类型 | 大小（字节） | 组成 |
|---|---|---|
| `String` | 24 | 指针 + 长度 + 容量（3 个机器字） |
| `Vec<u8>` | 24 | 同上 |
| `&String` | 8 | 一个地址（瘦指针，thin pointer） |
| `&Vec<u8>` | 8 | 一个地址 |
| `&str` | 16 | 指针 + 字节长度（胖指针） |
| `&[u8]` | 16 | 指针 + 元素个数（胖指针） |
| `Box<str>` | 16 | 指针 + 长度 |
| `&[u8; 4]` | 8 | 数组大小已知，一个地址就够 |

这张表解释了字符串相关类型最常被含糊带过的差异：

- `String` 是**所有者**：堆缓冲区归它，长度和容量都记在栈上；
- `&String` 只是「指向那 24 字节」的瘦指针，要多解一层才能拿到字节；
- `&str` 直接就是「字节地址 + 字节长度」，一次解引用就能直接访问字节（它是共享借用，只能读；写入要可变切片，如 `&mut [u8]`），也不关心数据是堆上的、静态区的还是别处的；`str` 字面量（如 `"hi"`）类型就是 `&'static str`，指向只读数据段。

实际写代码时几乎总是用 `&str` 做参数：`&String` 会因为解引用强转（deref coercion）自动变成 `&str`，于是 `String`、字面量、切片都能传进来。

```rust
fn shout(s: &str) -> String {
    s.to_uppercase()
}

fn main() {
    let owned = String::from("hello");
    let slice: &str = &owned; // &String 自动强转成 &str
    println!("{}", shout(slice));
    println!("{}", shout(&owned));

    // 切片引用本身是 Copy：复制它不会复制数据
    let r1 = slice;
    let r2 = slice;
    println!("{r1} {r2}");
}
```

输出：

```text
HELLO
HELLO
hello hello
```

切片语法基于字节范围，所以用它可以看清「视图」的本质——`&line[6..11]` 和整串共享同一块内存，只是起点偏移了 6：

```rust
fn main() {
    let line = String::from("hello world");
    let whole = line.as_str();
    let word = &line[6..11];
    println!("整个字符串的地址 = {:p}，长度 = {}", whole.as_ptr(), whole.len());
    println!("切片的地址       = {:p}，长度 = {}", word.as_ptr(), word.len());
    println!("两者地址相差     = {}", word.as_ptr() as usize - whole.as_ptr() as usize);
}
```

输出（地址每次运行都会变，重点是相差 6）：

```text
整个字符串的地址 = 0x27e415faee0，长度 = 11
切片的地址       = 0x27e415faee6，长度 = 5
两者地址相差     = 6
```

是视图就意味着能通过视图改原文：

```rust
fn main() {
    let mut arr = [1, 2, 3, 4, 5];
    let mid = &mut arr[1..4]; // 可变切片
    for x in mid.iter_mut() {
        *x *= 10;
    }
    println!("{arr:?}");

    let nums = [10, 20, 30, 40];
    let view: &[i32] = &nums[1..3];
    println!("view = {view:?}, len = {}, sum = {}", view.len(), view.iter().sum::<i32>());
}
```

输出：

```text
[1, 20, 30, 40, 5]
view = [20, 30], len = 2, sum = 50
```

> [!WARNING]
> 索引范围是**字节**范围，不是字符范围。`&s[0..2]` 如果切在字符中间会 panic（`byte index 2 is not a char boundary`）；`str` 上的 `len()` 也返回字节数。这一条在本章只需要知道「索引不可乱切」，UTF-8 与 `String` 的完整方法在第 6 章展开。

```rust,no_run
fn main() {
    let s = String::from("héllo");
    let bad = &s[0..2]; // 'é' 占两个字节，2 切在它中间
    println!("{bad}");
}
```

运行结果（注意是运行时 panic，不是编译错误）：

```text
thread 'main' (12345) panicked at src/main.rs:3:17:
end byte index 2 is not a char boundary; it is inside 'é' (bytes 1..3 of string)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

（`12345` 是线程号占位，每次运行不同；退出码 101，panic 机制见第 7 章。）

## 动手看看

下面这个程序把本章的三件事合在一起：借用参数、返回借用自参数的切片、以及「切片是视图」。它是完整可编译的程序，源码在 `snippets/ch04/word_stats.rs`。

```rust
{{#rustdoc_include ../snippets/ch04/word_stats.rs}}
```

输出：

```text
字节  0..4   Rust
字节  5..8   让
字节  9..15  借用
字节 16..22  变成
字节 23..38  编译期检查
整行 = Rust 让 借用 变成 编译期检查
```

对照着看三处：

1. `words(&line)` 只借用了 `line`，所以结尾那句 `println!("整行 = {line}")` 还能打印——如果签名是 `fn words(s: String)`，`line` 会在调用处被移动，那一行就编不过；
2. `words` 返回的 `Vec<(usize, &str)>` 里，每个 `&str` 都借用自参数 `s`（也就是 `line` 里的字节），没有复制任何字符串内容；
3. `&s[begin..i]` 的 `begin`、`i` 来自 `char_indices()`，所以永远落在字符边界上——这正是「字节索引不能乱切」的正确用法。

如果确实需要返回值在 `line` 被释放后仍然存在，就必须让它们拥有数据：

```rust
fn words_owned(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if c == ' ' {
            if let Some(begin) = start.take() {
                out.push(s[begin..i].to_string()); // 每个词分配一个新 String
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(begin) = start {
        out.push(s[begin..].to_string());
    }
    out
}

fn main() {
    let owned = words_owned("hello world");
    println!("{owned:?}"); // owned 不再借用任何东西
}
```

输出：

```text
["hello", "world"]
```

这两版的选择就是本章的核心权衡：**借用**便宜但受所有者寿命约束，**拥有**自由但有分配与拷贝的代价。绝大多数情况下先写借用版，编译器逼你克隆时再考虑成本。

## 报错现场

五个现场都按同一套路径读（第 2 章的诊断流程）：先看第一行 `error[Exxxx]:` 的**结论**，再看 `-->` 的**位置**，然后看编译器用 `-`、`^` 标出的**两处标记**，最后读 `help:`。

### 现场 1：`E0382` borrow of moved value

```rust,compile_fail
fn main() {
    let a = String::from("hello");
    let b = a;
    println!("{a}");
    println!("{b}");
}
```

```text
error[E0382]: borrow of moved value: `a`
 --> src/main.rs:4:16
```

定位路径：新东西在 `move occurs because ... does not implement the Copy trait`——它直接告诉你「`String` 不可 `Copy`，所以这一行是移动」。`- value moved here` 指出移动发生的位置，`^ value borrowed here after move` 指出越界使用的位置。两者之间就是问题所在。

三种修法，按「先考虑哪个」排序：

```rust
fn main() {
    // 修法 1（首选）：还需要用 a 就别移动它，传引用
    let a = String::from("hello");
    let len = a.len();
    let b = &a;
    println!("len = {len}, b = {b}, a = {a}");

    // 修法 2：确实需要两份数据，显式克隆，代价是分配 + 拷贝
    let c = String::from("hello");
    let d = c.clone();
    println!("c = {c}, d = {d}");

    // 修法 3：本来就是转移所有权，那就只在移动之后用新变量
    let first = String::from("hello");
    let second = first;
    println!("second = {second}");
}
```

输出：

```text
len = 5, b = hello, a = hello
c = hello, d = hello
second = hello
```

`E0382` 还有一个更隐蔽的变体：

- **隐式移动**：`for x in v {}` 会调用 `v.into_iter()`，把 `v` 移进循环；循环后再用 `v` 就报同一个码。修法是把 `v` 换成 `&v`（或 `v.iter()`）。

> [!NOTE]
> 「移动撞上借用」的另一种写法报的是**另一个码**：先借出 `&mut`，随后又直接使用原变量（`let t = s;`）→ `E0505`：cannot move out of `s` because it is borrowed（并会并列标出 borrow of `s` occurs here 与 move out of `s` occurs here）。本质仍是「借用区间没结束」，修法与 `E0502`/`E0499` 相同；速查表也收了它。

### 现场 2：`E0502` 共享与可变借用重叠

```rust,compile_fail
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    v.push(4);
    println!("first = {first}");
}
```

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
```

定位路径：报错标题的那句「as mutable ... also borrowed as immutable」就是别名规则的原文；`- immutable borrow occurs here` 与 `^^^^^^^^^ mutable borrow occurs here` 是重叠的两处；`immutable borrow later used here` 解释了共享借用为什么还没结束。

三种修法：

```rust
fn main() {
    // 修法 1（首选）：先把数据取出来（i32 是 Copy），不再持有借用
    let mut v = vec![10, 20, 30];
    let first = v[0];
    v.push(40);
    println!("first = {first}, v = {v:?}");

    // 修法 2：把借用关进更小的作用域，用完立刻结束
    let mut w = vec![10, 20, 30];
    {
        let head = &w[0];
        println!("head = {head}");
    }
    w.push(40);
    println!("w = {w:?}");

    // 修法 3：确实需要独立的一份，显式克隆，代价是分配 + 拷贝
    let mut u = vec![10, 20, 30];
    let copy = u.clone();
    u.push(40);
    println!("copy = {copy:?}, u = {u:?}");
}
```

输出：

```text
first = 10, v = [10, 20, 30, 40]
head = 10
w = [10, 20, 30, 40]
copy = [10, 20, 30], u = [10, 20, 30, 40]
```

注意修法 3 的输出：`copy` 是克隆那一刻的快照，`u` 之后的修改不影响它。

### 现场 3：`E0499` 同时两个可变借用

```rust,compile_fail
fn main() {
    let mut s = String::from("hello");
    let r1 = &mut s;
    let r2 = &mut s;
    println!("{r1} {r2}");
}
```

```text
error[E0499]: cannot borrow `s` as mutable more than once at a time
 --> src/main.rs:4:14
```

定位路径：与 `E0502` 同一套读法，只是这次冲突的两方都是 `&mut`。`first borrow later used here` 说明第一个借用为什么还活着。

修法同样是「让两个区间不重叠」：用完第一个再借第二个（把第一个的最后一次使用提前），或者干脆只在需要时临时借。

```rust
fn main() {
    let mut s = String::from("hello");

    // 修法：先借、先用、先结束
    {
        let r1 = &mut s;
        r1.push('!');
    }
    let r2 = &mut s;
    r2.push('?');
    println!("{s}");
}
```

输出：

```text
hello!?
```

> [!TIP]
> 同一个变量连着两次 `&mut` 却不重叠区间，编译器完全接受。`E0499` 抱怨的永远是**区间重叠**，不是「你借了两次」。

### 现场 4：`E0515` 返回局部变量的引用

```rust,compile_fail
fn dangling<'a>() -> &'a String {
    let s = String::from("hello");
    &s
}
```

```text
error[E0515]: cannot return reference to local variable `s`
 --> src/main.rs:3:5
  |
3 |     &s
  |     ^^ returns a reference to data owned by the current function
```

定位路径：这句话可以直接翻译成中文——「返回了对当前函数自己拥有的数据的引用」。`s` 在这行之后就要被 drop，引用没有可指的东西。

修法只有两个方向：**返回拥有所有权的值**（`-> String`，或者返回 `&str` 的参数版本），或者**把数据的归属移到调用方**（让调用方先建好 `String`，函数只接收 `&str` 并返回借用它的切片，就像 4.6 节的 `first_word`）。

```rust
fn greeting() -> String {
    let s = String::from("hello");
    s // 把所有权交给调用方：不是借用，而是移动
}

fn main() {
    let g = greeting();
    println!("{g}");
}
```

输出：

```text
hello
```

### 现场 5：`E0106` missing lifetime specifier（露脸一次）

```rust,compile_fail
fn dangling() -> &String {
    let s = String::from("hello");
    &s
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:18
  |
1 | fn dangling() -> &String {
  |                  ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but there is no value for it to be borrowed from
help: instead, you are more likely to want to return an owned value
```

好消息：这个报错比看起来友好得多。它出现的绝大多数场合，正确答案都是 `help:` 里那句「你其实想返回一个拥有所有权的值」——把 `-> &String` 改成 `-> String` 就完事，根本不用碰生命周期语法。完整的标注语法、以及 `&'a` 到底怎么读，在第 16 章展开。

### 报错码速查

| 报错码 | 含义 | 首选修法 |
|---|---|---|
| `E0382` | 用了已经移动的值 | 传引用；真需要两份就 `.clone()` |
| `E0502` | 可变借用与共享借用区间重叠 | 先取出数据（`v[0]`）；或缩小借用作用域 |
| `E0505` | 在借用期内把被借的值移走 | 先结束借用（缩小作用域），或改成克隆 |
| `E0499` | 同一时刻两个可变借用 | 让两段借用区间不重叠（先借先用先结束） |
| `E0515` | 返回了指向局部数据的引用 | 返回拥有所有权的值，或改从参数借用 |
| `E0106` | 返回的引用没有可挂靠的生命周期 | 多半是改返回 `String`；标注语法见第 16 章 |
| `E0204` | 给含非 `Copy` 字段的类型实现 `Copy` | 换成 `Clone`，或想想为什么要复制 |

## 小结

- **为什么**：GC 把释放推给运行时，手动管理把释放推给人的记忆，Rust 把释放责任变成类型系统里的性质——编译期检查，运行时零开销。三条规则是它的展开：任一时刻只有一个所有者；所有者出作用域即 drop；值可以移动，不能移动时只能借用，而借用受别名规则约束。
- **move** 是所有权交接，不是深拷贝也不是共享：赋值、传参、返回都会移动，移动后原变量作废（`E0382`）。
- **`Copy`** 承诺「按位复制即可」，所以只给没有资源的类型；拥有堆内存的 `String`/`Vec`/`Box` 不 `Copy`，要复制必须显式 `.clone()` 并承担代价。
- **借用**的核心是别名规则：同一时刻要么多个 `&T`，要么一个 `&mut T`。而「同一时刻」= 借用区间，借到**最后一次使用**为止（NLL）。
- **引用总是有效**：悬垂引用是编译错误（`E0106`/`E0515`），不是运行时事故；这条不变量正是「不用 GC 也不用人工检查」的来源。
- **切片**：`&[T]`/`&str` 是「指针 + 长度」的胖指针，`str`/`[T]` 是 DST，`String`/`Vec<T>` 是「指针 + 长度 + 容量」的所有者。三者记清楚，字符串与集合的报错就不再吓人。

## 练习

**题 1.** 下面两段代码，只有第一段能编译。解释第二段报哪个错、为什么，并给出两种修法。

```rust
fn main() {
    let x = 5;
    let y = x;
    println!("{x} {y}");
}
```

```rust,compile_fail
fn main() {
    let s = String::from("hi");
    let t = s;
    println!("{s} {t}");
}
```

**题 2.** 预测下面程序的编译结果。如果编不过，报哪个码？报错里会被标出来的两处分别是什么？

```rust,compile_fail
fn main() {
    let mut s = String::from("hello");
    let r = &s;
    s.push('!');
    println!("{r}");
}
```

**题 3.** 本机实测：`String` 是 24 字节，`&str` 是 16 字节，`&String` 是 8 字节。分别解释这三个数字由什么组成；并说明为什么 `fn f(s: str)` 编译不过，而 `fn f(s: &str)` 可以。

<details><summary>参考答案</summary>

**题 1.** 第一段能过，因为 `i32: Copy`——`let y = x;` 是按位复制，`x` 仍然有效。第二段报 `E0382`（borrow of moved value: `s`）：`String` 不实现 `Copy`，`let t = s;` 是一次移动，`s` 立刻作废，之后 `println!("{s}")` 就是「使用已移动的值」。

两种修法：（a）还需要 `s` 就用借用——`let t = &s;`，`t` 是 `&String`；（b）确实要两份独立的字符串——`let t = s.clone();`，代价是一次堆分配加拷贝。

**题 2.** 编不过，报 `E0502`。真实输出（关键几行）：

```text
error[E0502]: cannot borrow `s` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
  |
3 |     let r = &s;
  |             -- immutable borrow occurs here
4 |     s.push('!');
  |     ^^^^^^^^^^^ mutable borrow occurs here
5 |     println!("{r}");
  |                - immutable borrow later used here
```

被标出的两处是 `let r = &s;`（共享借用开始）与 `s.push('!');`（可变借用）；最后一行 `later used here` 才是关键——它说明 `r` 的借用区间一直延伸到 `println!`，正好覆盖了 `push` 那一刻。

修法举例：（a）把 `println!("{r}")` 挪到 `s.push('!')` 之前，借用区间提前结束；（b）如果你要的是「改动前的内容」，就克隆出独立的一份——`let old = s.clone(); s.push('!'); println!("{old} {s}");`。注意（b）里不能再保留 `r`：只要 `r` 后面还有一次使用，借用区间就会盖住 `push`，编译照样不过。

**题 3.** `String` = 指针（8）+ 长度（8）+ 容量（8）= 24，它是一个值的**所有者**：知道缓冲区在哪、用了多少、能装多少；`&str` = 指针（8）+ 字节长度（8）= 16，它是**借用**：不需要容量，因为不负责分配与释放；`&String` = 一个地址 8 字节，它指的是那 24 字节的 `String` 本身，要拿内容还得再解一层。

`fn f(s: str)` 编不过，因为 `str` 没有编译期已知大小（DST），无法按值压栈、也无法知道要传多少字节；`fn f(s: &str)` 传的是一个固定 16 字节的胖指针，长度信息就藏在指针里，所以可以。

</details>

## 延伸阅读

- The Book：所有权（[ch04-01](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)）、引用与借用（[ch04-02](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)）、切片（[ch04-03](https://doc.rust-lang.org/book/ch04-03-slices.html)）
- The Reference：引用类型（<https://doc.rust-lang.org/reference/types/pointer.html>）、切片类型（<https://doc.rust-lang.org/reference/types/slice.html>）、`str`（<https://doc.rust-lang.org/reference/types/str.html>）、动态大小类型（<https://doc.rust-lang.org/reference/dynamically-sized-types.html>）、析构与 drop 顺序（<https://doc.rust-lang.org/reference/destructors.html>）、生命周期省略规则（<https://doc.rust-lang.org/reference/lifetime-elision.html>）
- Nomicon：所有权（<https://doc.rust-lang.org/nomicon/ownership.html>）、引用（<https://doc.rust-lang.org/nomicon/references.html>）
- 标准库：`Copy`（<https://doc.rust-lang.org/std/marker/trait.Copy.html>）、`Clone`（<https://doc.rust-lang.org/std/clone/trait.Clone.html>）、`Drop`（<https://doc.rust-lang.org/std/ops/trait.Drop.html>）
- 报错码逐条解释（`rustc --explain` 的完整版）：<https://doc.rust-lang.org/error_codes/E0382.html>，把编号换成 `E0499`、`E0502`、`E0515`、`E0106`、`E0204` 同理
- 非词法生命周期（NLL）的官方公告：<https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html#non-lexical-lifetimes>
- 两阶段借用（编译器实现细节）：<https://rustc-dev-guide.rust-lang.org/borrow-check/two-phase-borrows.html>
