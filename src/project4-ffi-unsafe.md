# 项目 4：封装系统 API（FFI 与 unsafe）

前面四章把系统层的零件一件件拆开了：`unsafe` 与 UB（第 23 章）、裸指针与未初始化内存（第 24 章）、数据布局与 `repr`（第 25 章）、FFI 与 ABI（第 26 章）。这一章把零件装成一个能用的东西：`examples/sys-probe/` —— 零依赖的小库，把三个 Windows 系统 API 包成安全函数。

三个 API 的难度刚好递增：一个无参数、无失败的取时间函数；一个「把结果写进出参」的结构体函数；一个要在别的线程上回调你的函数。第三件才是真正的考验：回调没有上下文参数、panic 不能穿过 FFI 边界、注册与注销必须配对。代码放在仓库的 `examples/sys-probe/`，`[dependencies]` 是空的。

## 这一章解决什么问题

### 要做的东西

公开面很小：四个自由函数、一个结构体、一个事件类型、一个句柄，外加一个错误枚举和一个 `Result` 别名。

| 公开函数 | 封装的 Win32 API | 难在哪 |
|---|---|---|
| `uptime_millis() -> Result<u64>` | `GetTickCount64` | 最简形状：无参数、无失败路径 |
| `system_file_time() -> Result<FileTime>`、`unix_time_millis()` | `GetSystemTimeAsFileTime` | 出参结构体：`#[repr(C)]` 布局 + 裸指针 + 未初始化内存 |
| `watch_ctrl_events(handler) -> Result<CtrlGuard>` | `SetConsoleCtrlHandler` | 回调：系统在**新线程**上调用你、只能靠全局状态找闭包、panic 不许出去 |

一次调用要穿过三层，而**平台的分叉只在中间那一层**：

<svg viewBox="0 0 700 266" width="100%" role="img" aria-label="三层结构：上层是三个跨平台入口（unix_time_millis 由 system_file_time 组合出），中间是 platform 层（windows 与 unsupported 两套同签名实现），下层 Windows 上是 Kernel32.dll 的真调用，非 Windows 上没有对应 API" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">安全 API → 平台层（cfg 只在这里分叉）→ 系统</text>
<g fill="none" stroke="currentColor" stroke-width="1.5">
<rect x="112" y="32" width="180" height="46" rx="6" />
<rect x="300" y="32" width="180" height="46" rx="6" />
<rect x="488" y="32" width="204" height="46" rx="6" />
<rect x="104" y="100" width="588" height="76" rx="6" stroke-dasharray="5 3" />
<rect x="112" y="124" width="278" height="44" rx="6" />
<rect x="402" y="124" width="282" height="44" rx="6" />
<rect x="112" y="196" width="278" height="36" rx="6" />
<rect x="402" y="196" width="282" height="36" rx="6" />
</g>
<g font-size="12" fill="currentColor" text-anchor="middle" font-family="monospace">
<text x="202" y="52">uptime_millis()</text>
<text x="390" y="52">system_file_time()</text>
<text x="590" y="52">watch_ctrl_events()</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle">
<text x="202" y="68">全是安全函数</text>
<text x="390" y="68">全是安全函数</text>
<text x="590" y="68">返回 CtrlGuard</text>
</g>
<g font-size="12" fill="currentColor">
<text x="112" y="116">platform 层：两套实现，签名逐字相同</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle" font-family="monospace">
<text x="251" y="142">platform/windows.rs</text>
<text x="543" y="142">platform/unsupported.rs</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle">
<text x="251" y="160">unsafe extern "system" + unsafe 块</text>
<text x="543" y="160">只回 Err(Unsupported)，零 unsafe</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle">
<text x="251" y="219">Kernel32.dll：真调用</text>
<text x="543" y="219">没有这个 API</text>
</g>
<g stroke="var(--links)" stroke-width="2" fill="none">
<line x1="202" y1="78" x2="202" y2="100" />
<line x1="390" y1="78" x2="390" y2="100" />
<line x1="590" y1="78" x2="590" y2="100" />
<line x1="251" y1="168" x2="251" y2="196" />
</g>
<g stroke="var(--warning-border)" stroke-width="2" stroke-dasharray="4 3" fill="none">
<line x1="543" y1="168" x2="543" y2="196" />
</g>
<g fill="var(--links)">
<polygon points="202,100 197,90 207,90" />
<polygon points="390,100 385,90 395,90" />
<polygon points="590,100 585,90 595,90" />
<polygon points="251,196 246,186 256,186" />
</g>
<text x="556" y="188" font-size="12" fill="var(--warning-border)">不会到系统</text>
<text x="0" y="254" font-size="12" fill="currentColor" opacity="0.75">上层一个 cfg 都不用写；换平台只换中间这一层，公开签名一字不改。</text>
</svg>

先看它跑起来的样子（在 `examples/` 里跑可以省掉 `--manifest-path`）：

```console
$ cargo run --manifest-path examples/Cargo.toml -p sys-probe --example probe
开机时长        ：195525000 ms
FILETIME        ：134368207705187259 个 100 纳秒
其中的 Unix 毫秒 ：1792347170518
std::time 对照   ：1792347170519 ms
控制事件处理器   ：已注册
第二次注册       ：被拒（本进程已经注册过一个控制事件处理器）
控制事件处理器   ：已注销
```

第二行的 `FILETIME` 是 Windows 自己的时间表示（100 纳秒一格），第四行是同一瞬间用标准库算出来的值 —— 两条路差 1 ms，因为两次取样之间时间在走。最后三行是控制事件回调的注册与注销：真按 Ctrl+C 时，你写的闭包会在系统新起的线程上被调用。

完整代码在 `examples/sys-probe/`：`src/lib.rs`（公开面）、`src/error.rs`、`src/file_time.rs`、`src/ctrl.rs`、`src/platform.rs` + `src/platform/`，加上 `tests/probe.rs` 与 `examples/probe.rs`。

### 验收标准

做完要满足下面五条，缺一条都算没做完：

1. `cargo test --manifest-path examples/Cargo.toml -p sys-probe` 全绿：单元测试、集成测试、文档测试三层都在里面，Windows 上走的是真 FFI；
2. 同一个 crate 在非 Windows 上也编译、也让全部测试通过：每个入口都回 `ProbeError::Unsupported`（Linux CI 覆盖的正是这条）；
3. 公开面只有安全函数、一个带 `# Safety` 段的 `unsafe fn`（`FileTime::read_raw`）、一个不可自己构造的句柄；`unsafe` 只落在 `platform/windows.rs` 与 `file_time.rs` 两个文件里，其余模块用 `#![forbid(unsafe_code)]` 钉死；
4. 用户闭包里的 panic 不会把进程 abort，而且有测试固定住这一点；
5. 正文里每段引用的代码都是 crate 里的原文，一个字不改。

