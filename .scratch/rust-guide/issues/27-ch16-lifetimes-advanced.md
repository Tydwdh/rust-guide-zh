# 27 第 16 章 生命周期进阶：variance、HRTB 与 drop check

Type: task
Status: claimed

## Question

写第 16 章：**生命周期的形式化直觉** —— 子类型从哪来、`for<'a>` 怎么用、drop 时借用怎么查。

- **范围**：variance（协变 / 逆变 / 不变：`&'a T`、`&'a mut T`、`fn(T)`、`PhantomData` 各自；为什么需要不变）；HRTB（`for<'a> Fn(&'a str)` 语法与真实用例，如把闭包传给需要任意生命周期的函数）；drop check 与 drop 顺序（为什么「析构里还会用借用」要单独检查；`#[may_dangle]` 只提名字与用途）；`PhantomData` 的三重用法（占位、调方差、影响 drop check）；生命周期省略规则温故（第 4 章的直觉 → 精确规则表）。
- **素材**：`research/02`（I6）；nomicon 的 subtyping / variance / dropck / PhantomData 节；reference 的 lifetime elision 页。
- **边界**：智能指针 → 17；别名模型与形式化（Stacked/Tree Borrows）→ 35；`unsafe` 与 UB → 23。
- 篇幅 4500–6000 汉字（本章偏难，允许偏长）；八节模板；练习 2–3 题带答案；nightly-only 只标注。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch16-lifetimes-advanced.md`）。

## Comments

- 2026-10-18 初稿：文件 `src/ch16-lifetimes-advanced.md`（6332 汉字，不含代码块；八节模板齐全，练习 3 题带答案）+ 片段 `snippets/ch16/drop_order.rs`、`snippets/ch16/hrtb_callback.rs`；自验 `mdbook test -c ch16-lifetimes-advanced.md` → **exit 0**（33 个 `rust` 块 + 14 个 `rust,compile_fail` 块 + 15 个 `text` 输出块 + 4 个练习题面 `rust,ignore`）；门禁有效性另用「章末临时注入 `let _: u32 = "x";`」反证：exit 101、`1 failed`，探针随后删除；实测输出、遗留、新术语、衔接与图示需求见下列分组。
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
    3. 结构体变型推导（“invariance wins all conflicts”）用 Nomicon 表述 + 三个方向的赋值实验间接支撑，未对混合字段结构体做穷举验证。
    4. 报错输出的 `--> src/main.rs:行:列` 按该代码块自身行号对齐（与第 4 章同一呈现口径）；行号由逐块真跑取得，但路径做了 `src/main.rs` 归一。
  - 新术语（GLOSSARY 缺，建议协调者并入）：`variance | 变型（variance）`；`covariant | 协变`；`contravariant | 逆变`；`invariant | 不变`；`HRTB (higher-ranked trait bound) | 高阶 trait 约束（HRTB）`；`drop check / dropck | 析构检查（drop check）`；`sound generic drop | 健全的泛型析构`；`drop scope | 析构作用域`；`late-bound lifetime | 晚绑定生命周期`。正文首现处中英并列，后续以中文为主；「variance」的中文选了「变型」，若协调者改定「型变/方差」需全章替换（出现 1 次作为中文名 + 表格表头）。
  - 邻章衔接：《34 手写集合》承接 `#[may_dangle]` 与 `_owns_T: PhantomData<T>`；`PhantomData<*const T>` 的 `!Send` 与《19 并发基础》的 Send/Sync 呼应；《22 Pin 与自引用》可复用本章 dropck 直觉；第 4 章的 `E0106` 与 NLL 已在正文回指（16.6 开头）；别名模型只字未提、留给《35》（在边界外）。
  - 图示需求（本轮按批次约定只用 `text` 块与表格，未自画 SVG）：①16.2 想要「三种变型的赋值方向」对比图（三个包装类型 + 可通过/不可通过箭头）；②16.4 想要「借用区间 vs 析构点」时间轴图（Inspector/World，说明为什么必须“严格”outlives）；③16.6 想要「省略规则决策树」（输入里生命周期个数 → 输出怎么填）。
  - 主动取舍：①未加「语言对照角」——C++/Java 的 variance 断言需核对各自一手文档，本轮离线无法取证，宁缺勿错；②报错现场做 4 个（dropck / `&mut` 不变 / 逆变+HRTB / `E0392`），`E0106`、`E0637`、`E0599` 不单开现场，改为在 16.3、16.6 就地给真实输出 + 章末速查表；③16.3 增补「两个输入时输出绑定关系」一类，并纠正了初稿里「函数指针类型别名必须手写 `for<'a>`」的错说法（省略规则本就能补出高阶形式）；④篇幅 6332 汉字（不含代码块）略超 6000 上限约 5%，票面已注明本章偏难允许偏长；如需压到 6000 以内，可删 16.2 的 `Mixed` 示例与 16.6 的 trait object 默认生命周期小节（各约 150–200 字）。
