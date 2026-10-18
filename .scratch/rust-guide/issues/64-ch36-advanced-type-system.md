# 64 第 36 章 高级类型系统（nightly 标注）

Type: task
Status: claimed

## Question

写第六篇第三章：**高级类型系统**（素材 = D3：unstable-book `specialization`/`generic_const_exprs`/`adt_const_params`；coherence 细节）。

- **范围**：coherence 细节（孤儿规则精确版、fundamental 类型、覆盖判定）——stable 侧用 `compile_fail` 实测冲突；negative impls（`negative_impls`）、specialization（`specialization`/`min_specialization`）、`adt_const_params`/`generic_const_exprs`（均 nightly）——**全部 WARNING 标注 + 只给文档化示例**（本机无 nightly，不伪造输出）。
- **要求**：每个 unstable 特性给稳定面替代方案（若有）与追踪 issue 编号（逐条核官方）；`compile_fail` 错误码与错误类匹配。
- **边界**：基础 trait/泛型 → ch12/13/15 回指；nightly 工具链约定 → ch02 口径一句。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案。
- **DoD**：§8 —— 独占 `src/ch36-advanced-type-system.md` + `snippets/ch36/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch36-advanced-type-system.md` exit 0；探针 `.scratch/tmp-64/`；收尾 `Status: claimed` + `## Comments`。

## Comments

### 文件

- 独占写入：`src/ch36-advanced-type-system.md`（H1 保持占位原样，八节模板齐全；4315 汉字）、`snippets/ch36/`（新建，6 个可跑文件）。
- `snippets/ch36/`：`orphan_ok.rs`（外部 trait + 本地类型 T0，以及本地类型出现在 trait 实参位置 `impl<T> From<Crate> for Vec<T>`）、`fundamental_box.rs`（`impl Index<usize> for Box<Point>`）、`macro_push.rs`（generic_const_exprs 的 stable 替代：宏展开 `1=>2,2=>3,3=>4`）、`any_downcast.rs`（specialization 的运行期替代）、`mode_u8.rs`（adt_const_params 替代之一：`u8` + `const { assert!(...) }`）、`mode_marker.rs`（替代之二：marker 类型 + 关联常量）。
- 未碰任何禁写文件（SUMMARY/GLOSSARY/book.toml/WRITING/其它票）。

### 自验

- `mdbook test -c ch36-advanced-type-system.md` → exit 0（最终内容重复跑两次均 0）。
- 全部 6 个 snippet 单独 `rustc --edition 2024` 编译 0 warning，并真跑取输出（命令与输出已逐字粘进正文 console 块）：`orphan_ok` → `(1, 2)` / `空向量长度 = 0`；`fundamental_box` → `p[0] = 7`；`macro_push` → `[1, 2, 3]`；`any_downcast` → `其他类型` / `String，3 字节`；`mode_u8` → `fast` / `safe`；`mode_marker` → `fast` / `safe`。
- 章内 9 个 `rust,compile_fail` 块逐个用探针实测过错误类与错误码（输出块为实测输出节选，删了 `-->` 路径行）：E0117（`impl Display for Vec<Point>`）、E0210×2（`impl<T> Display for T`；`impl<T> From<Point> for Box<T>`）、E0119（本地 trait 的 blanket + `Vec<T>`）、E0658（`impl !Send for Handle`，note #68318 + help "use marker types for now"）、E0277（`PhantomData<Rc<()>>` 的 `assert_send`）、E0658（`default fn`，#31844）、`Mode` is forbidden as the type of a const generic parameter、generic parameters may not be used in const operations。
- 额外探针（结论已写进正文明细表/练习答案，探针目录已删）：`impl<T> From<Point> for Vec<T>` 通过；`impl Index<usize> for Vec<Point>` E0117；`impl<T> From<Local> for T` E0210；`impl<T> From<T> for Point` E0119（与 core 的 `impl<T> From<T> for T` 冲突）；`impl Index<usize> for &Point` 通过；双 crate 探针（自建 `dep` rlib，`pub trait ForeignTrait2<T,U>`）确认 `impl<T> ForeignTrait2<T, MyType<T>> for MyType2` **编译通过**。
- 探针目录 `.scratch/tmp-64/` 与 snippets 里生成的 `.exe/.pdb` 已删除。

### 遗留不确定

- `rustc --explain E0210` 里那段 `impl<T> ForeignTrait2<T, MyType<T>> for MyType2 // error` 与 1.98 实际行为**不符**（实测两种参数顺序都通过，因为 `T0 = MyType2` 是本地类型且排在首位，扫描区间为空）。故正文**没有引用**该例，改用自己实测的 `From<Point> for Box<T>`。
- nightly 侧一律 `rust,ignore`（未编译、无输出声称），只给 unstable book / RFC 1210 的文档化示例 + 追踪号；未在本机验证 nightly 语义。`ConstParamTy` 的 derive 同受 `adt_const_params` 门控，正文已注明。
- `min_specialization` 的使用仅以「1.98 自带 std 源码 `library/core/src/lib.rs` 开着该 feature、`library/core/src/array/mod.rs` 等处有 `default fn`」为依据；未逐条读 std 内部实现细节。

### 新增术语（建议并表，本章已按此使用）

