# 第 2 章 读懂编译器：报错的解剖与修复路径

## 这一章解决什么问题

在第 1 章你跑通了第一个程序。接下来的几周里，你打交道最多的对象不会是标准库，也不会是文档，而是编译器的报错。

Rust 把大量检查从运行期搬到了编译期。这带来一个必须接受的交换：**程序要先编译通过才能运行**；换来的是错误在你还没离开编辑器的当下就被指出，并且指到了具体位置。别的语言里要靠运行、靠猜、靠日志才能发现的问题，在这里变成了一条条消息。

多数人在这时会走错一步：把报错当噪音，反复试到它消失，然后把「这样写能过」记成规则背下来。这样写半年，你仍然不知道自己在写什么；一旦代码复杂一点，试错就试不动了。

这一章给你另一条路：把报错当作信息源，系统地读它。读完你应该能做到：

- 把任意一条诊断（diagnostic，编译器给出的问题报告）拆成「消息、位置、上下文、建议」四块，逐块读；
- 用最小复现（minimal reproduction）把一段上百行的程序缩成十行以内的病灶；
- 认出五个最常见的错误家族，知道每个家族为什么发生、怎么修；
- 用 `cargo check` 建立「改一点、编一次」的快速循环。

本章只谈编译期。运行期的错误处理在第 7 章展开，lint 与工程化配置在第 32 章展开，所有权（ownership）的完整规则在第 4 章展开——本章负责把引子埋好。本章所有输出都用本机 `rustc 1.98.0` 实跑得到；为方便对照，示例中的文件名统一为 `main.rs`，路径信息已省略。

## 为什么这样设计

先看别的语言怎么对待「代码有问题」。

| 语言 | 检查时机 | 你要付的代价 | 你得到的信息 |
|---|---|---|---|
| C++ | 编译期（部分） | 模板错误一屏、定位时常不准 | 少 |
| Python | 运行期 | 没执行到的代码永远不报错 | 运行时的栈回溯 |
| Rust | 编译期（+ 运行期） | 必须先编过才能跑 | 精确位置 + 上下文 + 建议 |

Rust 的选择是：把所有能在编译期回答的问题都提前回答。这就是报错显得多的原因——编译器承担了更多职责，也因此知道得更多。它知道每个变量的类型、每个值的去向、每个引用的存活区间，而这些你自己不一定算得出来。把它当一个挑剔但知情的同事：他每句话都在给线索。

一条诊断里有四类信息，读的顺序也大致如此：

1. **消息**：它认为哪里错了。一句话，先读这句。
2. **位置**：出错的地方，以及与之相关的其它地方。
3. **上下文**（`= note:`）：它掌握、而你缺的那条事实，比如「`String` 没有实现 `Copy`」。
4. **建议**（`help:`）：可以照着做的修法，有时直接给一份 diff 形式的补丁。

还有一条基本区分：**error 阻止产出，warning 不阻止**。看到 warning 不要直接跳过，它往往在告诉你一条你还不了解的语言规则。

## 机制与规则

### 一条诊断的解剖

用最常见的「类型不匹配」当标本：

```rust,compile_fail
fn main() {
    let n: i32 = "42";
    println!("{n}");
}
```

编译它，得到（Rust 1.98，下同）：

```text
error[E0308]: mismatched types
 --> main.rs:2:18
  |
2 |     let n: i32 = "42";
  |            ---   ^^^^ expected `i32`, found `&str`
  |            |
  |            expected due to this

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0308`.
```

短短几行里，每一块都有固定职责：

```text
error[E0308]: mismatched types      <- ① 级别 + 错误代码 + 一句话结论
 --> main.rs:2:18                   <- ② 位置：文件:行:列
  |
2 |     let n: i32 = "42";          <- ③ 相关的源代码行
  |            ---   ^^^^ expected `i32`, found `&str`   <- ④ span 与它的标签
  |            |
  |            expected due to this <- ⑤ 次级 span 的标签
  |
  = note: ……                        <- ⑥ 编译器补充的事实（本例没有）
