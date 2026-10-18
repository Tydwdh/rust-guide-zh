# 第 23 章 unsafe 与 UB

## 这一章解决什么问题

第 22 章结尾留下一句结论：「不使用 `unsafe` 时，你在 pin 的世界里不可能写出不健全的代码」。这一章把 `unsafe` 本身摊开：它究竟打开了哪几扇门，每扇门后面的契约（contract）长什么样，越界之后到底会发生什么。

「越界」有个准确的名字：**未定义行为（undefined behavior，UB）**。UB 不是「结果不对」，而是编译器对程序的任何部分都不再作保证——它可能照常跑出你预期的结果，可能换个优化档就变成另一个结果，也可能直接消失。于是分辨「危险」与「UB」是本章的第二个目标：死锁很危险，但不是 UB；读一块刚被释放的内存看起来风平浪静，却是 UB。

读完这一章，你能：读懂 `unsafe fn` 的 `# Safety` 条款是谁的责任；把一处 `unsafe` 包进安全 API 并守住它的不变量；拿 Reference 的 UB 清单判断一段代码是否踩线；用 Miri 在测试里抓 UB（需要 nightly）。指针、provenance 与 `MaybeUninit` 放到第 24 章，布局与 `repr` 放到第 25 章，FFI 放到第 26 章，别名模型的正式讨论放到第 35 章——这一章只负责把判据立起来。

## 为什么这样设计

### Rust 里住着两套语言

Nomicon 开篇就是这么定位的：

> Rust can be thought of as a combination of two programming languages: Safe Rust and Unsafe Rust.

译文：可以把 Rust 看成两门语言的组合——Safe Rust 和 Unsafe Rust。

Safe Rust 是「真正的 Rust」：只要写它，你永远不会遇到悬垂指针、use-after-free，或任何形式的 UB；标准库给的设施也足够写出高性能的惯用代码。需要 Unsafe Rust 的场合只有三类：要跟别的语言说话、要写标准库没暴露的低层抽象、要做类型系统理解不了的事。

为什么非得有第二套语言？因为静态分析天生保守。The Book 的措辞很直白：

> When the compiler tries to determine whether or not code upholds the guarantees, it’s better for it to reject some valid programs than to accept some invalid programs.

译文：编译器在判断代码是否满足保证时，宁肯拒绝一些正确的程序，也不愿接受一个错误的程序。

`unsafe` 就是给这种场合准备的出口：你说「相信我」（原文 **"Trust me, I know what I’m doing."**），然后承担后果。这套划分的收益是：你拿到了对实现细节的低层控制，又不必把整个程序交给一门不安全语言。

### 不对称的信任

分界线既然存在，两边怎么相处？Reference 先给了 UB 的地位：

> Rust code is incorrect if it exhibits any of the behaviors in the following list. This includes code within unsafe blocks and unsafe functions. unsafe only means that avoiding undefined behavior is on the programmer; it does not change anything about the fact that Rust programs must never cause undefined behavior.

译文：只要代码出现下列任何一种行为，它就是错的，`unsafe` 块和 `unsafe fn` 体内的代码也不例外。`unsafe` 的含义只是「避免 UB 的责任在程序员身上」，它并不改变「Rust 程序绝不允许触发 UB」这件事。

接着是**健全性（soundness）**的定义：

> It is the programmer’s responsibility when writing unsafe code to ensure that any safe code interacting with the unsafe code cannot trigger these behaviors. unsafe code that satisfies this property for any safe client is called sound; if unsafe code can be misused by safe code to exhibit undefined behavior, it is unsound.

译文：写 unsafe 代码的人有责任保证，任何与它打交道的安全代码都无法触发上述行为。对任意安全客户端都满足这条性质的 unsafe 代码称为**健全的（sound）**；如果安全代码能把它用出 UB，它就是**不健全的（unsound）**。

注意定义里的「任意安全客户端」。Nomicon 用一句话点出随之而来的不对称：

> The design of the safe/unsafe split means that there is an asymmetric trust relationship between Safe and Unsafe Rust. Safe Rust inherently has to trust that any Unsafe Rust it touches has been written correctly. On the other hand, Unsafe Rust cannot trust Safe Rust without care.

译文：安全/unsafe 的划分意味着两者之间是一种不对称的信任关系。Safe Rust 必须信任它碰到的 Unsafe Rust 写对了；反过来，Unsafe Rust 却不能不加小心地信任 Safe Rust。

