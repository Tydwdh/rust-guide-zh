# 57 项目 4：封装系统 API（FFI 与 unsafe）

Type: task
Status: resolved

## Question

写第四篇收尾项目：**封装系统 API**（`examples/sys-probe/` 已建 stub；crate 名 `sys-probe`；作者替换 stub）。

- **目标**：把 Windows 系统 API 安全封装成零依赖库：`extern "system"` + `#[link(name = "kernel32")]`（如 `GetTickCount64`；可选加 `GetSystemTimeAsFileTime` 走 `#[repr(C)]` 结构体 + 裸指针路径）；对外只暴露安全 API（如 `uptime_millis() -> Result<u64, ProbeError>`）；覆盖 ch23–26 概念并让读者看见取舍：`unsafe fn` 的契约与 `# Safety` 文档、`unsafe {}` 块的最小化、`#[repr(C)]` 布局契约、FFI 边界与 panic（`catch_unwind` 必要性说明）、错误设计。
- **跨平台**：`#[cfg(windows)]` 走真 FFI；`#[cfg(not(windows))]` 提供 `Err(ProbeError::Unsupported)` 降级分支并配测试——保证 `cargo test` 在 Linux CI 也能编译/通过（本机只实测 Windows 路径，票里注明）。
- **章结构**：对齐项目 1 成稿（逐块实现 / 设计取舍 / 报错现场 / 测试 / 小结 / 练习 2–3 题带答案）；正文引 crate 真实代码；Windows 实跑输出真粘。
- **素材**：已定稿 ch23–26 与项目 1；Windows API 须引一手文档（learn.microsoft.com）；本机 1.98 实测。
- **DoD**：§8 —— 独占 `src/project4-ffi-unsafe.md` + `examples/sys-probe/**`（含其 `Cargo.toml`）+ 本票；禁 `git`/禁 build；自验：`CARGO_TARGET_DIR=.scratch/tmp-57/target cargo test --manifest-path examples/Cargo.toml -p sys-probe` 全绿 + `mdbook test -c project4-ffi-unsafe.md` exit 0；探针 `.scratch/tmp-57/`；**勿动** `examples/Cargo.toml`（members 已配）、`examples/README.md`、其他 crate；收尾 `claimed` + `## Comments`。

## Comments

**文件（只碰票内列出的文件）**

- 章 `src/project4-ffi-unsafe.md`：保留占位 H1「项目 4：封装系统 API（FFI 与 unsafe）」，715 行；2 张内嵌 SVG；24 条 `{{#include}}`（全部写在围栏内，awk 逐行核过）；章内结构照项目 1：要做的东西（表 + 图 + 真跑输出）→ 验收标准 → 设计取舍（7 条，每条给「为什么不选另一个」）→ 逐块实现（5 块 + 概念落点对照表）→ 报错现场（3 条编译错误）→ 测试 → 收尾 → 小结 → 练习（3 题带答案）→ 延伸阅读。
- 代码 `examples/sys-probe/`（968 行）：`Cargo.toml`（无 `[dependencies]`）+ `src/{lib,error,file_time,ctrl,platform}.rs` + `src/platform/{windows,unsupported}.rs` + `tests/probe.rs` + `examples/probe.rs`。
- 未动 `examples/Cargo.toml`、`examples/README.md`、其它 crate、SUMMARY、book.toml；`examples/Cargo.lock` 尺寸/mtime 未变（443 B / 02:02）。

**实现要点（与票的逐条对应）**

