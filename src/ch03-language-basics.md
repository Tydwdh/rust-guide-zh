# 第 3 章 语言基础：变量、类型、函数与控制流

## 这一章解决什么问题

如果你会 C、Java、Python 或 Go 中的任意一门，本章的语法扫一眼就能懂个大概。真正需要花力气的是三个「默认值」上的差异：Rust 在**数值类型之间没有隐式转换**、**没有 null**、**不让变量在赋值之前被读取**。这三条不是风格偏好，而是类型系统对你做出的承诺；后面所有让人安心的地方（编译过了就基本不会崩、所有权（ownership）不需要 GC）都建立在它们之上。

读完本章你能做到：

- 用 `let` 建立不可变绑定（binding），知道什么时候该写 `mut`、什么时候该用遮蔽；
- 说出标量与复合类型的清单，知道整数默认 `i32`、浮点默认 `f64`，以及整数溢出（integer overflow）在 debug 与 release 下的两种行为；
- 理解「块、`if`、`loop` 都是表达式」，说得清 `;` 为什么决定一个块的值；
- 写出带参数与返回值的函数，知道尾表达式与 `return` 的分工；
- 在 `if`、`loop`、`while`、`for` 之间做出选择，并看懂 E0308、E0384、E0381 这几种基础报错。

## 为什么这样设计

先把心智模型钉成一句话：**每个值在编译期就有一个确定的类型，这个类型不会在背后悄悄改变；编译器据此把能提前发现的问题全部拦下。**

三个「没有」，各自对应一类历史悠久的 bug：

**数值类型之间没有隐式转换。** C 的整型提升与有无符号混用制造了一整类难查的 bug（`if (i < sizeof(x))` 里 `int` 会被抬成 `unsigned`）；Java 也有窄化转换的坑，只是限制得严一些。Rust 的取舍是：**转换必须写出来，并且分成两种语义**——`as` 表示「我知道可能丢数据，就这么干」，`try_into()` 表示「可能失败，返回结果让你检查」。代码里出现哪个，读者一眼就知道作者在想什么。

**没有 null。** `null`（或 `None`、`nullptr`）的问题是它让类型撒谎：一个 `String` 类型的变量居然可能是「没有字符串」，于是每次使用都可能炸。Rust 把「可能没有」变成普通类型 `Option<T>`：`Option<String>` 与 `String` 是两个类型，编译器不允许你直接把前者当后者用（第 5 章展开）。同时，Rust 里不存在「未定义值」这种状态。

**没有「未初始化」这种可读取的状态。** 变量在被读取之前必须确实被赋过值（写成 `let x: i32;` 这样的声明是合法的，只是读到它就算错，见「报错现场」⑤）；编译器做「确定赋值」分析，只要存在一条「读到它但没赋过值」的路径就报错。收益有两个：编译器可以假定每个读到的值都有效；「这块内存什么时候该释放」变成可以静态判定的问题——这是所有权不需要 GC 的前提（第 4 章）。

把这三条与常见语言摆在一张表里：

| 关注点 | C / Java / Python | Rust |
|---|---|---|
| 变量默认可否修改 | 可以（Java 要写 `final`；Python 只是重新绑定） | 不可变；要改写 `let mut` |
| 整数之间的转换 | 隐式提升 / 窄化 | 显式 `as`，或 `try_into()` |
| 「值可能不存在」 | `null` / `None` / `nullptr` | `Option<T>` |
| 块与 `if` 有没有值 | 以语句为主（Python 有表达式） | 处处是表达式 |

## 机制与规则

### 绑定：`let`、`mut` 与遮蔽

```rust
fn main() {
    let answer = 42;        // 不可变绑定：默认状态
    let mut counter = 0;    // 要改就显式写 mut
    counter += 1;
    println!("{answer} {counter}");
}
```

`mut` 只做一件事：允许对这个绑定赋值。它不是「声明变量」的标记（Rust 里变量天生就有），而是一个权限开关；它还让**书面代码自带说明——这块会被改动**，读代码时省一次来回查找。

