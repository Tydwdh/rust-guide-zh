# 43 第 25 章 数据布局与 repr

Type: task
Status: resolved

## Question

写第四篇第三章：**数据布局与 repr**。

- **范围**：`repr(Rust)` 无保证（字段重排、填充）与「别依赖它」；`repr(C)` 的字段顺序契约；`repr(transparent)`；niche 优化（`Option<&T>`/`Option<Box<T>>`/`Option<NonZeroU32>`，与第 17 章 `List` 例子呼应）；enum 布局（判别式、payload 与 niche 的合流）；DST 与胖指针（`str`/`[T]`/`dyn Trait`，`size_of_val`/`align_of_val`）。
- **素材**：`research/02-knowledge-structure.md` 的 I15 行（依赖 I13；`reference/type-layout.html`、`nomicon/repr-rust.html`、`nomicon/other-reprs.html`、`nomicon/exotic-sizes.html`）；实测工具：`size_of`/`align_of`/`offset_of!`/`size_of_val`（1.98 全稳定）。
- **边界**：FFI 的 ABI 契约 → 第 26 章；vtable 细节 → 第 13 章（回指）；别名模型 → 第 35 章。
- **篇幅**：4000–5500 汉字；八节模板；练习 2–3 题带答案；布局数字必须本机实测（注明平台：Windows x86-64），不跨平台硬断言。
- **DoD**：§8 —— 独占 `src/ch25-data-layout-repr.md` + `snippets/ch25/` + 本票；禁 `git`/禁 build；自验 `mdbook test -c ch25-data-layout-repr.md` 绿；探针 `.scratch/tmp-43/`；收尾 `claimed` + `## Comments`。

## Comments

**文件（只动这些）**
- `src/ch25-data-layout-repr.md`：初稿成稿，八节齐全（问题/为什么/机制与规则/动手看看/报错现场/小结/练习+答案/延伸阅读），5380 汉字，内嵌 2 张手写 SVG。
- `snippets/ch25/` 四个完整程序，正文用 `{{#rustdoc_include}}` 引入：`structs.rs`（字段顺序与填充、repr(C)、align、transparent）、`enums.rs`（判别式宽度 + niche 表）、`list.rs`（第 17 章 `List` 的偏移与 `BigList` 对照）、`dst.rs`（胖指针与 `size_of_val`/`align_of_val`）。
- 未碰其他文件；探针目录 `.scratch/tmp-43/` 收尾已删。

**自验**
- `mdbook test -c ch25-data-layout-repr.md` → **exit 0**（本轮跑 4 次一致；失败时 exit 101）。
- 注入失败反证（均还原，`cmp` 与原文件一致）：① `snippets/ch25/structs.rs` 末尾追加一个类型错误 → `test result: FAILED. 7 passed; 1 failed` + `ERROR One or more tests failed`；② 正文内联块 `let s: &str = "hello";` 改成 `= 42;` → exit 101。说明 include 的片段与内联块都真的被编译执行。
- 本章计 8 个 doctest：4 个 include + 2 个 `compile_fail` + 1 个内联 `size_of_val` 示例 + 练习二里的 `#[repr(C)] struct Id(u32);`。
- 四个 snippet 另用 `rustc --edition 2024 -O` 单独编译通过（输出即正文粘贴的那些）。

**实测清单（Windows x86-64、Rust 1.98；正文凡数字都注明平台）**
- 基本类型 size/align：`bool`/`u8`…`u128`、`usize`、`f32`/`f64`、`char`。
- `Mixed{a:u8,b:u32,c:u8,d:u64}` = 16/8，偏移 `d`0 `b`8 `a`12 `c`13；同字段 `#[repr(C)]` = 24/8，偏移 `a`0 `b`4 `c`8 `d`16。
- `#[repr(C)] CHeader{u8,u32}` = 8/4（`tag`0 `len`4）；`#[repr(C, align(16))] Aligned16(u64)` = 16/16；`#[repr(transparent)] Meters(f64)` = 8/8；`#[repr(C)] Id(u32)` = 4/4。
- 枚举：`Plain3`（无 repr、三变体）= 1/1、`Option<Plain3>` = 1；`#[repr(u8)] Tagged8` = 1/1、`Option<Tagged8>` = 1；`#[repr(u32)] Tagged32` = 4/4；`TwoU64` = 16/8、`Option<TwoU64>` = 16；`#[repr(C)] CMsg{Pair(u8,u32), One(u64)}` = 16/8。
- niche：`Option<&u64>`/`Option<Box<u64>>`/`Option<NonZeroU32>`/`Option<bool>`/`Option<Option<bool>>`/`Option<fn()>` 均与内层同大小；`Option<u32>` = 8；`Option<*const u8>` = 16；`Option<NonNull<u32>>` = 8。
- `List{Cons(i32,Box<List>),Nil}` = 16/8（`i32`@4、`Box`@8、`Option<List>` = 16）；`BigList{Cons(u64,Box<BigList>),Nil}` = 16/8（`Box`@0、`u64`@8、`Option<BigList>` = 24）。
- DST：`&[u64]`/`&str`/`Box<[u64]>`/`Box<str>`/`&dyn Debug`/`Box<dyn Debug>` 均 16；`&[u64;3]`/`Box<[u64;3]>` 均 8；`size_of_val("héllo")` = 6、`align_of_val` = 1；`&dyn Debug` 指 `Circle` 8/8、指 `Dot` 0/1；`[u16;0]` = 0/2。
- 报错逐字：E0277（`size_of::<str>`）、E0793（`&p.b` 指向 packed 字段）、E0690（transparent 两个非平凡字段）、E0658（枚举变体进 `offset_of!` 仍属 nightly）。后两个只在正文里以关键行呈现，未配 compile_fail 块。
- 练习一另测：`Two{a:u8,b:u64}` = 16/8、偏移 `b`0 `a`8；`#[repr(C)] CTwo` = 16/8、偏移 `a`0 `b`8。

