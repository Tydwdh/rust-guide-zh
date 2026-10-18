# 第 37 章 编译器与代码生成

## 这一章解决什么问题

前面三十几章的结论，多半建立在一个前提上：你知道编译器会对你的代码做什么。迭代器链为什么能和手写循环编译成同一件事（第 10 章）、`unsafe` 的边界为什么划在那里（第 23 章）、`&dyn Trait` 为什么多一个机器字（第 13 章）——答案最后都指向同一个地方：**编译流水线**。

读完本章你应当能：

- 说清 HIR、MIR、LLVM IR、机器码四段各干什么，并用 `--emit=mir|llvm-ir|asm|obj` 把对应产物落到磁盘上；
- 读懂一段 MIR：基本块、终结指令、返回值槽，以及为什么「整数溢出检查」在 MIR 里是一条 `assert`；
- 说清单态化（monomorphization）发生在哪一步、什么样的泛型会真的撑大二进制，并用符号表数出来；
- 把 `-C` 选项和最终代码对上：`opt-level`、`overflow-checks`、`debuginfo`、`codegen-units` 各在哪一层留痕；
- 写出第一条 `asm!`，知道它为什么是 `unsafe`；知道 naked function 的规则与它稳定的版本；
- 说出 LTO 发生在哪一层。

**边界**：性能方法学与 PGO 在第 29 章；字节级的布局账在第 25 章；UB 与优化的关系在第 23 章；`panic` 策略与交叉编译在第 40 章。

> [!NOTE]
> 本章所有产物都是本机实跑出来的，口径与全书一致：Windows x86-64、Rust 1.98（随附 LLVM 22.1.8）。但请记住 rustc 自己就声明过这些中间格式不稳定——下面这段是 `--emit=mir` 生成文件的开头三行，逐字照抄：

```text
// WARNING: This output format is intended for human consumers only
// and is subject to change without notice. Knock yourself out.
// HINT: See also -Z dump-mir for MIR at specific points during compilation.
```

读结构，不要背字面；换个 Rust 版本，输出就会变。

## 为什么这样设计

心智模型：**编译器不是「读源码、写机器码」的一趟翻译，而是把程序一级一级「降低」（lowering）**。每一层都比上一层更接近机器，携带的语义更少：

- **HIR（high-level IR）**：宏已展开、语法糖已拆、名字已解析，但泛型还是「一份代码代表所有类型」。
- **MIR（mid-level IR）**：控制流被压成基本块加跳转。借用检查、常量求值、大部分语言内的优化都在这一层做。
- **LLVM IR**：泛型已经**单态化**（每个具体类型一份代码），优化在这里是机器无关的：内联、循环变换、常量折叠。
- **机器码**：LLVM 后端做指令选择、寄存器分配、栈帧布局。这一步之后 Rust 就管不着了。

为什么要分这么多层？因为**每层解决一类问题**：借用检查需要「现在有没有正在生效的借用」这种高层信息，只能放在 MIR；「哪个物理寄存器空着」只在最后一步才知道。分层的另一个好处是可以复用——LLVM 那一整套机器无关优化不必自己写一遍。

代价是：想知道「我这句话到底编译成了什么」，你必须主动往下游走一趟。`--emit` 就是这条路——它不改变编译结果，只把中间产物另存一份给你看。

## 机制与规则

### 37.1 四段流水线与它们的中间产物

下图把四段、每段的职责，以及把该段落到磁盘上的命令摊在一起：

<svg viewBox="0 0 760 226" width="100%" role="img" aria-label="rustc 的四段流水线 HIR、MIR、LLVM IR、机器码，以及每段对应的观察命令" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="18" font-size="14" fill="currentColor">rustc 的四段流水线：从左到右，语义越来越少、越来越像机器</text>
<rect x="4" y="34" width="172" height="80" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="90" y="56" font-size="14" fill="currentColor" text-anchor="middle">HIR</text>
<text x="90" y="78" font-size="12" fill="currentColor" text-anchor="middle">宏已展开、名字已解析</text>
<text x="90" y="100" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">-Zunpretty=hir</text>
<rect x="196" y="34" width="172" height="80" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="282" y="56" font-size="14" fill="currentColor" text-anchor="middle">MIR</text>
<text x="282" y="78" font-size="12" fill="currentColor" text-anchor="middle">基本块 + 跳转；借用检查</text>
<text x="282" y="100" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">--emit=mir</text>
<rect x="388" y="34" width="172" height="80" rx="6" fill="none" stroke="var(--links)" stroke-width="1.5" />
<text x="474" y="56" font-size="14" fill="currentColor" text-anchor="middle">LLVM IR</text>
<text x="474" y="78" font-size="12" fill="currentColor" text-anchor="middle">泛型已单态化：每型一份</text>
<text x="474" y="100" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">--emit=llvm-ir</text>
<rect x="580" y="34" width="172" height="80" rx="6" fill="none" stroke="currentColor" stroke-width="1.5" />
<text x="666" y="56" font-size="14" fill="currentColor" text-anchor="middle">机器码</text>
<text x="666" y="78" font-size="12" fill="currentColor" text-anchor="middle">指令选择、寄存器分配</text>
<text x="666" y="100" font-size="12" font-family="monospace" fill="var(--links)" text-anchor="middle">--emit=asm / obj</text>
<line x1="176" y1="74" x2="192" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="184" y1="68" x2="192" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="184" y1="80" x2="192" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="368" y1="74" x2="384" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="376" y1="68" x2="384" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="376" y1="80" x2="384" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="560" y1="74" x2="576" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="568" y1="68" x2="576" y2="74" stroke="currentColor" stroke-width="1.5" />
<line x1="568" y1="80" x2="576" y2="74" stroke="currentColor" stroke-width="1.5" />
<text x="0" y="142" font-size="12" fill="currentColor">-C opt-level 与 -C overflow-checks 从 MIR 起就在产物里看得见（37.4）；</text>
<text x="0" y="164" font-size="12" fill="currentColor">asm! 往函数体里插一段汇编，naked_asm! 构成整个函数体，global_asm! 插在模块级（37.5、37.6）；</text>
<text x="0" y="186" font-size="12" fill="currentColor">LTO 发生在 LLVM IR 这一层：把跨 crate 的优化推迟到链接期再做（37.7）。</text>
<text x="0" y="212" font-size="12" fill="currentColor">注意 MIR 那一段的方框描边：它是本章唯一能同时看到「借用检查」和「优化痕迹」的层。</text>
</svg>