Nomicon 举的例子是 `BTreeMap`：它内部有 unsafe 代码，却必须假定外部传来的 `Ord` 实现可能不满足「全序」（`Ord` 是 safe trait，谁都能实现），所以它的 unsafe 代码要写得「即使 `Ord` 是假的也不出 UB」——结果可能错得离谱，但不会 UB。第 9 章说过 `Ord` 是安全侧无法强制的语义契约，这里的补充是：**unsafe 代码不能信任别人塞给它的实现**；要让「实现方必须满足某条编译器验不了的保证」变成可依赖的承诺，需要 `unsafe trait`（23.4）。

## 机制与规则

### 23.1 门开在哪：`unsafe` 开放的能力

`std` 的 `unsafe` 关键词页把额外能力列成五条：

- Dereference raw pointers（解引用裸指针）
- Implement unsafe traits（实现 unsafe trait）
- Call unsafe functions（调用 unsafe fn）
- Mutate statics (including external ones)（修改静态变量，含外部静态）
- Access fields of unions（访问联合体字段）

The Book 也是同样的五条（它把最后两条写成「访问或修改可变静态变量」「访问联合体字段」）。而 Reference 给出的是更长的八条清单。它在上面五条之外还做了两处收紧：静态那条是「读**或**写可变或 unsafe 的**外部**静态变量」（读也算在内，且点名外部静态），联合体那条限定为「给字段赋值除外」。此外还多出三条与平台或 edition 相关的条目：

1. 从不带 `target_feature` 属性的函数里调用带该属性的安全函数；
2. 声明 `extern` 块（2024 之前不需要 `unsafe`）——第 26 章展开；
3. 给条目应用 unsafe 属性，如 `#[unsafe(no_mangle)]`——第 26 章展开。

这里有个容易混的点：不可变静态（`static FOO: u32`）随便读；要 `unsafe` 的是**可变静态（mutable static）**。

`std` 文档还点出 `unsafe` 的两种用法：**声明契约**与**兑现契约**，别混着看。

> The unsafe keyword has two uses: to declare the existence of contracts the compiler can’t check, and to declare that a programmer has checked that these contracts have been upheld.

译文：`unsafe` 有两种用法：声明存在一条编译器无法检查的契约，以及声明程序员已经检查过、这些契约确实被遵守了。

哪边是哪边，标准库自己的总结最清楚：

| 写法 | 含义 |
|---|---|
| `unsafe fn` | 调用它意味着你必须遵守一条编译器管不了的契约 |
| `unsafe trait` | 实现它意味着你必须遵守一条编译器管不了的契约 |
| `unsafe {}` | 块内操作所需的前提，程序员已经检查并保证成立 |
| `unsafe impl` | 实现该 trait 所需的前提，程序员已经检查并保证成立 |

前两行是**声明**（有契约存在），后两行是**兑现**（我保证它成立）。所以 `unsafe` 不等于「危险」，它等于「这里有一条编译器管不了的约定」。

### 23.2 `unsafe` 不是「关掉检查」

最常见的一条误解，The Book 用一句话按下去：

> It’s important to understand that unsafe doesn’t turn off the borrow checker or disable any of Rust’s other safety checks: If you use a reference in unsafe code, it will still be checked.

译文：要明白，`unsafe` 不会关掉借用检查器，也不会禁用 Rust 的其他安全检查：在 unsafe 代码里用引用，照样会被检查。

`unsafe` 块里写两次 `&mut x`，报错和外面一模一样：

```rust,compile_fail
fn main() {
    unsafe {
        let mut x = 1;
        let a = &mut x;
        let b = &mut x;
        println!("{a} {b}");
    }
}
```

```text
error[E0499]: cannot borrow `x` as mutable more than once at a time
 --> src/main.rs:5:17
  |
4 |         let a = &mut x;
  |                 ------ first mutable borrow occurs here
5 |         let b = &mut x;
  |                 ^^^^^^ second mutable borrow occurs here
```

（输出按本章约定节选：开头还有一条 `unused_unsafe` 警告，末尾的 `error: aborting…` 与 `For more information…` 略去。）

`unsafe` 块给的是**额外能力**，不是关闭检查的开关（顺带：这个块里没有任何 unsafe 操作，rustc 会额外给一条 `unnecessary unsafe block` 警告，来自 `unused_unsafe` lint——上面节选把它省掉了）。所以要把它写小。The Book 的理由很实在：

> Keep unsafe blocks small; you’ll be thankful later when you investigate memory bugs.

译文：把 unsafe 块写小；以后查内存 bug 时你会感谢自己。

把心智模型固定下来：**`unsafe` 是一个显式标记，把「编译器管不了、但人必须管」的地方标出来**。标记越少越小，需要人审查的面积就越小。

