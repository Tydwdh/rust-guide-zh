# 15 第 6 章 集合与字符串

Type: task
Status: claimed
Blocked by: 06

## Question

写基础篇第 6 章：Vec、String/&str 的日常用法、HashMap、迭代与常用 API、UTF-8 与索引陷阱。

- 素材：02 §5.2（字符串/索引坑）；01 §2.3（「各种字符串的区别」是实测读者卡点）。
- 心智模型：容量与增长；所有权在集合操作里的三种表现（借出 / 取出 / 消耗）。
- 章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：`src/ch06-collections-strings.md`（H1 = `# 第 6 章 集合与字符串`，八节模板 + 可选「语言对照角」）；`snippets/ch06/word_count.rs`（完整程序，经 `{{#rustdoc_include}}` 引入）。篇幅：汉字 3081 个（含中文标点 3693 个；统计脚本见下）。
- 自验：
  - `mdbook test -c ch06-collections-strings.md` → `INFO Testing chapter '第 6 章 集合与字符串': "ch06-collections-strings.md"`，exit=0（15 个用例全过）。
  - 负向验证门禁有效：临时在 `word_count.rs` 末尾加一行类型错误 → `test result: FAILED. 15 passed; 1 failed`、exit=101；随后已还原。
- 实测输出清单（全部在 rustc 1.98.0 / edition 2024 本机跑出）：
  1. Vec 扩容轨迹 `len=1 cap=4` / `len=5 cap=8` / `len=9 cap=16`（`Vec<i32>`）；`with_capacity(4)` → `0/4`、push 后 `1/4`。
  2. 索引 vs get：`v[1] = 20`、`v.get(1) = Some(20)`、`v.get(9) = None`。
  3. 三种遍历输出（`1 2 3` / `[10, 20, 30]` / `10 20 30`）。
  4. `"你好".len() = 6`、`chars().count() = 2`；`"héllo🦀".len() = 10`、`chars().count() = 6`；`as_bytes() = [228, 189, 160, 229, 165, 189]`。
  5. `char_indices()` 两次输出（你好：0→你、3→好；héllo🦀：0 h、1 é、3 l、4 l、5 o、6 🦀）。
  6. `&"你好"[0..1]` 运行期 panic：`end byte index 1 is not a char boundary; it is inside '你' (bytes 0..3 of string)`，exit=101，位置 `src/main.rs:3:18`。注意 1.98 的 panic 首行是 `thread 'main' (13728) panicked at …`（线程号每次不同；格式来自 `library/std/src/panicking.rs:273`），正文已注明。
  7. `s[0]` 编译错误：E0277「the type `str` cannot be indexed by `{integer}`」＋help「the trait `SliceIndex<str>` is not implemented for `{integer}`」＋note「you can use `.chars().nth()` or `.bytes().nth()`」。
  8. 练习 2 的 E0502 全文（`cannot borrow v as mutable because it is also borrowed as immutable`，`src/main.rs:4:5`）。
  9. HashMap：`old = Some(10), now = Some(20)`、`contains 红? false`、`红 = Some(5)`。
  10. HashMap 遍历顺序同一二进制四次全不同（正文贴了四行 keys 输出）。
  11. `String::from_utf8([228,189,160,229,165,189])` → `Ok("你好")`；`String::from_utf8(vec![0xff])` → `Err(FromUtf8Error { .. })`。
  12. `"你好".get(..1) = None`、`.get(..3) = Some("你")`、`.chars().nth(1) = Some('好')`。
  13. `snippets/ch06/word_count.rs` 输出：字节 55 / 字符 40 / 不同词 8 / `Rust: 2` `你好: 2` `brown: 1` / 最长词 `brown`。
- 遗留 / 与票面不符处：
  - 票里写的 compile_fail 报错是「cannot index into a value of type String」；1.98 实测是 E0277「the type `str` cannot be indexed by `{integer}`」（`String` 经 `Deref` 到 `str`）。正文按实测写。
  - 「首次 push 后容量 4」是 `Vec<i32>` 在本机的实测值（实现细节，RawVec 对元素 size==1 用 8），正文只作为「实测轨迹」陈述，未当作规范。
  - 没有贴 Vec 越界（`v[9]`）的 panic 输出（实测为 `index out of bounds: the len is 3 but the index is 9`）；本章报错现场只演示 String 侧，避免篇幅膨胀。