- 三个 API 难度递增：`GetTickCount64`（值返回）→ `GetSystemTimeAsFileTime`（出参结构体：`MaybeUninit::uninit()` + `as_mut_ptr()` 传给 `*mut FileTime`，函数写满后再 `assume_init`）→ `SetConsoleCtrlHandler`（回调：系统新线程、全局槽位、panic 边界）。声明一律 `#[link(name = "kernel32")] unsafe extern "system"`，签名手写。
- 公开面：`uptime_millis` / `system_file_time` / `unix_time_millis` / `watch_ctrl_events` 四个安全函数 + `FileTime`（`#[repr(C)]`、字段私有、`from_ticks`/`ticks`/`unix_millis`，以及唯一 `unsafe fn read_raw` 带 `# Safety` 段）+ `CtrlEvent`（newtype + 5 个关联常量，**不做 `#[repr(u32)]` 枚举**，避免「任意 u32 → 枚举」产生无效值）+ `CtrlGuard`（`Drop` 即注销）+ `ProbeError` / `Result`。
- 跨平台：差异只在 `src/platform.rs` 这一个文件里（windows / not(windows) 两处判断，选 `platform/windows.rs` 或 `platform/unsupported.rs`，同名同签名）；降级实现零 unsafe、全部 `Err(ProbeError::Unsupported { api })`；签名由最弱一侧决定（Windows 上不可能失败的 `uptime_millis` 也返回 `Result`）。
- `unsafe` 最小化：只落在 `platform/windows.rs`（调系统 API 的块）与 `file_time.rs`（那次 `ptr.read()`）两个文件里；`error.rs`、`ctrl.rs`、`platform/unsupported.rs` 顶部 `#![forbid(unsafe_code)]`；crate 根 `#![deny(unsafe_op_in_unsafe_fn)]`（2024 起本就是 warn）+ `#![warn(missing_docs)]`；每个 `unsafe {}` 块配 `// SAFETY:` 说明。
- 布局契约：`const _` 里 4 条断言（size 8 / align 4 / `offset_of!(low)`、`offset_of!(high)`），编译期守住「和 Windows FILETIME 对得上」。
- panic 边界：`trampoline`（`extern "system" fn(u32) -> i32`）里 `catch_unwind` 就地兜住用户闭包，转成 TRUE/FALSE；槽位锁中毒用 `unwrap_or_else(into_inner)` 恢复；`CtrlGuard::drop` 传**同一个函数指针**去注销（传 NULL 只是改「忽略 Ctrl+C」属性，不是摘处理器 —— 见文末 F1 的真信号验证）、再清槽位，Drop 里不 panic。
- 错误设计：`#[non_exhaustive] enum ProbeError { Unsupported{api}, Os{api, code, source: io::Error}, BeforeUnixEpoch{ticks}, HandlerInUse }`；`Display` 只印错误码（不印随系统语言变的系统消息），系统消息走 `source()` 原因链。

**自验（全绿）**

1. `CARGO_TARGET_DIR=.scratch/tmp-57/target cargo test --manifest-path examples/Cargo.toml -p sys-probe` → exit 0：单元 12 + 集成 6 + 文档 9（其中 3 个 `no_run`，libtest 记为 `compile`）全过。Windows 真路径在集成测试里跑过：真调 `SetConsoleCtrlHandler`（注册 → 拒绝第二个 → 注销 → 再注册）、`unix_time_millis` 与 `SystemTime::now()` 差 < 2 s、uptime 单调。**这条集成测试只能观察到槽位与调用成败**；「系统那边确实把处理器摘掉了」用真信号探针单独验过（见文末「审校回执处理」F1）。
2. `mdbook test -c project4-ffi-unsafe.md` → exit 0。敏感性抽验：临时追加一个故意编不过的 `rust` 块 → `FAILED. 6 passed; 1 failed; 26 ignored`、exit 101（证明章内块真被编译、不是假绿），随后还原。
3. `cargo check --target x86_64-linux-android --all-targets` → 通过：非 Windows 分支连同它的单元测试与集成测试一起编译（本机装了 android target、没装 linux-gnu；运行结果留给 Linux CI）。
4. `cargo clippy -p sys-probe --all-targets` → 0 warning（首轮报 `clippy::type_complexity`，为此给槽位类型加了 `type Handler = Box<dyn FnMut(CtrlEvent) -> bool + Send>`）；`cargo fmt -p sys-probe -- --check` 干净；`cargo build --all-targets` 0 warning。
5. §9 SVG 自查（2 张图）：块内 0 空行；无 `<script>`/`xlink`/`url(`；svg 根有 `viewBox` + `width="100%"`、无 `height`（`height=` 的命中全来自 `<rect>`）；颜色只用 `currentColor`/`var(--links)`/`var(--warning-border)`/`none`；字号 12–14；`role="img"` + `aria-label`；`&`/`<`/`>` 已实体化。栅格化目视（`.scratch/tmp-57/fig*-render.svg`，`var()` 换字面量 `#20609f`/`#ff8e00`，同票 60 的口径）—— 第一轮发现底部文字出血与框线相撞，已重排坐标后复查通过（图 2 的 `catch_unwind` 标签与第一个内框上沿贴得较紧，见遗留 5）。
6. 24 条 `{{#include}}` 的起止行逐条打印核对，全部落在 doc 注释/项首与闭合 `}` 上。

