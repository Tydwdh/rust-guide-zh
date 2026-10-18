# 第 26 章 FFI 与 ABI

## 这一章解决什么问题

Rust 程序很少孤立运行。它要调用 C 写的库——从操作系统 API 到某个只给你 `.h` 和 `.dll`/`.so` 的厂商 SDK；反过来，你写的 Rust 库也常被 Python、C++ 或 C 调用。跨语言这条通道叫**外部函数接口（FFI，Foreign Function Interface）**，而它遵守的二进制层面的约定叫**应用二进制接口（ABI，application binary interface）**：参数放哪个寄存器或栈位置、返回值怎么传、谁负责清理栈、符号叫什么名字、结构体的字节怎么排。

本章只做一件事：把「Rust 与 C 互调」这条路上每一步的契约写清楚——声明、导出、字符串、回调、panic、变参。读完你应当能：

- 在 2024 版里写出正确的 `unsafe extern "C"` 声明，并知道 `unsafe` 为什么是必需的；
- 用 `#[unsafe(no_mangle)]` 把 Rust 函数暴露成 C 符号；
- 在两侧传字符串，用 `CString`/`CStr`/`c"..."` 而不是拿裸指针乱转；
- 把 Rust 函数作为回调交给 C，并且知道 panic 会在哪一层被拦下；
- 分得清哪些 FFI 能力已经稳定、哪些还在 nightly。

**边界先划清**：什么算未定义行为（UB，undefined behavior）、UB 清单在第 23 章；字段偏移、对齐、`repr(C)` 的具体规则在第 25 章；`no_std` 与嵌入式场景的组合在第 41 章。本章只谈跨语言通话本身。

## 为什么这样设计

先看一个不太直觉的事实：**Rust 自己的 ABI 没有稳定性保证**。Reference 在讲 ABI 字符串时写得直白：`unsafe extern "Rust"` 是 Rust 函数与闭包的默认约定，而 "The Rust ABI offers no stability guarantees."（译自 Rust Reference）。这句话意味着两件事。其一，Rust 编译器可以在版本之间改变参数传递方式、改变结构体布局、改变符号名，只要「Rust 内部自洽」就行。其二，跨语言时你**必须**显式指定一个双方都认的 ABI，那个 ABI 通常是 C 的。

那 C 的 ABI 为什么能当公约数？因为它是平台上的既成事实：每个目标平台都有主流 C 编译器选定的调用约定（calling convention），链接器、动态库格式、调试信息都围着它转。Reference 对 `extern "C"` 的定义正是这个意思：`unsafe extern "C"` — The "C" ABI matches the default ABI chosen by the dominant C compiler for the target.（译自 Rust Reference）

第二个设计决定是：**`extern` 块必须写 `unsafe`**。这不是为了吓人，而是因为块里每一条声明都是你对编译器的一句保证，而编译器**没法核对**。用 Reference 的话说，外部块提供的是「当前 crate 之外定义的项」的声明，它们是 "akin to unchecked imports"（译自 Rust Reference：类似未经检查的导入）。签名写错了会怎样？crate 的 edition 指南把后果写得很清楚：Adding the unsafe keyword helps to emphasize that it is the responsibility of the author of the extern block to ensure that the signatures are correct. If the signatures are not correct, then it may result in undefined behavior.（引自 The Rust Edition Guide）从 Rust 1.82 起所有 edition 都可以写 `unsafe extern`，从 2024 版起它变成**必需**。

第三个决定类似：`no_mangle`/`export_name`/`link_section` 这类属性也要加 `unsafe`，因为它们动的是**全局符号命名空间**。平时的 Rust 函数名会被编译器修饰（符号修饰，symbol mangling）成唯一的符号，撞名几乎不可能；一旦你关掉符号修饰或者自己指定名字，撞名就从「不可能」变成「你负责别撞」。edition 指南的措辞是：Marking the attribute with unsafe highlights that there are safety requirements that must be upheld that the compiler cannot verify on its own.（引自 The Rust Edition Guide）

把这三条合起来，本章的心智模型就很短：**FFI 是一条缝，缝的两侧各有一套规则；Rust 侧的每一处 `unsafe`，都是「我知道缝那边是什么」的一句承诺。** 布局那一半的承诺在第 25 章（`repr(C)`），调用与命名这一半在本章。

再纠一个常见误解：跨语言**不是**「Rust 单方面去适配 C」。你既可能是调用方（声明别人的函数），也可能是被调用方（导出自己的函数），两侧规则完全一样——**每一侧都只能看见对方 ABI 允许看见的那部分**。泛型、生命周期、trait 对象、`String`/`Vec`、`Result`、`!`（无值类型）在边界上统统消失，只剩整数、浮点、裸指针、函数指针和 `#[repr(C)]` 复合类型。这不是 Rust 的缺陷，任何两套类型系统对缝都一样：能过去的东西，必须是双方都精确知道每个比特含义的东西。