`--emit=<类型>` 一次可以要多种，逗号隔开。下面几节都用同一份源码（`snippets/ch37/codegen.rs`）：

```rust
{{#rustdoc_include ../snippets/ch37/codegen.rs}}
```

```console
$ rustc --edition 2024 --emit=mir,llvm-ir,asm codegen.rs
$ ls codegen.mir codegen.ll codegen.s
codegen.ll  codegen.mir  codegen.s
```

各类型的默认文件名（`rustc --help` 给的表）：

| 类型 | 产物 | 说明 |
|---|---|---|
| `mir` | `CRATE_NAME.mir` | rustc 自己的中间表示，文本 |
| `llvm-ir` | `CRATE_NAME.ll` | 交给 / 出自 LLVM 的 IR，文本 |
| `asm` | `CRATE_NAME.s` | 目标平台汇编，文本 |
| `obj` | `CRATE_NAME.o` | 目标文件，二进制 |
| `llvm-bc` | `CRATE_NAME.bc` | LLVM 位码，二进制 |

> [!TIP]
> 别用 `-o` 给中间产物改名：它会把默认文件名整个顶掉——本机实测 `rustc --emit=mir -o out.exe codegen.rs` 把 MIR 文本写进了 `out.exe`。要指定文件名，用 `--emit=mir=codegen.txt` 这种形式。

汇编那一段还有个小口径要交代：本机（Windows MSVC 目标）的 `--emit=asm` 输出是 **AT&T 语法**（`addl %edx, %ecx`），而 37.5 的 `asm!` 模板默认按 **Intel 语法**写（`add {0}, {1}`）。两处不一样不是错：`asm!` 里是模板，`.s` 里是汇编器认的格式。

再往上还有一类开关，本章只提名字：`-Zunpretty=hir`、`-Zunpretty=mir`、`-Zunpretty=thir-tree`（不产出目标文件，只打印某个中间表示）。它们是 nightly 专属，stable 上直接拒绝：

```console
$ rustc --edition 2024 -Zunpretty=hir codegen.rs
error: the option `Z` is only accepted on the nightly compiler

help: consider switching to a nightly toolchain: `rustup default nightly`
```

> [!WARNING]
> `-Z` 系列（`-Zunpretty=hir`/`hir,typed`/`mir`/`thir-tree`、`-Zdump-mir=all`、`-Zmir-opt-level=N`）**只在 nightly 可用**，没有稳定替代品。本章正文一律用 stable 的 `--emit`；要看 HIR、要看 MIR 在每一个优化 pass 前后的样子，只能换 nightly 工具链自己试，输出随版本变化，本书不给快照。

### 37.2 MIR：看得见借用检查的那一层

MIR 的形态很规整：一个函数体 = 一串**基本块（basic block）**，每块以一条**终结指令（terminator）**收尾（`return`、`goto`、`switchInt`、`call`、`assert`…）；块里的每条语句都在「值槽（MIR 的局部变量，local）」之间搬数据，值槽写成 `_0`、`_1`……其中 `_0` 固定是返回值槽，参数从 `_1` 起。

来看本章示例里那个函数 `add` 的 MIR（第 4 行起）：

```text
fn add(_1: u32, _2: u32) -> u32 {
    debug a => _1;
    debug b => _2;
    let mut _0: u32;
    let mut _3: (u32, bool);

    bb0: {
        _3 = AddWithOverflow(copy _1, copy _2);
        assert(!move (_3.1: bool), "attempt to compute `{} + {}`, which would overflow", copy _1, copy _2) -> [success: bb1, unwind terminate(abi)];
    }

    bb1: {
        _0 = move (_3.0: u32);
        return;
    }
}
```

逐行读：

- `debug a => _1` 是变量名到值槽的映射；`_0` 是返回值槽，`_3` 是临时值。`AddWithOverflow` 返回「和」与「有没有溢出」的二元组。
- 第二条语句就是 `assert`：没溢出才走 `bb1`，否则进 `unwind terminate(abi)` 分支。**「整数溢出现在会 panic」这句语言规则，在 MIR 里就是这一条终结指令。** 37.4 会看到，关掉溢出检查之后这两行一起消失。
- `copy` / `move` 是 MIR 的搬运操作；`copy` 要求类型是 `Copy`，`move` 会把源槽置为「已移动」。
- 那条 `unwind terminate(abi)` 与 `add` 的 `extern "C"` 有关：panic 不能跨过 C ABI 展开，于是这个分支被编译成直接终止进程；换成 Rust ABI，这里就是 `unwind continue`（对照第 26 章）。

