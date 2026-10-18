# 24 第 13 章 dyn 兼容性与 trait 对象

Type: task
Status: claimed

## Question

写第 13 章：**`dyn` 与 trait 对象** —— 动态分发的规则、代价与选型。

- **范围**：对象安全（哪些 trait 能变成 `dyn`，哪些不能；`Self: Sized` 手法；方法与接收者的限制）；`dyn Trait` 的三种持有形式（`&dyn`/`Box<dyn>`/`Arc<dyn>`）与各自场景；vtable 是什么（对照第 9 章的单态化：动态分发的代价在哪）；upcasting（`dyn Sub → dyn Super`，按 1.98 事实写）；`dyn` vs 泛型 vs `impl Trait` 的选型清单；`Box<dyn Error>`/`&dyn Iterator` 的实用例。
- **素材**：`research/02`（I2 条目与 §5）；`reference/items/trait-objects.html`；nomicon 的 vtable 节。
- **边界**：RPIT/AFIT → 14；API 设计 → 18；`unsafe` 里 vtable 布局细节 → 25/26。
- 篇幅 4000–5500 汉字；八节模板；练习 2–3 题带答案；nightly-only 只标注；可选「语言对照角」（Java 接口 / C++ 虚函数）。
- DoD：按 `WRITING.md` §8 批次协议（自验 `mdbook test -c ch13-dyn-trait-objects.md`）。

## Comments

- 2026-10-18 初稿：文件 `src/ch13-dyn-trait-objects.md`（H1 `# 第 13 章 dyn 兼容性与 trait 对象`，5106 汉字，八节模板 + 可选「语言对照角」）；片段 `snippets/ch13/{shapes,catalogue,errors,iterators,dispatch}.rs`（5 个完整可跑程序，全部以 `{{#rustdoc_include}}` 引入正文并被测）。
- 自验：`mdbook test -c ch13-dyn-trait-objects.md` → **exit 0**。门禁有效性另做真跑确认：临时往章末插一个坏块 → rustdoc 报 `17 tests`、`1 failed`（E0425）、exit 101；revert 后复绿。明细：17 个块中 15 pass、1 ignored（练习里的 `rust,ignore` 代码片段）、3 个 `compile_fail`（E0038 / E0191 / map-on-trait-object）全部按预期编译失败。
- 实测输出清单（`rustc --edition 2024`，本机 Windows x86_64 / 1.98.0；探针与临时编译原在 `.scratch/tmp-24/`，收尾已删）：
  1. 5 个片段真跑输出已逐字贴进正文：`shapes`（面积 + `&dyn Shape` 16 字节 + `size_of_val` 读出 8/0 + `dyn Any` 下转型）、`catalogue`（5 种 upcasting 形式）、`errors`（`?` 装箱 / `downcast_ref` / `Send + Sync` 版）、`iterators`、`dispatch`。
  2. LLVM IR（`rustc -C opt-level=0 --emit=llvm-ir`）：`@vtable.1`/`@vtable.2` 各一份且形如 `<{ [24 x i8], ptr }>`；`run_dyn` 体内 `getelementptr … i64 24` → `load ptr` → 间接 `call i32 %1`；`run_generic::<AddOne>`/`<TimesTwo>` 两份单态化副本 + 直接调用；调用点里 `run_dyn` 多收一个 `@vtable.1` 实参。正文引用为逐字摘录（长行未改）。
  3. 报错实测（关键行已贴正文，文件名与复现命令一致）：E0038 六类（泛型方法、supertrait 链、`trait X: Sized`、关联常量、`Self` 返回、无接收者关联函数、`async fn`、RPIT）、E0191（`dyn Iterator` 缺 `Item`）、E0599（`where Self: Sized` 的方法在 `dyn` 上调用）、无错误码的 ``the `map` method cannot be invoked on a trait object``（指向 std 源码 `iterator.rs:833`）、E0605（非 supertrait 方向强转）、E0371（手写 `impl Shape for dyn Shape`）。
  4. upcasting 版本事实：`releases.md` Version 1.86.0 (2025-04-03)“Stabilize upcasting trait objects to supertraits.” + Reference `[coerce.unsize.trait-upcast]` 规则原文；1.98 实测 `&`/`&mut`/`Box`/`Arc`/`Rc`、隐式强转、`dyn Pet + Send → dyn Animal`（丢 auto trait）全部通过。
  5. 标准库源码核对：`alloc/src/boxed/convert.rs:493/525`（`Box<dyn Error>` 的 `From`）、`core/src/iter/traits/iterator.rs:831-834`（`map` 的 `where Self: Sized`）与 `:4237`（`impl<I: Iterator + ?Sized> Iterator for &mut I`）、`alloc/src/boxed/iter.rs:18`（`Box<I>: Iterator`）、`core/src/clone.rs:194` + std 文档（`pub trait Clone: Sized`）。