**遗留不确定 / 请协调者裁断**
1. **与第 17 章的 niche 表述对不上（重要）。** 第 17 章写 `List` 的 `Nil`「折进 `Box` 的空指针 niche，没有独立的判别字段」。本机实测：`List`（i32 载荷）`i32`@4、`Box`@8，且 `Option<List>` 仍为 16 字节 —— 说明判别式另有位置且留着空位；只有换成 `u64` 载荷（`BigList`）时 `Box` 才跑到偏移 0、`Option<BigList>` 变成 24 字节，那才是真的借了空指针 niche。判据：`Option<X>` 的大小是「判别式是否借走空位」的最快探针——空位一旦被占满，`Option` 必然变大。本章按实测写，并点明「16 字节这个数在本机复现了，但判别式的落点更挑形状」，没有引用第 17 章那句 niche 论断。**第 17 章不在本票范围，我没有改**；建议协调者把第 17 章那句括注改成本章口径，否则两章对同一段代码的说法不一致。
2. niche 优化本身不是文档保证（官方只保证「正确性必需」的那些条款），正文已按「当前实现、别依赖」措辞；`Option<*const u8>` 为 16 字节也是实测值而非保证。
3. `repr(C, packed)` 的「5 字节、对齐 1」只在反例附近的正文里以文字给出，没有可一键复现的 snippet；要补的话可在 `snippets/ch25/` 加一个。
4. 图 2 把 `List` 开头 4 字节画成合并的「判别式 + 填充」：只能确定这 4 字节里含判别式与填充，判别式本身是 1 字节还是 4 字节无法从 `size_of`/偏移判定，故不拆开画。

**新增术语（GLOSSARY 未收录，请并表）**
- 对齐（alignment）、填充（padding）、判别式（discriminant）。
- 空位（niche）：类型中不可能出现的位模式；建议 `_Avoid_: 空洞`。正文首现给「空位（niche）」，其后用 niche。
- 其余沿用表内既有条目：`DST`/`动态大小类型`/`胖指针`（已按 `_Avoid_: 宽指针` 校正）/`vtable`/`trait 对象`/`未定义行为`。

**衔接提示**
- 第 17 章 `List`：沿用 16 字节口径，补出偏移与 `Option` 大小（见上第 1 条）。
- 第 13 章：vtable 与胖指针只回指一句，`&dyn Trait` 16 字节的来源在本章解释；未展开 vtable 内部布局。
- 第 24 章：provenance/别名只在边界句回指，未展开。
- 第 26 章：`extern "C"`、C 字符串、`repr(C)` 在 FFI 侧的细则全部留白并明确移交。

**图示（本章 2 张内嵌 SVG，均单块无空行、无脚本/外链、`viewBox` + `width="100%"` 无 `height`、只用主题变量、带 `role`+`aria-label`、图内 `&`/`<`/`>` 已转义、偏移刻度无对齐文本故未用 `xml:space`）**
- 图 1：`Mixed` vs `CMixed` 字节网格（16 vs 24 字节，标出填充与字段起点偏移）。
- 图 2：`List` vs `BigList` 字节网格，右侧注记 `Option` 大小。
- 已从正文抽出单独渲染核对过版式（字段宽度、刻度、右侧注记均不溢出 `viewBox`）。

### 交叉审校第一轮（reviewer RvCh25：6 major + 6 minor）—— 已逐条修复

**复验用的新实测（Windows x86-64、Rust 1.98，本轮新增）**
- `Option<Option<bool>>` 的字节：`Some(Some(false))` = `0x00`、`Some(Some(true))` = `0x01`、`Some(None)` = `0x02`、**外层 `None` = `0xff`**；`Option<bool>::None` = `2`；`Option<NonZeroU32>::None` = `0`。审校给的 255 属实——我原先写的「3」是伪测（同一行另两项实测正确）。
- `offset_of!(Header<dyn Debug>, tag)`（容器是 DST）= `0`，编译通过 → std 口径是「**字段**类型须 `Sized`，容器可以是 DST」。
- `size_of_val(*const u64)` → `error[E0308]: mismatched types`（`expected &_, found *const u64`）；`size_of_val_raw` → `error[E0658]: use of unstable library feature 'layout_for_ptr'`（issue #69835）。