再看带循环的 `sum_slice`，它的 MIR 有 8 个基本块，循环被摊平成「回边」：

```text
    bb0: {
        _2 = const 0_u32;
        _3 = <&[u32] as IntoIterator>::into_iter(copy _1) -> [return: bb1, unwind continue];
    }

    bb1: {
        _4 = move _3;
        goto -> bb2;
    }

    bb2: {
        _6 = &mut _4;
        _5 = <std::slice::Iter<'_, u32> as Iterator>::next(copy _6) -> [return: bb3, unwind continue];
    }

    bb3: {
        _7 = discriminant(_5);
        switchInt(move _7) -> [0: bb6, 1: bb5, otherwise: bb4];
    }

    bb4: {
        unreachable;
    }

    bb5: {
        _8 = copy ((_5 as Some).0: &u32);
        _10 = copy _2;
        _11 = copy (*_8);
        _9 = add(move _10, move _11) -> [return: bb7, unwind unreachable];
    }

    bb6: {
        _0 = copy _2;
        return;
    }

    bb7: {
        _2 = move _9;
        goto -> bb2;
    }
```

读法：`bb3` 的 `switchInt` 是 `match`/`if let` 的形态——0 走 `bb6`（`None`，返回累加值），1 走 `bb5`（`Some`，取出元素做加法）。`bb5 → bb7 → bb2` 就是循环的回边。`bb4: unreachable;` 是穷尽性在 MIR 层的落地：判别式只可能是 0 或 1，其余情况编译器直接写死「到不了」。

**MIR 不是刚降级出来的那一份。** 加上 `-O` 再打一次，`sum_slice` 只剩 5 个基本块，而且多出一批名字里带 `inlined` 的 scope：

```text
    scope 1 {
        debug acc => _0;
        scope 2 {
            debug iter => _2;
            ...
            scope 17 (inlined <std::slice::Iter<'_, u32> as Iterator>::next) {
```

也就是说，**一部分优化是 rustc 自己在这一层做的**。这就是「零成本抽象」里迭代器能在优化档下彻底消失的原因：`next` 的调用点先在 MIR 层被内联，基本块从 8 个压到 5 个，LLVM 接手时已经看不到迭代器了。

### 37.3 单态化：一份泛型代码，五份机器码

**单态化（monomorphization）**指编译期为每个具体类型生成一份泛型代码的实例——它是「泛型不慢」的来源，也是「泛型会让二进制变大」的来源。它发生在**从 MIR 到 LLVM IR 之间**：MIR 里泛型函数还是一份（形参写着 `T`），到 LLVM IR 就是每个用到的类型各一份。

`snippets/ch37/mono.rs` 是一份最小样本：一个泛型函数 `wrap<T>`，用五种类型各调一次。

```rust
{{#rustdoc_include ../snippets/ch37/mono.rs}}
```

先看默认档下目标文件里的符号（`llvm-nm` 来自 rustup 的 `llvm-tools-preview`，不进 `PATH`，位置见第 29 章 29.5）：

```console
$ rustc --edition 2024 --emit=obj --crate-name mono mono.rs
$ llvm-nm.exe mono.o | findstr wrap
00000000 t _RINvCsfyZ5b0mWrQz_4mono4wrapbEB2_
00000000 t _RINvCsfyZ5b0mWrQz_4mono4wrapcEB2_
00000000 t _RINvCsfyZ5b0mWrQz_4mono4wrapdEB2_
00000000 t _RINvCsfyZ5b0mWrQz_4mono4wrapmEB2_
00000000 t _RINvCsfyZ5b0mWrQz_4mono4wrapyEB2_
```

五份。`wrap` 后面那个字母就是实例化用的类型（v0 符号修饰的类型编码，对照 rustc 手册的符号修饰一节）：`b` 是 `bool`、`c` 是 `char`、`d` 是 `f64`、`m` 是 `u32`、`y` 是 `u64`，正好是样本里那五个类型。**同一份泛型代码变成了五份各自独立的机器码。**

再看代价。同一份源码、同一个目标文件，`text` 段一个是 922 字节、一个是 374 字节：

```console
$ rustc --edition 2024 -O --emit=obj --crate-name monoO mono.rs
$ llvm-size.exe mono.o monoO.o
   text	   data	    bss	    dec	    hex	filename
    922	    436	      0	   1358	    54e	mono.o
    374	    200	      0	    574	    23e	monoO.o
```

两个数字说的是同一件事的两面：**默认档**下五份复制品都在，膨胀是真的；**`-O` 之后**用 `llvm-nm` 数 `wrap` 得到 0。原因不难猜——`wrap` 小到没道理不内联，内联进 `main` 之后那五份函数没人调用了，被顺带删掉。这正是「零成本抽象」在小型泛型上成立的机制，也是它**不保证**在所有泛型上成立的原因。

> [!WARNING]
> 「泛型一定被优化掉」是误解：会被消掉的是**能内联的小函数**。一个几百行、在十个类型上实例化、又被多个下游 crate 各自使用的泛型函数，就是十份真实的代码。观察手段就是本节这两把尺子——数符号（`llvm-nm`）、比大小（`llvm-size`）。复制品**可能**存在，就得在发布前量一次。

### 37.4 `-C` 选项：一条从命令行到机器码的线

第 29 章讲「档位怎么选、时间怎么量」；这一节只讲**一个 `-C` 选项会在产物的哪一层留痕**。看懂这条线，以后遇到「这个选项到底改了什么」就不用查博客。