### 23.3 `unsafe fn`：契约在调用方

`unsafe fn` 只做一件事：告诉调用方「调用我要满足一条我无法替你检查的前提」。所以前提得写下来——标准库用文档注释里的 `# Safety` 段落，`Vec::set_len` 是个典型：

> /// # Safety
> ///
> /// - `new_len` must be less than or equal to `capacity()`.
> /// - The elements at `old_len..new_len` must be initialized.

译文：`new_len` 必须小于等于 `capacity()`；`old_len..new_len` 范围内的元素必须已初始化。

读到一个 `unsafe fn`，先找 `# Safety`，再问「这个前提在我的调用点成立吗」。`std` 文档强调：这类前提编译器无法强制，所以**写下来很重要**。

调用方的义务则是写出 `unsafe {}` 和一条 `// SAFETY:` 注释，说明为什么前提成立——待会儿 `snippets/ch23/boundary.rs` 里 `get_or` 的调用点就是这个样子：

```rust,ignore
// SAFETY: 上面的判断保证了 i < x.len()。
unsafe { get_i32_unchecked(x, i) }
```

2024 改了这里的一个老习惯：以前 `unsafe fn` 的函数体本身就是隐式 `unsafe` 块，`unsafe` 一个记号同时承担两个角色（既声明「调用方要守约」，又声明「函数体内的操作我已保证」）。`std` 文档解释了为什么要拆开：

> On old editions, unsafe fn also acts like an unsafe {} block around the code inside the function. This means it is not just a signal to the caller, but also promises that the preconditions for the operations inside the function are upheld. Mixing these two meanings can be confusing, so the unsafe_op_in_unsafe_fn lint has been introduced and enabled by default since edition 2024 to warn against that and require explicit unsafe blocks even inside unsafe fn.

译文：在旧 edition 上，`unsafe fn` 还兼作函数体外面的一个 `unsafe {}` 块。这意味着它不只是给调用方的信号，还顺带承诺了函数体内各操作的前提成立。把这两种含义混在一起会让人困惑，所以引入了 `unsafe_op_in_unsafe_fn` lint，并从 2024 edition 起默认启用，用来警告这种情况，并要求即使在 `unsafe fn` 里也要写出显式的 `unsafe` 块。

Edition Guide 的说法更短：

> The unsafe_op_in_unsafe_fn lint now warns by default.

译文：`unsafe_op_in_unsafe_fn` lint 现在默认警告。

也就是说，2024 的 `unsafe fn` **只**声明契约，不再替函数体兜底：体内照样要有 `unsafe {}`（`snippets/ch23/boundary.rs` 里那个模块把这条 lint 提成 `deny`，防止自己写漏）。

### 23.4 `unsafe trait`：承诺在实现方

`unsafe trait` 把责任压在另一端：实现它的人必须守住一条编译器验不了的契约。最著名的三个是 `Send`、`Sync` 与 `GlobalAlloc`；前两者能自动推导，手写 `unsafe impl Send for X` 时才是在说「我检查过了，`X` 跨线程移动是安全的」（第 19 章用过这个记号）。

`unsafe trait` 的契约会变成 unsafe 代码可以依赖的**前提**：只要实现方守约，别的代码就有理由去调用原本不该调用的 `unsafe fn`。`std` 文档给的例子是「实现方承诺一定返回偶数」的 trait，让一个安全函数敢于用 `unreachable_unchecked`——本书把它完整地写进了 `snippets/ch23/boundary.rs`（见「动手看看」）。这里只把**撒谎的实现**单独拎出来：与那边相比，唯一的差别是 `value` 直接返回 `*self`。

```rust,no_run
unsafe trait Even {
    fn value(&self) -> i32;
}

// 撒谎的实现：3 是奇数，却自称满足「返回偶数」的契约
unsafe impl Even for i32 {
    fn value(&self) -> i32 {
        *self
    }
}

fn half_of_even<T: Even>(x: T) -> i32 {
    if x.value() % 2 == 0 {
        x.value() / 2
    } else {
        // SAFETY: 契约保证 value() 是偶数，这个分支不可达。
        unsafe { std::hint::unreachable_unchecked() }
    }
}

fn main() {
    println!("half = {}", half_of_even(3i32));
}
```

于是 `half_of_even(3)` 这个**完全安全的调用**走进了 UB。未优化构建下 1.98 当场拦住了它（括号里的线程号每次运行都不同）：

```text
thread 'main' (5456) panicked at src/main.rs:17:18:
unsafe precondition(s) violated: hint::unreachable_unchecked must never be reached

This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.
```