同一个名字可以用 `let` 再绑定一次，这叫**遮蔽（shadowing）**：

```rust
fn main() {
    let x = 5;
    let x = x + 1;              // 新绑定，遮住旧的
    let x = "现在 x 是 &str";    // 连类型都可以换
    println!("{x}");
}
```

遮蔽与 `mut` 的区别值得单独记住：

- `mut` 改的是**同一个绑定**的值：类型不能变。
- 遮蔽是**新绑定**顶掉旧名字：类型可以变；旧绑定还在，只是没法再按名字访问它。
- 怎么选：值在原地反复变化（计数器、缓冲区）用 `mut`；把值沿一串转换往下传（`let s = s.trim(); let s = s.to_lowercase();`）用遮蔽。遮蔽让你不必发明 `x2`、`x3` 这种名字。

### 类型全景：标量、元组、数组

标量类型（不能再拆成更小的值）：

| 类别 | 类型 | 要点 |
|---|---|---|
| 有符号整数 | `i8` `i16` `i32` `i64` `i128` `isize` | 「默认整数」是 `i32` |
| 无符号整数 | `u8` `u16` `u32` `u64` `u128` `usize` | 不能表示负数 |
| 浮点 | `f32` `f64` | 「默认浮点」是 `f64` |
| 布尔 | `bool` | 只有 `true` / `false` |
| 字符 | `char` | 一个 Unicode 标量值 |

`isize` / `usize` 是「与指针同宽」的整数（本机的 64 位平台上占 8 字节）；数组下标与长度一律用 `usize`。字面量支持几种进制与 `_` 分隔（纯语法细节，查表即可）：

| 写法 | 例子 |
|---|---|
| 十进制 | `98_222` |
| 十六进制 / 八进制 / 二进制 | `0xff` / `0o77` / `0b1111_0000` |
| 字节（仅 `u8`） | `b'A'` |

字面量的类型靠推断，推断不出时才用默认值——下面的程序把这两件事一起验证了：

```rust
fn main() {
    let whole = 42;    // 推断不出别的，用默认：i32
    let ratio = 3.5;   // 推断不出别的，用默认：f64
    println!(
        "{} {}",
        std::any::type_name_of_val(&whole),
        std::any::type_name_of_val(&ratio)
    );
    println!("char 占 {} 字节，'中' 的标量值 = {}", std::mem::size_of::<char>(), '中' as u32);
}
```

（`type_name_of_val` 只是标准库里的调试小工具，用来把推断结果打出来。）

`char` 不是「一个字节」也不是「一个字」，而是一个 Unicode 标量值：它装得下 `'中'`、`'😀'`，但装不下代理对（`char::from_u32(0xD800)` 返回 `None`）。字符串与 UTF-8 的细节在第 6 章。

复合类型有两个，够用一整本书：

```rust
fn main() {
    let pair = (1, "两个");    // 元组：元素类型可以不同
    let (a, b) = pair;         // 解构（destructuring）——模式匹配（pattern matching）的入门用法，第 5 章展开
    println!("{} {} {} {}", pair.0, pair.1, a, b);

    let arr: [i32; 3] = [10, 20, 30];  // 数组：长度是类型的一部分
    let zeros = [0u8; 4];              // 语法糖：4 个 0u8
    println!("{} {} {}", arr.len(), zeros.len(), arr[0]);
}
```

- 元组：定长，元素类型可以各不相同，用 `.0`、`.1` 访问。空元组 `()` 叫**单元类型（unit type）**，含义是「没有值」，后面会反复见到。
- 数组 `[T; N]`：定长、同类型；`N` 是类型的一部分（`[i32; 3]` 与 `[i32; 4]` 是两个类型），通常放在栈上。要「长度可变」的序列就换 `Vec<T>`（第 6 章）。
- 下标越界会被检查：常量下标越界直接编译失败；运行期下标越界 panic（`index out of bounds`）。

类型标注（annotation）绝大多数时候不用写：编译器从初始值和后续用法推断。三种情况必须写：函数签名（下面讲）；推断不出唯一答案（去掉下面那行 `: i32`，`parse` 的目标类型无从得知，报 E0284 `type annotations needed`，也可以写成 `"42".parse::<i32>()`）；以及想用默认值以外的类型（`let small: u8 = 200;`）。