> [!NOTE]
> `unsafe` 属性与 `unsafe extern` 是**语法**层面的迁移，不改变运行时行为。它们的作用是让「这里需要人负责」在代码里显形，也让 review 有地方盯。

## 机制与规则

### 26.1 声明外部函数：`unsafe extern "C"` 块

最小的声明长这样——块里只有签名，没有函数体：

```rust
use std::ffi::c_char;

unsafe extern "C" {
    fn add(a: i32, b: i32) -> i32;
    unsafe fn strlen(p: *const c_char) -> usize;
}
```

块里的项**默认就是 `unsafe`**：调用它们必须写在 `unsafe` 块里。Reference 的原话是 "A function declared in an extern block is implicitly unsafe unless the safe function qualifier is present."（译自 Rust Reference）从 1.82 起你可以逐个标注 `safe`，把「我核对过这条签名，任何输入都安全」这件事说出来：

```rust
unsafe extern "C" {
    // 任何 f64 都是合法输入
    safe fn sqrt(x: f64) -> f64;
    // 指针必须有效，不能标 safe
    unsafe fn strlen(p: *const std::ffi::c_char) -> usize;
    // 不写限定符 = unsafe
    fn free(p: *mut std::ffi::c_void);
}
```

`safe` 只在你能给出保证时使用。给它标错，等于把 UB 送进安全代码。

ABI 字符串可以省略，但**别省**：Reference 专门提醒 "The extern syntax without an explicit ABI is being phased out, so it's better to always write the ABI explicitly."（译自 Rust Reference）常用的几个：

| ABI 字符串 | 用途 |
|---|---|
| `"C"` | 目标平台主流 C 编译器的默认约定；跨语言默认选它 |
| `"system"` | 在 Windows x86_32 上，非变参函数等同 `stdcall`、变参函数仍等同 `"C"`；其余平台等同于 `"C"`。调 Windows API 用它 |
| `"C-unwind"` / `"system-unwind"` | 与上两者相同，但允许**展开（unwind）**穿过边界（见 26.6） |
| `"Rust"` | Rust 自己的约定，无稳定性保证，只在 Rust 内部用 |
| `"win64"` / `"sysv64"` / `"cdecl"` / `"stdcall"` / `"fastcall"` / `"thiscall"` / `"aapcs"` / `"efiapi"` | 平台专用约定，多半只在特定架构上可用 |

要链接哪个库，用 `#[link]` 说清楚。名字与种类由它告诉链接器：

```rust,ignore
#[link(name = "c26")]
unsafe extern "C" {
    fn c_add(a: i32, b: i32) -> i32;
}
```

`kind` 可以是 `"dylib"`（默认）、`"static"`，Windows 上还有 `"raw-dylib"`（由编译器生成导入信息，不需要导入库文件），macOS 上有 `"framework"`。

### 26.2 导出给 C：`#[unsafe(no_mangle)]`

反方向：让 C 能按名字找到你的函数。需要两件事同时成立——**ABI 是 C 的**（`extern "C"`），**符号名不被修饰**（`#[unsafe(no_mangle)]`）：

```rust
#[unsafe(no_mangle)]
pub extern "C" fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    // C 看不到 Rust 的模块路径，所以在本例里我们直接当普通函数调用
    assert_eq!(rust_add(20, 22), 42);
}
```

Reference 对 `no_mangle` 的说明是 "The symbol for the item will be the identifier of the item's name."（译自 Rust Reference）——函数叫 `rust_add`，符号就叫 `rust_add`。想指定别的名字用 `#[unsafe(export_name = "...")]`。为什么它 unsafe？Reference：This attribute is unsafe as an unmangled symbol may collide with another symbol with the same name (or with a well-known symbol), leading to undefined behavior.（引自 Rust Reference）

把你编成动态库交给 C，用 `--crate-type=cdylib`：

```console
$ rustc --edition 2024 --crate-type=cdylib src/lib.rs -o mylib.dll
```

> [!WARNING]
> 别把「导出符号可用」误当成「ABI 稳定」。Rust 的标准库类型（`String`、`Vec`、`Result`…）**没有**跨语言的稳定布局，`cdylib` 的边界上只能出现 C ABI 类型：整数、浮点、裸指针、函数指针、`#[repr(C)]` 结构体。要传复杂数据，就把它序列化，或者设计成「创建/使用/销毁」三步的不透明句柄。

### 26.3 C 字符串：`CString`、`CStr`、`c"..."`

