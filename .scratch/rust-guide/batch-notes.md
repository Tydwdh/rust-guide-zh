# 基础篇批量写作 · 协调台账（票 09–21）

> 协调者维护；写作者只读不写。记录各票交付 / 裁定 / 新术语 / 图示需求 / 遗留 —— 供术语并表、08 图示决策与最终收尾。

## 已落地（4/13）

### 票 09 前言（`src/preface.md`）— 已裁定，退修中
- 交付：1258 汉字，六小节；自验 exit 0 + 负向对照（注入类型错误 → 101，还原后 md5 一致）。
- 裁定（按稿不动作）：① `unsafe` 首现不加括注（英文关键字，概念首现在第 23 章）；② 「包管理器」不算「包（package）」术语首现；③ Reference 定位语按 1.98 快照核实结果；④ 「语言对照角」保持可选承诺；⑤ 措辞不与 04 逐字对齐。
- 退修（已发）：① 「已经能写 Rust…从第 9 章往下读」歧义；② 隐藏行说明与渲染事实不符（详见「跨票勘误」）。
- 新术语：无。图示需求：无。

### 票 10 第 1 章 起步（`src/ch01-getting-started.md`）— 初稿到
- 交付：2789 汉字；无 snippets（示例内联）；自验 exit 0。
- 实测：rustup/rustc/cargo 版本、`cargo new` 全流程、check/run/build/--release、`printlin!` 报错全文、E0601、rustc 单文件。
- 取舍：耗时数字标注为并发写盘下的量级参考；路径噪声声明式压缩。
- 新术语（待并表）：**profile（构建配置）** —— 建议入表（`dev`/`release` 编译档位；_Avoid_: 配置文件）。
- 图示需求（转 08）：cargo 编排流程（横向管线）、rustup/cargo/rustc 三层分工、`target/debug` 树 + 可删缓存/Cargo.lock 标注、check/build/run 产出差异。
- 遗留：无阻塞项。

### 票 15 第 6 章 集合与字符串（`src/ch06-collections-strings.md`）— 初稿到
- 交付：3081 汉字；`snippets/ch06/word_count.rs`；自验 exit 0（15 用例）+ 负向对照。
- 亮点：全部输出真跑真粘；发现票面报错描述与 1.98 实测不符（「cannot index into a value of type String」实为 E0277 `the type str cannot be indexed by {integer}`，经 `Deref`），按实测写。
- 新术语（待并表）：集合（collection）、哈希映射（hash map）、容量（capacity）、重新分配（reallocation）、元素（element）、Unicode 标量值（Unicode scalar value）、字素簇（grapheme cluster）、entry API（保留英文）。
- 图示需求（转 08）：`"你好"` 6 字节布局 + 边界 + `&s[0..1]` 切点；Vec len/capacity 增长与搬移（4→8→16）；`str`/`&str`/`String` 与 `[T]`/`&[T]`/`Vec<T>` 并排。
- 遗留：Vec 越界 panic 未贴（控篇幅，接受）；1.98 panic 首行含 tid 已注明。

### 票 16 第 7 章 错误处理（`src/ch07-error-handling.md`）— 初稿到
- 交付：3368 汉字；`snippets/ch07/config_error.rs`；自验 exit 0（18 doctest：14 编译/运行 + 4 compile_fail）+ no_run 负向对照。
- 实测：`None.unwrap()` panic、`RUST_BACKTRACE` 帧 0–6、`main → Result` 的 Err 输出（exit 1）、Display/Debug/source 三视角、E0277 三处。
- 新术语（待并表）：展开（unwind，与「宏展开」区分）、错误链（error chain）、错误传播（error propagation）。
- 裁定：`Result`/`Option`/`unwrap`/`expect`/`ok_or`/`source` 不立条目（API 名，按代码原名保留）。
- 图示需求（转 08）：`?` 语义流程、`source()` 错误链指向、unwind-清理 vs abort 对比。
- 遗留：note 源码路径按 §2 剔噪（接受）。

## 跨票勘误与裁定

- **隐藏行机制（实测 mdbook 0.5.4）**：`# ` 行渲染为 `<span class="boring">`；`book.js` 对含隐藏行的代码块自动加 `hide-boring`（CSS `display:none`）并注入「Show hidden lines」眼睛按钮 —— **默认收起、可点开**。WRITING.md §1「渲染时自动隐藏」成立，已补「读者可展开」提醒。
- **linkcheck2 首次启用**（book.toml，2026-10-18）：clean 构建 exit 0；构建中途对 ch04/ch05 的 `[a..b]`/`<string>` 警告属写作中瞬时态（改稿后消失），最终链以落地态为准。
- **跨章承诺**：前言承诺第 9、10 章「C++/Java/Python 对照」→ 已转达两章落实（或回信给替代）。

## 待办（协调者）
- [ ] 其余 9 票落地即验（`mdbook test -c`）+ 全量链（build 扫 ERROR/WARN + `mdbook test` + `cargo test --manifest-path examples/Cargo.toml`）。
- [ ] 交叉审校：每章一个 reviewer 子代理（§3+§5）→ 分流修复。
- [ ] 术语并表进 `GLOSSARY.md`（本文件各「新术语」逐条 + profile 裁定）。
- [ ] 图示需求汇总 → 08 票定「图示体系」。