本次验证用稳定版 rustc 1.98.0、edition 2024，机器是 x86_64-pc-windows-msvc。下面的终端记录都真跑过，只省掉 cargo 自己的 `Compiling` / `Finished` / `Running` 行与 `error: process didn't exit successfully …` 之后的补充说明。非 Windows 分支在本机用交叉编译目标做了编译检查（`cargo check --target x86_64-linux-android --all-targets`，本机装了这个目标、没装 linux-gnu）—— 它**跑**起来的样子只能在 Linux CI 上看到。

## 设计取舍

动手之前先定几件影响全局的事，每条都给「为什么不选另一个」。

**1. 做成库，不是二进制。** 库才可能被测试：二进制只能端到端跑，而这里最需要固定住的恰恰是「回调 panic 之后进程还在」这种内部行为。库还逼着你回答「公开面长什么样」—— 这一步会把「哪些东西必须 unsafe」筛出来。代价是多一层模块划分，收益是公开面能写下来。

**2. 零依赖，自己写声明；不引 `windows-sys`。** 真实的 Windows 项目应当优先用现成的绑定 crate：几万个函数的声明、常量、结构体由维护者与社区反复核对，比手抄靠谱。本书这里反过来选，是因为本章要教的正是**声明本身**：`#[link(name = "kernel32")]` 告诉链接器去哪里找符号，`extern "system"` 说的是调用约定，每个签名错了就是你自己的 UB（第 26 章：声明的正确性由写块的人负责）。引了 `windows-sys`，你看到的是安全包装，边界会被藏起来一次。这个取舍要写清楚：**教学选后者，生产选前者。**

**3. 签名由「最弱的那一侧」决定：`uptime_millis` 在 Windows 上不可能失败，却仍返回 `Result`。** 因为跨平台库只有一个签名，而非 Windows 上必须能表达「这里根本没有这个 API」。替代方案有两条，都不好：只编 Windows（Linux CI 上整个 crate 消失，公开面没人盯）、返回 `Option`（丢掉「为什么没有」这个信息）。代价是 Windows 用户多写一个 `?` —— 这个代价换来的是一份在两种平台上都成立的签名，值得。配套的纪律是：`cfg` 只出现在 `src/platform.rs` 这一个文件里（windows / not(windows) 两处判断），上层代码一个字都不用改。

**4. `unsafe` 只留在最内层：两个文件，三个模块上锁。** `unsafe` 只出现在 `src/platform/windows.rs`（调用系统 API 的那几个块）和 `src/file_time.rs`（读裸指针的那一次 `ptr.read()`）里；`error.rs`、`ctrl.rs`、`platform/unsupported.rs` 顶部都写着 `#![forbid(unsafe_code)]` —— 这不是装饰：它让「这三个模块不许碰 unsafe」由编译器守，而不是由 review 守。公开面上留一个 `unsafe fn` 当契约入口：`FileTime::read_raw`，C 侧把「指向 `FILETIME` 的指针」递过来时用它读值。库无法检查这个指针，所以它只能是 `unsafe`，把契约写进 `# Safety` 段 —— 这就是第 23 章说的「契约的落点」。

**5. 一个错误枚举，四种失败。** `ProbeError` 分成「平台没有这个 API」「系统调用失败（带 Win32 错误码 + 原因链）」「值换算不出来」「用法不对」四类。不用 `Box<dyn Error>`：种类固定，调用方要按种类分支（第 13、18 章）。枚举标 `#[non_exhaustive]`：以后加变体不算破坏性改动。`Os` 变体里同时存错误码和 `io::Error`：错误码是给程序看的主事实（可比较、可断言），`io::Error` 挂在 `source()` 上，需要时能印出系统自带的消息。

**6. 控制事件只留一个槽位。** Windows 允许一个进程注册多个处理器，调用顺序是「后注册的先调用，直到有一个返回 TRUE」；本库只留一个槽位 —— 这样「谁处理这个事件」才有唯一答案，也让「注销」不必猜是哪一层。代价是不能把两个闭包叠起来用；想组合，就在自己那一个闭包里组合。槽位是全局的 `static HANDLER: Mutex<Option<Handler>>`（`Handler` 就是 `Box<dyn FnMut(CtrlEvent) -> bool + Send>`）：回调签名里没有「上下文参数」这一格（C 的 `BOOL (*)(DWORD)` 就一个整数），闭包除了全局槽位无处可放。

**7. `CtrlEvent` 是新类型，不是枚举。** 系统送来的是 `DWORD`，值域里可能有我们不认识的码。写成 `#[repr(u32)] enum CtrlEvent { … }` 再 `transmute` 转换，就等于允许构造一个「不是任何变体」的值 —— 那是未定义行为（第 23 章：类型必须始终持有有效值）。这里存原始数字，只给已知的五个码取名字，未知码原样保留。

## 逐块实现

下面每块都是从 `examples/sys-probe/src/*.rs` 切出来的原文，块末注明用到哪几章的概念。

> [!NOTE]
> 这些块大多标了 `ignore`：它们是跨模块互相引用的片段，`mdbook test` 会把每个块当独立程序编译，单独的片段自然编译不过（例如 `ProbeError::os` 用了同文件里定义的变体）。片段本身与文件逐字一致；整份代码由下面的 `cargo test` 真跑验证。少数几个不依赖 crate 内部名字的块是真编译的（`catch_unwind` 演示、报错现场的最小复现）。

### 1. 错误类型：`src/error.rs`

先把可能出错的地方列全 —— 第 7、18 章的老习惯：

```rust,ignore
{{#include ../examples/sys-probe/src/error.rs:13:49}}
```

`Unsupported` 带着 `api` 字段（缺的是哪个 API），`BeforeUnixEpoch` 带着原始计数（出错时手里有什么就给什么）。两个内部构造函数服务两套平台实现：

```rust,ignore
{{#include ../examples/sys-probe/src/error.rs:51:68}}
```

`unsupported` 只在非 Windows 那份实现里用，`os` 只在 Windows 那份里用 —— 两套实现各自编一次，另一个就成「死代码」，所以这里显式 `#[allow(dead_code)]`，而不是让构造函数跟着 `cfg` 走（跟着走的话，两边都得维护两份签名）。`os` 里那行 `io::Error::from_raw_os_error` 是标准库给 Windows 错误码准备的入口：传进去的是 `GetLastError` 的值，出来的是能挂原因链的对象。