最后一行是这一章最该记住的一句：这是 Rust 给一部分 unsafe 操作加的**尽力而为**的检查——官方文档明说它可能不触发，绝不能当成保证。开关跟随 `-C debug-assertions`（默认在未优化构建里开着）：本机实测，默认构建与 `-C opt-level=2 -C debug-assertions=on` 会触发，`-C debug-assertions=off`、`-C opt-level=1`、`-C opt-level=2` 都不触发。它也不是 1.98 才有的——1.78 的发布说明（Libraries 段）写着 "Many unsafe precondition checks now run for user code with debug assertions enabled"，1.80 又给 `unchecked_{add,sub,neg,mul,shl,shr}` 补上了检查（原文：Add `assert_unsafe_precondition` to `unchecked_{add,sub,neg,mul,shl,shr}` methods）。至于同一份代码加 `-O`：它没有任何输出就消失，退出码 `0xC000001D`（Windows 的非法指令）——UB 被优化器拿去做假设的典型表现。

两个细节容易看反：`value` 是**安全函数**，调用它不需要遵守任何前提，只有实现 `Even` 的人需要；`half_of_even` 也是安全函数，内部却用了 unsafe 操作——安全函数里可以有 `unsafe` 块，只要它对外不要求调用方做任何事（23.9 说的「把 unsafe 关进安全 API」）。`std` 文档提醒过，`unsafe trait` 里放安全函数、与安全 trait 里放 `unsafe fn`，是两种不同的形状，别混。

第 16 章还见过第三种组合：`unsafe impl<#[may_dangle] T> Drop for Vec<T>`——`Drop` 本身是安全 trait，「不访问 `T` 里的数据」这条无法验证的额外保证用 `unsafe impl` 声明。

### 23.5 可变静态

可变静态是一块全局可写的内存，编译器无法保证没有数据竞争，所以读写它需要 `unsafe`。2024 还收紧了一步：**对可变静态取引用本身**就被拒，`static_mut_refs` 这条 lint 默认 deny。下面是最小复现（默认 `edition 2024`）：

```rust,compile_fail
static mut COUNTER: u32 = 0;

fn main() {
    unsafe {
        COUNTER += 1;
        println!("{}", COUNTER);
    }
}
```

```text
error: creating a shared reference to mutable static
 --> src/main.rs:6:24
  |
6 |         println!("{}", COUNTER);
  |                        ^^^^^^^ shared reference to mutable static
  |
  = note: shared references to mutable statics are dangerous; it's undefined behavior if the static is mutated or if a mutable reference is created for it while the shared reference lives
  = note: for more information, see <https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html>
  = note: `#[deny(static_mut_refs)]` (part of `#[deny(rust_2024_compatibility)]`) on by default
```

`println!` 的格式化参数要取引用，于是撞上这条规则；改成 `let v = COUNTER;` 再打印只读的一份副本就没事。真正的答案通常是别用它：需要全局可变状态时用 `AtomicU32`（第 20 章）或 `Mutex`（第 19 章），让编译器帮你排除数据竞争。2024 对 `extern` 块和 `no_mangle` 一类属性的同样要求（`unsafe extern`、`#[unsafe(no_mangle)]`）留到第 26 章。

### 23.6 UB 清单

写 unsafe 代码时手边要有一份判据。Reference 列的是这一份（标题为「Behavior considered undefined」），逐条如下，括号里是原文用语：

1. **数据竞争（data races）**；
2. 从**悬垂的**或**基于未对齐指针的**位置读写（悬垂 = 指向的字节并非全部属于同一个活着的分配；尺寸为 0 的指针永远不算悬垂，哪怕它是空指针）；
3. 做一次越界的**偏移式位置投影（offsetting place projection）**——即字段访问、元组索引、数组/切片索引；
4. **破坏指针别名规则（aliasing rules）**；
5. **修改不可变字节（mutating immutable bytes）**：常量提升出来的字节、`static`/`const` 初始化里被延长到 `'static` 的借用、不可变绑定或静态拥有的字节（`UnsafeCell` 内的除外），以及共享引用（含经 `Box` 与嵌套字段到达的）指向的字节；
6. 通过编译器内部函数（intrinsic）触发 UB；
7. 执行当前平台不支持的平台特性（`target_feature`）——除非该平台明确文档化过这样用是安全的；
8. 用错误的**调用 ABI（call ABI）**调用函数，或让展开（unwind）越过不允许展开的栈帧；
9. **产生无效值（producing an invalid value）**；
10. 内联汇编（inline assembly）用法错误；
11. 违反 Rust 运行时的假设（例如栈帧没跑析构就被释放）。