**实测输出清单（都已粘进正文）**

- example 真跑（`cargo run … -p sys-probe --example probe`）：`开机时长 195525000 ms` / `FILETIME 134368207705187259 个 100 纳秒` / `其中的 Unix 毫秒 1792347170518` / `std::time 对照 1792347170519 ms` / `已注册` / `第二次注册：被拒（本进程已经注册过一个控制事件处理器）` / `已注销`。
- `cargo test` 三段真跑（12 / 6 / 9 的用例名与 `test result` 行）。
- 报错现场三条最小复现各自 `rustc --edition 2024 main.rs` 真跑：E0133（`--> main.rs:7:5`）、`error: extern blocks must be unsafe`（`--> main.rs:1:1`）、E0080（`--> main.rs:13:5`，`evaluation panicked: assertion failed`）。
- panic 越界 abort 现场（`thread caused non-unwinding panic. aborting.` 前含 `panic in a function that cannot unwind`）与 `catch_unwind` 修法（`调用后：0（进程还活着）`）各跑一遍；正文注明省略约 40 行强制栈回溯、线程号每次不同。
- 旁证探针：`#[link(name = "kernel32.dll")]` → `LINK : fatal error LNK1181`；**未被引用的 extern 声明即使库名不存在也不链接**（rustc 会省略）——后者是「报错二里那个修法块标 `rust`（真编译）在非 Windows 上也安全」的依据。
- 文档链接核对：正文 6 条 Rust 侧链接在 1.98 本地快照里逐条存在；5 条 Windows API 链接均为本次抓取的 learn.microsoft.com 页面，引文逐字取自该页。
- 探针与复现文件（`.scratch/tmp-57/`：`target/`、`repro1|repro2|repro3/main.rs`、`abort|guard/main.rs`、`linkprobe/`、`matchprobe/`、`globprobe/`、`probe-book/`、图渲染件）已随目录删除。

**遗留不确定**

1. 非 Windows 分支只做了**编译检查**（`x86_64-linux-android`），没在本机跑过测试；「Linux CI 上 `cargo test` 全绿」是推断而非实测（正文已按此措辞写明）。若协调者能在 Linux 上跑一次，重点看 `platform/unsupported.rs` 的单元测试与 `tests/probe.rs` 的 `not_windows` 模块（各自断言 `Unsupported { api }` 的名字与结尾文案）。
2. panic 越界导致的进程中止，在 Windows 上实测被 `cmd /c` 报成退出码 9（与「abort → 3」的常见印象不符，未再深挖）；正文因此只写「进程立即中止（abort）」，不写退出码。
3. 控制事件注册要求进程附着控制台：无控制台的宿主里 `SetConsoleCtrlHandler` 会失败，`ctrl_handler_lifecycle_works_end_to_end` 与 example 会回 `ProbeError::Os`（本机 cmd/PowerShell 下一直成功，未加容错分支）。
4. 回调执行期间持有槽位锁：文档已写明「别在处理器里 drop 句柄或再注册」（会自锁），但没做可重入设计；这是「一个槽位」取舍的代价，正文已交代。
5. 图 2 的 `catch_unwind 包住这三个框` 标签贴着第一个内框上沿（目视不压线，但余量小）；若要更宽松需加高 viewBox 或把标签移到框外。
6. 正文报错现场的报错文本以「以 Rust 1.98 为例」标注；错误信息文案（尤其 `extern blocks must be unsafe` 无 help 提示）随 rustc 版本变，重跑时可能多出建议行。

**新增术语**：无。沿用 GLOSSARY 既有条目（FFI、ABI、调用约定、契约、裸指针、未定义行为（UB）、`Send`、锁中毒、RAII 相关表述）。正文出现的 `HandlerRoutine`/`PHANDLER_ROUTINE`/`FILETIME`/`CtrlGuard` 等是 Windows 头文件名或本项目标识符，不作术语收录。