剩下的两个 trait 实现是公开面的一部分：

```rust,ignore
{{#include ../examples/sys-probe/src/error.rs:70:92}}
```

`Display` 只印错误码，不印系统消息：系统消息随系统语言变，错误码不变，测试里断言文本才靠得住；想要系统消息的人可以顺着 `source()` 拿。`source()` 只给 `Os` 变体返回原因，其余变体没有更底层的原因 —— 「有没有原因链」在这里是类型事实，不是约定。

**这块用到第 7 章的自定义错误与 `?`、第 9 章的 `Display`、第 18 章的 `#[non_exhaustive]`、`Result` 别名与 `source` 原因链。**

### 2. `FILETIME` 与布局契约：`src/file_time.rs`

Windows 的时间戳叫 `FILETIME`，文档的定义是 "Contains a 64-bit value representing the number of 100-nanosecond intervals since January 1, 1601 (UTC)."（引自 learn.microsoft.com 的 FILETIME 页）。它的 C 定义是两个 `DWORD`：

```text
typedef struct _FILETIME {
  DWORD dwLowDateTime;
  DWORD dwHighDateTime;
} FILETIME, *PFILETIME, *LPFILETIME;
```

Rust 这边必须把这份布局原样抄下来，才敢往 FFI 签名里放：

```rust,ignore
{{#include ../examples/sys-probe/src/file_time.rs:7:32}}
```

两处细节值得停一下：字段是私有的（`repr(C)` 的布局按**声明顺序**排，字段名只在 Rust 这边有意义 —— 取 C 的名字反而会撞上命名习惯的 lint）；要造一个值只能走 `from_ticks`，谁都没法拼出一个布局不对的 `FileTime`。构造与拆解：

```rust,ignore
{{#include ../examples/sys-probe/src/file_time.rs:43:61}}
```

`from_ticks` / `ticks` 是 `const fn`：能用在常量里，也不会因为「要拼两个 32 位字段」而产生任何运行时开销。真正需要动脑的是换算：

```rust,ignore
{{#include ../examples/sys-probe/src/file_time.rs:63:81}}
```

`EPOCH_OFFSET_TICKS` 是 1601 到 1970 之间的格数（11644473600 秒 × 10 000 000）。先 `checked_sub` 再除：1970 年以前的值换不出 `u64` 毫秒，那就报错，而不是让减法回绕成天文数字。FILETIME 那一页还有一句提醒，正好解释为什么不在这里做减法之外的算术："It is not recommended that you add and subtract values from the FILETIME structure to obtain relative times."（引自同一页）—— 相对时间要在换算之后的整数上算，别在 100 纳秒计数上抠。

布局这件事不靠记忆，靠编译器：

```rust,ignore
{{#include ../examples/sys-probe/src/file_time.rs:108:117}}
```

四条 `assert!` 写在 `const` 块里：大小、对齐、两个字段的偏移。任何一条不成立，crate 直接编不过 —— 第 25 章讲的「`repr(C)` 是承诺」在这里变成了可执行的检查。改动结构体（比如手滑多加一个字段）时，编译器会在这一行拦下你，而不是等运行时读出错值。

最后是公开面上唯一的 `unsafe fn`：

```rust,ignore
{{#include ../examples/sys-probe/src/file_time.rs:83:105}}
```

`ptr.read()` 需要两件事同时成立：指针有效（已初始化、对齐到 4 字节）、读取期间那块内存不被改写。库里两件都检查不了，所以它落在调用方头上 —— `# Safety` 段就是把这两条写下来，`unsafe` 块上的 `// SAFETY:` 注释则是调用方自己的记账。另外注意函数体内的 `unsafe { ptr.read() }`：2024 起 `unsafe_op_in_unsafe_fn` 默认警告（第 33 章），`unsafe fn` 里的 unsafe 操作也要显式包块 —— 这样 review 时能一眼看出「这个函数里真正危险的是哪一行」。

**这块用到第 23 章的 `unsafe fn` 契约与 `# Safety`、第 24 章的裸指针与合法性、第 25 章的 `repr(C)` 布局与 `offset_of!`、第 9 章的 `const fn`。**

### 3. 平台层：`src/platform/windows.rs`

分叉点只有一个文件：

```rust,ignore
{{#include ../examples/sys-probe/src/platform.rs:1:21}}
```

两行 `cfg` 各选一个模块，导出同一组名字。上层（`ctrl.rs`、`lib.rs`）只 import `platform::…`，不知道 `cfg` 存在。降级那一份短到可以整段看：

```rust,ignore
{{#include ../examples/sys-probe/src/platform/unsupported.rs:1:28}}
```

它证明了两件事：公开签名能在非 Windows 上编译；以及「所有入口都会回 `Unsupported`」这句话是有代码支撑的，不是文档里的说法。

真家伙在另一半。先是类型映射与四个声明：

```rust,ignore
{{#include ../examples/sys-probe/src/platform/windows.rs:12:36}}
```

四行声明对应 Windows 头文件里的四个函数，`#[link(name = "kernel32")]` 把它们指向 Kernel32.dll。ABI 写 `"system"`：它在 x86 上是 `stdcall`、在 x64 上就是 C 的约定 —— 也就是 Win32 文档里 `WINAPI` 宏的含义（第 26 章的 ABI 表里，`"system"` 那一行说的正是这件事）。块里默认就是 `unsafe`，所以调用时必须写 `unsafe` 块。签名里出现的都是 C 认得的形状：`u64`、`i32`、裸指针、可空函数指针，还有一个 `#[repr(C)]` 结构体 —— 边界上能过去的只有双方都精确知道每比特含义的东西（第 26 章）。

前两个包装各只有几行：

```rust,ignore
{{#include ../examples/sys-probe/src/platform/windows.rs:38:55}}
```

`get_tick_count_64` 是最简形状：一个 `unsafe` 块、一句 `SAFETY` 注释。`get_system_time_as_file_time` 值得细看：出参是 `*mut FileTime`，而 Rust 这边不能造一个「假装已初始化」的 `FileTime`（第 24 章：未初始化内存不能当已初始化值用），所以先 `MaybeUninit::uninit()`，把裸指针交给函数，等它写满 8 字节再 `assume_init`。两段 `SAFETY` 注释刚好是这次调用的全部前提：指针可写、大小对齐正确；函数保证写满。如果函数可能中途失败（写一半就返回），`assume_init` 就不合法了 —— 这里的 `GetSystemTimeAsFileTime` 返回 `VOID`、没有失败路径，才有这个写法。