```rust
fn main() {
    let n: i32 = "42".parse().unwrap();
    println!("{n}");
}
```

两种转换各有明确语义——一个不会失败，一个会告诉你失败：

```rust
fn main() {
    let big: i32 = 300;
    println!("300 as u8 = {}", big as u8);        // 44：只保留低 8 位
    println!("3.9 as i32 = {}", 3.9_f64 as i32);  // 3：朝零截断
    println!("-1 as u8 = {}", (-1_i32) as u8);    // 255：按位重解释

    let fits: i64 = 200;
    let small: u8 = fits.try_into().unwrap();     // 检查后转换：200 装得下
    println!("{small}");
}
```

装不下时 `try_into` 返回错误，`.unwrap()` 把错误变成 panic（`Result` 与错误处理是第 7 章）：

```rust,should_panic
fn main() {
    let huge: i64 = 300_000;
    let fail: u8 = huge.try_into().unwrap();
    println!("{fail}");
}
```

```text
thread 'main' (28120) panicked at src\main.rs:3:36:
called `Result::unwrap()` on an `Err` value: TryFromIntError(PosOverflow)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

（以 Rust 1.98 为例；括号里的数字是操作系统线程 ID，每次运行不同。）

### 表达式与语句：分号决定值

语句是「让程序一步步做事」，表达式是「算出一个值」。在 Rust 里，这条界线由 `;` 划开：**一个块的最后一个不带分号的表达式，就是块的值**；没有这样的表达式，块的值是单元类型 `()`。

```rust
fn main() {
    let a = { 1 + 1 };       // 块的值是 1 + 1
    let b = { 1 + 1; };      // 多一个分号，值变成 ()
    println!("a = {a}, b = {b:?}");
}
```

```text
a = 2, b = ()
```

注意编译器对第二种写法还会顺带警告：`unused arithmetic operation that must be used`——它知道你算完就把结果丢了，多半是笔误。警告不是错误，但这类警告通常就是 bug。

`if` 也是表达式，所以「按条件取一个值」不需要三元运算符：

```rust
fn main() {
    let n = 7;
    let parity = if n % 2 == 0 { "偶" } else { "奇" };
    println!("{n} 是{parity}数");
}
```

两条硬规则：两个分支必须是同一个类型（否则 E0308，见「报错现场」②）；没有 `else` 的 `if` 值是 `()`，所以 `let x: i32 = if c { 1 };` 报 E0317。分支超过两三种、或者要按「值的形状」分支时，用 `match`——它同样是表达式（第 5 章）。

### 函数：尾表达式与 `return`

```rust
fn double(n: i32) -> i32 {
    n * 2           // 尾表达式（tail expression）：这就是返回值
}

fn early(n: i32) -> i32 {
    if n < 0 {
        return 0;   // 提前退出才用 return
    }
    n * 3
}

fn main() {
    println!("{} {}", double(21), early(-5));
}
```

- 参数必须写类型；返回类型写成 `-> T`，不写就是 `()`。函数体是一个块表达式——「块的值就是返回值」与上一节是同一条规则。
- 风格约定：**最后一个表达式的值默认就是返回值**，`return` 留给提前退出（`early` 里的那句）。忘了去掉尾表达式的分号，函数就返回 `()`（见「报错现场」①）。
- 同一模块内，函数在定义之前就能被调用——Rust 的条目（item）解析与书写顺序无关。
- 三样东西 Rust 没有，先记下来免得以后白找：**没有函数重载**、**没有默认参数**、**没有可变参数**。同一个名字只有一份定义；需要「同一操作作用于多种类型」时用泛型（generics）与 trait（行为契约，第 9 章），需要可选参数时用 `Option` 或构造式 API（第 18 章）。
- 名字后面带 `!` 的是宏（macro，第 1 章已见，第 27 章展开）；`pub` 控制可见性（第 8 章展开）。

### 控制流：`if`、`loop`、`while`、`for`

Rust 的条件必须是 `bool`，不接受「非零即真」。四种循环各有位置：

- `loop`：无限循环，也是**唯一允许 `break` 带值**的循环——「先做事，达到条件时把结果交出来」写起来很自然。
- `while 条件`：条件为真就继续。`while` 与 `for` 里不能 `break` 带值（E0571）。
- `for x in 序列`：遍历。`0..n` 是左闭右开范围（含头不含尾），`0..=n` 含尾。`for` 是迭代器（iterator）的语法糖，第 10 章展开。
- `continue` 跳到下一轮，`break` 结束整个循环；给循环加标签（`'outer:`）就能一次跳出多层。

