# 27 第 16 章 生命周期进阶：variance、HRTB 与 drop check

Type: task
Status: resolved

## Question

写第 16 章：**生命周期的形式化直觉** —— 子类型从哪来、`for<'a>` 怎么用、drop 时借用怎么查。

- **范围**：variance（协变 / 逆变 / 不变：`&'a T`、`&'a mut T`、`fn(T)`、`PhantomData` 各自；为什么需要不变）；HRTB（`for<'a> Fn(&'a str)` 语法与真实用例，如把闭包传给需要任意生命周期的函数）；drop check 与 drop 顺序（为什么「析构里还会用借用」要单独检查；`#[may_dangle]` 只提名字与用途）；`PhantomData` 的三重用法（占位、调方差、影响 drop check）；生命周期省略规则温故（第 4 章的直觉 → 精确规则表）。
- **素材**：`research/02`（I6）；nomicon 的 subtyping / variance / dropck / PhantomData 节；reference 的 lifetime elision 页。
- **边界**：智能指针 → 17；别名模型与形式化（Stacked/Tree Borrows）→ 35；`unsafe` 与 UB → 23。
- 篇幅 4500–6000 汉字（本章偏难，允许偏长）；八节模板；练习 2–3 题带答案；nightly-only 只标注。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch16-lifetimes-advanced.md`）。

## Comments

- 2026-10-18 初稿：文件 `src/ch16-lifetimes-advanced.md`（初稿 6332 汉字、术语替换与措辞微调后为 6336，均不含代码块；八节模板齐全，练习 3 题带答案）+ 片段 `snippets/ch16/drop_order.rs`、`snippets/ch16/hrtb_callback.rs`；自验 `mdbook test -c ch16-lifetimes-advanced.md` → **exit 0**（33 个 `rust` 块 + 14 个 `rust,compile_fail` 块 + 15 个 `text` 输出块 + 4 个练习题面 `rust,ignore`）；门禁有效性另用「章末临时注入 `let _: u32 = "x";`」反证：exit 101、`1 failed`，探针随后删除；实测输出、遗留、新术语、衔接与图示需求见下列分组。
  - 片段实跑（与正文输出逐字一致）：`rustc --edition 2024 snippets/ch16/drop_order.rs -o drop_order && ./drop_order` → exit 0，8 行 drop + `总顺序：…`，末尾 `assert_eq!` 通过；`rustc --edition 2024 snippets/ch16/hrtb_callback.rs -o hrtb_callback && ./hrtb_callback` → `alpha` / `delta`。
  - 实测输出清单（探针全部放在私有目录 `.scratch/tmp-27/`，收尾已删除）：
    1. `E0597` dropck：`Inspector<'_>`/`World<'_>`，`19:38`，尾行 `` `world.days` dropped here while still borrowed / borrow might be used here, when `world` is dropped and runs the destructor for type `World<'_>` ``；对照实验：删掉 `impl Drop` 后同一段编译通过（仅 dead_code 警告）。
    2. `E0597` `&mut T` 不变性（`assign`，`9:28`，含 `` type annotation requires that `world` is borrowed for `'static` ``）。
    3. `E0308` 逆变/HRTB：`` one type is more general than the other ``，note 为 `` expected fn pointer `for<'a> fn(&'a _) -> ()` / found fn item `fn(&'static _) -> &'static str {store}` ``。
    4. `E0392`（lifetime 与 type parameter 各一版，help 提到 `PhantomData`）。
    5. `E0637` `Parser<&str>` → help 直接给出 `for<'a> Parser<&'a str>`。
    6. `E0106` 四例：`fn get_str() -> &str`、`fn frob(s, t) -> &str`、`fn first(w: &Words) -> &str`（`` it is borrowed from `w`'s 2 lifetimes ``）、`` F: Fn(&str, &str) -> &str ``（含 `` = note: for more information on higher-ranked polymorphism, visit …/nomicon/hrtb.html ``）。
    7. `Box<dyn Display>` 默认 `+ 'static`（`'_` 修法）。
    8. 变型方向实测：协变通过（`&'a str` 借 `&'static str`；`Co<'static>` → 更短 `'a`）；不变失败（`Inv<'a>`、`Cell<&'a str>`）；逆变通过（`Contra<'short>` → `Contra<'static>`，反方向失败）。均用「`fn takes<'a>(_: &'a String, _: 包装<'a>)`」把目标区间钉到局部变量，避免推断变量把子类型变成合一。
    9. `PhantomData`：`PhantomData<*const u8>` 使类型 `!Send`（`E0277`，note 指向 `PhantomData<*const u8>`），`PhantomData<T>` 通过；`size_of::<PhantomData<u64>>()` = `size_of::<PhantomData<&'static str>>()` = 0。
    10. HRTB：具名 `'a` 的 bound（`fn count<'a, F: Fn(&'a str) -> usize>`）调用局部变量失败（`E0597`，`` argument requires that `s` is borrowed for `'a` ``）；`for<'a>` 与 `annotate` 版本通过；两输入版 `for<'x, 'y> Fn(&'x str, &'y str) -> &'x str` 通过并输出 `3`。
    11. 闭包高阶推断：裸 `let f = |s: &str| -> &str {…};` 用两个不同寿命的实参失败（`` `'1` must outlive `'2` ``）；有期望签名时（`needs(|s: &str| -> &str { s })`）通过；塞进结构体字段再调 `for<'a>` 方法报 `E0599`（note：`` the method was found for `FirstWord<F>` ``）。
    12. 析构顺序程序真实输出；1.91 release note 条目（“Lower pattern bindings in the order they're written and base drop order on primary bindings' order”）逐字核对。
    13. std 一手证据：`library/alloc/src/vec/mod.rs:4295` `const unsafe impl<#[may_dangle] T: Destruct, A: Allocator + Destruct> Drop`；`library/core/src/ptr/unique.rs:35` 的 `PhantomData<T>` 及其注释原文 “this marker has no consequences for variance, but is necessary for dropck to understand that we logically own a `T`”；`library/alloc/src/raw_vec/mod.rs:75` 同款字段。`Vec<&str>` + `drop(s)` 片段在 1.98 可编译（may_dangle 效果），已进正文并受 `mdbook test` 覆盖。
  - 遗留不确定：
    1. 「1.98 起 `&mut` 的 unsize 强转允许在不变位置缩短生命周期」只在本机 1.98 验证了可编译（`Cell<&'static mut i32>` → `Cell<&mut dyn Send>`）；未在 ≤1.97 上对照复现「以前不通过」，正文按 releases.md 原文陈述并标出出处。
    2. 「普通 `impl<T> Drop for Vec<T>` 会被 dropck 拒绝」沿用 Nomicon 的既有结论（其页给了 godbolt 链接）；本机无法构造等价自定义容器复现（裸指针不受借用检查跟踪，PhantomData 不产生借用区间）。
    3. 结构体型变推导（“invariance wins all conflicts”）用 Nomicon 表述 + 三个方向的赋值实验间接支撑，未对混合字段结构体做穷举验证。
    4. 报错输出的 `--> src/main.rs:行:列` 按该代码块自身行号对齐（与第 4 章同一呈现口径）；行号由逐块真跑取得，但路径做了 `src/main.rs` 归一。
  - 新术语（GLOSSARY 缺，建议协调者并入）：`variance | 型变（variance）`；`covariant | 协变`；`contravariant | 逆变`；`invariant | 不变`；`HRTB (higher-ranked trait bound) | 高阶 trait 约束（HRTB）`；`drop check / dropck | 析构检查（drop check）`；`sound generic drop | 健全的泛型析构`；`drop scope | 析构作用域`；`late-bound lifetime | 晚绑定生命周期`。正文 `型变（variance）` 只在首现（「这一章解决什么问题」的现象表）带英文，之后统一用「型变」；「variance」的中文名经协调者裁定统一为**型变**（初稿曾用「变型」，已按裁定全章替换，见末条修回记录）。
  - 邻章衔接：《34 手写集合》承接 `#[may_dangle]` 与 `_owns_T: PhantomData<T>`；`PhantomData<*const T>` 的 `!Send` 与《19 并发基础》的 Send/Sync 呼应；《22 Pin 与自引用》可复用本章 dropck 直觉；第 4 章的 `E0106` 与 NLL 已在正文回指（16.6 开头）；别名模型只字未提、留给《35》（在边界外）。
  - 图示需求（本轮按批次约定只用 `text` 块与表格，未自画 SVG）：①16.2 想要「三种型变的赋值方向」对比图（三个包装类型 + 可通过/不可通过箭头）；②16.4 想要「借用区间 vs 析构点」时间轴图（Inspector/World，说明为什么必须“严格”outlives）；③16.6 想要「省略规则决策树」（输入里生命周期个数 → 输出怎么填）。
  - 主动取舍：①未加「语言对照角」——C++/Java 的 variance 断言需核对各自一手文档，本轮离线无法取证，宁缺勿错；②报错现场做 4 个（dropck / `&mut` 不变 / 逆变+HRTB / `E0392`），`E0106`、`E0637`、`E0599` 不单开现场，改为在 16.3、16.6 就地给真实输出 + 章末速查表；③16.3 增补「两个输入时输出绑定关系」一类，并纠正了初稿里「函数指针类型别名必须手写 `for<'a>`」的错说法（省略规则本就能补出高阶形式）；④篇幅超出 6000 字上限约 5%，按票面「本章偏难，允许偏长」保留，协调者已裁定不删减（`Mixed` 示例与 trait-object 默认生命周期小节都是内容，不是水分）。