第 9 条最需要展开，因为「无效」是**按类型**定义的：`bool` 只能是 0 或 1；函数指针不能为空；`char` 不能是代理对、不能超过 `char::MAX`；`!` 类型的值根本不允许存在；整数、浮点与裸指针必须已初始化；枚举要有合法的判别式；结构体/元组/数组要求每个字段都有效；引用与 `Box` 必须对齐、非空、不悬垂，并指向有效值。Reference 把这条说得毫无余地：

> The Rust compiler assumes that all values produced during program execution are “valid”, and producing an invalid value is hence immediate UB.

译文：Rust 编译器假定程序执行期间产生的所有值都是「有效」的，因此产生无效值立即构成 UB。

「产生」的触发点包括：给位置赋值、从位置读出、传给函数或算符、从函数返回。另外：未初始化内存对任何有取值限制的类型都隐含地无效（只有联合体与填充字节可以读）；UB 还会**污染整个程序**——C 侧触发的 UB 同样会带上其中的 Rust 代码。

Reference 也提醒，这份清单不是终审判决：

> The following list is not exhaustive; it may grow or shrink. … Please read the Rustonomicon before writing unsafe code.

译文：下面这份清单并不完备，它可能变长也可能变短。……写 unsafe 代码之前请先读 Rustonomicon。

### 23.7 危险但不是 UB

另一半判据同样重要：下面这些行为**不是** UB（Reference 的「Behavior not considered unsafe」），所以它们不会因为「我写了 `unsafe`」而变得合法或非法——它们是错误，但不是 UB：

- 死锁；
- 内存与其他资源泄漏；
- 不调用析构就退出（`std::process::exit`、`mem::forget`）；
- 通过指针泄漏暴露随机化的基址；
- **整数溢出**：debug 构建必须插入检查并 panic，其他构建可以 panic 也可以按二进制补码环绕，环绕的结果本身是定义好的，只是「算错了」；
- 逻辑错误：例如实现 `Hash` 与 `Eq` 时忘了「相等的值必须有相同哈希」，或者改动 `BTreeMap` 里的键。

把两类分开的价值很直接：死锁与逻辑错误都不是 UB，也就不在 Miri 的 UB 检查清单里；「这个 bug 难以复现」同样不是 UB 的证据。反过来，遇到确实 UB 的代码，别指望「我这边跑起来是对的」能证明什么——「动手看看」里那两条输出已经说明问题。

### 23.8 用 Miri 抓 UB

Miri 是官方提供的动态 UB 检测器。The Book 的对比很清楚：

> Whereas the borrow checker is a static tool that works at compile time, Miri is a dynamic tool that works at runtime.

译文：借用检查器是编译期工作的静态工具，而 Miri 是运行期工作的动态工具。

它跑你的程序或测试，检查自己理解的那些规则（别名、未初始化内存、越界、无效值……）。代价是比本机执行慢得多——Miri 把程序当解释执行，不生成机器码——而且**只在 nightly 上提供**：Cargo Book 的 `cargo miri` 手册页（DESCRIPTION）里写着 "This command is only available on the nightly channel."。

```console
$ rustup +nightly component add miri
$ cargo +nightly miri test
$ cargo +nightly miri run
```

`rustup +nightly component add miri` 只往系统里装工具，不改变你项目用的工具链。想让它检查得更严，可以打开严格 provenance 模式（第 24 章解释 provenance）：

```console
$ set MIRIFLAGS=-Zmiri-strict-provenance
$ cargo +nightly miri test
```

（PowerShell 里是 `$env:MIRIFLAGS = "-Zmiri-strict-provenance"`；Unix 上是 `export MIRIFLAGS=…`。）

> [!WARNING]
> Miri 只在 nightly 上提供，第 31 章讲怎么把它接进 CI。本机只有 stable 1.98，上面的命令没有运行结果可贴；本章的 UB 演示改用 debug 与 `-O` 的结果差异。Miri 也**不是万能的**：它只检查自己建模的那部分规则（清单还可能变），跑不到的分支不检查，23.7 那一类「危险但非 UB」它不当作 UB 报——内存泄漏另算，Miri 会单独报（除非加 `-Zmiri-ignore-leaks`）。

### 23.9 把 unsafe 关进盒子