```rust
fn main() {
    let mut count = 0;
    let found = loop {
        count += 1;
        if count == 3 {
            break count * 10;
        }
    };
    println!("loop 返回 {found}");

    let mut i = 0;
    while i < 3 {
        i += 1;
    }
    println!("while 结束 i = {i}");

    for v in [1, 2, 3, 4] {
        if v % 2 == 0 {
            continue;
        }
        print!("{v} ");
    }
    println!();

    'outer: for a in 0..3 {
        for b in 0..3 {
            if a * b == 2 {
                println!("a={a}, b={b}");
                break 'outer;
            }
        }
    }
}
```

选择顺序可以记成一句：知道次数或要遍历集合就用 `for`；条件驱动用 `while`；「至少做一次再决定停不停」，或者循环要把一个结果带出来，用 `loop`。别再写 `while (true)`——那正是 `loop`。

### 注释与文档注释

```rust
//! 写在文件开头的模块级文档注释（doc comment；第 8 章讲模块，第 32 章讲文档工程）。

// 行注释：从这儿到行尾
/* 块注释；可以嵌套 /* 内层也生效 */ 这一点和 C/C++ 不同 */

/// 文档注释：附在下面的条目上，会被 rustdoc 收集（`cargo doc` 渲染成 HTML）。
///
/// 返回 `n` 的两倍。
fn double(n: i32) -> i32 {
    n * 2
}

fn main() {
    println!("{}", double(2));
}
```

`///` 与 `//!` 都是文档注释，区别只在「挂在谁身上」：`///` 挂它下面那个条目，`//!` 挂它所在的作用域（文件/模块）本身。写不写文档注释是可选的；把 public API 写清楚是第 32 章的主题。

## 动手看看

把本章的元素合成一个完整程序，源码在 `snippets/ch03/tour.rs`：

```rust
{{#rustdoc_include ../snippets/ch03/tour.rs}}
```

把它整段贴进 `src/main.rs`，`cargo run` 的真实输出（本机 1.98）：

```text
遮蔽后 x = 6
42 默认是 i32，3.5 默认是 f64
char 占 4 字节，'中' 的标量值 = 20013
元组：1 和 两个，解构后 1 / 两个
数组长度 3，第一个元素 10
7 是奇数
double(21) = 42
loop 返回 30
while 结束 i = 3
奇数之和 = 4
300 as u8 = 44
200_i64.try_into::<u8>() = 200
```

十二行输出对应正文七组元素，建议逐行回指一遍；其中「42 默认是 i32」「操作数类型不同就报错」「`as` 静默丢数据、`try_into` 会拒绝」这三件事，是本章最值得带走的。

## 报错现场

下面每一组都是最小复现加 1.98 实测的关键行。把它们放进 `src/main.rs` 后 `cargo run`，得到的就是这些输出（省略了与结论无关的行）。

### ① 分号：多一个、少一个

少写分号把两条语句粘在一起：

```rust,compile_fail
fn main() {
    let a = 1
    let b = 2;
    println!("{} {}", a, b);
}
```

```text
error: expected `;`, found keyword `let`
 --> src\main.rs:2:14
  |
2 |     let a = 1
  |              ^ help: add `;` here
3 |     let b = 2;
  |     --- unexpected token
```

多写分号则会让块的值变成 `()`，于是函数返回值对不上：

