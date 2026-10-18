# 44 第 26 章 FFI 与 ABI

Type: task
Status: claimed

## Question

写第四篇第四章：**FFI 与 ABI**。

- **范围**：`extern "C"`（2024 起 `unsafe extern` 块；未写 `unsafe` 是 2024 的 hard error 还是 lint —— 以实测为准）；`#[no_mangle]` → `#[unsafe(no_mangle)]`（2024）；C 字符串（`CStr`/`CString`、`c"..."` 字面量）；回调（`extern "C" fn`、跨 FFI 边界 `catch_unwind` 防 panic 展开）；`#[repr(C)]` 契约；`c_variadic` 状态（nightly 标注）。给一个**可跑的最小端到端例子**（环境允许就用 `extern "C"` + 自编译 C 或 `link`；若本机无 C 编译器，用「同进程 Rust 内 `extern "C" fn` 互调 + `#[unsafe(no_mangle)]` 导出符号」替代，并说明局限）。
- **素材**：`research/02-knowledge-structure.md` 的 I16 行（依赖 I15/I13；`nomicon/ffi.html`、`reference/items/external-blocks.html`、`reference/abi.html`、`std/ffi/`）；本机 1.98 快照核对 `CStr`/`CString` API。
- **边界**：UB 清单 → 第 23 章；布局/repr → 第 25 章（回指）；`no_std`/嵌入式 → 第 41 章（一句）。Windows 本机若有 `cl.exe`/`gcc` 可尝试真编译 C 片段（先探测，失败就退回纯 Rust 演示并在票里说明）。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；与 C 的引文须核一手文档（C 标准草案/CPython 文档等，别引二手）。
- **DoD**：§8 —— 独占 `src/ch26-ffi-abi.md` + `snippets/ch26/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch26-ffi-abi.md` 绿；探针 `.scratch/tmp-44/`；收尾 `claimed` + `## Comments`。**注意**：crate 级编译（若做独立 C 工程）也放 `.scratch/tmp-44/`，别落 `target/`。

## Comments

**状态：初稿完成，自验绿；等协调者交叉审校。**

### 文件
- `src/ch26-ffi-abi.md`（八节模板齐全，正文 4258 汉字，落在 4000–5500 区间）
- `snippets/ch26/callbacks.rs`（新）：进程内 `extern "C"` 回调 + `#[unsafe(no_mangle)]` 导出 + panic 三条防线
- `snippets/ch26/cstrings.rs`（新）：`CString`/`CStr`/`c"..."` 往返
- 未碰任何其他文件（SUMMARY/book.toml/WRITING/GLOSSARY/别章/别 crate 均未动；未跑 git）

### 自验
- `mdbook test -c ch26-ffi-abi.md` → **exit 0**（全绿）。
- 注入失败反证：临时在章末追加一个 `rust` 块（`let x: i32 = "这不是数字";`）→ `mdbook test -c ch26-ffi-abi.md` → **exit 101**，日志为 `ERROR rustdoc returned an error: ... error[E0308]: mismatched types` + `test result: FAILED. 13 passed; 1 failed; 7 ignored`；还原后重跑 → exit 0。
- **踩坑记录（给同批写作者）**：本机 mdbook 0.5.4 的 `test` 子命令**成功时不回放 rustdoc 输出**，只在失败时打印（源码 `mdbook-driver-0.5.4/src/mdbook.rs::test_chapter` 捕获 `output`，仅 `!status.success()` 时 `eprintln!`）。所以「绿」的表现是：只有一行 ` INFO Testing chapter ...` + exit 0，**没有** `test result: ok` 之类的行——别把「没输出」误判成「没测」。全章实跑是 13 passed / 7 ignored（交叉审校修订后为 **14 passed / 6 ignored**，见下「交叉审校修订」）。
- 章内被 `mdbook test` 真编译执行的可跑块：26.1 两条声明块、26.2 导出块、26.3 两条、26.4、26.5 两条、26.6 无、26.7 变参声明块、26.8 安全包装、报错现场两条 `compile_fail`、练习答案 2 的修正版；6 个 `rust,ignore` 块（`#[link]` 声明、C 端到端 main.rs、build.rs、练习 1 题面、练习答案 1、练习 2 题面——后两个若真跑会 abort，故必须 ignore）。