help: ……                            <- ⑦ 建议，可能是可直接照抄的 diff（本例没有）
```

两个 span（源码中的一段位置，本书沿用英文）要合起来读：

- `^^^^` 是**主 span**：错误发生在 `"42"` 这个字面量上；
- `---` 是**次级 span**：为什么这里会错——因为 `n` 声明为 `i32`。

编译器其实在说一句话：「你把 `&str` 放进声明为 `i32` 的变量里了，而这个变量的类型就在这一行」。习惯性地把两个 span 拼成一句话，大部分报错就不用猜了。

头部的错误代码不是装饰：`rustc --explain E0308` 会打印这个错误的完整解释和例子，末尾那行提示就是在告诉你这件事。

### warning 与 error

两者的区别只有一条：**error 让编译失败，warning 不会**。下面三段代码都能编译成可执行文件，只是编译器会提醒你。

```rust
fn main() {
    let total = 3;
    println!("程序结束");
}
```
```text
warning: unused variable: `total`
 --> main.rs:2:9
  |
2 |     let total = 3;
  |         ^^^^^ help: if this is intentional, prefix it with an underscore: `_total`
  |
  = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

```rust
use std::collections::HashMap;

fn main() {
    println!("没有用到 HashMap");
}
```
```text
warning: unused import: `std::collections::HashMap`
 --> main.rs:1:5
  |
1 | use std::collections::HashMap;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```rust
fn helper() -> i32 {
    42
}

fn main() {
    println!("hello");
}
```
```text
warning: function `helper` is never used
 --> main.rs:1:4
  |