先看全貌（默认值来自 rustc 手册的代码生成选项表与本机 `rustc -C help`）：

| 选项 | 默认 | 在哪里留下痕迹 |
|---|---|---|
| `-C opt-level` | `0` | 决定优化在 MIR 层与 LLVM 层各跑多深。`-O` 是 `-C opt-level=3` 的简写 |
| `-C overflow-checks` | 跟 `debug-assertions` | 决定 MIR 里有没有 `AddWithOverflow` + `assert` 那两条 |
| `-C debug-assertions` | 只在 `opt-level=0` 开 | `debug_assert!` / `cfg(debug_assertions)` 的代码留不留 |
| `-C debuginfo` | `0` | 目标文件里有没有调试段（本机是 `.debug$S`、`.debug$T`）。`-g` 是 `-C debuginfo=2` 的简写 |
| `-C codegen-units` | 非增量 16、增量 256 | crate 被切成几个 LLVM 模块 |
| `-C panic` | 随目标平台 | panic 后展开还是中止；它和 ABI 一起决定 MIR 里 unwind 分支写成什么（第 26、40 章） |
| `-C lto` | 未指定＝thin local LTO | 跨 crate 的优化范围（37.7，取舍见第 29 章） |

默认值的链条值得背下来一条：**`overflow-checks` 默认跟随 `debug-assertions`，而 `debug-assertions` 默认只在 `opt-level=0` 时开**。所以「release 档下整数溢出会回绕」不是单独的一条规则，而是这两个默认值叠出来的结果。

看实测。**第一组：`-C overflow-checks`。** 37.2 里 `add` 的 MIR 有 `AddWithOverflow` 和 `assert`；关掉之后：

```console
$ rustc --edition 2024 -C overflow-checks=off --emit=mir codegen.rs
```

```text
fn add(_1: u32, _2: u32) -> u32 {
    debug a => _1;
    debug b => _2;
    let mut _0: u32;

    bb0: {
        _0 = Add(copy _1, copy _2);
        return;
    }
}
```

模块里的 `bb1` 没了，`AddWithOverflow` 变成普通 `Add`，`assert` 整条消失。而且顺序可以反过来：`-C opt-level=2 -C overflow-checks=on` 会让检查**回到**优化档里（实测，MIR 与默认档的 `add` 一致）。这就是「选项 → 代码」的第一条完整链：**一个开关 → 两条 MIR 语句 → 一段汇编分支 → 一次可能的 panic**。

**第二组：`-C opt-level`。** 同一个 `add`，看汇编（这是最后一道菜，前面所有层的变化都汇聚到这里）：

```console
$ rustc --edition 2024 --emit=asm codegen.rs
$ rustc --edition 2024 -C opt-level=2 --emit=asm codegen.rs
```

默认档（`opt-level=0`）的完整函数体，末尾从 `.seh_handlerdata` 起的那四行（`ud2` 之后的段声明与 `.seh_endproc`）没抄：

```text
add:
.Lfunc_begin1:
.seh_proc add
	.seh_handler __CxxFrameHandler3, @unwind, @except
	pushq	%rbp
	.seh_pushreg %rbp
	subq	$48, %rsp
	.seh_stackalloc 48
	leaq	48(%rsp), %rbp
	.seh_setframe %rbp, 48
	.seh_endprologue
	movq	$-2, -8(%rbp)
	movl	%ecx, %eax
	movl	%eax, %ecx
	addl	%edx, %ecx
	movl	%ecx, -12(%rbp)
	cmpl	%eax, %ecx
	jb	.LBB13_2
	movl	-12(%rbp), %eax
	.seh_startepilogue
	addq	$48, %rsp
	popq	%rbp
	.seh_endepilogue
	retq
.LBB13_2:
.Ltmp2:
	leaq	anon.62d286c84577314f14f3ace68a483eee.4(%rip), %rcx
	callq	_RNvNtNtCs9OCfJ4KTcUD_4core9panicking11panic_const24panic_const_add_overflow
	nop
.Ltmp3:
	jmp	.LBB13_4
.LBB13_4:
	ud2
```

而 `-C opt-level=2` 的那一份，整个函数体就是三行：

```text
add:
	leal	(%rcx,%rdx), %eax
	retq
```

那堆 `.seh_*` 是 Windows 的异常处理元数据，默认档下这个函数带着一整套（好让 panic 能跨函数展开栈）；`leal` 与 `callq` 则是溢出检查在机器码里的落地。一个 `-C opt-level` 把三十几行变成 2 行，并且顺手把 `panic_const_add_overflow` 这个符号从代码里删掉了——因为检查本身没了。

往上游退一层，`codegen.ll` 里 `@add` 的开头是这样（默认档，只取到分支为止）：

```text
define dso_local i32 @add(i32 %a, i32 %b) unnamed_addr #3 personality ptr @__CxxFrameHandler3 {
start:
  %_3.0 = add i32 %a, %b
  %_3.1 = icmp ult i32 %_3.0, %a
  br i1 %_3.1, label %panic, label %bb1
```

（`panic` 那个基本块里是对 `panic_const_add_overflow` 的 `invoke`——写法与 MIR 不同，落地的是同一件事。）`-C opt-level=2` 那份则是完整的五行：

```text
define dso_local noundef i32 @add(i32 noundef %a, i32 noundef %b) unnamed_addr #3 {
start:
  %_0 = add i32 %b, %a
  ret i32 %_0
}
```

（`anon.62d286c8…` 里那串哈希由 `-C metadata` 派生：换个 crate 名、目录或编译选项它就会变。符号名里的 `C…` 同理——它们是本机这次构建的产物，跟着你复现一遍会得到不同的值。）