C 的字符串没有长度字段，靠末尾的 `\0` 定界，中间不允许有 `\0`；Rust 的 `String`/`str` 恰恰相反：长度是存下来的，中间可以有 `\0`。所以两边传字符串必须显式转换。标准库给了三种表示：

| 类型 | 谁拥有 | 怎么来 | 用途 |
|---|---|---|---|
| `CString` | Rust 拥有 | `CString::new("...")` | 把 Rust 字符串交给 C，活到你自己决定的那一刻 |
| `&CStr` | 借用别人的内存 | `CStr::from_ptr`、`from_bytes_until_nul`、`c"..."` | 读 C 给的字符串 |
| `c"..."` | 静态存储 | 字面量（Rust 1.77 起稳定） | 常量参数，零分配 |

std 的定位很具体：CString and CStr are useful when you need to transfer UTF-8 strings to and from languages with a C ABI, like Python.（引自 `std::ffi` 模块文档）两者与 Rust 字符串的关系也有现成的类比：&CStr is to CString as &str is to String: the former in each pair are borrowing references; the latter are owned strings.（引自 `std::ffi::CStr` 文档）

```rust
use std::ffi::{c_char, CStr, CString};

// 借用型：从 C 的指针造一个 &CStr，只有 UTF-8 校验通过才拿到 &str
unsafe fn read_from_c(p: *const c_char) -> String {
    // SAFETY: 调用方保证 p 指向以 nul 结尾的字符串
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

fn main() {
    // 拥有型：内部有 nul 会被拒收
    let owned = CString::new("hi").unwrap();
    assert!(CString::new("a\0b").is_err());

    // 字面量：直接就是 &'static CStr
    let literal: &CStr = c"hi";
    assert_eq!(owned.as_c_str(), literal);

    // 逆向：C 写进缓冲区，我只知道「里面有 nul」
    let mut buf = [0u8; 8];
    buf[..2].copy_from_slice(b"ok");
    let parsed = CStr::from_bytes_until_nul(&buf).unwrap();
    assert_eq!(parsed.to_str().unwrap(), "ok");
}
```

几个容易踩的点：

- `CStr::from_ptr` 是 `unsafe`，它的安全条件包括「指针非空、整段内存在一个分配里、nul 在 `isize::MAX` 之内、期间不被改写」（见 `std::ffi::CStr` 文档的 Safety 一节）。
- `to_str` 会做 UTF-8 校验并返回 `Result`；`to_string_lossy` 用替换字符兜底。C 侧字符串**不保证**是 UTF-8，别无条件 `to_str().unwrap()`。
- `c_char` 不等于 Rust 的 `char`：前者是 `i8`/`u8` 这类 8 位整数（随平台而定），后者是 Unicode 标量值。
- `&CStr` 不要出现在 FFI 签名里。`CStr` 文档明确说它 "does not have a guaranteed layout (the repr(transparent) notwithstanding) and should not be placed in the signatures of FFI functions."（引自 `std::ffi::CStr` 文档）交给 C 的永远是 `as_ptr()` 出来的指针。

### 26.4 布局契约：`#[repr(C)]`（回指第 25 章）

调用约定只管「怎么传」，不管「字节怎么排」。结构体要跨语言，必须 `#[repr(C)]`：字段按声明顺序排列、按 C 的规则对齐与填充。第 25 章讲了完整规则与反例，这里只强调一句：**没有 `#[repr(C)]` 的结构体不可以出现在 FFI 签名里**，它的布局、字段顺序、填充都可能随编译器变化。枚举同理（`#[repr(C)]` 或带显式判别值类型的 `#[repr(u8)]` 等）。

```rust
#[repr(C)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

unsafe extern "C" {
    fn dist2(p: Point) -> i32;
}
```

值传递与指针传递都合法，但大结构体按值传会复制——C 侧的写法这里是「按值」，Rust 侧就也得写「按值」，不能自作主张改成一个字节数不同的类型。

### 26.5 回调：把 Rust 函数交给 C

函数指针类型写成 `extern "C" fn(A) -> R`，它就可以作为参数交给 C：

```rust
extern "C" fn double(x: i32) -> i32 {
    x * 2
}

// 这句「接收函数指针并按 C 的约定调用」的代码，和 C 侧是同一种东西
extern "C" fn apply(f: extern "C" fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

fn main() {
    assert_eq!(apply(double, 21), 42);
}
```

注意区分两件事：**定义**一个 `extern "C" fn` 只表示「调用时用 C 的约定」，它是普通的安全函数，调用它不需要 `unsafe`；而调用 `extern` 块里**声明**的函数则需要 `unsafe`（默认如此，标了 `safe` 的除外）。

