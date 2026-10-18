# 65 第 37 章 编译器与代码生成

Type: task
Status: claimed

## Question

写第六篇第四章：**编译器与代码生成**（素材 = D4：rustc book `codegen-options`、reference `inline-assembly`、`linker-plugin-lto`）。

- **范围**：编译流水线鸟瞰（HIR→MIR→LLVM IR→机器码）；MIR 观察（`--emit=mir` stable 真跑；`-Zunpretty` 标 nightly）；单态化与代码膨胀（`--emit=llvm-ir`/asm 对照、符号重复观察）；`-C` 常用选项（opt-level/debuginfo/codegen-units/overflow-checks——讲「选项→代码」一条线，与 ch29 的 profile 层不重复）；inline asm（`core::arch::asm!` stable 最小例真跑）；naked functions（nightly 标注）；LTO 一句带过（回指 ch29 已验证结论）。
- **要求**：dump 输出真跑真粘（注明「本机 x86-64、随版本浮动」）；不得给未跑的输出。
- **边界**：性能方法学/PGO → ch29 回指；布局 → ch25；UB 与优化 → ch23。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；按 §9 内嵌 1 张流水线 SVG（可选）。
- **DoD**：§8 —— 独占 `src/ch37-compiler-codegen.md` + `snippets/ch37/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch37-compiler-codegen.md` exit 0；探针 `.scratch/tmp-65/`；收尾 `Status: claimed` + `## Comments`。

## Comments

**文件**：`src/ch37-compiler-codegen.md`（占位 H1 保留，正文全量替换，八节模板齐全）；新增 `snippets/ch37/{codegen,mono,asm_add,naked_add,cgu}.rs`（5 个完整可编译程序，全部经 `{{#rustdoc_include}}` 进正文，即全部受 `mdbook test` 编译+执行）。

**自验（§3 第 2 步）**
- `mdbook test -c ch37-compiler-codegen.md` → **exit 0**（正文定稿后跑；章节含 9 个 Rust 块 = 5 个 `{{#rustdoc_include}}` + 3 个 `compile_fail` + 1 个 `rust`）。
- 门禁反证（证明块真的被编译，不是假绿）：临时往正文追加一个类型错误块 → exit 101；把 `snippets/ch37/cgu.rs` 改坏 → exit 101；两者恢复后 → exit 0（`cgu.rs` 恢复后与备份 `cmp` 一致）。共 3 次运行，全部本机 1.98。
- 未跑 `mdbook build` / `mdbook serve`（按批次协议留给协调者）；未跑任何 `git` 命令。

**真跑真粘的输出清单**（口径：Windows x86-64、Rust 1.98 `(88d9e12ae 2026-08-18)`、LLVM 22.1.8；源码 `snippets/ch37/*.rs` 原样）
1. `--emit=mir`：文件头 3 行原文（WARNING/HINT）；`add` 的 MIR（`AddWithOverflow` + `assert … unwind terminate(abi)`，因 `extern "C"`）；`sum_slice` 8 个基本块（`bb2→bb3→bb5→bb7→bb2` 回边、`bb4: unreachable`）；`-O` 版 `add` 变纯 `Add`、`sum_slice` 掉到 5 块并出现 `scope 17 (inlined <std::slice::Iter<'_, u32> as Iterator>::next)`；`-C overflow-checks=off` 版 `add`。
2. `--emit=llvm-ir`：`@add` 的 `add i32` + `icmp ult` + `br` + `invoke …panic_const_add_overflow`；opt-level=2 版 `%_0 = add i32 %b, %a; ret`。
3. `--emit=asm`：opt-level=0 的 `add` 完整函数体（33 行，含 `.seh_*`；正文只略去末尾 `.seh_handlerdata` 起的段声明，已在正文注明）；opt-level=2 的 3 行（`add:` / `leal (%rcx,%rdx), %eax` / `retq`）。
4. 单态化：`llvm-nm mono.o | findstr wrap` → 5 个符号（`…4wrapb/c/d/m/y…`，对照 rustc 手册 v0 符号修饰的 basic-type 表：`b`=bool、`c`=char、`d`=f64、`m`=u32、`y`=u64）；`llvm-size` → `text` 922 B vs `-O` 374 B；`-O` 时 `llvm-nm | grep -ci wrap` = 0。
5. `-C debuginfo`：`wc -c dbg0.o dbg2.o` → 8860 / 34895（约 4×）；`llvm-readobj --section-headers` 在 `debuginfo=2` 下见 `.debug$S`、`.debug$T`，`=0` 下一个调试段都没有；`=1` 16836、`line-tables-only` 16864（正文写「16 KB 出头」）。
6. `-C codegen-units`：12 函数 crate（`cgu.rs`）上 `-C codegen-units=16 --emit=llvm-ir` → 11 个 `…-cgu.NN.rcgu.ll`（`cgu.00` 14 个 define，全库共 29）；`-C codegen-units=1` → 1 个文件、29 个 define。
7. `-C` 值错误三条原文：`overflow-checks=maybe`、`opt-level=9`、`codegen-units=0`。
8. `asm!` 最小例：`rustc --edition 2024 -O asm_add.rs` 编译并运行 → `5` / `18446744073709551615`；`--emit=asm` 里注入片段被 `#APP`/`#NO_APP` 夹住（`addq %rdx, %rax`）。`-Zunpretty=hir` 在 stable 上的拒绝原文一并粘出。
9. naked：`#[unsafe(naked)] pub extern "C"` + `core::arch::naked_asm!("lea eax, [rcx + rdx]", "ret")` → 编译运行得 `5`，`--emit=asm` 正好 `add:`/`leal …`/`retq`；同一属性用在 Rust ABI 上报 `error[E0658]: #[naked] is currently unstable on extern "Rust" functions`（issue #138997）。
10. 报错现场三条：E0133（`asm!` 无 `unsafe`，含 note 原文）、E0787（naked 函数体多语句，含两处 span）、`-C` 值错误。