1 | fn helper() -> i32 {
  |    ^^^^^^
  |
  = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

三条 warning 的读法一致：

- 第一行说明哪儿不对劲，`^^^` 指到具体的名字；
- `help:` 直接给出修法（变量名加下划线前缀 `_total`，表示「我知道它没被用，我是有意的」）；
- `= note: #[warn(unused_variables)] ... on by default` 告诉你这条警告由哪个 lint（编译器内置的静态检查规则）控制、默认是否开启。lint 名可以写进代码或 Cargo 配置来调整（第 32 章展开）。

warning 也可能升级为 error：edition 2024 里，给 `static mut` 取引用从「警告」变成了「拒绝编译」。所以「warning 不影响运行」不等于「可以不管」。

### 连带错误：改一个，再编一次

一个根因可以产生好几条 error。下面这段代码只是漏了 `use std::io::Write;`：

```rust,compile_fail
use std::io::stdout;

fn main() {
    let a = stdout();
    a.write_all(b"first\n");
    let b = stdout();
    b.write_all(b"second\n");
}
```
```text
error[E0599]: no method named `write_all` found for struct `Stdout` in the current scope
    --> main.rs:5:7
...
error[E0599]: no method named `write_all` found for struct `Stdout` in the current scope
    --> main.rs:7:7
...
error: aborting due to 2 previous errors
```

两条错误，一个原因——两处 span（5:7 和 7:7）正对应两次调用。这类情况应该这样处理：

1. 只看**第一条**，把它修掉；
2. 重新编译；
3. 还有错，重复。

不要一次修十条。相邻的错误常常互相牵连，按顺序修，你才能准确知道每一步改变了什么。同理，末尾的 `aborting due to N previous errors` 只是计数，不是新问题。

## 动手看看

### 五步读报错流程

把上面的内容压成一张可以照着走的清单：

1. **读第一行**：`error[E····]: ...`，先只读这句，建立假设。
2. **看位置**：`-->` 指的文件、行、列，以及 `^^^` 指到的代码。
3. **看上下文与建议**：`= note:` 补了什么事实？`help:` 给了什么改法？有 diff 就照抄。
4. **缩到最小复现**：一眼看不出来，就把程序削到只剩出错的部分（见下一节）。
5. **修完复验**：重新编译。报错消失才算修好，别靠感觉。

### 最小复现：从 8 行到 5 行

最小复现是本书里出现频率最高的动作：**删掉一切与错误无关的代码，直到剩下最小的一段仍然报同一个错**。

看一段「像真实项目里写出来」的代码：

```rust,compile_fail
fn main() {
    let scores = vec![90, 80, 70];
    let mut sum = 0;
    for score in scores {
        sum += score;
    }
    println!("总分 {sum}，一共 {} 项", scores.len());
}
```
```text
error[E0382]: borrow of moved value: `scores`
   --> main.rs:7:34
    |
  2 |     let scores = vec![90, 80, 70];
    |         ------ move occurs because `scores` has type `Vec<i32>`, which does not implement the `Copy` trait
  4 |     for score in scores {
    |                  ------ `scores` moved due to this implicit call to `.into_iter()`
...
  7 |     println!("总分 {sum}，一共 {} 项", scores.len());
    |                                        ^^^^^^ value borrowed here after move
...
help: consider iterating over a slice of the `Vec<i32>`'s content to avoid moving into the `for` loop
    |
  4 |     for score in &scores {
    |                  +
```

信息很长：三个相关位置、一条说明、一条建议。想知道「移动」到底是怎么回事，就把求和的部分删掉：

```rust,compile_fail
fn main() {
    let v = vec![1, 2, 3];
    for x in v {}
    println!("{}", v.len());
}
```
```text
error[E0382]: borrow of moved value: `v`
   --> main.rs:4:20
    |
  2 |     let v = vec![1, 2, 3];
    |         - move occurs because `v` has type `Vec<i32>`, which does not implement the `Copy` trait
  3 |     for x in v {}
    |              - `v` moved due to this implicit call to `.into_iter()`
  4 |     println!("{}", v.len());
    |                    ^ value borrowed here after move
...
note: `into_iter` takes ownership of the receiver `self`, which moves `v`
...
help: consider iterating over a slice of the `Vec<i32>`'s content to avoid moving into the `for` loop
    |
  3 |     for x in &v {}
    |              +
```

同一个错误，但一切都变短了：剩下两处相关位置，而且都是必须懂的部分。**这就是最小复现的价值**——它删掉的不是错误的难度，而是无关代码带来的噪音。

删代码用二分法最快：先删一半，错误还在 → 主犯在这一半；错误没了 → 主犯在你刚删掉的那一半（把它加回来，再对另一半重复）。经验上，一个错误缩到十行以内还没看懂，说明你还没找到主犯。

### 快速循环：cargo check

把「编一次」的成本压到最低，才能提高循环次数。日常改用：

```console
$ cargo check
```

Cargo Book 的原话是：「编译包，但不做代码生成的最后一步，因此比 `cargo build` 快」。代价是它不产出可执行文件，且只在代码生成阶段才出现的诊断不会被报告。改一点、跑一次 `cargo check`，比反复 `cargo run` 快得多。

出错的输出长这样（诊断部分与 `rustc` 一致，路径已省略）：

```console
$ cargo check
   Checking checkdemo v0.1.0 (...)
error[E0308]: mismatched types
 --> src/main.rs:2:18
  |
2 |     let n: i32 = "42";
  |            ---   ^^^^ expected `i32`, found `&str`
  |            |
  |            expected due to this

For more information about this error, try `rustc --explain E0308`.
error: could not compile `checkdemo` (bin "checkdemo") due to 1 previous error
```

修好之后：

```console
$ cargo check
    Checking checkdemo v0.1.0 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
```

注意两点：`cargo check` 把 rustc 的诊断原样转给你，只在末尾换成自己的总结句 `error: could not compile ...`；成功时那行 `Finished` 就是你可以继续往下走的信号。

## 报错现场

上面的方法要用在这些家族上。下面五条是基础篇里撞到最多的。每条都给出最小复现（全部进了本书的编译测试）、1.98 的真实输出关键行、为什么会这样，以及一个能编译通过的对照版本。

### ① E0308：类型不匹配

最小复现与完整输出见「机制与规则」一节的解剖。修法就是让类型对上：

```rust
fn main() {
    let n: i32 = 42;
    println!("{n}");
}
```

线索全在标签里：`expected `i32`, found `&str`` —— 编译器告诉你它要什么、你给了什么。如果这里的 `"42"` 本来就该是字符串形式的数字，转换的写法在第 3 章展开。

### ② E0425：找不到名字

```rust,compile_fail
fn main() {
    let total = 3;
    println!("{totl}");
}
```
```text
error[E0425]: cannot find value `totl` in this scope
 --> main.rs:3:16
  |
3 |     println!("{totl}");
  |                ^^^^
  |
help: a local variable with a similar name exists
  |
3 |     println!("{total}");
  |                   +
```

为什么：Rust 只认识你声明过的名字。**拼写错误**、**在声明的作用域之外使用**、**忘了导入**都会落到这里。本次 `help` 直接给出了改法：`+` 表示「在这里插入一个字符」，照着改即可。

修法（能编译）：

```rust
fn main() {
    let total = 3;
    println!("{total}");
}
```

注意，`help` 是编译器基于名字相似度给出的**猜测**，不是保证。改名之前看一眼语义对不对；如果 `totl` 本来就该是另一个变量，那正确答案是声明它，而不是改名。

### ③ E0599：方法不存在（trait 不在作用域）

```rust,compile_fail
use std::io::stdout;

fn main() {
    let mut out = stdout();
    out.write_all(b"hello\n");
}
```
```text
error[E0599]: no method named `write_all` found for struct `Stdout` in the current scope
    --> main.rs:5:9
     |
   5 |     out.write_all(b"hello\n");
     |         ^^^^^^^^^
     |
    ::: ...\library\std\src\io\mod.rs:1611:8
     |
1611 |     fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
     |        --------- the method is available for `Stdout` here
     |
     = help: items from traits can only be used if the trait is in scope
help: trait `Write` which provides `write_all` is implemented but not in scope; perhaps you want to import it
     |
   1 + use std::io::Write;
```

为什么：`write_all` 不是 `Stdout` 自带的方法，而是 `Write` trait（行为契约：一组方法签名，类型声明自己实现它）提供的方法；**trait 提供的方法，trait 必须在作用域里才能调用**（trait 的设计见第 9 章）。这条报错难得地给了三条线索：`the method is available for `Stdout` here`（方法确实存在，别再怀疑拼写）、`items from traits can only be used if the trait is in scope`（明确病因），以及一行可以直接照抄的 `use`。

修法：加一行导入。修好后的完整程序在 `snippets/ch02/report.rs`：

```rust
{{#rustdoc_include ../snippets/ch02/report.rs}}
```

再提醒一句：这类 `help` 里有时会附带「相似名字」建议（本例是 `out.write(...)`）。那条**不要照抄**——`write` 与 `write_all` 语义不同。`help` 是线索，不是命令。

### ④ E0382：移动之后又使用（所有权引子）

```rust,compile_fail
fn main() {
    let s = String::from("hi");
    let t = s;
    println!("{t} {s}");
}
```
```text
error[E0382]: borrow of moved value: `s`
 --> main.rs:4:20
  |
2 |     let s = String::from("hi");
  |         - move occurs because `s` has type `String`, which does not implement the `Copy` trait
3 |     let t = s;
  |             - value moved here
4 |     println!("{t} {s}");
  |                    ^ value borrowed here after move
  |
help: consider cloning the value if the performance cost is acceptable
  |
3 |     let t = s.clone();
  |              ++++++++
```

为什么：`String` 这类类型没有实现 `Copy`。所以 `let t = s;` 不是「复制一份」，而是把 `s` 的值**移动（move）**给了 `t`；移动之后 `s` 不再有效，第 4 行再读它就是错的。那句 `move occurs because `s` has type `String`, which does not implement the `Copy` trait` 注解，就是在交代这条错误的病因。

修法取决于你想要什么：

```rust
fn main() {
    let s = String::from("hi");
    let t = s.clone();   // 确实想要两份独立的数据
    println!("{t} {s}");
}
```
```rust
fn main() {
    let s = String::from("hi");
    let len = s.len();   // 只是想看一眼，借用就够了
    println!("{s} 长度为 {len}");
}
```

第一种用 `clone()` 显式复制（有开销，写在脸上）；第二种只借用（borrow，不夺走所有权地访问），移动就不会发生。**编译器给的 `help` 只提了 `clone`，因为它不知道你的意图**——该不该克隆是你的判断。所有权与借用的完整规则在第 4 章。

### ⑤ E0502：借用冲突

```rust,compile_fail
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    v.push(4);
    println!("{first:?} {v:?}");
}
```
```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> main.rs:4:5
  |
3 |     let first = &v[0];
  |                  - immutable borrow occurs here
4 |     v.push(4);
  |     ^^^^^^^^^ mutable borrow occurs here
5 |     println!("{first:?} {v:?}");
  |                ----- immutable borrow later used here
```

为什么：第 3 行的 `&v[0]` 是共享借用（`&`），第 4 行的 `push` 需要可变借用（`&mut`）。规则是：同一时刻，共享借用可以有多个，可变借用只能有一个，且不能与共享借用共存。这条规则由借用检查器（borrow checker）在编译期执行。三条 span 组成一条完整证据链——共享借用在此发生、可变借用在此发生、共享借用**此后还要用**；最后一条是关键，没有它就不算冲突。

注意这条错误**没有 `help`**：编译器也不确定你想做什么，只能把事实摆出来（第 4 章会把这条规则和它的理由讲全）。修法：调整顺序，让两个借用不重叠：

```rust
fn main() {
    let mut v = vec![1, 2, 3];
    v.push(4);          // 先改
    let first = v[0];   // 后取；i32 是 Copy 类型，取出来就是一份新值
    println!("{first:?} {v:?}");
}
```

运行结果是：

```text
1 [1, 2, 3, 4]
```

### 把报错读成设计反馈

把这五条放在一起看，它们并不杂乱：

- E0308、E0425 是「你写的东西和你想的不一样」——名字或类型没对上；
- E0599 是「缺了一条声明」——trait 没进作用域；
- E0382、E0502 是「**数据结构没想清楚**」——谁拥有这个值、谁只是在看它、要看多久。

前两类是打字和记忆问题，后一类是设计问题。所有权报错频繁出现，通常不是编译器在刁难你，而是它在指出：这段代码里有两个地方都想要同一个值的控制权。这时正确的动作不是到处加 `clone()` 把报错压下去，而是回到数据结构，决定「谁拥有它」。

这也是本书把诊断训练放在第 2 章的原因：从今天起，每条报错都是一次设计反馈。判断的依据，第 4 章给。

## 小结

- 一条诊断 = 消息 + 位置 + 上下文（`note`）+ 建议（`help`）；**主 span 说「在哪儿」，次级 span 说「为什么」**，两个要一起读。
- `error` 阻止产出，`warning` 不阻止；但 warning 一样在告诉你语言规则，别当噪音。一个根因常常生成多条 error：修第一条，重新编译，再修下一条。
- 看不出问题时缩到最小复现——删掉无关代码，而不是删掉难度；二分法最快。
- `cargo check` 跳过代码生成，是日常最省时间的循环；`cargo build` 才产出可执行文件。
- 五个常见家族：E0308 类型不匹配、E0425 找不到名字、E0599 方法/trait 不在作用域、E0382 移动后使用、E0502 借用冲突。前两个看名字和类型，后三个看所有权与借用。
- 报错是设计反馈，不是判决书。

## 练习

1. 下面这段代码有一个编译错误。用五步流程读它，写出「主 span 指什么、次级 span 指什么」，再修好它。

```rust,compile_fail
fn main() {
    let count = 0;
    println!("{counts}");
}
```

2. 不看答案，预测下面两段代码哪段能编译、哪段不能，并说明原因。

```rust
fn main() {
    let s = String::from("hi");
    let len = s.len();
    println!("{s} {len}");
}
```
```rust,compile_fail
fn main() {
    let s = String::from("hi");
    let t = s;
    println!("{s}");
}
```

3. 下面这段代码想先把 `40` 加进 `nums`，再打印第一个元素和整个向量，但报 E0502。解释它为什么冲突，并给出至少两种修法。

```rust,compile_fail
fn main() {
    let mut nums = vec![10, 20, 30];
    let first = &nums[0];
    nums.push(40);
    println!("{first} {nums:?}");
}
```

<details><summary>参考答案</summary>

**1.** 主 span 是 `counts` 这六个字符（`^^^^^^`）：报错发生在使用这个名字的地方；这个错误**没有次级 span**——因为 `counts` 这个名字压根不存在，编译器没有「为什么」可指。它的 `help` 是名字相似度猜测：把 `counts` 改成 `count` 即可。修法：

```rust
fn main() {
    let count = 0;
    println!("{count}");
}
```

注意改名修好的是「名字对不上」，不是「语义」。如果本意是另一个计数器，正确做法是声明它。

**2.** 第一段能编译：`s.len()` 只借用 `s`（方法签名是 `&self`），借用结束后 `s` 依然有效。第二段不能编译：`let t = s;` 把 `s` 移动给了 `t`，之后 `println!("{s}")` 读的是已失效的值，报 E0382。这是本章第 ④ 条的翻版——它和「能不能用」无关，只和「谁拥有」有关。

**3.** 冲突发生在第 3 行与第 4 行之间：`&nums[0]` 产生了共享借用，`nums.push(40)` 需要可变借用，而第 5 行还要继续使用那个共享借用，因此两个借用必须共存。两种修法：

- 调整顺序，让借用不重叠（把 `push` 提到取引用之前）。这是本章第 ⑤ 条给的对照版本。
- 不长期持有借用：把需要的值立刻取出来（`let first = nums[0];`，`i32` 是 `Copy` 类型），之后再 `push`。

如果两个修法都别扭，通常是信号：你需要重新想想「谁拥有 `nums`、谁只是借看」。第 4 章会给你判断标准。

</details>

## 延伸阅读

- 错误索引（每个 `E····` 代码的官方解释）：<https://doc.rust-lang.org/error_codes/error-index.html>
- Cargo Book 的 `cargo check` 手册页：<https://doc.rust-lang.org/cargo/commands/cargo-check.html>
- rustc 一书的 lint 列表（warning 名字的权威来源）：<https://doc.rust-lang.org/rustc/lints/index.html>
- 下一章要展开的所有权：<https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html>