C 侧的函数指针经常允许为空。Rust 里不要用 `transmute` 去凑，用 `Option`：可空函数指针的布局由**空指针优化（nullable pointer optimization）**保证，`None` 就是空指针。Nomicon 举的例子就是回调注册：

```rust
// C 侧：void register_cb(int (*f)(int));
extern "C" fn apply_or_square(f: Option<extern "C" fn(i32) -> i32>, x: i32) -> i32 {
    match f {
        Some(f) => f(x),
        None => x * x,
    }
}

extern "C" fn inc(x: i32) -> i32 {
    x + 1
}

fn main() {
    assert_eq!(apply_or_square(Some(inc), 5), 6);
    assert_eq!(apply_or_square(None, 5), 25);
}
```

`Option<extern "C" fn(...)>` 与 C 的 `int (*)(int)` 是同一个东西，大小也一样——Nomicon 的说法是这个优化 "unlike other optimizations it is guaranteed to apply to eligible types."（译自 Nomicon）

最后一句所有权提醒：`extern "C"` 只说怎么调用，**不说谁负责释放**。要是想让 C 长期持有 Rust 的堆对象，标准做法是 `Box::into_raw` 交出去、需要时 `Box::from_raw` 收回来（同理 `CString::into_raw`/`from_raw`）；漏掉 `from_raw` 就是内存泄漏，收两次就是 UB。

**不透明句柄（opaque handle）** 是这条路上最常用的形状：C 头文件里只有 `typedef struct engine engine;`，看不到字段；Rust 侧 `Box::into_raw` 交出一个 `*mut Engine`，C 把它当 `engine*` 存着，通过 `engine_step`/`engine_free` 这类函数操作它，最后由 `engine_free` 调 `Box::from_raw` 收回。这样 Rust 的内部类型一个字段都不用暴露给 C，布局怎么变都行。约束只有一条：**创建与销毁必须配对，而且要在文档里写死谁负责调用哪个**。第 23 章 23.6 的 UB 清单第 2 条（悬垂读写）正是这里最常见的翻车方式。

### 26.6 panic 与展开：三条防线

这是 FFI 里最容易出错的一段，值得单独记住三条事实。

**事实一：panic 不允许越过非 `-unwind` 的 ABI 边界。** 越过会发生什么？Nomicon 写得很直接："panic will cause the process to safely abort."（引自 Nomicon `FFI and unwinding` 一节）注意结果是**中止进程**，不是 UB、也不是把错误交给对方——因为 C 那边根本没有「展开」这个概念，它无法在栈上跑 Rust 的析构。

**事实二：`catch_unwind` 必须放在边界之内。** 如果 panic 已经越过了 `extern "C"` 函数，进程早就中止了，外面再包一层 `catch_unwind` 也没机会执行。正确的位置是：**在 `extern "C" fn` 自己的函数体里**包住可能 panic 的代码。

**事实三：确实需要穿透边界时，用 `-unwind` ABI。** 把两侧的函数类型都写成 `extern "C-unwind"`，panic 就能像普通 Rust 调用一样穿过去，在那边的 `catch_unwind` 里被接住。Nomicon：If you expect Rust panics or foreign (e.g. C++) exceptions to cross an FFI boundary, that boundary must use the appropriate -unwind ABI string.（引自 Nomicon）

> [!NOTE]
> panic 策略会盖过 ABI 字符串：用 `panic = "abort"` 编译时，`panic!` 一律立刻中止进程——哪怕相关函数的 ABI 是 `-unwind`。Nomicon 的原话是："Compiling with panic=abort will still cause panic! to immediately abort the process, regardless of which ABI is specified by the function that panics."（引自 Nomicon）所以 `-unwind` 只在 `panic = "unwind"`（默认）下才有意义。

> [!WARNING]
> `-unwind` 是给「你确实要跨语言传播异常」的场景准备的。反向的情形——C++ 的异常进入 Rust——Nomicon 明确说是 UB："A foreign exception entering Rust will cause undefined behavior."（引自 Nomicon）它同时提醒："Note that the interaction of catch_unwind with foreign exceptions is undefined, as is the interaction of panic with foreign exception-catching mechanisms (notably C++'s try/catch)."（引自 Nomicon）所以别把 `catch_unwind` 当作 C++ 的 `try`。

顺带一条 2024 的语法变化：在 `unsafe fn`（包括 `unsafe extern "C" fn`）里调用其他 unsafe 操作，也需要显式的 `unsafe` 块——`unsafe_op_in_unsafe_fn` 这个 lint 从 2024 起默认警告。写 FFI 包装函数时你会反复撞上它，加块就好。

### 26.7 可变参数：声明已稳定，定义还在 nightly