**第三组：`-C debuginfo`。** 它不进代码，只进目标文件：

```console
$ rustc --edition 2024 -C debuginfo=0 --emit=obj --crate-name dbg0 codegen.rs
$ rustc --edition 2024 -C debuginfo=2 --emit=obj --crate-name dbg2 codegen.rs
$ wc -c dbg0.o dbg2.o
 8860 dbg0.o
34887 dbg2.o
43747 total
```

同一份源码，`-C debuginfo=2` 把目标文件从约 8.8 KB 抬到约 34.9 KB（约 4 倍）。这两个数会随构建目录的绝对路径长度浮动几十字节——调试段里嵌了路径，别把它们当常量背。多出来的是 `.debug$S`（CodeView 的符号与行号）和 `.debug$T`（类型）两个段，用 `llvm-readobj --section-headers` 能直接看到；`debuginfo=0` 的产物里一个调试段都没有。`1`（limited）和 `line-tables-only` 落在中间，本机实测都在 16 KB 出头——只留文件名与行号，不留变量和类型。

**第四组：`-C codegen-units`。** 它管的是「crate 切成几块交给 LLVM」。`--emit=llvm-ir` 恰好把这个切法暴露出来——每个代码生成单元（codegen unit，CGU）一个 `.ll` 文件：

```console
$ rustc --edition 2024 -C codegen-units=16 --emit=llvm-ir --crate-name cgus cgu.rs
$ ls cgus.*.ll
cgus.cgus.7c1bd7561f78ee66-cgu.00.rcgu.ll
cgus.cgus.7c1bd7561f78ee66-cgu.01.rcgu.ll
...（本机这次一共 11 个，cgu.00 到 cgu.10）
$ rustc --edition 2024 -C codegen-units=1 --emit=llvm-ir --crate-name cgus1 cgu.rs
$ ls cgus1*.ll
cgus1.ll
```

来看这个切法长什么样。示例用的是 `snippets/ch37/cgu.rs`——十几个小函数，足够切出多块：

```rust
{{#rustdoc_include ../snippets/ch37/cgu.rs}}
```

切多块的意义是**并行**：LLVM 可以同时处理这些模块，但每块看到的上下文变少、跨块优化的机会也跟着少。手册的建议是「想压编译时间就调大，想要更好的代码就调小、甚至设成 1」——典型的「编译时间换运行速度」（方法学见第 29 章）。注意 CGU 数**不保证等于**你设的值：本机设 16 时切出 11 个，设 1 就是 1 个。

`-C` 的值写错时，报错很直接（原样照抄）：

```text
error: incorrect value `maybe` for codegen option `overflow-checks` - one of: `y`, `yes`, `on`, `true`, `n`, `no`, `off` or `false` was expected
```

```text
error: optimization level needs to be between 0-3, s or z (instead was `9`)
```

```text
error: value for codegen units must be a positive non-zero integer
```

三个报错都发生在编译开始之前，和源码无关——照报错里给出的取值改就行。

> [!NOTE]
> `-C target-cpu` / `-C target-feature` 也属于这一族，但它们改的是机器码那一段（指令选择与向量化），默认值取决于目标平台。要查本机支持什么：`rustc --print target-cpus`、`rustc --print target-features`。

### 37.5 inline asm：第一条 `asm!`

有些事语言表达不了：读时间戳计数器、执行 `cpuid`、写中断向量。Rust 的做法是内联汇编，三条宏分工明确（Reference 的 inline assembly 一节）：

- `asm!`：写在**函数体内**，插进编译器生成的那段汇编里；
- `naked_asm!`：只能写在 naked function 里，构成**整个函数体**（37.6）；
- `global_asm!`：写在**模块级**，整段汇编直接进 `.s`，可以手写完整函数。

稳定的架构是 x86 / x86-64、ARM、AArch64（含 Arm64EC）、RISC-V、LoongArch、s390x、PowerPC 系；在不支持的目标上用这三条宏，编译器直接报错。

最小例（`snippets/ch37/asm_add.rs`）：

```rust
{{#rustdoc_include ../snippets/ch37/asm_add.rs}}
```

```console
$ rustc --edition 2024 -O asm_add.rs -o asm_add.exe
$ .\asm_add.exe
5
0
```

第二行是故意留的：`u64::MAX` 加 1 本该溢出，机器指令直接回绕成 0——`asm!` 里的 `add` **不受 `-C overflow-checks` 管**。这一行就说明了它的定位：一扇通向语言规则之外的门。

读法拆开看：

- 第一个参数是**模板字符串**，可以有多个（按顺序拼接），`{0}`、`{1}` 是操作数占位符（也可以写成 `{out}` 这类命名形式）。x86 上模板默认按 **Intel 语法**，目的在前、源在后：`add {0}, {1}` 就是 `{0} += {1}`。
- 操作数方向有 `in`、`out`、`inout`、`lateout`、`inlateout`，外加 `const`（编译期常量）、`sym`（函数符号）、`label`（跳转标签）。`inout(reg) a => out` 一行同时声明「输入 `a`、结果写回 `out`」，物理寄存器由编译器挑。
- `options(...)` 是你给编译器的**承诺**：`nomem` 表示不碰内存，`nostack` 表示不动栈，`pure` 表示没有副作用。不写 `options` 时，编译器默认这段汇编可能读写任何内存、改任何寄存器、跳到任何地方，于是会格外保守；`options` 就是把这些假设收窄，换回优化空间。承诺错了就是 UB。