**逐条**
1. major、niche 表 `Option<Option<bool>>` 行 → 说明改为「外层 `None` 用 `0xff`」；表后补一句「哪个取值表示 `None` 只是当前实现（实测 2 / `0xff` / 0），不是接口，别背」。
2. major、`TwoU64` 的 `Option` 大小解释写反 → 改为「独立判别式只用了两个取值、还剩大量空位，`Option` 的判别式塞得进去」，与练习三口径一致，并拿 `BigList`（空位被占满 → `Option` 24 字节）作反例。
3. major、回指第 17 章用的旧口径 → 改为「第 17 章数过 `List` 是 16 字节、判别字段就在开头（那种 `u64` 载荷的形状才会折进空指针）；这里把偏移与 `Option` 大小补齐」，与协调者勘误后的 `ch17:142` 一致。
4. major、「前三个都要求 `T: Sized`」 → 改为「`size_of`/`align_of` 要求 `T: Sized`；`offset_of!` 要求被取偏移的**字段** `Sized`，容器可以是 DST」，并补上 std 文档那个 `Header<T: ?Sized>` 形状的可跑例子（输出 `0`，已进门禁）。
5. major、`size_of_val`/`align_of_val` 的入参 → 改为「只接收**引用**（`fn size_of_val<T: ?Sized>(val: &T) -> usize`）」，并注明裸指针版 `size_of_val_raw`/`align_of_val_raw` 到 1.98 仍是 nightly（`layout_for_ptr`、#69835），附 E0308 / E0658 实测。
6. minor、「细指针」→「瘦指针」（GLOSSARY 记 `_Avoid_: 细指针`；全书仅此一处）。
7. minor、判别式首现 → 补「（discriminant）」（首现处为 §2，已加；后文不再重复）。
8. minor、「显式 `repr` 会关掉它」 → 限定为「只限带字段的枚举」，并补「无字段枚举不受影响——上表 `Tagged8` 带着 `repr(u8)`，`Option<Tagged8>` 仍是 1 字节」。
9. minor、小结 7 条 → 原第一、二条合并，压回 **6 条**。
10. minor、两张 SVG 的偏移刻度字号 → 16 处 `font-size="10"` 全部提到 **12**；两张图重新渲染核对，刻度与字段框无重叠、右侧注记不溢出。
11. minor、「`repr(C)` 是唯一『为了布局本身』而存在的表示」 → 改为「唯一自带一套**确定布局算法**的表示」。
12. minor、`repr` 引文缺原文 → 对齐那处（`:75`）与 DST 指针那处（`:262`）都补上**英文原文**（逐字核对本机快照）并加「译：」；同时把另外三处「引号里放译文」的写法（`repr(C)` 边界、C enum 警告、DST 只能待在指针后面）改成不含引号的间接叙述，消除「引号内必须是原话」的隐患。

**复审后自验**
- `mdbook test -c ch25-data-layout-repr.md` → **exit 0**（修复前一度 5624 汉字超上限，已压回 5472）。
- 本章 doctest 由 8 个增至 **9 个**（新增 `offset_of!` DST 容器示例）；对该新块注入类型错误 → `test result: FAILED. 8 passed; 1 failed`、**exit 101**，还原后 exit 0 —— 新块确实在门禁内被编译。
- 本轮只动 `src/ch25-data-layout-repr.md` 与本票；`snippets/ch25/` 四个文件未改（新增示例直接内联在正文，不涉及 snippet）。

## Answer

2026-10-19 批次收尾（用户验收通过）。

- 交付：`src/ch25-data-layout-repr.md`（5472 汉字；9 doctest；4 片段 `structs/enums/list/dst.rs` + 2 张自画 SVG）。
- 自验：`mdbook test -c ch25-data-layout-repr.md` exit 0；注入反证 `FAILED. 8 passed; 1 failed` → 还原。
- 交叉审校（RvCh25；6 major + 6 minor）全修：**伪测纠正**（`Option<Option<bool>>` 实为 255 非 3）；`TwoU64` Option 解释与练习三统一；**ch17 回指与勘误版同步**；`offset_of!` 字段 Sized/容器 DST；`size_of_val` 仅引用（`*_raw` nightly）；瘦指针；discriminant 括注；niche 关停限定「带字段」；小结 6 条；SVG 字号 10→12；「确定布局算法」措辞；引文补原文。
- 术语并表（协调者）：对齐 / 填充 / 判别式 / 空位（niche）已入 GLOSSARY。
- 全量链（批次级）：同上一档口径全绿。
- 遗留：无。