- 新术语（GLOSSARY.md 未收，请协调者统一并表）：集合（collection）、哈希映射/哈希表（hash map，正文只用类型名 `HashMap`）、容量（capacity）、扩容/重新分配（reallocation）、元素（element）、Unicode 标量值（Unicode scalar value）、字素簇（grapheme cluster）、entry API（保留英文）。
- 邻章衔接：
  - 显式回指：第 3 章（数组 `[T; N]`）、第 4 章（所有权 / 借用 / 切片 / `Drop`）、第 5 章（`Option` 与模式匹配）、第 7 章（panic vs `Result`；`String::from_utf8` 的 `Result`）、第 10 章（迭代器细节，三种遍历形态表可回收）、第 17 章（`Deref` / deref coercion）。
  - 第 7 章可回收本章的「`v.get` 返回 `Option`」「`insert` 返回旧值」；第 10 章可回收三种遍历表格。
- 图示需求（本批用 `text` 块顶替，待 08 票定体系后回填）：
  1. `"你好"` 的 6 字节布局 + 字符边界 + `&s[0..1]` 切点示意（正文现为 text 图）。
  2. Vec 的 len/capacity 增长与搬移三步图（4→8→16）。
  3. `str`/`&str`/`String` 与 `[T]`/`&[T]`/`Vec<T>` 的并排对照图。
- 主动取舍：
  - 迭代器链（`iter().map().collect()`）只用到、不展开，明确指向第 10 章；`sort_by` 同理。
  - 加了一节可选「语言对照角」（WRITING §4 允许），因为字符串/集合的跨语言差异是本章最大卖点；未引入 mermaid/SVG。
  - 习题 2 用 `rust,compile_fail`，让 `mdbook test` 正向断言它编译失败；练习题与答案不含会 panic 的普通 `rust` 块。
- 2026-10-18 交叉审校修回（reviewer RwCh06：1 major + 7 minor，只动本章文件）：7 条全部采纳。
  1. **major｜C++ 事实错误**：`std::string::substr` 在 `pos > size()` 时抛 `std::out_of_range`，不是「不检查」→ 改为「`operator[]` 完全不检查，`substr` 只在 `pos > size()` 时抛 `std::out_of_range`，两者都不关心 UTF-8 字符边界」。
  2. `vec![1, 2, 3]` 与「空 `Vec` 再 push」不是可观察等价 → 改为「内容等价，容量轨迹不同」，并把实测值写进正文：`vec![1, 2, 3].capacity() == 3`，空 `Vec` push 三次 `== 4`（本机 1.98 实测）。
  3. `push`/`pop` 不是严格 O(1) → 改「摊还 O(1)（偶尔那一次会触发扩容、搬 n 个元素）」。
  4. 迭代器产出类型写错：`HashMap<&str, u32>::iter()` 产出 `(&&str, &u32)`，不是 `(&str, &u32)` → 已改正，并说明 `map(|(w, n)| (*w, *n))` 解引用成 `(&str, u32)`。
  5. 「Unicode 标量值」首现补英文：`Unicode 标量值（Unicode scalar value）`；顺带给「语言对照角」里的码点/码元补 `code point` / `code unit`。
  6. 版本标注补齐：现场二定位路径段尾加「（以 Rust 1.98 为例；行号按你项目里的实际位置。）」；练习 2 答案的 E0502 输出块后加「（以 Rust 1.98 为例。）」。
  7. 「动手看看」词频注释补就近指向：「`iter()` / `map()` / `collect()` / `sort_by` 属于迭代器与闭包，第 10 章展开」。
  - 修回复验：`mdbook test -c ch06-collections-strings.md` → exit=0（15 个用例全过）；篇幅汉字 3171（含中文标点 3809），仍在 3000–4000 区间。