**它为什么是 `unsafe`。** 因为模板字符串里的内容编译器**完全不检查**：借用检查、类型检查、边界检查都建立在「编译器看得懂」的前提上，而 `asm!` 的内容只是一个字符串，编译器只认识里面那几个占位符。第 23 章的判据在这里同样成立——**你写下的汇编违反了操作数声明或调用约定，编译器没有任何义务兜底。**

最后看一眼它落在产物里的位置。`asm!` 注入的片段会被 `#APP` / `#NO_APP` 两个标记夹住（默认档编译 `asm_add.rs`，节选 `add_via_asm` 的函数体）：

```text
	movq	%rcx, %rax
	#APP

	addq	%rdx, %rax

	#NO_APP
	movq	%rax, (%rsp)
```

`#APP` 与 `#NO_APP` 之间的那行就是你的模板——这是「我写的东西确实进了机器码」的直接证据。这里显示的是 AT&T 语法的 `addq %rdx, %rax`，而模板写的是 Intel 语法的 `add {0}, {1}`：同一条指令，两种写法。

### 37.6 naked functions：整个函数体由你写

`asm!` 是「往编译器生成的函数里插一段」，naked function 相反：**整个函数体由你写，编译器一条指令都不加**——没有序言（prologue）、没有尾声（epilogue）、没有栈帧调整。

它**不是 nightly 特性**。`naked_functions` 从 **1.88** 起稳定，1.88 的 release notes 原文是：

```text
- [Stabilize `#![feature(naked_functions)]`.](https://github.com/rust-lang/rust/pull/134213)
  Naked functions allow writing functions with no compiler-generated epilogue and prologue, allowing full control over the generated assembly for a particular function.
```

（大意：naked 函数让你写出没有编译器生成的序言与尾声的函数，从而完全掌控某个函数生成的汇编。）本机在 stable 1.98 上实测可用。**唯一的例外**：把 `#[naked]` 用在 **Rust ABI** 的函数上仍然是不稳定的，报 E0658 并指向 tracking issue #138997——所以下面的例子用的是 `extern "C"`（这也合理：naked 函数连调用约定都得自己兑现）。

```rust
{{#rustdoc_include ../snippets/ch37/naked_add.rs}}
```

```console
$ rustc --edition 2024 naked_add.rs -o naked_add.exe
$ .\naked_add.exe
5
```

这个函数的整段汇编就是下面两行（`--emit=asm` 实测，`#[unsafe(no_mangle)]` 让符号名保持 `add`）：

```text
add:
	leal	(%rcx,%rdx), %eax
	retq
```

编译器没加任何东西——连 `retq` 都是源码里那句 `"ret"`。规则按 Reference 的 `naked` 属性一节，要点如下：

- 属性写法是 `#[unsafe(naked)]`（MetaWord 形式，2024 edition 要求 `unsafe(...)` 包裹）；函数**必须是外部 ABI**（不能是 Rust ABI）。
- 函数体必须**恰好是一个 `naked_asm!` 调用**，不能有多余语句——这是报错现场第二条 E0787 的来源。
- 你必须在汇编里自己返回，不能「掉出」函数体末尾；跳转与返回都要符合调用约定与签名。
- 不能和 `inline`、`track_caller`、`target_feature` 一起用；`unused_variables` 之类的 lint 在 naked 函数里被抑制。
- 编译器**不得**复制这段汇编——除非函数是多态的、正在被单态化（这条保证对「在 naked 函数里定义符号」的写法很重要）。

需要它的场合不多：启动代码、中断向量、引导（bootloader）阶段那种必须精确控制入口的场景。日常写 Rust，`asm!` 已经够；`global_asm!` 是第三条路——整段汇编放模块级，适合手写完整函数或放 `.section` 指令。

### 37.7 LTO 发生在哪一层

最后一句话收尾：**LTO（link time optimization，链接期优化）在 LLVM IR 那一层**。前面四段流水线是「每个 crate 各走一遍」；LTO 把它推迟——依赖编译成带 LLVM 位码的 rlib，到最终链接时再把参与的模块合成一个大模块优化。它改的不是 MIR，也不是借用检查（那些早就做完了），而是 LLVM 能看到多少上下文：跨 crate 内联之所以可行，正是因为到了这一层所有模块都是 LLVM IR。

工程侧的结论第 29 章已经给过（profile 里的 `lto` 键各值是什么意思、`lto = false` 不等于不做 LTO、`-C lto` 值写错时的报错），这里只补一句原理上的分工：`-C linker-plugin-lto` 是这项能力的延伸——把 LTO 推到「实际的链接步」，于是可以和其他 LLVM 系工具链（Clang 编译的 C/C++、LLVM Flang 编译的 Fortran）跨语言做过程间优化，代价是参与链接的每个对象都得用同一种 LTO 模式（默认 thin）。用不上跨语言就不用管它。

## 动手看看

四个示例都在本机跑过，源码在 `snippets/ch37/`。命令都假设你在那些文件所在的目录里（`--edition 2024` 是本书统一口径：不带它时默认 2015 edition，`#[unsafe(naked)]` 这类新语法会直接报错）。

**一、把四段产物一次看全。** 用 `codegen.rs`（一个孤立函数 `add`，一个带循环的 `sum_slice`）：

```console
$ rustc --edition 2024 --emit=mir,llvm-ir,asm codegen.rs && ls codegen.mir codegen.ll codegen.s
codegen.ll  codegen.mir  codegen.s
```

