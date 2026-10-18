# 前言

这本书讲 Rust，从零开始，一直到你能写出结构清楚、改得动、读得懂的程序。它不假设你会 Rust，也不假设你系统学过泛型（generics）、迭代器（iterator）或编译期概念——这些在书里现场建立。

主线不是「语法清单」，而是**理解设计**：为什么 Rust 把所有权（ownership）、借用（borrow）、生命周期（lifetime）做成编译期规则，这些规则换来了什么，代价又落在哪里。

## 这本书写给谁

起点线：会至少一门主流语言（哪门不限），用过命令行和包管理器，对函数、类型、错误处理不陌生。

本书**不**要求你：

- 会 Rust——第 1 章从安装工具链、写出第一个程序开始；
- 懂泛型、迭代器、闭包（closure）、编译期检查——第 9、10 章现场建立，配 C++/Java/Python 的对照；
- 懂所有权、借用、生命周期——第 4 章用一个心智模型一次讲透，第二篇再往深走。

你已经会写 Rust 的话：第 1、3 章快速翻过，第 2、4 章照读（诊断训练与所有权模型是主干）；第 5–8 章按需跳读，第 9 章起正常往下。若你完全没写过程序，这本书太陡。

## 这本书承诺什么

读完你应该能：

- 解释 Rust 几个核心设计为什么长这样（含底层模型）：所有权与移动、借用与生命周期、编译期检查的边界、零成本抽象（zero-cost abstraction）到底「零」在哪；
- 独立写、读、改自己的 Rust 程序，卡住时知道该查什么；
- 读懂编译器报错，从报错回到真正的问题点。

诊断训练（读报错、定位、改对）不是附加练习，而是贯穿全书的方法：第 2 章拆解报错的结构，此后每章都有「报错现场」，带你走一遍定位路径。

## 讲法

四条约定，全书写法都从这儿来。

1. **心智模型先行**。先讲「为什么这样设计」，再给规则——规则只有在你清楚它挡了什么时才记得住。
2. **诊断贯穿**。每个概念都配上它典型的失败现场，从编译器输出反推问题；性能与并发问题也按这条路径定位。
3. **示例可验证**。绝大多数示例进自动检查：能跑的会真的被编译运行，故意报错的会被断言「必须编译失败」；少数标 `rust,ignore` 的片段（外部依赖、多文件）不编译。
4. **自包含**。不假设你读过别的书；需要时说清楚，章末「延伸阅读」只给官方一手链接。

## 怎么用这本书

- **顺序**：第一篇（第 1–11 章 + 项目 1）按顺序读，后面的内容依赖前面建立的心智模型；第二篇起也按目录顺序推进，第六篇是补充章节，可以跳读。
- **动手**：每章有「动手看看」和带答案的练习；先自己写再看答案，差别很大。
- **每章的骨架**固定为：这一章解决什么问题 → 为什么这样设计 → 机制与规则 → 动手看看 → 报错现场 → 小结 → 练习 → 延伸阅读。跨语言差异大的地方还会加「语言对照角」，写给带着别的语言习惯的你。

## 示例怎么跑，报错怎么看

书里的示例以 stable 的 Rust 为准：写这本书时是 **1.98.0**，语言 edition 为 **2024**（非编译器版本，1.85 起稳定）。单个文件这样跑（Windows 上把 `./main` 换成 `.\main`）：

```console
$ rustc --edition 2024 main.rs
$ ./main
```

带依赖的项目用 Cargo（第 1 章建工程，第 8 章讲 Cargo）。输出以 Windows 上的运行为准，平台差异会标注；只在 nightly 上可用的特性一律用警告框标出（仅 nightly）。

普通代码块可以直接复制去跑：

```rust
fn main() {
    let greeting = "你好";
    println!("{greeting}, Rust");
}
```

有些片段带几行默认收起的辅助行（源码里以 `# ` 开头，点代码块的眼睛按钮展开），只为让片段保持完整、能编译；完整文件见仓库该章的源文件或 `snippets/`。

讲报错时，本书给两样东西：一个**最小复现**（源码里标了 `compile_fail`，本来就编不过），加一段**真实输出的关键几行**（标注编译器版本）。例如：

```rust,compile_fail
fn main() {
    let s = String::from("hello");
    let t = s;                  // 所有权转移到 t
    println!("{s}");            // 错误：s 已经被移走
    println!("{t}");
}
```

以 Rust 1.98.0 为例，关键几行是：

```text
error[E0382]: borrow of moved value: `s`
 --> main.rs:4:16
  |
3 |     let t = s;                  // 所有权转移到 t
  |             - value moved here
4 |     println!("{s}");            // 错误：s 已经被移走
  |                ^ value borrowed here after move
```

为什么报错、每个词指什么，第 2、4 章会带你逐行拆开；这类错误不是意外，而是教学内容。

## 一手资料在哪

这本书自包含，但不会替你重复官方文档；查证时优先看这几处（工具链自带离线副本，`rustup doc --book` 等命令可打开；`--std`、`--nomicon`、`--reference`、`--cargo`、`--edition-guide`、`--error-codes` 同理）：

| 资料 | 地址 | 定位 |
|---|---|---|
| The Book（The Rust Programming Language） | <https://doc.rust-lang.org/book/> | 主线教程，讲到并发与 unsafe |
| The Rust Reference | <https://doc.rust-lang.org/reference/> | 语言规则的权威条目（语法、语义……），供查证 |
| The Rustonomicon | <https://doc.rust-lang.org/nomicon/> | unsafe Rust 深水读物，讲未定义行为（UB, undefined behavior） |
| 标准库文档（std） | <https://doc.rust-lang.org/std/> | 类型、方法与 trait（行为契约），查 API 首选 |
| The Cargo Book | <https://doc.rust-lang.org/cargo/> | 依赖与 workspace、构建、发布 |
| The Rust Edition Guide | <https://doc.rust-lang.org/edition-guide/> | edition 改了什么、怎么迁移 |
| Error Codes Index | <https://doc.rust-lang.org/error_codes/> | `error[E0382]` 编号的解释页 |

准备就绪后，翻到第 1 章。