- 重叠（overlap）／重叠判定：一致性的第二个检查口，判定对象是 impl 头部。
- 负实现（negative impl）：`impl !Trait for T`，本章用「negative impls（负实现）」首现写法。
- 特化（specialization）／最小特化（min_specialization）：按 unstable book 的 feature 名保留英文为主。

### 需协调者处理

- `GLOSSARY.md` 第 735–736 行的「基础类型构造器（fundamental type constructor）」只列了 `&T`/`&mut T`/`Box<T>`，Reference glossary 含 `Pin<T>`（原话：「`&`, `&mut`, `Box`, and `Pin` are fundamental.」）；另补一句「fundamental 构造器不能覆盖其他类型」更完整。请并表时决定。
- 图示：本章内嵌 1 张 SVG（孤儿规则判定流程，`viewBox 0 0 760 258`，单块无空行、无脚本、仅主题变量、`role="img"` + `aria-label`、`<`/`>` 已转实体）。我无法自查渲染，请 `mdbook build` 后目视确认连线与箭头位置。

### 衔接

- 回指口径：trait/关联类型→第 12 章，`dyn`→第 13 章，const 泛型/GAT→第 15 章，`Send`/`Sync`→第 19 章，`Pin`→第 22 章，`compile_fail`/doc test→第 31 章；nightly 工具链口径走第 1 章（ch02 无安装内容），正文 WARNING 里给了 `rustup toolchain install nightly`。
- 与第 35 章（别名模型）无重叠；与第 15 章的分工：第 15 章讲 const 泛型的边界与 GAT，本章只从 coherence/一致性角度谈 const 参数类型与算术为何受限。

### 审校修复（RvCh36 回执，全部采纳；F10 归协调者）

**F1（blocker）** 判据写反，已按实测改写。新增判别探针（stable 1.98）：`impl<T> Display for Vec<T>` → **E0117**；`impl<T> Display for Option<T>` → **E0117**；`impl<T> Display for Box<T>` → **E0210**；`impl<T> From<T> for Vec<T>` → **E0210**。结论：**头部存在未覆盖类型参数 → E0210；没有本地类型、却也没有未覆盖参数 → E0117**。改动位置：机制与规则「错误码分工」要点、判定表（E0117 三行 + E0210 三行，含依据）、SVG 的「找不到」分支改为「有未覆盖参数 → E0210／否则 → E0117」、报错现场 E0210 末尾分工句、小结第 2 条；与表格其余各行已自洽。

**F2（major）** 采纳。两 crate 探针：`dep` v1 只导出 `pub trait MyTrait {}`，下游 `struct Local; impl dep::MyTrait for Local {}`（`Local: Display`）通过；`dep` v2 追加 `impl<T: Display> MyTrait for T {}` 后，同一份下游代码报 `E0119: conflicting implementations of trait 'MyTrait' for type 'Local'` + `= note: conflicting implementation in crate 'dep'`；把下游类型换成不实现 `Display` 的则 v2 仍通过。练习 2 题面与答案据此重写（答案内嵌该实测报错块）；「为什么必须全局唯一」一段补「拥有不等于随便加」的语义版本陷阱。

**F3（major）** 采纳：ch02 确无工具链安装内容，改指**第 1 章**（其「安装与验证：rustup」一节），并直接给出 `rustup toolchain install nightly`、`cargo +nightly build`。

**F4（major，方案 A）** 采纳：删掉两条 Reference 引文（一致性定义、重叠定义）与 E0119 的机制叙述，替换为「### 从第 12 章的基线出发」一段显式回指，点名 ch12 的 2×2、newtype 逃生门与「重叠即报错」；四个术语表中 fundamental 行改为回指 ch12 的原文出处、只保留判定后果，并删掉重复引的英文原句。保留 ch12 没有的：`T0` 优先的扫描顺序、E0210 两种报错形态、10 行判定表。本章仍只有 1 张图（判定流程图）。章首 NOTE 已点名「第 12 章 §一致性（coherence）与孤儿规则的细节」。

**F5（minor）** 采纳：「编译是局部的」改为 trait 的所有者 / 类型的所有者 / 当前 crate 三处。

**F6（minor）** 采纳：括号改为「第 12 章的扩展 trait 就是这么写的；newtype 的家在第 9 章」。

**F7（minor）** 采纳：`default fn` 的 text 块恢复 `| /`、`| |` 续行标记（仍按全章口径省掉 `-->` 路径行）。

**F8（minor）** 采纳：首现处（「这一章解决什么问题」清单）写「negative impls（负实现）」，「为什么越界的三件事」一节补 `impl !Trait for T` 形状；票面「新增术语」中 specialization 一条改写为「保留英文」。

**F9（minor）** 采纳：SVG 第三行删掉「E0210 → 通过」的错误边，「否」改为从判定框底部绕行到「通过」的折线；行高 50→56、`viewBox` 高 258→262，`aria-label` 同步。

**F10** 未动 `GLOSSARY.md`（归协调者并表）。

**复验**：`mdbook test -c ch36-advanced-type-system.md` → exit 0（全部改完后重跑）；汉字数 4735（仍在 4000–5500）；探针仅落在 `.scratch/tmp-64/`，已删除。