**衔接提示**

- 正文交叉引用：第 7、8、9、13、17、18、19、23、24、25、26、33 章；对 ch26 26.6 的「三条防线」只写「第 26 章：事实二」，未复述其内容。
- 票 60 正在给 26.6 加「三条防线」图；若该图最终落在正文，本章 §4 里那句「换成一张图看得更清楚」可改成回指 26.6 的图（可选，不影响正确性）。
- 本章条目已由协调者接在 `src/SUMMARY.md` 第四篇末尾（未动 SUMMARY）。
- 协调者收尾可跑：`cargo run --manifest-path examples/Cargo.toml -p sys-probe --example probe`（真值随运行时刻变，输出格式与正文一致）。

**图示需求**：本章自带 2 张（三层结构 / 控制事件调用链），暂不需追加；若后续要「注册—回调—注销」的时序图，可从 §4 的 `CtrlGuard::drop` 段扩展。

---

## 审校回执处理（RwProj4 交叉审校：1 blocker + 2 major + 6 minor，逐条已改）

**F1（blocker，已修）· `CtrlGuard::drop` 没有真的注销。** 原实现 `SetConsoleCtrlHandler(None, false)`；Win32 文档里 `HandlerRoutine` 为 NULL 的那段讲的是「让进程忽略 / 恢复正常处理 Ctrl+C」这个属性，**不是**摘处理器。修法：`platform::set_console_ctrl_handler` 的形参从 `Option<HandlerRoutine>` 收成必填的 `HandlerRoutine`（内部仍以 `Some(handler)` 传给原始声明，`Option` 只留在原始声明里），注册与注销都传 `trampoline` —— 注销传同一指针这件事在签名层面就不给踩空的机会。**独立验证**（`.scratch/tmp-57/probe-ctrl/`，已随目录删除；父子进程，子进程用 `CREATE_NEW_PROCESS_GROUP` 单独成组，父进程发 `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, 子进程 PID)`）：传 NULL 时 `remove_ret=1`（调用「成功」）但真信号一到处理器照样被调用（`calls=1`）；传同一指针时 `calls=` 那行根本来不及打印，子进程以 `0xc000013a`（`STATUS_CONTROL_C_EXIT`）退出。正文 §4 已补这段真跑对照与「为什么形参不带 `Option`」的解说；`ctrl.rs` 注释同步；`tests/probe.rs` 的 lifecycle 用例补了边界说明（本用例只覆盖槽位与调用成败，真注销要发真信号，不放进测试）。票面此前「真注册/注销」的说法已更正。
**F2（major，已修）· `unsafe` 分布说法与代码不符。** 实际是**两个文件**（`platform/windows.rs` 调系统 API 的块、`file_time.rs` 里那次 `ptr.read()`），`#![forbid(unsafe_code)]` 落在 `error.rs` / `ctrl.rs` / `platform/unsupported.rs` **三个模块**（`file_time.rs` 不能 forbid，因为 `read_raw` 就在那儿）。正文 :100、:116、:693（小结）与 `lib.rs` crate 文档、`Cargo.toml` 注释、票面均改为这个口径。
**F3（major，已修）· 图 2 虚线框框错了对象。** 原图把 `trampoline` 也框进「catch_unwind」，视觉与代码相反。现改为虚线框只框「HANDLER 槽位 + 你的闭包」两框（`x=342 w=370`），标注改成「这两步在 trampoline 体内，由 catch_unwind 包住」，`aria-label` 同步；正文「回调链上有三个框」改为「四个框」。
**F4（minor，已修）· doctest 行号过期。** 重跑 `cargo test -p sys-probe` 后把单元 / 集成 / 文档三段输出整段重贴，行号刷新为 `lib.rs 46/65/78/18`、`ctrl.rs 124`、`file_time.rs 15/45/68/94`。
**F5（minor，已修）· 「会看到空槽」与代码不符。** 注册整段持锁，事件只会阻塞到闭包就位。正文与 `ctrl.rs` 注释改为「整段都在同一把锁里：注册期间事件会先阻塞在这把锁上，等闭包装好才拿到它，不存在空档；先登记的理由是失败路径干净（不需要回滚）」。
**F6（minor，已修）· `cfg` 口径三处不一。** 统一为「只出现在 `src/platform.rs` 这一个文件里（windows / not(windows) 两处判断）」（正文 :114、平台层小节、小结，以及票面）。
**F7（minor，已修）· 两处回指落空。** `Mutex::new` 那句去掉章号（改述为「是 `const fn`，所以 `static … = Mutex::new(None)` 直接可用」）；「第 17 章的 RAII」两处改为「第 17 章的 `Drop`（析构）」。
**F8（minor，已修）· 图 1 的 `aria-label`。** 改为「上层是三个跨平台入口（unix_time_millis 由 system_file_time 组合出）」。
**F9（minor，已修）· 栈回溯行数。** 两处「约 40 行」改为「中间整整一段强制打印的栈回溯已删 —— 本机实测 36 帧」/「（此处省略整段栈回溯：本机实测 36 帧）」。