最后是工程口径：unsafe 代码不该散落在业务逻辑里，而要包成安全 API——对外只暴露安全接口，不变量自己守住。三条固定动作：**契约写在文档里**（`unsafe fn` 的 `# Safety` 段落、`unsafe trait` 的实现方义务）；**兑现写在调用点**（每个 `unsafe {}` 配一条 `// SAFETY:` 注释，标准库源码里到处都是这个约定）；**不变量自己持有**（安全 API 必须对**任意**安全客户端都健全，包括「调用者按文档组合 API」）。

边界划清之后，`unsafe` 就成了一组可审计的数字：有多少个 `unsafe {}`、各自的前提是什么、由谁保证。完全不需要时，还可以用 `#![forbid(unsafe_code)]` 让整个 crate 一个 `unsafe` 都不许出现。

## 动手看看

`snippets/ch23/boundary.rs` 是一个把上面所有规则串起来的完整程序：一个 `unsafe fn` 原语加安全外壳，一个 `unsafe trait` 加它授权的安全函数。

```rust
{{#rustdoc_include ../snippets/ch23/boundary.rs}}
```

在仓库根编译运行（Windows 写法；其他平台把 `boundary.exe` 换成 `boundary`、运行 `./boundary`）：

```console
$ rustc --edition 2024 snippets/ch23/boundary.rs -o boundary.exe
$ .\boundary.exe
```

```text
get_or(1) = 2
get_or(9) = -1
value = 42, half = 21
```

三行输出对应三条规则：`get_or(1)` 走的是「先检查、再调用 `unsafe` 原语」的安全外壳；`get_or(9)` 越界，返回兜底值，压根没碰 `unsafe`；`half_of_even` 则把 `Even` 的承诺当成了前提。

把 `unsafe` 用错的样子长什么样？下面这段代码很短，但三处都是 UB：把 `0x02` 强行解释成 `bool`（产生无效值）、释放了 `Vec` 之后继续读它的缓冲区（悬垂）、以及「什么都没发生」的表象。

```rust,no_run
fn main() {
    // 0x02 不是合法的 bool 值（合法值只有 0x00 与 0x01）
    let b: bool = unsafe { std::mem::transmute(2u8) };

    let v = vec![1i32, 2, 3];
    let p = v.as_ptr();
    drop(v); // 缓冲区已释放，p 从此悬垂

    println!("b = {b}");
    println!("old[0] = {}", unsafe { *p });
}
```

同一份源码，编译两次：

```console
$ rustc --edition 2024 src/main.rs -o ub.exe
$ .\ub.exe
```

```text
b = true
old[0] = 1
```

```console
$ rustc --edition 2024 -O src/main.rs -o ub_fast.exe
$ .\ub_fast.exe
```

```text
b = false
old[0] = 0
```

两次都 exit 0，编译器一声不吭，程序也没崩——但同一个程序给出了两组结果。这就是 UB：**问题不在「结果不对」，而在「没有结果是被保证的」**。所以 `mdbook test` 只保证示例能跑，抓 UB 要靠 Miri。

## 报错现场

### 现场一：解引用裸指针（E0133）

未优化构建下把解引用裸指针放到 `unsafe` 块外面，是最老牌的一条：

```rust,compile_fail
fn main() {
    let x = 42;
    let p = &raw const x;
    let v = *p;
    println!("{v}");
}
```

关键几行（工具链 1.98，末尾的 `error: aborting…` 与 `For more information…` 按本书约定略去）：

```text
error[E0133]: dereference of raw pointer is unsafe and requires unsafe block
 --> src/main.rs:4:13
  |
4 |     let v = *p;
  |             ^^ dereference of raw pointer
  |
  = note: raw pointers may be null, dangling or unaligned; they can violate aliasing rules and cause data races: all of these are undefined behavior
```

逐句读：`unsafe and requires unsafe block` 说的是**能力受限**，不是「你不该这么写」；`^^` 精确标出解引用那一步（`&raw const x` 那行反而安全）；`note` 列出四条越界方式——空、悬垂、未对齐、破别名，全都通向 UB。修法只有一种：把解引用放进 `unsafe {}`，并写清为什么 `p` 此刻有效。

### 现场二：`unsafe fn` 体内的 unsafe 操作（2024）

同样的代码在 2024 里换了个报法——先是警告，而且带 `E0133` 编号；这正好是 23.3 讲的那条 lint：

```rust
unsafe fn get_unchecked(x: &[i32], i: usize) -> i32 {
    *x.get_unchecked(i)
}

fn main() {
    let v = vec![1, 2, 3];
    println!("{}", unsafe { get_unchecked(&v, 1) });
}
```

```console
$ rustc --edition 2024 src/main.rs -o main.exe
```