- 2026-10-18 修回（协调者两条裁定后）：
  1. **篇幅不删减**：正文保持 6336 汉字（不含代码块）。协调者认定 16.2 的 `Mixed` 示例与 16.6 的 trait-object 默认生命周期小节属内容而非水分，为凑字数砍理解增量不划算；票面原本就写明本章偏难允许偏长，故未执行初稿 Comments 里提的「可删项」。
  2. **术语统一为「型变（variance）」**：全文 `变型` → `型变`（改前 18 处；`### 16.2 型变：三个方向` 小标题、16.5 的 `PhantomData` 型变表表头 `| 写法 | 模拟的字段 | 型变 |`、小结与延伸阅读链接标签一并在内），目前 `变型` 残留 0 处；`型变（variance）` 只在首现（现象表）带英文注解，后续统一用「型变」（改前另有 2 处重复英文注解，已删）；协变/逆变/不变保持不变。顺带修掉两处替换后读起来别扭的措辞：「比直接声明型变更不容易出错」→「比『直接声明想要的型变』更不容易出错」、「定型变（…）」→「指定型变（…）」。
  3. **复跑**：`mdbook test -c ch16-lifetimes-advanced.md` → **exit 0**（术语替换只发生在叙述与表格里，不涉及代码块；片段与报错输出零改动）。