然后按 37.2、37.4 的顺序对着读：`.mir` 里找 `AddWithOverflow` 与基本块编号，`.ll` 里找 `@add` 的 `icmp ult` 与 `br`，`.s` 里找 `add:` 那一段。同一个加法，三层三种长相——这就是本章的主线。

**二、改一个选项，看它流到哪一层。** 最省事的对照是 `-O`：

```console
$ rustc --edition 2024 --emit=mir=before.mir codegen.rs
$ rustc --edition 2024 -O --emit=mir=after.mir codegen.rs
$ findstr /c:"inlined" after.mir
```

`after.mir` 里会冒出一堆 `scope N (inlined ...)`，`sum_slice` 的基本块从 8 个掉到 5 个。再换 `-C overflow-checks=off`，`assert` 那一行消失。**这是本章最有用的一套练法：改一个开关，在 MIR、LLVM IR、汇编里各找一遍它的影子。**

**三、数单态化。** 用 `mono.rs`：先 `--emit=obj` 数 `wrap` 的符号（5 个），再加 `-O` 数一遍（0 个），最后用 `llvm-size` 比两个目标文件的 `text` 段。命令与输出见 37.3。

**四、写一条自己的汇编。** 从 `asm_add.rs` 出发：把模板改成 `"sub {0}, {1}"` 跑一遍；再把 `options(nomem, nostack, pure)` 整行删掉编译一次，比较汇编产物。第二件事才是重点——`options` 的每一条承诺，都是你换取优化空间时签的字。

> [!TIP]
> `llvm-nm`、`llvm-size`、`llvm-objdump`、`llvm-readobj` 都来自 `llvm-tools-preview`，与 rustc 同版本但不进 `PATH`（路径见第 29 章 29.5）。**别拿系统里另装的 GNU 工具凑合**：版本对不上，读出来的符号表会更乱。

## 报错现场

**现场一：`asm!` 忘了 `unsafe`。** 内联汇编的语义完全由你负责，忘了 `unsafe` 会直接编译失败（最小复现写在 `lib.rs` 里，`asm!` 在第 2 行）：

```rust,compile_fail
pub fn nop() {
    std::arch::asm!("nop");
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0133]: use of inline assembly is unsafe and requires unsafe block
 --> lib.rs:2:5
  |
2 |     std::arch::asm!("nop");
  |     ^^^^^^^^^^^^^^^^^^^^^^ use of inline assembly
  |
  = note: inline assembly is entirely unchecked and can cause undefined behavior
```

`E0133` 是「在安全代码里做了不安全的事」的统一错误码（第 23 章打过照面），这里的 note 说明了理由：inline assembly is entirely unchecked and can cause undefined behavior（大意：内联汇编完全不受检查，可能引起未定义行为）。修法是包一层 `unsafe { ... }`，并写一句 `// SAFETY:` 说清你的操作数与 `options` 承诺为什么成立。

**现场二：naked 函数里多写了语句。** 规则要求函数体恰好是一个 `naked_asm!`，多一行 `let` 就是这个结果：

```rust,compile_fail
#[unsafe(naked)]
pub extern "C" fn f() {
    let x = 1;
    core::arch::naked_asm!("ret");
    let _ = x;
}
```

```text
error[E0787]: naked functions must contain a single `naked_asm!` invocation
 --> lib2.rs:2:1
  |
2 | pub extern "C" fn f() {
  | ^^^^^^^^^^^^^^^^^^^^^
3 |     let x = 1;
  |     ---------- not allowed in naked functions
4 |     core::arch::naked_asm!("ret");
5 |     let _ = x;
  |     ---------- not allowed in naked functions
```

报错把两处违规都点出来了（第 3、5 行），箭头指回函数本身；另有一条 `unreachable statement` 警告——`naked_asm!("ret")` 之后的代码都到不了。修法只有一个：把函数体清成一个 `naked_asm!`，要初始化就在汇编里做。

**现场三：`-C` 的值写错。** 这类错误在编译开始前就结束，和源码无关（三条原文见 37.4）。它们值得单独记一嘴，因为报错信息本身就是一份取值清单——遇到不认识的 `-C` 选项，先看它接受什么值，比查文档快。

## 小结

- 流水线四段：**HIR**（宏与语法糖已拆、名字已解析）→ **MIR**（基本块 + 跳转，借用检查与 MIR 优化在这里）→ **LLVM IR**（泛型已单态化，机器无关优化）→ **机器码**（指令选择、寄存器分配）。`--emit=mir|llvm-ir|asm|obj` 把每一段落到磁盘；`-Zunpretty`/`-Zdump-mir` 是 nightly 专属。
- MIR 的读法：`_0` 是返回值槽，`bbN` 是基本块，块尾是终结指令。**「溢出 panic」在 MIR 里就是一条 `assert`**，关掉溢出检查会让它连同 `AddWithOverflow` 一起消失；带 `-O` 打出来的 MIR 里能看到 `inlined` 标记——一部分优化是 rustc 自己做的，不只 LLVM。
- 单态化发生在 MIR 与 LLVM IR 之间：MIR 里泛型是一份，代码生成时每个用到的类型一份。`llvm-nm` 数符号能直接看到复制品（本机样本 5 份），`llvm-size` 能给出体积差；`-O` 下小函数会被内联掉、复制品归零，但**这只对小函数成立**。
- `-C` 选项按「在哪一层留痕」来记：`overflow-checks` 改 MIR；`opt-level` 从 MIR 一路改到汇编（本机 `add` 的汇编从三十几行降到 2 行）；`debuginfo` 只改目标文件的调试段（0 到 2 让体积涨到约 4 倍）；`codegen-units` 改 crate 被切成几个 LLVM 模块。默认值链条：`overflow-checks` 跟 `debug-assertions`，后者只在 `opt-level=0` 开。
- `asm!` 在函数体内插汇编，`naked_asm!` 构成整个函数体，`global_asm!` 写在模块级；它们之所以是 `unsafe`，是因为字符串里的内容编译器一个字都不检查，`options(...)` 是你签下的承诺。
- naked function 从 **1.88** 起稳定（Rust ABI 除外），函数体必须恰好一个 `naked_asm!`，返回要自己写。LTO 在 LLVM IR 那一层，把跨 crate 的优化推迟到链接期，`-C linker-plugin-lto` 还能把它推到跨语言（工程侧结论见第 29 章）。