```text
warning[E0133]: call to unsafe function `core::slice::<impl [T]>::get_unchecked` is unsafe and requires unsafe block
 --> src/main.rs:2:6
  |
2 |     *x.get_unchecked(i)
  |      ^^^^^^^^^^^^^^^^^^ call to unsafe function
  |
  = note: consult the function's documentation for information on how to avoid undefined behavior
note: an unsafe function restricts its caller, but its body is safe by default
 --> src/main.rs:1:1
  |
1 | unsafe fn get_unchecked(x: &[i32], i: usize) -> i32 {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  = note: for more information, see <https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html>
  = note: `#[warn(unsafe_op_in_unsafe_fn)]` (part of `#[warn(rust_2024_compatibility)]`) on by default
```

（输出按本章约定节选：末尾的 `warning: 1 warning emitted` 与 `For more information…` 略去。）

`note: an unsafe function restricts its caller, but its body is safe by default` 就是新版口径的总结：函数体默认安全，`unsafe` 只约束调用方。修法是给这一行套上 `unsafe {}`；想让它变成硬性错误，就在 crate 或模块上加 `#![deny(unsafe_op_in_unsafe_fn)]`，或临时用 `-D unsafe_op_in_unsafe_fn` 找出所有该补的地方（`cargo fix --edition` 也会自动迁移，见第 33 章）。

## 小结

- Safe Rust 与 Unsafe Rust 是同一门语言的两半，分界线就是 `unsafe`。它有两种用法：`unsafe fn`/`unsafe trait` 声明契约，`unsafe {}`/`unsafe impl` 是「我已经检查过」的兑现。
- `unsafe` 不是关掉检查，而是**开放五项额外能力**（解引用裸指针、调用 `unsafe fn`、访问可变静态、实现 `unsafe trait`、访问联合体字段）并给需要人审的地方打标记；借用检查器照常工作。契约的方向也不一样：`unsafe fn` 由**调用方**遵守，`unsafe trait` 由**实现方**遵守。
- 2024 起 `unsafe fn` 的函数体不再隐含 `unsafe` 块（`unsafe_op_in_unsafe_fn` 默认警告），对可变静态取引用也被 `static_mut_refs` 默认拒绝。
- UB 是一份清单（Reference 逐条），不是「结果不对」的同义词：产生无效值、悬垂读写、破坏别名规则都在里面。它与「危险但非 UB」（死锁、泄漏、整数溢出、逻辑错误）要分开。
- 检测靠 Miri（nightly）；防错靠把 `unsafe` 关进安全 API，配 `# Safety` 条款与 `// SAFETY:` 注释。

## 练习

### 练习 1：`unsafe` 块能借两次 `&mut` 吗

```rust,ignore
let mut x = 1;
unsafe {
    let a = &mut x;
    let b = &mut x;
    *a += *b;
}
```

上面这段能编译吗？为什么？如果换成裸指针绕一圈（`let p = &raw mut x;` 再 `unsafe { &mut *p }`）会怎样？这段改写是安全的吗？

<details><summary>参考答案</summary>

第一种写法**编译不过**：两次 `&mut x` 会直接报 `E0499: cannot borrow x as mutable more than once at a time`，跟有没有 `unsafe` 无关（现场见 23.2）——`unsafe` 只额外开放 23.1 那五项能力，借用检查照跑。

题里说的「换成裸指针绕一圈」是这样：

```rust,no_run
fn main() {
    let mut x = 1;
    let p = &raw mut x;
    unsafe {
        let a = &mut *p; // 经由裸指针拿到的 &mut
        let b = &mut x; // 借用检查器自己发的那个 &mut
        *a += *b;
        println!("{x}");
    }
}
```

这段改写编译器**放行**了：裸指针的解引用是五项能力之一，经由 `p` 回来的那条路径不再受借用检查约束。代价是——你手上有一个从 `p` 来的 `&mut x`，还有一个编译器自己许可的 `&mut x`，两者同时活着，正是 23.6 第 4 条「破坏指针别名规则」，属于 UB。

这段代码在本机跑起来会打印 `2`，看起来完全正确——这正是 UB 最危险的地方。这种「用 unsafe 绕过借用检查」的写法要有效，前提是你**真的知道**两个引用不会互相踩到（例如它们指向不相交的区域），而有这个把握的正规做法是把断言写进类型或 API 里，而不是散在调用点（第 35 章展开别名模型）。
</details>

### 练习 2：找出这四处 `unsafe` 的问题