- 2026-10-18 交叉审校修回（RvCh16：1 blocker + 2 major + 5 minor；其余四轴通过、Nomicon/Reference 引文逐字核对无误）。逐条处理：
  1. **【blocker】练习 3 参考答案自相矛盾**（原答案让 `bag.holder` 借 `bag` 自己的字段 `bag.data`，实际报 `E0597`；而该块是 `rust,ignore`，门禁抓不到）。已按现场 1 的修法整段重写：题面 `struct Bag<'a> { holder: Option<Holder<'a>> }` **去掉 `data` 字段**，`data` 留在 `Bag` 外并声明在 `bag` 之前，再 `bag.holder = Some(Holder(&data));`；同时把题面块从 `rust,ignore` 改成 **`rust`**（现受 `mdbook test` 真跑），答案不再含独立代码片段，改为引现场 1 的 `World` 形态说明反例，逻辑方向已纠正（「`data` 在外且先声明 → 安全」；「挪回 `Bag` 内 → 报同一条 `E0597`；此形态下删 `impl Drop` 又能编译」）。手工复跑证据：同文本独立编译 + 运行 → exit 0，stdout `drop Holder(1)`，与答案「实测输出只有一行」一致；该块现计入受测块。
  2. **【major】术语违规**「调用两次不同**寿命**的字符串」→ 改「调用两次**生命周期**不同**的字符串」（GLOSSARY：生命周期 _Avoid_ 生存时间、寿命）。改后全文「寿命」出现 0 次。
  3. **【major】术语违规**「所以自己写**容器**时」→ 改「所以自己写**集合**、或写自家这类泛型类型时」（GLOSSARY：「集合」条 _Avoid_ 容器）。改后全文「容器」出现 0 次。
  4. **【minor】三层图第 3 行方向易误读** → 改为「3. 析构检查：被借用的数据必须严格比持有者活得久／（持有者的析构点要落在借用区间里）」，与 16.4 正文、小结的表述方向一致。
  5. **【minor】「释放」误用于 drop 语义** → `:116`（`world` 先被释放）、`:821`（`world` 在这里被释放却仍被借用）均改「析构」；`:471`「`Vec` 的析构只需要释放缓冲区」是内存释放，按审校意见保留。改后全文「释放」仅剩这 1 处。
  6. **【minor】源码行号** → 改为「本机 1.98：`struct Unique` 的定义从 `library/core/src/ptr/unique.rs:35` 开始，这条注释在 37–38 行」（35 行是 struct 定义行，注释在 37–38，已按本机 rust-src 复核）。
  7. **【minor】「drop glue」未解释** → 首现处补中文说明：「drop glue 指编译器自动生成的递归析构代码：一个类型即使没写 `Drop`，只要字段需要析构就带得有它」。全文「drop glue」仅此 1 处。
  8. **【minor】速查表 `E0599` 结论行不是真实报错文字** → 由 `method not found` 改为真实行 ``no method named `run` found for struct `FirstWord<…>` ``，并把「先去查什么」补上「/装进结构体字段了」。
  - 块数与计数变化：`rust` 33→**34**、`rust,ignore` 4→**2**（`compile_fail` 14、`text` 15 不变）；`mdbook test` 反证法实测计数 = **30 passed + 2 ignored**（注入失败探针时输出 `30 passed; 1 failed; 2 ignored`，探针随即删除）。
  - **复跑**：`mdbook test -c ch16-lifetimes-advanced.md` → **exit 0**（修回后最终稿）。修回只动正文与练习题面，两个 `snippets/ch16/*.rs` 与全部报错输出零改动。
  - 字数：6336 → **6497 汉字**（不含代码块），增量来自练习 3 的题面重写（含完整程序）；按协调者裁定保持不删减。