第三个包装多了一件事：错误码。

```rust,ignore
{{#include ../examples/sys-probe/src/platform/windows.rs:57:76}}
```

`SetConsoleCtrlHandler` 用返回值报成败（非零成功、零失败），文档的下一步是 "To get extended error information, call GetLastError."（引自 SetConsoleCtrlHandler 页）。取错误码必须**紧接着**失败的那次调用：它是每线程的「上一次错误」，中间任何一次 Win32 调用都可能覆盖它。所以这里的顺序是死的：调用 → 判返回值 → 立刻 `GetLastError`。

**这块用到第 23 章的 `unsafe {}` 最小化与 `// SAFETY:` 记账、第 24 章的 `MaybeUninit` 与 `assume_init`、第 25 章的 `#[repr(C)]` 类型进 FFI 签名、第 26 章的 `extern "system"` / `#[link]` / 声明的责任、第 18 章的错误码与原因链。**

### 4. 控制事件：`src/ctrl.rs`

这是本章的重头戏。Windows 的控制事件（Ctrl+C、Ctrl+Break、关窗、注销、关机）走一条回调：你注册一个函数，系统在**新线程**上调用它 —— 文档原文是 "When the signal is received, the system creates a new thread in the process to execute the function."（引自 HandlerRoutine 页）。回调签名只有一个 `DWORD`：`BOOL (*)(DWORD)`。没有「上下文参数」这一格，所以你想要的闭包只能放在全局：

```rust,ignore
{{#include ../examples/sys-probe/src/ctrl.rs:15:25}}
```

闭包的类型先取了个名字：`type Handler = Box<dyn FnMut(CtrlEvent) -> bool + Send>`，槽位就是 `Mutex<Option<Handler>>`。`Send` 是因为回调在系统线程上跑，`FnMut` 是因为处理器可能要记状态。`Mutex::new` 是 `const fn`，所以 `static HANDLER: Mutex<…> = Mutex::new(None)` 直接可用。

事件类型本身：

```rust,ignore
{{#include ../examples/sys-probe/src/ctrl.rs:27:57}}
```

五个常量对应文档里列出的五个码（0、1、2、5、6），`from_raw` 原样收下不认识的码。`Display` 把已知码翻成中文，未知码如实报数：

```rust,ignore
{{#include ../examples/sys-probe/src/ctrl.rs:59:71}}
```

注册这一步，顺序是这样定的：

```rust,ignore
{{#include ../examples/sys-probe/src/ctrl.rs:105:154}}
```

先向系统登记，再把闭包放进槽位。为什么是这个顺序 —— 整段都在同一把锁里：注册期间真有事件到达，回调会先阻塞在这把锁上，等闭包装好才拿到它，所以不存在「登记了却没人接」的空档；反过来（先放闭包后登记）则要在登记失败时回滚，失败路径白白多一条。至于注销，交给句柄：

```rust,ignore
{{#include ../examples/sys-probe/src/ctrl.rs:73:103}}
```

`CtrlGuard` 里没有状态，它存在的意义就是 `Drop`：句柄一离开作用域，处理器就撤掉。这里有个容易踩空的地方：**注销必须传当初注册的那个函数指针**。给 `HandlerRoutine` 传 NULL 不是「摘掉处理器」—— 文档里讲 NULL 的那一段说的是让进程「忽略 / 恢复正常处理 Ctrl+C」这个属性，处理器照样挂在系统里。这个坑不是纸上谈兵（探针程序不在 crate 里；父子进程各在自己的进程组，父进程用 `GenerateConsoleCtrlEvent` 发一个真的 CTRL_BREAK）：

```console
$ .\parent.exe .\child.exe
ready mode=null remove_ret=1
[父] mode=null GenerateConsoleCtrlEvent=1 err=0
calls=1
[父] mode=null 子进程退出：exit code: 0
ready mode=ptr remove_ret=1
[父] mode=ptr GenerateConsoleCtrlEvent=1 err=0
[父] mode=ptr 子进程退出：exit code: 0xc000013a
```

第一轮用 NULL 摘：`remove_ret=1`（调用本身「成功」），可信号一到，处理器还是被调用了（`calls=1`）。第二轮传注册时那个指针：`calls=` 那行根本没机会打印 —— 信号落到默认处理器上，进程直接被结束（`0xc000013a` 就是 Windows 的 Ctrl+C 退出码）。所以 `platform::set_console_ctrl_handler` 的形参是必填的 `HandlerRoutine` 而不是 `Option`：注销要传同一个指针这件事在签名层面就不给踩空的机会（原始声明里仍然是 `Option`，因为 C 的 `PHANDLER_ROUTINE` 本来就可空）。Drop 里另外两条纪律也都写在代码上：先让系统摘掉处理器、再清空槽位（反过来的窗口里来的事件会看到空槽，回报「没处理」，默认处理器就把进程结束了）；锁中毒也只能 `into_inner` 取回数据，不能在 Drop 里 panic（第 19 章）。

回调本身只有一件事要做 —— 别让 panic 出去：

```rust,ignore
{{#include ../examples/sys-probe/src/ctrl.rs:156:178}}
```

`catch_unwind` 把用户闭包的 panic 就地接住，转成「没处理」这个返回值。它为什么不能省？因为 panic 一旦跨过 `extern "system"` 这道边界，进程直接 abort —— 第 26 章引过 Nomicon 的原话："panic will cause the process to safely abort."。换成一张图看得更清楚：回调链上有四个框，只有第一个框属于系统。