```rust,ignore
static mut LOGS: Vec<String> = Vec::new();

fn log(msg: &str) {
    unsafe { LOGS.push(msg.to_string()) };
}

fn first_char(s: &str) -> char {
    unsafe { s.chars().next().unwrap_unchecked() }
}

unsafe fn cast(b: u8) -> bool {
    b != 0
}

fn main() {
    log("start");
    println!("{}", first_char(""));
    println!("{}", unsafe { cast(2) });
}
```

四处问题分别在哪？指出哪些是「契约没写清」，哪些会导致 UB。

<details><summary>参考答案</summary>

1. `static mut` + 无同步：`log` 对任意调用者开放，却在内部改可变静态。单线程看不出问题，多线程下就是数据竞争（23.6 第 1 条）——这个安全 API **不健全**，它没能在出口处排除掉会造成 UB 的用法。正规做法是换成 `Mutex` 一类自带同步的设施（第 19、20 章）。
2. `first_char("")` 取空字符串的第一个字符：`next()` 返回 `None`，`unwrap_unchecked` 的前提不成立，`char` 被从一个不存在的值「产生」出来——UB（23.6 第 9 条）。安全函数能被安全调用者喂出 UB，同样是不健全。
3. `unsafe fn cast` 的两处问题：函数体里没有任何 unsafe 操作（`b != 0` 是 safe 的），所以它不需要是 `unsafe fn`——真正该表达的是「只接受来自合法 `bool` 的字节」这条契约，若确实需要，就写进 `# Safety` 并只在保证成立时使用。
4. 这四处 `unsafe` 标记本身都用对了地方——`log` 要改可变静态、`first_char` 调 `unwrap_unchecked`、`main` 调 `unsafe fn cast`，都属于 23.1 那五项能力。问题在于它们背后没有契约：`log` 与 `first_char` 是**不健全的安全 API**（把 UB 暴露给安全调用者），`cast` 则是多余且缺 `# Safety` 的 `unsafe fn`。该做的是把不安全的东西换成能表达契约的东西：检查留在这层，`unsafe` 只放在真正做不安全操作的那一行。顺带一句：这份代码在 2024 下其实编译不过——`log` 里对 `LOGS` 取可变引用会先撞上 `static_mut_refs` 的默认 deny（`creating a mutable reference to mutable static`），同一条在 2021 下只是警告。
</details>

### 练习 3：`unsafe impl` 写错的代价

`unsafe impl Send for Wrapper` 只写一行，编译器不会拦。请说明：为什么标准库不把这个 trait 做成普通 safe trait，让编译器自动推导就好？以及手写它之前必须检查什么？

<details><summary>参考答案</summary>

`Send`/`Sync` 是 auto trait：字段全都实现了它，类型就自动实现。可编译器推导只看**结构**，看不出「内部藏着一个指向自己的裸指针」或「共享了别的线程不安全的句柄」；遇到它不认识的字段（裸指针、FFI 句柄）时推导会偏保守，这时才需要手写 `unsafe impl` 补一句「我检查过，它确实安全」。

手写之前逐条问：每个字段、以及经共享引用可达的每一块数据，跨线程移动或共享时会不会出现未同步的读写？会不会把 `!Send` 的句柄带过线程边界？会不会让两个线程同时拿到可写路径？有一条答不上来就别写——这句承诺写错，**完全安全的代码**也能把它用出 UB（机制同练习 1，被绕过的检查换成了 `Send`）。

顺带一个常见误判：`impl Unpin for X {}` 是 safe 语法、safe 编译，语义上却同样是承诺（第 22 章讲过它的「safe 语法、unsafe 后果」）。手写自动推导没给的东西之前，先问自己凭什么。
</details>

## 延伸阅读

- Reference 的 Unsafety（能力清单）：<https://doc.rust-lang.org/reference/unsafety.html>
- Reference 的 Behavior considered undefined（UB 清单，含有效值细则）：<https://doc.rust-lang.org/reference/behavior-considered-undefined.html>
- Reference 的 Behavior not considered unsafe（死锁、泄漏、整数溢出、逻辑错误）：<https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html>
- The Book 第 20.1 节（五项能力、`unsafe fn`、Miri 入门）：<https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html>
- Rustonomicon：Meet Safe and Unsafe（<https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html>）、How Safe and Unsafe Interact（<https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html>）
- `std` 的 `unsafe` 关键词页（两种用法、契约写法、`unsafe_op_in_unsafe_fn`）：<https://doc.rust-lang.org/std/keyword.unsafe.html>
- Edition Guide 的 `unsafe_op_in_unsafe_fn`（2024 变化）：<https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html>
- Miri（安装与用法）：<https://github.com/rust-lang/miri>