## Answer

2026-10-18 批次收尾（用户验收通过）。

- 交付：`src/ch16-lifetimes-advanced.md`（6497 汉字；八节模板 + 3 道练习）+ `snippets/ch16/`（`drop_order.rs`、`hrtb_callback.rs` 实跑输出与正文逐字一致）。
- 自验：`mdbook test -c ch16-lifetimes-advanced.md` → exit 0；注入失败反证 = 30 passed + 2 ignored；块结构：34 `rust` / 14 `compile_fail` / 15 `text` / 2 `ignore`。
- 交叉审校（RvCh16；**1 blocker** + 2 major + 5 minor）：①练习 3 参考答案自相矛盾（实测 E0597）→ 按现场 1 重写、块由 `ignore` 改 `rust` 真受门禁；②「寿命」「容器」违规清零、drop 义的「释放」改「析构」；③三层图方向、`unique.rs` 注释行号（37–38）、「drop glue」解释、速查表 E0599 文案。
- 术语并表（协调者）：型变 / 协变·逆变·不变 / HRTB / 析构检查 / 析构作用域 / 健全的泛型析构 / 晚绑定生命周期 等已入 GLOSSARY；「型变」全章统一（18 处）。
- 全量链（批次级）：`mdbook build` 0 ERROR/0 警告｜`mdbook test` 20 章 exit 0｜examples 19+2 passed。
- 遗留：`&mut` unsize 缩短仅证 1.98 可编译；`Vec` 的 `#[may_dangle]` 依赖 Nomicon 结论（票内记录）。