<svg viewBox="0 0 720 216" width="100%" role="img" aria-label="一次控制事件的调用链：系统线程调用 trampoline；取槽位与调闭包这两步在 trampoline 体内、由 catch_unwind 包住，panic 出不去，只把 1 或 0 交回系统" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">一次控制事件的调用链：只有第一个框在系统那一侧</text>
<g fill="none" stroke="currentColor" stroke-width="1.5">
<rect x="8" y="44" width="140" height="48" rx="6" />
<rect x="342" y="28" width="370" height="82" rx="6" stroke-dasharray="5 3" />
<rect x="170" y="44" width="150" height="48" rx="6" />
<rect x="350" y="44" width="150" height="48" rx="6" />
<rect x="530" y="44" width="172" height="48" rx="6" />
</g>
<g font-size="12" fill="currentColor" text-anchor="middle">
<text x="78" y="66">系统线程</text>
<text x="78" y="84">（Windows 新起）</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle" font-family="monospace">
<text x="245" y="66">trampoline(raw: u32)</text>
<text x="425" y="66">HANDLER 槽位</text>
<text x="616" y="66">你的闭包</text>
</g>
<g font-size="12" fill="currentColor" text-anchor="middle">
<text x="245" y="84">-&gt; i32</text>
<text x="425" y="84">Mutex&lt;Option&lt;Handler&gt;&gt;</text>
<text x="616" y="84">返回 bool</text>
</g>
<g font-size="12" fill="var(--links)">
<text x="350" y="40">这两步在 trampoline 体内，由 catch_unwind 包住</text>
</g>
<g stroke="var(--links)" stroke-width="2" fill="none">
<line x1="148" y1="68" x2="170" y2="68" />
<line x1="320" y1="68" x2="350" y2="68" />
<line x1="500" y1="68" x2="530" y2="68" />
</g>
<g fill="var(--links)">
<polygon points="170,68 160,63 160,73" />
<polygon points="350,68 340,63 340,73" />
<polygon points="530,68 520,63 520,73" />
</g>
<text x="0" y="136" font-size="12" fill="var(--warning-border)">少了这一步，panic 就沿同一条链越出边界：进程立即 abort（第 26 章）。</text>
<text x="0" y="160" font-size="12" fill="currentColor">返回值只有两种含义：1 = 我处理了，系统不再叫别人；0 = 没处理，交给下一个处理器（或默认行为）。</text>
<text x="0" y="182" font-size="12" fill="currentColor" opacity="0.75">闭包由槽位持有；系统回调没有上下文参数，所以「找得到你的闭包」这件事只能靠全局状态。</text>
<text x="0" y="204" font-size="12" fill="currentColor" opacity="0.75">注册失败时槽位保持干净：先向系统登记、后放闭包，失败路径只有一条。</text>
</svg>

#### 试金石：把 panic 留在边界里面

「panic 不许出去」这句话，光看代码感觉不到分量。写一个最小程序就能看见后果 —— 一个 `extern "system"` 函数，里面直接 panic：

```rust,no_run
// panic 想穿过 extern "system" 边界
extern "system" fn boom(_: u32) -> i32 {
    panic!("回调里 panic 了");
}

fn main() {
    let handler: extern "system" fn(u32) -> i32 = boom;
    println!("调用前");
    let code = handler(0);
    println!("调用后：{code}");
}
```

标 `no_run` 是必须的：它一跑就中止进程，不能当测试。真跑起来是这样（线程号每次不同；中间整整一段强制打印的栈回溯已删 —— 本机实测 36 帧）：

```console
$ rustc --edition 2024 main.rs -o panic-demo.exe
$ .\panic-demo.exe
调用前

thread 'main' (17876) panicked at main.rs:3:5:
回调里 panic 了
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' (17876) panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:225:5:
panic in a function that cannot unwind
…（此处省略整段栈回溯：本机实测 36 帧）
thread caused non-unwinding panic. aborting.
```

第二段「panic in a function that cannot unwind」是运行时的第二道 panic：它发现这次展开要跨出 `extern "system"` 帧，于是直接 `abort`。注意第一段 panic 消息照常打印了 —— **`catch_unwind` 影响的是「能不能接住」，不是「打不打印」**，panic hook 该响还是响。修法只有一个：把可能 panic 的代码包进 `catch_unwind`，位置就在这个函数体内 ——

```rust
// 在 extern "system" 函数体里就地兜住 panic
use std::panic::catch_unwind;

extern "system" fn guarded(_: u32) -> i32 {
    match catch_unwind(|| panic!("回调里 panic 了")) {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

fn main() {
    let handler: extern "system" fn(u32) -> i32 = guarded;
    println!("调用前");
    let code = handler(0);
    println!("调用后：{code}（进程还活着）");
}
```

```console
$ rustc --edition 2024 main.rs -o guarded-demo.exe
$ .\guarded-demo.exe
调用前

thread 'main' (33436) panicked at main.rs:5:27:
回调里 panic 了
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
调用后：0（进程还活着）
```

`catch_unwind` 要求闭包实现 `UnwindSafe`。这个闭包什么都没捕获（`CtrlEvent` 是 `Copy`，`HANDLER` 是 `static`，静态项不进捕获列表），所以不需要 `AssertUnwindSafe`；只有捕获了 `&mut` 之类「跨 panic 继续用有风险」的东西时才需要显式表态。crate 里那个真正的 `trampoline` 也是这样。

**这块用到第 19 章的 `Mutex` 与锁中毒、`Send` 约束、全局状态、第 23 章的「panic 与 unwind」、第 26 章的 FFI 边界与三条防线、第 17 章的 `Drop`（句柄即注销）。**

### 5. 公开面：`src/lib.rs`

最后把所有东西收在几行里：

```rust,ignore
{{#include ../examples/sys-probe/src/lib.rs:31:38}}
```

四个内部模块，三个 `pub use` 决定外面能看到什么：`ctrl` 的事件类型与句柄、`error` 的错误与 `Result` 别名、`file_time` 的 `FileTime`。函数本体都是一行转发：

```rust,ignore
{{#include ../examples/sys-probe/src/lib.rs:40:58}}
```

`uptime_millis` 的文档里写了分辨率（10–16 ms）与那句「不适合当高精度计时器」—— 这是 API 的一部分：用户拿它去测微秒级耗时，就是被文档放过去的。两个合成函数也一样薄：

```rust,ignore
{{#include ../examples/sys-probe/src/lib.rs:76:87}}
```

`unix_time_millis` 没有自己的系统调用，它是 `system_file_time` 加一次换算 —— 换算逻辑只有一处实现，`system_file_time` 的调用者也能直接用。

**这块用到第 8 章的模块与 `pub use`（公开面即承诺）、第 18 章的文档即接口（`# 错误` 段把每种失败写清楚）。**

### 概念落点对照