**引文来源**（本机 1.98 快照一手复核）：`rustc/codegen-options`（overflow-checks、debug-assertions、opt-level、debuginfo、codegen-units、lto 的取值与默认值；`no-prepopulate-passes`）、`reference/inline-assembly`（稳定架构表、三种宏、操作数方向、`options`）、`reference/attributes/codegen`（naked 规则：MetaWord/unsafe/单一 `naked_asm!`/外部 ABI/不得复制）、`releases.md` 1.88（`naked_functions` 稳定原文）、`rustc --help`（`-O` = opt-level=3、`-g` = debuginfo=2、`--emit` 类型表与默认文件名）、`rustc/symbol-mangling/v0`（basic-type 编码表）、`rustc/linker-plugin-lto`（跨语言 LTO、thin 默认）。

**⚠ 事实修正（请审校重点核）**：票面要求「naked functions（nightly 标注）」——按本机 stable 1.98 实测与 1.88 release notes，**`naked_functions` 自 1.88 起已稳定**（`#[unsafe(naked)]` + 外部 ABI 在 stable 编译、运行、`--emit=asm` 全部成功）。所以正文**没有**把它写成 nightly：写为 stable 1.88+，并单独标注唯一的 nightly 例外——**Rust ABI 上的 `#[naked]` 仍不稳定**（E0658 / #138997，原文已粘）。若这是与全书口径的冲突，请协调者裁决后我再改。

**新增术语（GLOSSARY 未收，建议并表；正文已按「中文（English）」首现格式）**：内联（inlining）、基本块（basic block）、终结指令（terminator）、中间表示（IR）、代码生成单元（codegen unit / CGU）、目标文件（object file）、调试信息（debug info）、序言／尾声（prologue / epilogue）、值槽（MIR 术语）、机器码（machine code）。

**图示**：正文 37.1 内嵌 1 张手写 SVG（四段流水线 + 每段观察命令 + 三条注记；`viewBox="0 0 760 226"`、`width="100%"`、无 `height`、单块无空行、只用 `currentColor`/`var(--links)`、`role="img"` 含 `aria-label`）。我无法自查渲染，**留协调者收尾目视**（HTML 渲染与换主题可读性）。

**篇幅**：正文去代码块约 5800 汉字（含代码约 5900）；同量级对照：已定稿 ch29 = 5882、ch25 = 5472（同一计数口径）。票面写的 4000–5500 略超，若必须压到 5500 以内请指示，我再砍（可动的都是叙述性段落，不影响验证数据）。

**遗留不确定 / 未写入正文的观察**
- `-C no-prepopulate-passes`：opt-level ≥ 1 时 `--emit=llvm-ir` 给的是 LLVM 跑过 pass 之后的 IR；加该选项能看到 rustc 交出的那份（本机 mono 例：opt-level=1 下 5 个 `wrap` 定义在 no-prepopulate 版里仍在，默认版已无）。该行为有 rustc book 的 `no-prepopulate-passes` 条目佐证，但「到底哪一步删掉了复制品」随优化等级不同（opt-level=2 时 MIR 里已见不到、opt-level=1 时 MIR 里 `wrap::<T>` 调用还在），本机未做逐 pass 归因——**正文因此只写可观察事实，不写机制归属**。
- `-C lto` / profile 的 `lto` 键、`-C linker-plugin-lto`：只按票面要求「一句带过 + 回指第 29 章」，跨语言 LTO 未实跑（本机无 clang/flang）。
- `-C target-cpu` / `target-feature` 仅在 NOTE 里给查询命令（`rustc --print target-cpus|target-features`），未展开实测。
- 探针与临时产物：`.scratch/tmp-65/`（含改坏-恢复用的备份）**已整体删除**；`snippets/ch37/` 只留 5 个 `.rs`（编译产生的 `.exe`/`.pdb` 已清）。

## 审校修复（RvCh37 回执：2 blocker + 8 minor，逐条处理）

