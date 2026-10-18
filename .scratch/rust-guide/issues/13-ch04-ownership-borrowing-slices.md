# 13 第 4 章 所有权、借用与切片

Type: task
Status: resolved
Blocked by: 06

## Question

写基础篇第 4 章（全书心智模型核心章）：move 语义、借用与可变借用、引用有效期的初识、切片与 `String`/`&str`/`str`、常见报错与修法。

- 先回答「为什么」（对比 GC 语言与手动内存管理），再给规则；每条规则配「违反它时编译器怎么说」。
- 素材：01 §2.3（要写：切片 vs 切片引用、字符串与指针类型差异、move/Copy/Clone）；02 §5（易踩坑清单）。
- 允许偏长（核心章）；章末 2–3 道理解题（带答案）。DoD 见地图 Notes + 06。

## Comments

- 2026-10-18 初稿：文件 `src/ch04-ownership-borrowing-slices.md`（H1 = `# 第 4 章 所有权、借用与切片`；正文 CJK 5401，含代码注释的全文 CJK 6045）+ 片段 `snippets/ch04/word_stats.rs`（`snippets/ch04/` 下只有这一个 .rs）。
  自验：`mdbook test -c ch04-ownership-borrowing-slices.md` → **exit 0**（19 个 rust 块被编译执行、14 个 `compile_fail` 断言失败成功）。
  include 覆盖验证：把 `snippets/ch04/word_stats.rs` 末尾追加一句类型错误后，同一条命令立刻在该章 `动手看看 (line 651)` 报 `error[E0308]`；改回即 exit 0 —— 证明 `{{#rustdoc_include}}` 引入的片段确实被 `mdbook test` 编译。
  片段独立复现（不落产物到 snippets/）：`rustc --edition 2024 snippets/ch04/word_stats.rs -o .scratch/tmp-13/ws.exe && D:/Rust/.scratch/tmp-13/ws.exe`。
  实测输出（全部本机 1.98 / edition 2024；探针置于 `.scratch/tmp-13/pN/src/main.rs`，故报错路径显示为 `src/main.rs`）：E0382 `4:16`；E0499 `4:14`；E0502 `4:5`（`Vec` 版，报错现场）与 `4:5`（`String` 版，练习答案）；E0515 `3:5`；E0106 `1:18`；E0204 `2:8`；非字符边界 panic `3:17`（exit 101）；`String::from("hello")` len/cap = 5/5；尺寸表 String24 / Vec<u8>24 / &String8 / &Vec<u8>8 / &str16 / &[u8]16 / Box<str>16 / &[u8;4]8；`&line[6..11]` 与整串地址相差 6；drop 顺序（进入 consume→drop b→main 结束前→drop c→drop a）；赋值先 drop 旧值；word_stats 六行输出。
  遗留：无未核实事实、无待补章节。两点 mdBook 用法请协调者留意（我未改 WRITING.md）：(1) `{{#rustdoc_include ...}}` 必须写在 ```rust 围栏**里面** —— links 预处理器只把目标文件内容原样塞进指令位置，裸写会渲染成普通段落、且 `mdbook test` 不编译它（WRITING.md §1 示例看起来是裸写）；(2) 「`{{#include}}` 不参与 mdbook test」也不准确：围栏写对时 include 内容会被编译（已实测）。
  新术语（请并表）：移动（move）→ 移动；克隆（clone）→ 克隆；析构（drop）→ 析构；动态大小类型（DST, dynamically sized type）→ 动态大小类型；胖指针（fat pointer）→ 胖指针；瘦指针（thin pointer）→ 瘦指针；解引用强转（deref coercion）→ 解引用强转；借用区间（borrow region）→ 借用区间；别名规则（aliasing rules）→ 别名规则；悬垂引用（dangling reference）→ 悬垂引用；重复释放（double free）→ 重复释放；`Copy`/`Clone`/`Drop` 保留英文。
  邻章衔接：正文回指第 2 章（诊断流程）；预告第 6 章（UTF-8/字符串方法，2 处）、第 7 章（panic 机制）、第 9 章（impl/trait 与 `impl Drop`）、第 16 章（生命周期标注与 elision 完整规则）、第 17 章（智能指针、内部可变性）。已回复 `Ch09Generics` 的确认：「`Copy`/`Clone` 在第 4 章讲过，叫法一致」。
  图示需求（本批统一 text 块）→ ① §4.5「借用区间时间轴」（现为 text 图：重叠→E0502 / 不重叠→通过）——若 08 票上 mermaid，这处最值得升级；② §4.7「`String`/`Vec`（ptr+len+cap） vs `&str`/`&[T]`（ptr+len） vs `str`/`[T]`（DST）」内存布局图（现为 text ASCII）；③ §4.3「move = 释放责任交接」的所有者状态转移图。
  主动取舍：research/02 §5.2#4（2024 edition 起 `if let` scrutinee / 块尾表达式的临时值 drop 时机变化）**未**写进正文——它属于临时值作用域，本章只建立「出作用域即 drop」，为避免混淆留给第 5/7 章；如需在本章补一句请告知。E0382 的 `for x in v` 隐式移动变体只给了一句话（完整复现票外实测）。
- 2026-10-18 交叉审校（reviewer RwCh04，1 major + 5 minor）修完 5 项，第 6 项按协调者裁定不改：
  ① **major**：错误码归属修正——「先借 `&mut`、再直接用原变量」实际是 **E0505** 而非 E0382（本机复跑：`error[E0505]: cannot move out of `s` because it is borrowed` @ `src/main.rs:4:13`，并并列标出 borrow of `s` occurs here / move out of `s` occurs here）。原文该条从「E0382 变体」列表中拆出，改为 `> [!NOTE]` 并写明 E0505；报错码速查表**新增 `E0505` 行**，同时把 `E0382` 行的「（或已被借走）」删掉以划清界限。
  ② panic 输出首行补线程号：`thread 'main' (12345) panicked at src/main.rs:3:17:`，并加半句「`12345` 是线程号占位，每次运行不同」。
  ③ `&str` 条目「一次解引用就能读写」→「一次解引用就能直接访问字节（它是共享借用，只能读；写入要可变切片，如 `&mut [u8]`）」。
  ④ NLL 版本表述 → 「自 Rust 1.31（edition 2018）引入 NLL（non-lexical lifetimes）起」。
  ⑤ 尺寸表 `&String` 行补英文：瘦指针（thin pointer）（胖指针（fat pointer）已在 §4.7 内存图中首现）。
  复跑：`mdbook test -c ch04-ownership-borrowing-slices.md` → **exit 0**；正文 CJK 5460（仍 < 5500）；67 个围栏配平、0 裸栅栏；`E0505` 出现 2 处（NOTE + 速查表）。

## Answer

2026-10-18 批次收尾（用户验收通过）。

- 交付：`src/ch04-ownership-borrowing-slices.md`（5460 汉字，核心章）+ `snippets/ch04/word_stats.rs`。
- 自验：`mdbook test -c ch04-ownership-borrowing-slices.md` exit 0（19 执行块 + 14 `compile_fail`；include 覆盖做了负向实证）。
- 交叉审校 6 条：修 5（**E0505 归属修正并补进速查表**、panic 线程号、`&str` 只读、NLL 版本、瘦指针英文），裁定 1 不改（预览列表首现次序）。全量链：`mdbook build` 0 WARN/ERROR（含 linkcheck2）｜`mdbook test` exit 0（13 章）｜`cargo test --manifest-path examples/Cargo.toml` 19 + 2 passed。
- 遗留不确定：无未核实事实。