| 章 | 概念 | 在本项目里的落点 |
|---|---|---|
| 17 | `Box`、`Drop`（析构） | `Box<dyn FnMut…>` 放进槽位；`CtrlGuard` 的 `Drop` 即注销 |
| 18 | 错误设计、`#[non_exhaustive]`、`source` | `ProbeError` 四变体、`Result` 别名、`Os` 的原因链 |
| 19 | `Mutex`、锁中毒、`Send`、全局状态 | `static HANDLER`；`unwrap_or_else(\|p\| p.into_inner())`；闭包的 `Send + 'static` |
| 23 | `unsafe`、`unsafe fn` 契约、`# Safety`、UB | `FileTime::read_raw` 的 `# Safety`；每个 `unsafe` 块的 `// SAFETY:`；未知码不用枚举 |
| 24 | 裸指针、`MaybeUninit`、`assume_init` | `GetSystemTimeAsFileTime(out.as_mut_ptr())` 与随后的 `assume_init`；`ptr.read()` |
| 25 | `#[repr(C)]`、`offset_of!`、布局断言 | `FileTime` 的布局；`const _` 里的四条断言 |
| 26 | `extern "system"`、`#[link]`、ABI、回调、panic 边界 | 整个 `platform/windows.rs`；`trampoline` 的 `catch_unwind`；注册/注销配对 |
| 7 | `Result`、`?`、错误码 | 包装层里「调用 → 判返回值 → 取错误码」的顺序 |

## 报错现场

### 报错一：调用 FFI 函数忘了 `unsafe` 块（E0133）

FFI 声明写好了，顺手在普通函数里调用：

```rust,compile_fail
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetTickCount64() -> u64;
}

fn uptime_millis() -> u64 {
    GetTickCount64()
}

fn main() {
    println!("{}", uptime_millis());
}
```

以 Rust 1.98 为例，关键几行：

```text
error[E0133]: call to unsafe function `GetTickCount64` is unsafe and requires unsafe block
 --> main.rs:7:5
  |
7 |     GetTickCount64()
  |     ^^^^^^^^^^^^^^^^ call to unsafe function
  |
  = note: consult the function's documentation for information on how to avoid undefined behavior
```

定位路径：`extern` 块里的函数默认是 `unsafe fn`（第 26 章：块里的项默认 unsafe，除非写 `safe`），调用它必须写在 `unsafe` 块里。项目里的修法是把调用包起来，并在块上写清依据：

```rust,ignore
// SAFETY: 没有参数、没有失败路径，Win32 文档也没有列出任何前置条件；
// 可以在任意线程调用。
Ok(unsafe { GetTickCount64() })
```

顺带一条 2024 的变化：`unsafe fn` 内部的 unsafe 操作也要显式包块（`unsafe_op_in_unsafe_fn` 默认警告，第 33 章）。所以本项目的 `unsafe fn read_raw` 里写的是 `unsafe { ptr.read() }`，而不是把整个函数体丢进一个 `unsafe` 块 —— 危险的那一行，一眼能找出来。

### 报错二：`extern` 块忘了写 `unsafe`（2024）

从旧代码抄来的声明块：

```rust,compile_fail
extern "system" {
    fn GetTickCount64() -> u64;
}

fn main() {
    // SAFETY: 没有参数、没有失败路径。
    println!("{}", unsafe { GetTickCount64() });
}
```

```text
error: extern blocks must be unsafe
 --> main.rs:1:1
  |
1 | / extern "system" {
2 | |     fn GetTickCount64() -> u64;
3 | | }
  | |_^
```

定位路径：`extern` 块里每一条声明都是「我对编译器的一句保证」—— 签名错了、ABI 写错了，编译器没法核对，后果是 UB。2024 版要求把这件事写在关键字上（1.82 起各 edition 都支持这个写法），修法就是加一个词：

```rust
#[cfg_attr(windows, link(name = "kernel32"))]
unsafe extern "system" {
    fn GetTickCount64() -> u64;
}
```

（`#[link]` 不参与编译期的检查，它只告诉链接器去哪里找符号；库名写错要到链接阶段才失败，报的是找不到输入文件。这一块把 `#[link]` 用 `#[cfg_attr(windows, …)]` 收在 Windows 上：本书的 CI 在 Linux 上跑 `mdbook test`，不这么收的话链接器会去找并不存在的 `libkernel32`。）

### 报错三：动了布局，布局断言先炸（E0080）

假设给 `FileTime` 加了一个字段，却没注意那两个 32 位字段的布局合同：

```rust,compile_fail
use std::mem::size_of;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileTime {
    low: u32,
    high: u32,
    // 多加一个字段：布局就变了
    extra: u32,
}

const _: () = {
    assert!(size_of::<FileTime>() == 8);
};

fn main() {
    println!("{}", size_of::<FileTime>());
}
```

```text
error[E0080]: evaluation panicked: assertion failed: size_of::<FileTime>() == 8
  --> main.rs:13:5
   |
13 |     assert!(size_of::<FileTime>() == 8);
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ evaluation of `_` failed here
```

定位路径：这不是运行时检查，是编译期求值失败 —— `const _` 块在编译期就要算出结果。修法有二：布局确实要变（那就同时改 `size_of` / `align_of` / `offset_of!` 四条断言，并重新核对 Windows 那边对应的结构体）；或者根本不该加这个字段（那就删掉）。不管哪条，**报错发生在编译期**，而不是某天在客户机器上读出一个错值。

## 测试

测试分三层，各管一段。单元测试在各模块的 `#[cfg(test)] mod tests` 里，测的是内部逻辑；`examples/sys-probe/tests/probe.rs` 是集成测试，只用公开 API；每个文档示例又是一层。

先看被公开面挡住的那部分：Windows 上的真值检查与降级分支各写一遍，`cfg` 让它们各跑一个平台。

```rust,ignore
{{#include ../examples/sys-probe/tests/probe.rs:33:44}}
```

```rust,ignore
{{#include ../examples/sys-probe/tests/probe.rs:74:92}}
```

第一段测的是开机时间：`>=` 固定住「不倒退」，10 年的上界是「这个值不像瞎编的」—— 这类断言要给宽到不可能误伤，只在该坏的时候坏。第二段是注册 → 拒绝第二个 → 释放 → 再注册：整条生命周期走一遍真的 `SetConsoleCtrlHandler`。

跑一遍的样子（在 `examples/` 里跑可省 `--manifest-path`）：