- **F01 ✅ blocker** `snippets/ch37/asm_add.rs`：`add_via_asm(u64::MAX - 1, 1)` → `add_via_asm(u64::MAX, 1)`；正文粘贴的输出同步改为 `5` / `0`，句子改写为「`u64::MAX` 加 1 本该溢出，机器指令直接回绕成 0」。原断言确实不成立（`(MAX-1)+1 = MAX`，根本没溢出）。复跑：`5` → `0`。
- **F02 ✅ blocker** 练习二：题干块 asm 改为 win64 正确的 `lea rax, [rcx + rcx], "ret"`；答案第 3 点重写为「参数与返回值得自己按调用约定兑现」，附实测证据——`lea rax, [rcx+rcx]` → `double(21) = 42`；SysV 写法 `mov rax, rdi` → 本机连跑 3 次得 `281404659110560`（不是 42，读到的根本不是参数）；答案里的修复块同步改为 `lea`（该块是 `rust`，受 `mdbook test` 编译）。
- **F03 ✅ minor** `<hash>`：重跑 `rustc --edition 2024 --emit=asm codegen.rs`，本机得 `anon.62d286c84577314f14f3ace68a483eee.4`（与审校观测一致，此前贴的是别的探针的产物）→ 已替换，并补一句「该哈希由 `-C metadata` 派生，换 crate 名／目录／编译选项就会变」。
- **F04 ✅ minor** 措辞改为「末尾从 `.seh_handlerdata` 起的那四行（`ud2` 之后的段声明与 `.seh_endproc`）没抄」。实测顺序：`ud2` → `.seh_handlerdata` → `.long` → `.section` → `.seh_endproc`。
- **F05 ✅ minor** 改为「从约 8.8 KB 抬到约 34.9 KB（约 4 倍）」，并注明会随构建目录的绝对路径长度浮动几十字节、别当常量背。
- **F06 ✅ minor** `wc -c` 输出补回 `43747 total` 行（复跑两次同值）。
- **F07 ✅ minor** E0658 原文改用双反引号 span：`` `#[naked]` is currently unstable on `extern "Rust"` functions ``。**另用私有探针书验证**了这种写法在 linkcheck2 下安全（见下「同轮自查」），span 里的 `[naked]` 不会被当成链接。
- **F08 ✅ minor** 开场回指「`Vec` 的求和…（第 6 章）」→「迭代器链为什么能和手写循环编译成同一件事（第 10 章）」（依据 ch10「零成本从哪来」与「这也是它和手写循环能编译成同一件事的原因」）。
- **F09 ✅ 裁定接受**：不压缩。长度记录（本机口径：去掉 ``` 代码块、只计 CJK）正文 **6079** 汉字、含代码总量 6087；审校口径 5714（差异来自是否计入表格/代码注释等）。先例 ch19/29 已远超票面上限。
- **F10 ✅ minor** 37.2 首现改为「值槽（MIR 的局部变量，local）」；其余术语并表归协调者。

### 同轮自查时一并补齐 / 复核
- **票面「`--emit=llvm-ir` 真跑真粘」此前只有转述**：37.4 第二组已补两个**真实 dump 块**——`@add` 默认档前 5 行、`-C opt-level=2` 完整 5 行；用脚本与本机 `cg0.ll` / `cg2.ll` 逐字节比对为 True。
- **逐字节复跑核验（脚本比对，全部 True）**：MIR `add`（opt0 与 `-C overflow-checks=off` 两版）、asm `add`（33 行，仅声明略去末尾 4 行）、上面两个 IR 块。
- **数值/名称类复跑一致**：`llvm-nm` 5 个 `wrap` 符号（`…wrapb/c/d/m/y…`）、`llvm-size` 922/374、`-O` 时 `llvm-nm | grep -ci wrap` = 0、`-C codegen-units=16` 切出 11 个 `…-cgu.NN.rcgu.ll`、`wc -c` 8860/34887/43747。
- **私有 linkcheck 探针书**：`.scratch/tmp-65fix/lc/`（自带 `[output.linkcheck2]`、`follow-web-links = false`）验证 F07 的嵌套反引号 span 与单反引号方括号 span 均 **exit 0**；该探针书不会碰仓库 `book/`，随后连同整个 `.scratch/tmp-65fix/` 一起删除。

### 修复后自验
- `mdbook test -c ch37-compiler-codegen.md` → **exit 0**。
- 块清单核验：注入一个类型错误块 → `9 passed; 1 failed; 0 ignored`（证明全章 **10 个 Rust 块全部参与编译，没有 `ignore` 漏网**，F02 改动的答案块也在其中）；恢复后 → exit 0。
- 复跑 `asm_add`（改动后）：`5` / `0`；复跑 naked 探针：`42` / `281404659110560`。
- 探针目录 `.scratch/tmp-65fix/` 已删除（只剩 `snippets/ch37/` 的 5 个 `.rs`）。