### 实测过的输出清单（命令 + 结果，均在 1.98.0 / x86_64-pc-windows-msvc）
1. **2024 硬错误实测**（本机 rustc 1.98.0）：`extern "C" {}` 未标 unsafe → `error: extern blocks must be unsafe`，**exit 1**（硬错误，不是 lint）；`#[no_mangle]` → `error: unsafe attribute used without unsafe` + `help: wrap the attribute in unsafe(...)`，**exit 1**。两段输出均按 `src/main.rs` 的真实路径原样粘进章内 `text` 块。
2. **`unsafe extern` 块内 `safe`/`unsafe` 限定符**（1.82 起）：`safe fn` / `unsafe fn` / 不写限定符三种写法编译通过。
3. **回调 + panic 三条防线**（`.scratch/tmp-44/probe/f.rs`，真跑）：
   - `drive(checked_div, 0)` 在 `extern "C" fn` 内部 `catch_unwind` → 返回 `-1`，进程存活；
   - panic 越过 `extern "C"` 边界 → stderr `panic in a function that cannot unwind` … `thread caused non-unwinding panic. aborting.`，退出码 **3221226505（0xC0000409）**；
   - 两侧都改 `extern "C-unwind"` 后，外层 `catch_unwind` 成功接住（`is_err() == true`），正常调用返回 25。
   - 「`catch_unwind` 放在外层抓不到」这一反例是实测出来的（`guarded` 版本直接 abort），章内 26.6 的三条事实即由此写。
4. **`unsafe_op_in_unsafe_fn`**：`unsafe extern "C" fn` 体内调用 unsafe fn → `warning[E0133]`（`#[warn(rust_2024_compatibility)]` on by default），确认 2024 是 **warn** 而非 deny。
5. **变参**：`unsafe extern "C" { fn c_vsum(count: i32, ...) -> i32; }` 声明在 stable 编译并可**真调用**（见第 6 条输出 `c_vsum(3, 10, 20, 12) = 42`）；`#![feature(c_variadic)]` → `error[E0554]: #![feature] may not be used on the stable release channel`。
6. **真 C 端到端（gcc 实测，非替代方案）**：本机 WinGet MinGW-W64 `gcc 16.1.0` + `ar`，`gcc -shared -O2 -o c26.dll c26.c -Wl,--out-implib,c26.lib` → MSVC 工具链的 rustc `--edition 2024 main.rs -L . -o demo.exe` 链接成功并真跑，输出（已粘入章内）：`c_add(20, 22) = 42` / `c_apply(triple, 7) = 21` / `c_dist2(Point {3, 4}) = 25` / `c_vsum(3, 10, 20, 12) = 42` / `c_strlen(CString) = 15` / `c_strlen(c"...") = 14`。**踩坑**：导入库文件名必须是 `c26.lib`（`#[link(name = "c26")]` 按名字找 `c26.lib`；叫 `libc26.lib` 会 `LNK1181`）。`cargo` 侧另跑通 `build.rs`（`gcc -c` + `ar rcs` 静态库 + `cargo::rustc-link-search/link-lib`），`cargo run -q` 输出与上面六行一致。
7. **导出符号名实测**：`rustc --crate-type=cdylib` 产出的 DLL 用 `objdump -p` 查导出表，符号名就是 `rust_add`（佐证 `no_mangle` 的语义与章内示例）。
8. **链接期错误实测**：把 `c_add` 写成 `c_ad` → `error: linking with link.exe failed: exit code: 1120`，`error LNK2019: 无法解析的外部符号 __imp_c_ad`（本机 link.exe 为中文消息，章内按实况粘并注明「随系统语言变化」）。
9. **本机 rustc 无 `printf`/`snprintf` 可链接符号**（MSVC UCRT 下 LNK2019），故变参演示用自编 C 库的 `c_vsum`，未用 libc 符号——章内没写这条，仅备忘。