```console
$ cargo test --manifest-path examples/Cargo.toml -p sys-probe

running 12 tests
test ctrl::tests::unknown_codes_are_kept_verbatim ... ok
test ctrl::tests::the_handler_sees_the_raw_code_and_its_answer_is_reported ... ok
test ctrl::tests::a_panicking_handler_is_contained_and_the_slot_still_works ... ok
test error::tests::other_variants_have_no_source ... ok
test error::tests::os_error_keeps_the_code_in_the_message_and_in_the_source_chain ... ok
test ctrl::tests::with_no_handler_the_event_is_reported_as_unhandled ... ok
test file_time::tests::before_the_epoch_is_an_error_not_a_wrap ... ok
test file_time::tests::fields_round_trip_through_ticks ... ok
test file_time::tests::read_raw_copies_the_value_behind_the_pointer ... ok
test file_time::tests::sub_millisecond_remainder_is_truncated ... ok
test file_time::tests::unix_epoch_is_zero_milliseconds ... ok
test error::tests::display_says_which_api_is_missing ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 6 tests
test file_time_arithmetic_is_platform_independent ... ok
test windows::ctrl_handler_lifecycle_works_end_to_end ... ok
test unknown_ctrl_codes_are_preserved ... ok
test windows::unix_time_millis_agrees_with_the_standard_library ... ok
test windows::uptime_is_plausible_and_monotonic ... ok
test windows::both_time_apis_describe_the_same_instant ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 9 tests
test sys-probe\src\lib.rs - system_file_time (line 65) - compile ... ok
test sys-probe\src\lib.rs - unix_time_millis (line 78) - compile ... ok
test sys-probe\src\lib.rs - uptime_millis (line 46) - compile ... ok
test sys-probe\src\file_time.rs - file_time::FileTime (line 15) ... ok
test sys-probe\src\lib.rs - (line 18) ... ok
test sys-probe\src\file_time.rs - file_time::FileTime::from_ticks (line 45) ... ok
test sys-probe\src\file_time.rs - file_time::FileTime::unix_millis (line 68) ... ok
test sys-probe\src\ctrl.rs - ctrl::watch_ctrl_events (line 124) ... ok
test sys-probe\src\file_time.rs - file_time::FileTime::read_raw (line 94) ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

（取自一次真跑；libtest 并行跑用例，顺序每次可能不同。9 个文档测试里有 3 个是 `no_run`，名字后面标着 `compile` —— 只编译不执行，因为它们的返回值随平台而异。合计：单元 12、集成 6、文档 9。）

三类路径都有覆盖：

- **正常路径**：`file_time_arithmetic_is_platform_independent`、`the_handler_sees_the_raw_code_and_its_answer_is_reported`（回调拿到的码、返回值映射成 1 / 0）、`ctrl_handler_lifecycle_works_end_to_end`（真调 `SetConsoleCtrlHandler`：注册 → 拒绝第二个 → 注销 → 再注册）；
- **边界**：1970 年前的时间戳、不到 1 ms 的余数被截掉、`u64::MAX` 的拆分与拼回、未知事件码（7、9）原样保留、空槽位回报「没处理」；
- **错误路径**：`a_panicking_handler_is_contained_and_the_slot_still_works`（panic 被接住、之后仍能换处理器 —— 顺带覆盖了锁中毒的恢复）、非 Windows 上每个入口都回 `Unsupported` 且说得清缺哪个 API。

两个测试写法上的细节值得记住。一是**全局状态的测试要串行**：槽位只有一个，用例并行跑就会互相抢，所以碰到它的用例先 `TESTS` 锁一下（`tests` 模块里那个 `static TESTS: StdMutex<()>`）。二是**崩溃类行为的验证要把 hook 静音**：`catch_unwind` 接得住 panic，但 panic hook 照样往 stderr 打一行，测试里临时 `take_hook` / `set_hook` 换掉，输出才不被打乱 —— 这也顺带证明了 hook 与捕获是两件事。

调试时两个顺手的手段：`cargo test -- --nocapture` 看测试里的打印；`cargo run --example probe` 直接把三个 API 跑一遍看真实数值。

## 收尾：用法、错误与下一步

### 用法

三个入口就这么多：

```rust,ignore
use sys_probe::{watch_ctrl_events, CtrlEvent, ProbeError};

fn main() -> Result<(), ProbeError> {
    println!("开机 {} ms", sys_probe::uptime_millis()?);
    println!("Unix 毫秒 {}", sys_probe::unix_time_millis()?);

    let _guard = watch_ctrl_events(|event| {
        println!("收到 {event}");
        event == CtrlEvent::CTRL_C // 只认 Ctrl+C，其余交给下一个处理器
    })?;
    Ok(())
}
```

示例程序 `examples/probe.rs` 把这三件事串起来跑（输出见本章开头）。它同时演示了错误路径：注册第二个处理器会被拒。

### 错误路径

四种失败各有各的话，全部来自 `Display`：

- `Unsupported`：`GetTickCount64 在当前平台上不可用` —— 非 Windows 上每个入口都长这样，`api` 字段说得出缺的是谁；
- `Os`：`SetConsoleCtrlHandler 调用失败（Win32 错误码 6）` —— 错误码在消息里，系统消息挂在 `source()` 上；
- `BeforeUnixEpoch`：`时间戳 12 早于 Unix 纪元（1970-01-01 UTC），换算不出 Unix 毫秒` —— 换不出来的值不硬凑；
- `HandlerInUse`：`本进程已经注册过一个控制事件处理器` —— 示例程序里真出现过的那条（原文如下）。

```text
第二次注册       ：被拒（本进程已经注册过一个控制事件处理器）
```

这些文案由单元测试钉住（`error::tests` 与 `ctrl::tests` 里各有用例），改文案会先弄响测试。`Display` 不印系统消息是刻意的：消息随系统语言变，错误码不变。

### 怎么继续扩展

想接着练的话，几个由易到难的方向：

1. **再加一个 API**：`GetSystemTimePreciseAsFileTime` 与现在这个只差一行声明，但因为它的「精确性」承诺依赖系统版本，正好练一遍「签名与语义都写进文档」；
2. **把回调能力补全**：现在只留一个处理器（想组合就在自己的闭包里组合）；要支持多个处理器，就得把槽位换成 `Vec<Box<dyn FnMut…>>`，并想清楚调用顺序与「谁负责注销」—— 这正是文档里 last-registered, first-called 那条规则要落地的地方；
3. **换成现成的绑定 crate**：把 `platform/windows.rs` 里的手写声明换成 `windows-sys`，其余代码一行不用改 —— 这一步能验证「平台层是唯一的分叉点」这个设计是否真的成立；
4. **真按一次 Ctrl+C**：在处理器里做清理（`flush` 输出、删临时文件），然后对比「处理器返回 TRUE」与「返回 FALSE」两种行为的差别（后者会走到默认处理器，进程结束）。

完整代码在 `examples/sys-probe/`，跑法只有一句：`cargo test --manifest-path examples/Cargo.toml -p sys-probe`。

## 小结

- 平台的差异只允许出现在一层：上层只认一组签名，`cfg` 只出现在 `platform.rs` 这一个文件里（windows / not(windows) 两处判断）；签名由最弱的那一侧（非 Windows）决定，所以每个入口都返回 `Result`。
- `unsafe` 只落在两个文件里（`platform/windows.rs` 调系统 API、`file_time.rs` 读裸指针），`error.rs`、`ctrl.rs`、`platform/unsupported.rs` 用 `#![forbid(unsafe_code)]` 钉死；公开面上唯一那个 `unsafe fn` 把契约写在 `# Safety` 段里，契约检查不了的部分交给调用方。
- 跨 FFI 的结构体必须 `#[repr(C)]`，而「布局没变」这件事不靠记忆：`size_of` / `align_of` / `offset_of!` 四条断言放在 `const` 块里，编译期就守住。
- 出参用 `MaybeUninit` + 裸指针，等到函数写满再 `assume_init`；这个写法成立的前提是「函数保证写满」，写进 `SAFETY` 注释里才算数。
- 回调是本章的重心：闭包没有上下文参数，只能住进全局槽位（`Mutex<Option<Handler>>`，`Handler = Box<dyn FnMut… + Send>`）；注册与注销必须配对 —— 注销要传**当初注册的那个函数指针**（传 NULL 只改「忽略 Ctrl+C」这个进程属性，摘不掉处理器），句柄的 `Drop` 是配对点。
- panic 绝不能穿过 `extern "system"` 边界：越过去就是进程 abort。`catch_unwind` 要放在回调函数体里 —— 边界内侧 —— 并且它不影响 panic hook 的打印。