## 练习

**一、解释。** 有人看到默认档下 `wrap` 有 5 个符号、`-O` 下是 0 个，得出结论：「单态化只是编译器的一种中间表示，最终代码里没有复制品。」这个结论哪里有问题？请说明在什么条件下它会明显不成立。

**二、找错。** 下面这段代码想「让函数体完全由自己控制」，但编译不过。找出至少两处违反规则的地方，并说明规则本身（提示：留意 ABI、函数体内容、以及返回值是怎么来的）。

```rust,compile_fail
#[naked]
fn double(x: u64) -> u64 {
    let y = x + x;
    core::arch::naked_asm!("lea rax, [rcx + rcx]", "ret");
    y
}
```

**三、预测。** 把 37.4 的 `codegen.rs` 分别用下面两组参数编译，然后回答：哪一种情况下 `add:` 那个符号的汇编里会多出 `callq ...panic_const_add_overflow`？为什么另一组不会有？

(a) `rustc --edition 2024 -C opt-level=2 --emit=asm codegen.rs`
(b) `rustc --edition 2024 -C opt-level=2 -C overflow-checks=on --emit=asm codegen.rs`

<details><summary>参考答案</summary>

**一、** 结论把「这一份样本的观察」当成了「语言的保证」。默认档下那 5 个符号是真实存在的机器码；`-O` 下归零，是因为 `wrap` 够小、被内联进调用者后没人再调用它们——这是**优化**的结果，不是单态化不存在。函数一大就不成立：几百行、带循环与多次分配、或者作为跨 crate 的 API 被多个下游各自实例化，都会留下真实的复制品。判断方法还是那两把尺子，而且必须在你要发布的那套选项下量。

**二、** 至少三处：

1. **ABI 不对。** `#[naked]` 不能用在 Rust ABI 的函数上（本机实测报 `E0658`，原文是 `` `#[naked]` is currently unstable on `extern "Rust"` functions ``）。要写成 `pub extern "C" fn double(...)`，并且属性要带 `unsafe`：`#[unsafe(naked)]`。
2. **函数体不是「恰好一个 `naked_asm!`」。** 上面既有 `let y = ...`，又有结尾的 `y`（尾表达式）。规则要求函数体只有一个 `naked_asm!` 调用，多一句就报 `E0787`。
3. **参数与返回值得自己按调用约定兑现。** naked 函数没有编译器生成的代码，「参数从哪来、结果放哪去」全由你写的汇编决定。本机 win64 下第一个参数在 `RCX`、返回值放 `RAX`，所以 `lea rax, [rcx + rcx]` 是对的（本机实测 `double(21)` 得 `42`）。不少教程里的例子抄的是 SysV 的 `mov rax, rdi`——在 Windows 上那不是参数所在的位置，本机实测 `double(21)` 拿到的是别的寄存器留下的值（这次是 `281404659110560`），与 `42` 无关。拷汇编片段时务必连同调用约定一起核。改完是：

```rust
#[unsafe(naked)]
pub extern "C" fn double(x: u64) -> u64 {
    core::arch::naked_asm!("lea rax, [rcx + rcx]", "ret");
}
```

**三、** (b) 会多出那个 `callq`。(a) 里 `opt-level=2` 让 `overflow-checks` 默认变成关（它跟随 `debug-assertions`，后者在非 0 优化档下默认关），于是 `add` 只剩一条 `leal` 加 `retq`。(b) 显式打开 `overflow-checks=on`，MIR 里就是 `AddWithOverflow` 加 `assert`，汇编里重新出现比较、条件跳转与那条 `panic_const_add_overflow` 调用——**优化与溢出检查是两个独立的开关**，别把 `-O` 当成「顺手关掉所有检查」。

</details>

## 延伸阅读

- 代码生成选项全表（含本节各选项的取值与默认值）：[Codegen Options](https://doc.rust-lang.org/rustc/codegen-options/index.html)
- 内联汇编的完整规范（模板、操作数、寄存器类、`options`）：[Inline assembly](https://doc.rust-lang.org/reference/inline-assembly.html)
- `naked` 属性的规则：[Codegen-related attributes](https://doc.rust-lang.org/reference/attributes/codegen.html)
- v0 符号修饰（本节用到的类型编码表）：[Symbol Mangling](https://doc.rust-lang.org/rustc/symbol-mangling/v0.html)
- 跨语言 LTO：[Linker-plugin LTO](https://doc.rust-lang.org/rustc/linker-plugin-lto.html)
- nightly 专属的中间表示开关：[The Unstable Book](https://doc.rust-lang.org/unstable-book/index.html)
- 优化与 UB 的关系（第 23 章的延伸）：[Behavior considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)