### 一手核对（英文引文逐条对照 1.98 快照 `share/doc/rust/html/`）
- `reference/items/external-blocks.html`：`akin to unchecked imports`、`The unsafe keyword is semantically required…`、`A function declared in an extern block is implicitly unsafe unless the safe function qualifier is present.`、`The extern syntax without an explicit ABI is being phased out…`、ABI 字符串表、`#[link]` 的 kind（dylib/static/raw-dylib/framework）。
- `reference/abi.html`：`no_mangle` 的符号名语义与 unsafe 理由（`…may collide with another symbol with the same name (or with a well-known symbol), leading to undefined behavior.`）。
- `nomicon/ffi.html`：`panic will cause the process to safely abort.`、`A foreign exception entering Rust will cause undefined behavior.`、`Note that the interaction of catch_unwind with foreign exceptions is undefined…`、`Compiling with panic=abort will still cause panic! to immediately abort…`、空指针优化段。
- `edition-guide/rust-2024/unsafe-extern.html`、`unsafe-attributes.html`、`unsafe-op-in-unsafe-fn.html`：三条对应引文（含 `Rust 1.82 added the ability in all editions…`）。
- `std/ffi/index.html`、`struct.CStr.html`、`struct.VaList.html`：`CString/CStr` 定位与类比、`&CStr` 无布局保证的原话、`nightly-only experimental API (c_variadic #44930)`。
- 版本事实取自 `releases.md`：C 字符串字面量 1.77.0 稳定；`unsafe extern` 块 1.82.0 稳定（RFC 3484）；变参声明 1.91.0 扩到 `sysv64/win64/efiapi/aapcs`。（因此正文只写「1.77/1.82/1.91」这三个可核实版本号，不写 C ABI 变参声明的起始版本——research §4.2 备注 2 的要求。）
- C 侧无外部引文（只引 Rust 官方文档）；C 代码为本机 gcc 16.1.0 真编译，`gcc`/`ar` 行为属工具自身，未做跨语言标准断言。

### 新增术语（建议并入 GLOSSARY.md，我未改该文件）
- **外部函数接口（FFI，Foreign Function Interface）**：跨语言调用通道。_Avoid_：外部接口
- **应用二进制接口（ABI，application binary interface）**：调用约定 + 符号命名 + 布局的二进制层面约定。_Avoid_：二进制接口
- **调用约定（calling convention）**：参数/返回值的传递与栈清理规则。
- **符号修饰（symbol mangling）**：编译器把函数名变成唯一符号名的过程。
- **空指针优化（nullable pointer optimization）**：`Option<&T>`/`Option<fn>` 等用 null 表示 `None`，布局有保证。
- **不透明句柄（opaque handle）**：C 只见类型名不见字段的指针；Rust 侧用 `into_raw`/`from_raw` 交接所有权。
- （`展开（unwind）` 已在本表，按既有条目用，未新增。）

### 衔接提示
- 回指：第 23 章已定稿，本章两处按 23.2 / 23.6 精确回指（跨章 findings #15 修订时对齐）；第 25、41 章仍是占位，只写章号。
- 第 25 章落定后，可考虑在 26.4 加一句指向其 `repr(C)` 示例；我这边只留了「第 25 章讲了完整规则与反例」。
- 第 23 章定稿后确认：其 23.6 UB 清单第 8 条（「用错误的调用 ABI 调用函数，或让展开（unwind）越过不允许展开的栈帧」）已覆盖本章 26.6 的事实一，两章措辞一致（**安全中止进程**，不是「结果不对」式的 UB）。**是否需要在本章 26.6 加一句指向 23.6 第 8 条，留给协调者决定**——我这次只修 findings 点名的回指，未擅自扩写。

### 遗留不确定
- 例 3 依赖读者本机有 C 编译器（章内已注明）；纯 Rust 的例 1/例 2 不依赖，`mdbook test` 门禁只覆盖后者——这是有意为之（C 编译产物无法进 `mdbook test` 链）。
- 例 3 的 Windows 命令按本机实况写（`gcc` 来自 MinGW-W64、rustc 目标是 MSVC）；非 Windows 平台的等价写法（`gcc -shared -fPIC -o libc26.so` + `-L . -l c26`）未在本机实测，章内未展开。
- `kind = "raw-dylib"` 只在正文提了一句用途，未实测（用 import lib 路径已够）。

### 图示需求（若要出图票）
- 建议一张「FFI 边界与 panic 三条防线」横向示意图：`Rust 调用方 →（extern "C" 边界）→ 回调方`，标出三种情形的位置——① 直接越界 → abort 点；② 边界内侧 `catch_unwind` → 就地转成返回值；③ 全链 `extern "C-unwind"` → 外层 `catch_unwind` 接住。与 26.6 的三条事实一一对应，值得画。