C 的 `printf` 这类变参函数，Rust 侧**声明**可以直接写 `...`：

```rust
unsafe extern "C" {
    fn c_vsum(count: i32, ...) -> i32;
}
```

这条声明在 stable 上可用（C ABI 一直支持；1.91 又把这一能力扩展到 `sysv64`、`win64`、`efiapi`、`aapcs`）。调用时的类型与个数由你自己保证——传错就是 UB，这也是 Reference 提醒不要给变参函数随便标 `safe` 的原因。

反过来，**在 Rust 里定义**一个变参函数仍然需要 nightly 的 `c_variadic` feature：参数写成 `ap: ...`，拿到 `std::ffi::VaList`，用 `next_arg::<T>()` 逐个取（等价于 C 的 `va_arg`；`next_arg` 本身是 `unsafe fn`，每取一个都得包一层 `unsafe`）。`VaList` 文档上的标记是 "This is a nightly-only experimental API. (c_variadic #44930)"（引自 `std::ffi::VaList` 文档）。

### 26.8 把 unsafe 收进安全包装

一次好的 FFI 绑定，最终对 Rust 使用者暴露的应该是安全函数。做法是：**unsafe 只出现在最内层**，出口处把能检查的都检查掉。

```rust
use std::ffi::{c_char, CStr};

// 扮演 C 侧：只认裸指针，长度靠扫描 nul
unsafe extern "C" fn raw_len(s: *const c_char) -> usize {
    // SAFETY: 由调用方的契约保证 s 以 nul 结尾且期间不被改写
    unsafe { CStr::from_ptr(s) }.to_bytes().len()
}

/// 安全包装：只收 `&CStr`，把「指针有效」这件事挡在门外。
fn len_of(s: &CStr) -> usize {
    // SAFETY: `&CStr` 保证以 nul 结尾，且生命周期覆盖本次调用
    unsafe { raw_len(s.as_ptr()) }
}

fn main() {
    assert_eq!(len_of(c"hello"), 5);
}
```

规律是：`unsafe` 边界越窄越好，最好窄到一个函数；wrapper 上写 `/// # Safety`（如果它还是 unsafe 的）或者干脆把它变成安全函数。第 23 章 23.2 的结论（`unsafe` 不是「关掉检查」）在这里尤其成立：把整段逻辑塞进一个 `unsafe` 块，只会让 review 无从下手。

## 动手看看

### 例 1：进程内互调——没有 C 编译器也能跑

下面这个程序把本章的三个要点放在一起：`extern "C" fn` 作回调、`#[unsafe(no_mangle)]` 导出符号、panic 的两种处理方式。完整代码在 `snippets/ch26/callbacks.rs`：

```rust
{{#rustdoc_include ../snippets/ch26/callbacks.rs}}
```

```console
$ rustc --edition 2024 snippets/ch26/callbacks.rs -o callbacks.exe
$ .\callbacks.exe
```

```text
drive(double, 21)      = 42
drive(checked_div, 4)  = 25
drive(checked_div, 0)  = -1
外层抓住 drive_unwind(raw_div, 0)：true
drive_unwind(raw_div, 5) = 20
```

（以 Rust 1.98 为例）stderr 上还会出现两条被 `catch_unwind` 拦住的 panic 提示，那是默认 panic hook 照常打印的结果；程序退出码是 0。这五行分别对应：普通回调；回调正常返回；回调内部把 panic 变成了 `-1`；`C-unwind` 让 panic 穿到外层被抓住；穿过去之后再正常返回。

### 例 2：字符串往返

`CString`（拥有）、`CStr`（借用）、`c"..."`（字面量）三者的用法在 `snippets/ch26/cstrings.rs`：

```rust
{{#rustdoc_include ../snippets/ch26/cstrings.rs}}
```

```console
$ rustc --edition 2024 snippets/ch26/cstrings.rs -o cstrings.exe
$ .\cstrings.exe
```

```text
CString::new        -> "hello"
c_style_len(c"hello") -> 5
from_bytes_until_nul -> "bytes"
CString::new("a\0b")  是 Err
```

### 例 3：真 C 库端到端

上面两例的「C 侧」都是 Rust 函数扮演的——它们证明调用约定一致，但没有证明「链到别人编译出来的目标文件」。这一节补上那一环。本机是 MSVC 目标的 Rust 工具链，另外装了 MinGW-W64 的 gcc 16.1.0（`ar` 随包提供），下面是实测过的完整流程。

C 侧 `c26.c`：