- 遗留不确定：
  1. vtable 头部 24 字节内部字段的顺序与含义**未**在正文断言（只写「头部元数据」，并注明属实现细节、布局留第 25 章）；IR 里的 `i64 24` 仅用于支持「方法槽在头部之后、每方法一格」。
  2. 「`async_fn_in_dyn_trait` 在 1.98 仍不稳定」取自 `research/02` §4.3，本票未自行跑 nightly 探针。
  3. `Box<I>: Iterator` 的实际签名是 `impl<I: Iterator + ?Sized, A: Allocator> Iterator for Box<I, A>`；正文按 `Box<I>` 表述（分配器参数属 std 内部，与 `allocator_api` 无关）。
  4. `errors.exe` 输出含 Windows 本地化系统消息（`系统找不到指定的文件。 (os error 2)`），按本机实况粘贴；非 Windows 读者会看到英文版本。
  5. 正文 IR 行号（10/19/28/162）与 mangled 名取自本机 `dispatch.ll`，换工具链版本会变（正文已说明）。
  6. 未实测 `Pin<P>` 接收者的分发，仅按 Reference 列入「可分发接收者」清单。
- 新术语（English | 建议中文）：`trait object` | trait 对象；`dyn compatibility` | dyn 兼容性；`object safety` | 对象安全（旧名，仅作历史说明）；`upcasting` | 向上转型；`downcast(ing)` | 向下转型；`vtable` | 建议保留英文；`auto trait` | 建议保留英文；`dyn` | 保留英文。（`fat pointer`/胖指针、`DST`/动态大小类型、`monomorphization`/单态化、`opaque type`/不透明类型已在表内，按表用。）
- 邻章衔接：第 9 章（单态化、`Box<dyn Display>` 预告）→ 本章接上并回指；第 12 章（并行票 23）已把「object safety → dyn 兼容性、upcasting 自 1.86」留给本章，措辞一致；第 14 章（RPIT/AFIT 不能 dyn）、第 15 章（GAT 不能 dyn）、第 18 章（`source()` 链与错误类型设计）、第 19 章（`Send`/`Sync`）、第 21/22 章（`Pin` + async）、第 25 章（vtable 布局）均已在正文以「（第 N 章展开）」指路；第 17 章「`Box<dyn Trait>` 同样是胖指针……（第 13 章）」一句由本章接上。
- 图示需求（本轮正文统一用 `text` 块；SVG 交票 22）：
  1. **胖指针 + vtable 布局图**（正文「vtable 与三种持有形式」，优先级最高）：`&dyn Shape` 两个机器字 → 数据指针指向具体值、vtable 指针指向「头部元数据 + 每方法一格」，两个具体类型各一张 vtable。
  2. **泛型 vs dyn 代码路径对照图**（正文「两条多态路线的账」）：单态化 = 3 份代码 + 直接调用（可内联）；dyn = 1 份代码 + vtable 取址 + 间接调用（不可内联）。
  3. **向上转型图**（正文「向上转型（1.86 起）」）：`dyn Pet → dyn Animal` 数据指针不变、只换 vtable 视图；旁标「横向（`dyn Shape` → `dyn Named`）不合法」。
- 2026-10-18 linkcheck2 处置（协调者全量构建报警）：`src/ch13-dyn-trait-objects.md` 第 68 行引 C++ 草案条款编号时写成 `*[class.virtual]*`，被 mdbook-linkcheck2 判为 `Potential incomplete link`。已改为行内代码形式 `` `[class.virtual]` ``（协调者建议的第二种改法）。全章核查后确认：其余方括号都在栅栏或行内代码里——IR 摘录的 `[24 x i8]`、程序输出的 `[100, 101, 102]` / `[2, 4, 6, 8, 10]`、报错行的 `error[E0038]` / `error[E0191]`，以及 admonition 标记 `[!NOTE]` / `[!WARNING]`，都不构成潜在链接。复跑 `mdbook test -c ch13-dyn-trait-objects.md` → 仍 **exit 0**。