### 交叉审校修订（RvCh26：0 blocker / 0 major / 5 minor，全部采纳）
只动了 `src/ch26-ffi-abi.md` 与本票，`snippets/ch26/` 未动（无需改）。

1. **引文出处错标（26.3）**：`&CStr is to CString as &str is to String…` 的出处由「`std::ffi` 模块文档」改为「`std::ffi::CStr` 文档」。已用快照复验：该句在 `std/ffi/struct.CStr.html` 命中（返回 True）、在 `std/ffi/index.html` 不命中（返回 False）；同段第一句 `CString and CStr are useful…` 则相反，出处保持为模块文档。
2. **ABI 表 `"system"` 行补变参例外（26.1 表）**：改为「在 Windows x86_32 上，非变参函数等同 `stdcall`、变参函数仍等同 `"C"`；其余平台等同于 `"C"`。调 Windows API 用它」——与 Reference `items/external-blocks.html` 的 `equivalent to extern "C" except on Windows x86_32 where it is equivalent to "stdcall" for non-variadic functions, and equivalent to "C" for variadic functions` 对齐，且与本章 26.7 的变参主题自洽。
3. **术语首现格式（为什么这样设计 + 小结）**：首现处补全英文——「主流 C 编译器选定的**调用约定（calling convention）**」；「平时的 Rust 函数名会被编译器**修饰（符号修饰，symbol mangling）**成唯一的符号，撞名几乎不可能；一旦你关掉**符号修饰**或者自己指定名字…」。小结里的「符号修饰」保持这个拼写，与首现一致。（术语并表仍由协调者执行，本票只提供拼写。）
4. **26.7 变参声明块去掉 `ignore`**：该声明在 stable 编译通过、无链接依赖，故改为 `rust` 让门禁覆盖「stable 上可以声明变参」这一主张。块数随之变化：原 13 passed / 7 ignored → 现 **14 passed / 6 ignored**。
5. **26.7 补半句**：`next_arg::<T>()` 后补「（等价于 C 的 `va_arg`；`next_arg` 本身是 `unsafe fn`，每取一个都得包一层 `unsafe`）」。

**修订后复跑（同一台机、同一命令）**：
- `mdbook test -c ch26-ffi-abi.md` → **exit 0**（输出仍只有 ` INFO Testing chapter '第 26 章 FFI 与 ABI': "ch26-ffi-abi.md"`）。
- 再次注入反证：章末临时加 `fn main() { let x: i32 = "坏"; }` → **exit 101**，`error[E0308]: mismatched types` + `test result: FAILED. 14 passed; 1 failed; 6 ignored`；还原后重跑 → **exit 0**（证明新去掉 `ignore` 的变参块确实进了校验面）。
- 探针目录 `.scratch/tmp-44-rv/` 已在收尾删除。

### 跨章 findings（随 RvCh23 转来，一并修）

**#15【minor·跨章回指】26.5 不透明句柄段收窄第 23 章的 UB 回指**：原句「第 23 章列过的那类 UB（重复释放、释放后使用）」有误——ch23 定稿的 23.6 UB 清单里**没有**「重复释放」，「释放后使用」也只以「悬垂」出现。现改为「第 23 章 23.6 的 UB 清单第 2 条（悬垂读写）」，与 ch23 定稿逐条对齐（该清单第 2 条即「从**悬垂的**或**基于未对齐指针的**位置读写」）。

**顺带对齐（同一处跨章回指的精度问题，我主动一并改，供评审复核）**：26.8 末句原写「第 23 章讲过的「`unsafe` 不是关掉检查的开关」」——加了引号实为转述。ch23 定稿里对应的是 §23.2 标题「`unsafe` 不是「关掉检查」」与小节正文「`unsafe` 块给的是**额外能力**，不是关闭检查的开关」。现改为「第 23 章 23.2 的结论（`unsafe` 不是「关掉检查」）」，不再冒充逐字引文。若评审认为此改动超出 #15 范围，可回退为原句。

**该项修后复跑**：`mdbook test -c ch26-ffi-abi.md` → **exit 0**（本次只改正文文字，块结构未变，仍是 14 passed / 6 ignored）。探针 `.scratch/tmp-44-x15/` 已删除。