```c
#include <stdarg.h>
#include <stdint.h>
#include <stddef.h>

typedef struct {
    int32_t x;
    int32_t y;
} point;

int32_t c_add(int32_t a, int32_t b) {
    return a + b;
}

int32_t c_apply(int32_t (*f)(int32_t), int32_t x) {
    return f(x);
}

int32_t c_dist2(point p) {
    return p.x * p.x + p.y * p.y;
}

int32_t c_vsum(int count, ...) {
    va_list ap;
    va_start(ap, count);
    int32_t total = 0;
    for (int i = 0; i < count; i++) {
        total += va_arg(ap, int32_t);
    }
    va_end(ap);
    return total;
}

size_t c_strlen(const char *s) {
    size_t n = 0;
    while (s[n] != '\0') n++;
    return n;
}
```

编成动态库，同时导出链接用的导入库（Windows）：

```console
$ gcc -shared -O2 -o c26.dll c26.c -Wl,--out-implib,c26.lib
```

Rust 侧 `main.rs`（`#[repr(C)]` 结构体、函数指针、变参、C 字符串各来一份）：

```rust,ignore
use std::ffi::{c_char, CStr, CString};

#[repr(C)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[link(name = "c26")]
unsafe extern "C" {
    fn c_add(a: i32, b: i32) -> i32;
    fn c_apply(f: extern "C" fn(i32) -> i32, x: i32) -> i32;
    fn c_dist2(p: Point) -> i32;
    fn c_vsum(count: i32, ...) -> i32;
    fn c_strlen(s: *const c_char) -> usize;
}

extern "C" fn triple(x: i32) -> i32 {
    x * 3
}

fn main() {
    unsafe {
        println!("c_add(20, 22)          = {}", c_add(20, 22));
        println!("c_apply(triple, 7)     = {}", c_apply(triple, 7));
        println!("c_dist2(Point {{3, 4}})  = {}", c_dist2(Point { x: 3, y: 4 }));
        println!("c_vsum(3, 10, 20, 12)  = {}", c_vsum(3, 10, 20, 12));

        let owned = CString::new("hello from Rust").unwrap();
        println!("c_strlen(CString)      = {}", c_strlen(owned.as_ptr()));

        let borrowed: &CStr = c"also from Rust";
        println!("c_strlen(c\"...\")        = {}", c_strlen(borrowed.as_ptr()));
    }
}
```

```console
$ rustc --edition 2024 main.rs -L . -o demo.exe
$ .\demo.exe
```

```text
c_add(20, 22)          = 42
c_apply(triple, 7)     = 21
c_dist2(Point {3, 4})  = 25
c_vsum(3, 10, 20, 12)  = 42
c_strlen(CString)      = 15
c_strlen(c"...")        = 14
```

（以 Rust 1.98 + gcc 16.1.0 为例）`c26.dll` 要和 `demo.exe` 放在一起，Windows 的加载器才找得到它。

真实项目里这两步（编 C、告诉 cargo 去哪找库）交给 `build.rs`：在 `Cargo.toml` 的 `[package]` 里加一行 `build = "build.rs"`。下面这份是实测跑通的版本，用 `gcc -c` 加 `ar` 走静态链接，不依赖任何外部 crate：

```rust,ignore
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=c26.c");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let obj = out_dir.join("c26.o");
    let status = Command::new("gcc")
        .args(["-c", "-O2", "-o"])
        .arg(&obj)
        .arg("c26.c")
        .status()
        .expect("找不到 gcc：FFI 例子需要 C 编译器");
    assert!(status.success(), "gcc 编译 c26.c 失败");

    let lib = out_dir.join("libc26.a");
    let status = Command::new("ar")
        .arg("rcs")
        .arg(&lib)
        .arg(&obj)
        .status()
        .expect("找不到 ar");
    assert!(status.success(), "打包 libc26.a 失败");

    println!("cargo::rustc-link-search=native={}", out_dir.display());
    println!("cargo::rustc-link-lib=static=c26");
}
```

```console
$ cargo run -q
c_add(20, 22)          = 42
c_apply(triple, 7)     = 21
c_dist2(Point {3, 4})  = 25
c_vsum(3, 10, 20, 12)  = 42
c_strlen(CString)      = 15
c_strlen(c"...")        = 14
```

两条 `cargo::rustc-link-*` 指令是这套流程的关键：一条把 `OUT_DIR` 加进链接器搜索路径，一条说明要链接的静态库名。

> [!TIP]
> 手上只有头文件时，别手抄声明。`bindgen` 能从 `.h` 生成 `extern` 块和 `#[repr(C)]` 定义，比人手可靠。第一版绑定生成出来的是裸 unsafe 接口，安全包装仍然要你自己写——这一步没人能替你做。

## 报错现场

### 现场一：2024 里忘了 `unsafe extern`

`src/main.rs`：

```rust,compile_fail
extern "C" {
    fn puts(s: *const std::ffi::c_char) -> i32;
}

fn main() {
    println!("hello");
}
```