```rust,compile_fail
fn five() -> i32 {
    5;
}

fn main() {
    println!("{}", five());
}
```

```text
error[E0308]: mismatched types
 --> src\main.rs:1:14
  |
1 | fn five() -> i32 {
  |    ----      ^^^ expected `i32`, found `()`
  |    |
  |    implicitly returns `()` as its body has no tail or `return` expression
2 |     5;
  |      - help: remove this semicolon to return this value
```

定位路径：两处 `help` 都直接给出改法（``help: add `;` here`` 与 `help: remove this semicolon`），照改即可。记住结论句里的 `found ()`：只要看到「期待 `i32`、得到 `()`」，先找是不是尾表达式的分号多写了。

### ② 分支类型不一致

```rust,compile_fail
fn main() {
    let flag = true;
    let x = if flag { 1 } else { "no" };
    println!("{x}");
}
```

```text
error[E0308]: `if` and `else` have incompatible types
 --> src\main.rs:3:34
  |
3 |     let x = if flag { 1 } else { "no" };
  |                       -          ^^^^ expected integer, found `&str`
  |                       |
  |                       expected because of this
```

定位路径：报错不在出错的那一臂上，而是在**另一臂**（`1` 下面那个 `-`）——`if` 是表达式，两个分支必须汇成同一个类型。修法有三种：让两臂同类型、把 `let` 的类型标注改成能容纳两者的写法（比如先各自 `.to_string()`），或者改成 `if` 只做副作用、不用它的值。

### ③ 整数溢出：编译期一条路，运行期两条路

编译期就能算出来的溢出，被 deny-by-default 的 lint 直接拦下：

```rust,compile_fail
fn main() {
    let max: i32 = i32::MAX;
    let x = max + 1;
    println!("{x}");
}
```

```text
error: this arithmetic operation will overflow
 --> src\main.rs:3:13
  |
3 |     let x = max + 1;
  |             ^^^^^^^ attempt to compute `i32::MAX + 1_i32`, which would overflow
  |
  = note: `#[deny(arithmetic_overflow)]` on by default