## 练习

1. `uptime_millis()` 在 Windows 上永远不会失败（`GetTickCount64` 没有失败路径），可它的签名仍然是 `Result<u64>`。为什么？如果这是一个只服务 Windows 的程序，你会怎么改签名？两种选择的代价分别是什么？
2. 有人建议把 `CtrlEvent` 写成 `#[repr(u32)] enum CtrlEvent { CtrlC = 0, CtrlBreak = 1, CtrlClose = 2, CtrlLogoff = 5, CtrlShutdown = 6 }`，进来时 `unsafe { transmute::<u32, CtrlEvent>(raw) }`。这样写会有什么问题？现在这个新类型版本在哪些地方更安全、哪些地方更麻烦？
3. 为什么 `catch_unwind` 必须写在 `trampoline`（那个 `extern "system" fn`）的函数体里，而不能写在注册它的 `watch_ctrl_events` 里？

<details><summary>参考答案</summary>

**1.** 这个 crate 是跨平台的：非 Windows 上根本不存在 `GetTickCount64`，而「不存在」这件事必须能被表达出来，`Result` 里的 `Unsupported` 就是它的载体。一个函数的签名要对**所有**编译目标成立，不能只在当前平台成立 —— 这是「签名由最弱的一侧决定」的实例。如果只服务 Windows，可以改成 `fn uptime_millis() -> u64`：调用点少一个 `?`，文档也不用解释平台差异。代价是不能再为别的平台编译：一旦有人想在 Linux 上跑测试或发布，就得把这层 API 拆开，而下游所有调用点都要跟着改。本项目选前者，因为「能在两种平台上跑同一批测试」是它存在的前提之一（Linux CI 是它唯一的第二双眼睛）。

**2.** 问题出在「枚举的每个值都必须有效」这条规则上：`#[repr(u32)]` 枚举能取值域里的 5 个码，而系统完全可能送来别的码（新版本 Windows 新增事件、或者第三方库发来的自定义信号）。`transmute` 本身不会当场炸，但它**产生了一个无效值** —— 而 Reference 把这件事定成立即构成 UB（第 23 章逐条列过那份清单）：从那一刻起，程序失去全部保证，后面观察到的任何行为都只是运气。这也是为什么这种 bug 常常在离现场很远的地方爆出来。新类型版本存原始数字：未知码原样保留、`Display` 如实报「未知控制事件（码 7）」，而且任何 `u32` 都是这个类型的合法值，「构造无效值」这件事根本不存在。麻烦的地方有两条：`match` 的分支必须写全路径（关联常量不能 glob 导入 —— `use CtrlEvent::*` 会报 E0432，因为 `CtrlEvent` 是结构体不是模块）；新类型也没有穷尽性检查，`_` 分支永远省不掉，编译器不会提醒你「漏了哪个已知码」。想要穷尽性就得改成带 `Unknown(u32)` 变体的枚举 —— 那等于承认枚举在这里只是「带名字的数字」，而这正是新类型在做的事。

**3.** 因为 panic 发生时，`watch_ctrl_events` 早就返回了，栈上只剩系统那条调用链。系统在自己的线程上调用 `trampoline`，`trampoline` 调用用户闭包；panic 只能在这条链上被接住，而这条链上唯一的 Rust 帧就是 `trampoline` —— 再往外就是 `extern "system"` 边界，越过去进程就 abort，外层无论包多少层 `catch_unwind` 都没有机会执行（第 26 章：事实二）。这也是判别「该把 `catch_unwind` 放哪」的通则：**它必须包住会 panic 的那段代码本身，而不是包住「提交这段代码」的那个调用**。

</details>

## 延伸阅读

- Rust Reference《External blocks》（`unsafe extern`、`#[link]`、ABI 字符串）：<https://doc.rust-lang.org/reference/items/external-blocks.html>
- Rust Reference《Type layout》（`#[repr(C)]`、对齐与填充）：<https://doc.rust-lang.org/reference/type-layout.html>
- Nomicon《FFI》（回调、panic 与 unwind、不透明句柄）：<https://doc.rust-lang.org/nomicon/ffi.html>
- `std::panic::catch_unwind`（含 `UnwindSafe` 约束）：<https://doc.rust-lang.org/std/panic/fn.catch_unwind.html>
- `std::mem::MaybeUninit`（未初始化内存的合法用法）：<https://doc.rust-lang.org/std/mem/union.MaybeUninit.html>
- `std::error::Error::source`（原因链）：<https://doc.rust-lang.org/std/error/trait.Error.html>
- Windows API：`GetTickCount64`：<https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64>
- Windows API：`GetSystemTimeAsFileTime`：<https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemtimeasfiletime>
- Windows API：`FILETIME` 结构体：<https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime>
- Windows API：`SetConsoleCtrlHandler`：<https://learn.microsoft.com/en-us/windows/console/setconsolectrlhandler>
- Windows API：`HandlerRoutine` 回调（事件码与超时）：<https://learn.microsoft.com/en-us/windows/console/handlerroutine>