以 Rust 1.98 为例，编译输出：

```text
error: extern blocks must be unsafe
 --> src/main.rs:1:1
  |
1 | / extern "C" {
2 | |     fn puts(s: *const std::ffi::c_char) -> i32;
3 | | }
  | |_^

error: aborting due to 1 previous error
```

**这是硬错误（exit 1），不是警告。** 定位路径：报错位置指的是整个块，不是某一行；消息里的 `extern blocks must be unsafe` 就是字面要求。修法：`extern "C" {` 改成 `unsafe extern "C" {`。迁移老代码时可以用 `cargo fix --edition`，它会挂上 `missing_unsafe_on_extern` 这个 lint 自动补关键字，但**不会**替你核对签名。

### 现场二：`#[no_mangle]` 少了 `unsafe`

```rust,compile_fail
#[no_mangle]
pub extern "C" fn answer() -> i32 {
    42
}

fn main() {
    println!("{}", answer());
}
```

```text
error: unsafe attribute used without unsafe
 --> src/main.rs:1:3
  |
1 | #[no_mangle]
  |   ^^^^^^^^^ usage of unsafe attribute
  |
help: wrap the attribute in `unsafe(...)`
  |
1 | #[unsafe(no_mangle)]
  |   +++++++         +

error: aborting due to 1 previous error
```

**同样是硬错误。** 注意 `help` 直接给出了机械修法：把属性名包进 `unsafe(...)`。同一个坑还适用于 `export_name` 与 `link_section`。

### 现场三：拼错符号名要到链接期才暴露

名字写错时，rustc 一声不响——`extern` 声明本来就不检查是否存在，问题交给链接器。把 `c_add` 写成 `c_ad`，编译输出（节选关键两行）：

```text
error: linking with `link.exe` failed: exit code: 1120
  |
  = note: ... error LNK2019: 无法解析的外部符号 __imp_c_ad ...
```

（link.exe 的消息随系统语言变化。）**定位路径**：错误出现在「链接」阶段而不是编译阶段，说明 Rust 代码本身没问题，是**某个符号没人定义**；`__imp_` 前缀说明它找的是动态库的导入符号。排查顺序：符号名拼写 → `#[link(name = ...)]` 里的库名 → 库文件是否在链接器的搜索路径里（`-L` 或 `build.rs` 的 `rustc-link-search`）。

## 小结

- **ABI 是二进制层面的约定**：参数怎么传、符号叫什么、结构体怎么排。Rust 的 ABI 没有稳定性保证，跨语言一律显式写 `extern "C"`（或 `"system"` 等平台约定）。
- **`extern` 块是「未经检查的导入」**：里面的项默认为 `unsafe`，写对签名是你的责任。2024 起块本身必须写 `unsafe`，`no_mangle`/`export_name`/`link_section` 也必须包进 `unsafe(...)`——这两条都是硬错误。
- **导出给 C 需要两件事**：`pub extern "C" fn` 加 `#[unsafe(no_mangle)]`；参数与返回值只能出现 C ABI 类型，结构体要 `#[repr(C)]`（第 25 章）。
- **字符串三件套**：`CString` 拥有、`CStr` 借用、`c"..."` 字面量。`&CStr` 不进 FFI 签名，传 `as_ptr()`。
- **回调用 `extern "C" fn` 函数指针**，可空的用 `Option<...>`（空指针优化保证布局）。符号修饰、布局、生命周期三件事，都要自己说清楚。
- **panic 不许越界**：越过非 `-unwind` 边界会安全中止进程。要就地处理就在 `extern "C" fn` 内部 `catch_unwind`；确定要穿透就把整条链改成 `extern "C-unwind"`。外部异常进入 Rust 是 UB。
- **变参**：声明 `...` 在 stable 可用；自己定义变参函数仍需 nightly 的 `c_variadic` 与 `VaList`。
- 最后落到工程习惯上：**unsafe 只留在最薄的一层**，对外暴露安全 API。

## 练习

**1. 找错并改**

下面这段打算把 Rust 函数暴露给 C，但在 2024 版里编译不过。指出一共几处问题，并给出修法。

```rust,ignore
extern "C" {
    fn c_log(msg: *const c_char) -> i32;
}

#[no_mangle]
pub extern "C" fn report(code: i32) -> i32 {
    let text = CString::new(format!("code={code}")).unwrap();
    unsafe { c_log(text.as_ptr()) }
}
```

**2. 预测运行结果**

```rust,ignore
extern "C" fn risky(x: i32) -> i32 {
    if x == 0 { panic!("boom") }
    x
}

fn main() {
    let r = std::panic::catch_unwind(|| risky(0));
    println!("caught: {}", r.is_err());
}
```