```

（1.98 实测：`cargo check` 对这个程序退出码是 0，`cargo run` 才报错——别只用 `check` 判断「能不能跑」。）

运行期才发生的溢出，取决于构建 profile。程序在 `snippets/ch03/overflow.rs`：

```rust,should_panic
{{#rustdoc_include ../snippets/ch03/overflow.rs}}
```

`cargo run`（debug）：

```text
加 1 之前：127

thread 'main' (23960) panicked at src\main.rs:4:5:
attempt to add with overflow
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

`cargo run --release`（同一个程序，没有重新编译成别的逻辑）：

```text
加 1 之前：127
加 1 之后：-128
```

这就是「整数溢出的两种行为」：debug 里 panic（退出码 101），release 里按二进制补码环绕（`127 + 1` 变成 `-128`）。原因不是「优化把检查删了」，而是**溢出检查本来就是一个可配置的开关**：cargo 的 `dev` profile 打开 `overflow-checks`，`release` 关闭；rustc 层面由 `-C overflow-checks` 控制，默认跟随 `-C debug-assertions`。想让 release 也检查，在 `Cargo.toml` 里写：

```toml
[profile.release]
overflow-checks = true
```

反过来，如果你**就是要**环绕或饱和语义，别赌 profile 配置，用方法把意图写死：

```rust
fn main() {
    let n: i8 = 127;
    println!("{}", n.wrapping_add(1));    // 环绕
    println!("{}", n.saturating_add(1));  // 饱和
    println!("{:?}", n.checked_add(1));   // 检查：失败返回 None
}
```

```text
-128
127
None
```

### ④ 类型不匹配：数值类型之间没有隐式转换

```rust,compile_fail
fn takes_i64(v: i64) -> i64 {
    v * 2
}

fn main() {
    let a: i32 = 21;
    println!("{}", takes_i64(a));
}
```

```text
error[E0308]: mismatched types
 --> src\main.rs:7:30
  |
7 |     println!("{}", takes_i64(a));
  |                    --------- ^ expected `i64`, found `i32`
  |                    |
  |                    arguments to this function are incorrect
  |
note: function defined here
 --> src\main.rs:1:4
  |
1 | fn takes_i64(v: i64) -> i64 {
  |    ^^^^^^^^^ ------
help: you can convert an `i32` to an `i64`
  |
7 |     println!("{}", takes_i64(a.into()));
  |                               +++++++
```

定位路径：注意 `help` 给的是 `.into()` 而不是 `as`——`i32` 到 `i64` 是**无损**转换，标准库为它实现了 `From`，所以 `into()` 能用；`as` 通常也能写，但那是「强制转换」的语气，而无损转换用 `into()` 更能表达意图。反过来，`i64` 到 `i32` 可能丢数据，就不给 `into()`，要么 `as`、要么 `try_into()`。

同样的错误还有另一副面孔——条件不是 `bool`：

```rust,compile_fail
fn main() {
    let flag = 1;
    if flag {
        println!("真");
    }
}
```

```text
error[E0308]: mismatched types
 --> src\main.rs:3:8
  |
3 |     if flag {
  |        ^^^^ expected `bool`, found integer
```

### ⑤ 不可变赋值与未初始化

```rust,compile_fail
fn main() {
    let n = 3;
    n += 1;
    println!("{n}");
}
```

```text
error[E0384]: cannot assign twice to immutable variable `n`
 --> src\main.rs:3:5
  |
2 |     let n = 3;
  |         - first assignment to `n`
3 |     n += 1;
  |     ^^^^^^ cannot assign twice to immutable variable
  |
help: consider making this binding mutable
  |
2 |     let mut n = 3;
  |         +++
```

```rust,compile_fail
fn main() {
    let x: i32;
    if x > 0 {
        println!("yes");
    }
}
```

```text
error[E0381]: used binding `x` isn't initialized
 --> src\main.rs:3:8
  |
2 |     let x: i32;
  |         - binding declared here but left uninitialized
3 |     if x > 0 {
  |        ^ `x` used here but it isn't initialized
  |
help: consider assigning a value
  |
2 |     let x: i32 = 42;
  |                ++++
```

定位路径：`E0384` 只能靠改代码解决（加 `mut`，或者承认这里该用遮蔽建一个新名字）；`E0381` 的「延迟初始化」是合法写法——`let x; if c { x = 1 } else { x = 2 }` 能过，编译器会检查**所有**取值路径都赋过值。

## 小结

- 绑定默认不可变，`mut` 是唯一的开关；遮蔽是新建绑定（可以换类型），`mut` 是同一个绑定换值（不能换类型）。
- 标量：整数（默认 `i32`）、浮点（默认 `f64`）、`bool`、`char`（Unicode 标量值，4 字节）；复合：元组与定长数组 `[T; N]`。类型能推断就不写，函数签名必须写。
- 数值类型之间没有隐式转换：`as` 是「我知道可能丢数据」，`try_into()` 是「可能失败，给你 `Result`」；无损转换优先用 `into()`。
- 块、`if`、`loop` 都是表达式；`;` 把值丢掉换成 `()`。`if` 两臂必须同类型，`loop` 可 `break` 带值。
- 函数体是块表达式，尾表达式的值就是返回值；`return` 只用于提前退出；没有重载、默认参数、可变参数。
- 整数溢出的行为由 profile 决定：debug panic、release 环绕；要写死语义就用 `wrapping_add` / `saturating_add` / `checked_add`。

## 练习

1. Rust 为什么不给 `i32` 到 `i64` 做隐式转换？这挡住了哪些其他语言里常见的 bug？在什么情况下你会写 `as`，什么时候改用 `try_into()`？

2. 下面三段各自报什么错？分别说出错因：

   ```rust,compile_fail
   // A
   fn f() -> i32 {
       1 + 1;
   }

   fn main() {
       println!("{}", f());
   }
   ```

   ```rust,compile_fail
   // B
   fn main() {
       let x: i32;
       println!("{x}");
   }
   ```

   ```rust,compile_fail
   // C
   fn main() {
       let a: u8 = 1;
       let b: i32 = 2;
       let c = a + b;
       println!("{c}");
   }
   ```

3. 下面这段**编译失败**。猜是哪一行、报哪条错误；删掉 `mut` 又会怎样？

   ```rust,compile_fail
   fn main() {
       let mut n = 1;
       let n = n + 1;
       n += 1;
       println!("{n}");
   }
   ```

<details><summary>参考答案</summary>

1. 隐式转换意味着「你自己都没意识到」的换型，这正是 bug 的高发区：C/C++ 的整型提升与有符号/无符号混用（例如把 `int` 与 `sizeof` 的结果比较，`int` 被抬成无符号后负数变巨大）；Java 把窄化转换误当提升用而丢高位；Python 甚至不区分整数与浮点以外的数值宽度，换成大整数时会静默改性能与溢出语义。Rust 要求显式，是为了让「这里换了类型」成为代码里可见的事实：`as` 用于**你确认可以丢数据或按位重解释**的场合（比如把 `u32` 的位模式当 `i32`，或明确知道取值范围），`try_into()` 用于**可能失败、需要检查**的场合（把外部输入、来自宽类型的值收窄）。判断标准是「失败是否可能」：不可能失败用 `into()`，可能失败用 `try_into()`，需要丢数据才用 `as`。

2. A 报 `E0308: mismatched types`，`expected i32, found ()`：函数体的块值由尾表达式决定，`1 + 1;` 结尾多了分号，值被丢掉，块的值变成 `()`，与声明的 `i32` 冲突；报错里会附 `help: remove this semicolon`。B 报 `E0381: used binding x isn't initialized`：`let x: i32;` 只声明不赋值，`println!` 读到的是未初始化绑定；注意 Rust 允许「先声明后赋值」，但所有取值路径都必须赋过值。C 报两条错：`E0308: mismatched types`（`expected u8, found i32`）与 `E0277: cannot add i32 to u8`（附带 `help: the trait Add<i32> is not implemented for u8`）：`a` 是 `u8`、`b` 是 `i32`，`+` 要求两边同一类型，Rust 不做隐式提升，改法是显式写 `a as i32 + b` 或 `i32::from(a) + b`。

3. 失败在 `n += 1;`，报 `E0384: cannot assign twice to immutable variable`（并提示 `consider making this binding mutable`）。原因是 `let n = n + 1;` 那行是**遮蔽**：它新建了一个不可变绑定，把外层 `let mut n` 的名字遮住了，而可改性属于绑定本身，不会跟着名字传下去。删掉 `mut` 的话，`let mut n = 1;` 变成 `let n = 1;`——但程序仍然失败，因为后面那个新绑定依旧不可变，`n += 1` 照样报 `E0384`。想让它编译过，要么不要遮蔽（`let mut n = 1; n += 1;`），要么两处都写 `let mut`。

</details>

## 延伸阅读

- The Book 第 3 章（变量、类型、函数、注释、控制流五节都在这里）：<https://doc.rust-lang.org/book/ch03-00-common-programming-concepts.html>
- The Reference：变量（`let` 绑定、`mut`、遮蔽的规范定义）：<https://doc.rust-lang.org/reference/variables.html>
- The Reference：语句与表达式（`;`、块值的正式规则）：<https://doc.rust-lang.org/reference/statements-and-expressions.html>
- The Reference：数值类型（各整数类型的取值范围表）：<https://doc.rust-lang.org/reference/types/numeric.html>
- The Reference：运算符，含「溢出」一节：<https://doc.rust-lang.org/reference/expressions/operator-expr.html>
- The Cargo Book：profiles（`overflow-checks`、`debug-assertions` 的默认值）：<https://doc.rust-lang.org/cargo/reference/profiles.html>
- std：整数的 `wrapping_add` / `saturating_add` / `checked_add`：<https://doc.rust-lang.org/std/primitive.i32.html>
- std：`char`：<https://doc.rust-lang.org/std/primitive.char.html>
- std：数组 `[T; N]`：<https://doc.rust-lang.org/std/primitive.array.html>
