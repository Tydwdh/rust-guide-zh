# 示例代码（实战项目工作区）

本书的**实战项目**都放在这个工作区里（章节内的小片段不放这里 —— 示例组织的约定见 `WRITING.md` §1）。工作区根就是本目录的 `Cargo.toml`（`resolver = "3"`，成员见 `members`）。

| crate | 对应内容 | 入口 |
|---|---|---|
| [`todo-cli`](todo-cli/) | 项目 1（基础篇收尾，第 11 章之后） | `cargo run -p todo-cli -- --help` |
| [`expr-eval`](expr-eval/) | 项目 2（类型与 trait 主干篇收尾，第 18 章之后） | `cargo run -p expr-eval -- --help` |
| [`task-queue`](task-queue/) | 项目 3（并发与异步篇收尾，第 22 章之后） | `cargo run -p task-queue --example demo` |

在本目录（`examples/`）里跑：

```console
$ cargo test                                  # 工作区全部测试
$ cargo run -p todo-cli -- --help             # 用法
```

或者留在仓库根、不改 cwd：

```console
$ cargo test --manifest-path examples/Cargo.toml
$ cargo run --manifest-path examples/Cargo.toml -p todo-cli -- --help
```

后续各篇的项目（2–5）会陆续加进这个工作区；每个 crate 的写法与它对应的章节见上表。