**修后复验（全绿，命令同票 57 口径）**

- `CARGO_TARGET_DIR=.scratch/tmp-57/target cargo test --manifest-path examples/Cargo.toml -p sys-probe` → exit 0：单元 12 / 集成 6 / 文档 9 全过。
- `cargo clippy -p sys-probe --all-targets` → 0 warning；`cargo fmt -p sys-probe -- --check` → 干净。
- `cargo check --target x86_64-linux-android --all-targets` → 通过。**这一步在改签名时抓到过一处真错**：`platform/unsupported.rs` 的单元测试还按旧签名传 `None`（E0308），已改成传一个 `extern "system" fn`（降级实现永不调用它）—— 这也是「非 Windows 分支要编译检查」的价值所在。
- `mdbook test -c project4-ffi-unsafe.md` → exit 0；24 条 `{{#include}}` 的起止行重做逐条核对（`windows.rs` 57:76、`ctrl.rs` 15:25 / 27:57 / 59:71 / 73:103 / 105:154 / 156:178、`lib.rs` 31:38 / 40:58 / 76:87、`tests/probe.rs` 74:92……全部落在 doc 注释/项首与闭合 `}` 上）；两张 SVG 块内 0 空行、颜色仍只用主题变量；图 2 重新栅格化目视通过（虚线框已只覆盖槽位与闭包两框）。
- 探针目录 `.scratch/tmp-57/`（含 `probe-ctrl/` 真信号探针与渲染件）已删除；`examples/Cargo.lock` 仍未变。

**修后遗留（相对首次交付的变化）**

1. 原「遗留不确定 1」（非 Windows 只做编译检查）仍成立，但现在多了一层保障：修 F1 时正是这条检查抓出了降级单测的签名不匹配。
2. 新增一条认知：控制事件**注销**的正确玩法是传同一函数指针；这对「多个处理器叠用」的扩展方向（见「怎么继续扩展 2」）更重要 —— 多槽位实现时每层注销都要各自传自己的指针。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/project4-ffi-unsafe.md` + `examples/sys-probe/`（零依赖：`GetTickCount64`/`GetSystemTimeAsFileTime`/`SetConsoleCtrlHandler` 封装；`#[cfg]` 只在 platform.rs；非 Windows 降级分支）。
- 自验：`cargo test -p sys-probe` 12 单测 + 6 集成 + 9 文档全绿；clippy 0 警告、fmt 干净；`x86_64-linux-android --all-targets` 编译通过；`mdbook test -c` exit 0；24 条 include 行段逐条核对。
- 交叉审校 RwProj4（1 blocker + 2 major + 6 minor）全修：**`CtrlGuard::drop` 真注销**（NULL/FALSE → 传同一函数指针，真信号对照修复；并修掉降级实现里的 E0308）、unsafe 分布口径、图 2 catch_unwind 包裹范围、doctest 行号重贴、持锁窗口叙述、cfg 口径、回指、图 1 aria-label、栈回溯行数。
- 术语并表：无新增。
- 全量链（批次级）全绿。
- 遗留：非 Windows 分支只编译未跑（需 Linux CI）；abort 退出码正文未写（规避）。