这段代码能打印出 `caught: true` 吗？为什么？要得到「不中止进程、把错误变成返回值」的效果，应该怎么改？

**3. 说明为什么**

为什么 `&CStr` 不能直接写在 `extern "C"` 声明里，而要写成 `*const c_char`？

<details><summary>参考答案</summary>

**1.** 三处：

- `extern "C"` 块缺 `unsafe`：改成 `unsafe extern "C"`，否则报 `error: extern blocks must be unsafe`。
- `#[no_mangle]` 缺 `unsafe`：改成 `#[unsafe(no_mangle)]`，否则报 `error: unsafe attribute used without unsafe`。
- 缺 `use std::ffi::{c_char, CString};`，`c_char` 与 `CString` 都不在作用域里。

改完之后的版本：

```rust,ignore
use std::ffi::{c_char, CString};

unsafe extern "C" {
    fn c_log(msg: *const c_char) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn report(code: i32) -> i32 {
    let text = CString::new(format!("code={code}")).unwrap();
    // SAFETY: text 在本次调用期间存活，指针以 nul 结尾
    unsafe { c_log(text.as_ptr()) }
}
```

还有一处**逻辑**上的隐患值得注意：`text` 是局部 `CString`，调用返回后它就被释放了。这里没问题，因为 `c_log` 只在调用期间读；但如果 `c_log` 把指针存起来以后再用，那就是悬垂指针 —— 这种场景要用 `CString::into_raw` 把所有权交出去，约定好谁来 `from_raw`。

**2.** 不能。`risky` 的类型是 `extern "C" fn(i32) -> i32`，panic 越过这个 ABI 边界时进程会安全中止（本机实测是 `panic in a function that cannot unwind` 加 `thread caused non-unwinding panic. aborting.`），外层那个 `catch_unwind` 根本没机会执行。两种改法：

- 把 `catch_unwind` 挪进 `risky` 内部，让 panic 不出边界：

```rust
extern "C" fn risky(x: i32) -> i32 {
    match std::panic::catch_unwind(|| {
        if x == 0 { panic!("boom") }
        x
    }) {
        Ok(v) => v,
        Err(_) => -1,
    }
}
```

- 或者两侧都改成 `extern "C-unwind" fn`，让 panic 合法穿到外层，由外层的 `catch_unwind` 接住。

顺带记一句：即便抓住了 panic，默认 panic hook 仍会把消息打到 stderr；要安静下来得自己设 `std::panic::set_hook`。

**3.** 因为 `&CStr` 的布局没有跨语言保证。`CStr` 是 DST（动态大小类型），`&CStr` 是「指针 + 长度」的胖指针，而 C 只认一个裸指针；`std::ffi::CStr` 的文档明确写着它没有保证的布局、不应放进 FFI 函数签名。跨语言传参只能传双方都精确知道的东西：`*const c_char`。另外，裸指针还去掉了 Rust 的生命周期信息——「这块内存能活多久」必须由接口契约写清楚，而不是靠类型系统替 C 记住。

</details>

## 延伸阅读

- Nomicon, FFI 与 unwind：[https://doc.rust-lang.org/nomicon/ffi.html](https://doc.rust-lang.org/nomicon/ffi.html)
- Rust Reference, External blocks：[https://doc.rust-lang.org/reference/items/external-blocks.html](https://doc.rust-lang.org/reference/items/external-blocks.html)
- Rust Reference, ABI：[https://doc.rust-lang.org/reference/abi.html](https://doc.rust-lang.org/reference/abi.html)
- Rust Reference, Linkage：[https://doc.rust-lang.org/reference/linkage.html](https://doc.rust-lang.org/reference/linkage.html)
- `std::ffi`：[https://doc.rust-lang.org/std/ffi/index.html](https://doc.rust-lang.org/std/ffi/index.html)
- `std::ffi::CStr`：[https://doc.rust-lang.org/std/ffi/struct.CStr.html](https://doc.rust-lang.org/std/ffi/struct.CStr.html)
- `std::ffi::CString`：[https://doc.rust-lang.org/std/ffi/struct.CString.html](https://doc.rust-lang.org/std/ffi/struct.CString.html)
- Edition Guide, Unsafe extern blocks：[https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-extern.html](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-extern.html)
- Edition Guide, Unsafe attributes：[https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html)
- The Cargo Book, Build Scripts：[https://doc.rust-lang.org/cargo/reference/build-scripts.html](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- `bindgen`：[https://rust-lang.github.io/rust-bindgen/](https://rust-lang.github.io/rust-bindgen/)
- `libc` crate 文档：[https://docs.rs/libc](https://docs.rs/libc)

