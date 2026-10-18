# 第 8 章 模块、包与依赖

## 这一章解决什么问题

第 1 到第 7 章的程序都写在一个文件里。真实项目撑不了多久就会变形：几百行堆在 `main.rs`，两个用途不同的函数重名，想把其中一块代码复用到下一个项目只能复制粘贴。

Rust 用三层结构解决这个问题：crate 是编译单元，包（package）是发布单元，模块（module）是 crate 内部的命名空间（namespace）与可见性（visibility）节点。这一章把这三层讲清楚，再回到 `Cargo.toml`，把「用别人的代码」这件事做完。

读完这一章你能：

- 把单文件程序拆成多文件、多模块，并清楚每一个名字谁看得见；
- 用 `pub` / `pub(crate)` 划出「对外 API」和「内部实现」的界线；
- 读懂 `Cargo.toml` 的 `[package]` 与 `[dependencies]`，知道 `Cargo.lock` 在解决什么，并给项目加上第一个外部依赖（dependency）。

## 为什么这样设计

多数语言的可见性默认是宽松的：Java 成员默认包内可见，Python 用下划线表示「约定别碰」（解释器不管），C 用 `static` 限制文件作用域但跨文件重名要到链接期才打架。它们的边界靠约定、文档或工具维持。

Rust 的默认值相反：**一切私有**，想让外面用，你就显式写 `pub`。这样设计换来三件事：

1. 边界由编译器静态检查，越界直接编译不过 —— 不需要运行时反射，也不靠文档纪律。
2. 编译器知道哪些项没有被外部用到，就敢做跨 crate 的优化（第 29 章）。
3. 没有 `pub` 的东西天然不是 API，改它不算破坏兼容。

crate 是这套边界的硬墙：跨 crate 只能用 `pub` 的东西；而且从 crate 根到你打算用的那一项，路径上每一环都必须可见 —— 中间断一环，这一项对外就是私有的。第 12 章设计 trait API、第 18 章谈 SemVer 时都会回到这条规则。crate 同时是编译的最小单位：改一个 crate 只会重编它自己和依赖它的下游，这也是构建缓存与并行编译的粒度（第 29 章讲剖析时会看到这条边界的成本）。

三个词先钉死，本书始终按这个用法：

| 词 | 是什么 | 典型载体 |
|---|---|---|
| crate | 编译单元：编译器一次处理的最小单位 | `src/main.rs`（二进制）或 `src/lib.rs`（库） |
| 包（package） | 发布单元：一个 `Cargo.toml` 描述的一组 crate | crates.io 上的一条记录 |
| 模块（module） | crate 内部的命名空间，也是可见性的作用域 | `mod foo;` |

> [!NOTE]
> 中文里「包」常被含糊地用来指 crate（「这个包里的函数」）。本书严格区分：说 crate 就是编译单元，说包就是 Cargo 的 package。crate 一词不翻译。

## 机制与规则

### crate：编译单元

`cargo new demo` 生成：

```text
demo/
├── Cargo.toml
└── src/
    └── main.rs
```

`src/main.rs` 是 **crate 根（crate root）**：编译器从这里出发，把整个 crate 编译成一个二进制。把 `main.rs` 换成 `lib.rs`（用 `cargo new --lib demo`）就得到库 crate —— 它没有 `main`，存在的意义是让别人 `use`。

一个包可以由多个 crate 组成：

- `src/lib.rs`（至多一个）：库 crate，名字默认等于包名；
- `src/main.rs`（可选）：与包同名的二进制 crate；
- `src/bin/*.rs`（可选）：每个文件一个额外的二进制 crate；
- `tests/`、`examples/`（可选）：各自独立编译的 crate（第 11 章讲测试布局）。

同一个包里的二进制可以直接使用本包的库，按 crate 名 `use`，不需要写路径依赖。包名里的 `-` 在库 crate 名里会变成 `_`（Cargo 参考：库目标名默认是包名把连字符替换成下划线），所以 `[package] name = "my-tool"` 对应代码里的 `use my_tool::...`。

### module：crate 内的命名空间

`mod` 有两种写法。内联写法：

```rust
mod geometry {
    pub struct Point {
        pub x: f64,
        pub y: f64,
    }

    impl Point {
        pub fn new(x: f64, y: f64) -> Self {
            Self { x, y }
        }

        pub fn norm(&self) -> f64 {
            (self.x * self.x + self.y * self.y).sqrt()
        }
    }
}

fn main() {
    let p = geometry::Point::new(3.0, 4.0);
    println!("|p| = {}", p.norm());
}
```

```console
$ cargo run
|p| = 5
```

外置写法是 `mod catalog;` —— 注意结尾的分号，没有花括号。编译器会去**同名文件**里找模块内容。查找规则是固定的（Reference「Modules」）：祖先模块名当目录，模块名加 `.rs`。

| 模块路径 | 默认文件 |
|---|---|
| crate | `src/main.rs` 或 `src/lib.rs` |
| `crate::catalog` | `src/catalog.rs` |
| `crate::catalog::items` | `src/catalog/items.rs` |

于是下面这棵树是合法的：

```text
shop/
├── main.rs             // mod catalog;    → 定义 crate::catalog
├── catalog.rs          // pub mod items;  → 定义 crate::catalog::items
└── catalog/
    └── items.rs        // 文件内容 = 模块 items
```

关键点在于：**`items.rs` 放在 `catalog/` 目录下，是因为 `mod items;` 写在 `catalog.rs` 里，而不是因为名字。** 把 `mod items;` 挪到 `main.rs`，文件就必须挪到 `src/items.rs`。这是「文件位置」类报错（E0583）的头号原因。

另一种同样合法的布局是 `mod.rs` 风格：

```text
shop/
├── main.rs
└── catalog/
    ├── mod.rs          // pub mod items; → 定义 crate::catalog
    └── items.rs
```

两种布局功能完全等价。新代码推荐前一种：文件名就是模块名（`catalog.rs` 里住着 `catalog`），编辑器和 `grep` 都更好找；`mod.rs` 写法下同一目录会堆满同名 `mod.rs`，跳转时只能靠标签页区分。

> [!TIP]
> 模块树最好和领域结构对齐，而不是和「类型 / 工具 / 常量」这类技术分类对齐。`catalog::items::Product` 比 `models::Product` 更能说明产品属于哪块业务。第 12 章设计库 API 时会再回到这条建议。

### 可见性：默认私有

私有 = 「本模块及其后代可见」。只有两个例外：`pub trait` 的关联项默认公开，`pub enum` 的变体默认公开。

`pub` 的语义比直觉窄：**一个 `pub` 项能从外部访问，前提是路径上所有祖先模块都可见**。本章第一个报错例子就是这条规则。

给成员加限定词可以精确控制范围：

| 写法 | 谁可见 |
|---|---|
| （不写） | 本模块及其后代 |
| `pub(crate)` | 整个 crate |
| `pub(super)` | 父模块及其后代，等价 `pub(in super)` |
| `pub(self)` | 本模块，完全等价于不写 |
| `pub(in crate::a)` | 路径 `crate::a` 及其后代 |
| `pub` | 从 crate 根一路可达时，其他 crate 也能用 |

`pub(in path)` 的规则比看起来严（Reference「Visibility and privacy」）：路径必须是**祖先模块**，每一段都必须直接指向模块（不能是 `use` 起的别名），2018 起必须以 `crate` / `self` / `super` 开头。

实践原则是**最小可见性**：先不写 `pub`，等编译器报错再放大。字段尤其如此。第 5 章见过 `struct` 的字段默认私有，外部要造一个只能走构造函数：

```rust
mod inventory {
    pub struct Item {
        pub name: String,
        cost: u32, // 私有字段：只有本模块能读
    }

    impl Item {
        pub fn new(name: &str, cost: u32) -> Self {
            Self { name: name.to_owned(), cost }
        }

        pub fn cost(&self) -> u32 {
            self.cost
        }
    }
}

fn main() {
    let item = inventory::Item::new("键盘", 199);
    println!("{}: {}", item.name, item.cost());
}
```

`pub(crate)` 是最常用的中间档：它把「内部工具」和「对外 API」分开。下面的 `normalize` 是内部实现，别的 crate 看不到：

```rust
mod text {
    pub(crate) fn normalize(s: &str) -> String {
        s.trim().to_lowercase()
    }

    pub fn key(s: &str) -> String {
        self::normalize(s) // 同模块内也可以直接写 normalize(s)
    }
}

fn main() {
    println!("key = {}", text::key("  Rust  "));
}
```

```console
$ cargo run
key = rust
```

### 路径与 use

路径有四种开头：

- `crate::` —— 从本 crate 根开始，最稳，重构时不用数 `super`；
- `self::` —— 从当前模块开始；
- `super::` —— 从父模块开始，可以叠（`super::super::`）；
- 裸标识符 —— 先在当前模块的作用域里找，找不到再当成外部 crate 名（extern prelude）。

```rust
mod server {
    pub struct Config {
        pub port: u16,
    }

    pub mod net {
        use super::Config; // 父模块 server

        pub fn bind(cfg: &Config) -> String {
            format!("127.0.0.1:{}", cfg.port)
        }
    }
}

fn main() {
    let cfg = server::Config { port: 8080 };
    println!("{}", server::net::bind(&cfg));
}
```

`use` 声明把一条路径绑定成当前模块里的名字，默认和非 `pub` 项一样私有。它支持几种写法：

```rust
mod net {
    pub mod http {
        pub fn get(url: &str) -> String {
            format!("GET {url}")
        }

        pub fn post(url: &str) -> String {
            format!("POST {url}")
        }
    }
}

use crate::net::http::{self, get}; // 一次引入父模块和其中一项

fn main() {
    println!("{}", get("/index"));
    println!("{}", http::post("/index"));
}
```

花括号里还可以写 `self`（引入父模块本身）与 `*`（glob，慎用：会引入一堆名字，容易撞名）。想改名就用 `as`，比如 `use crate::net::http::get as http_get;` —— 名字换成领域词汇后，调用点会更好读。

`pub use` 是**重导出（re-export）**：把一个名字在自己的模块里再公开一次。库常用它搭门面（facade），让用户不必知道内部层级：

```rust
mod engine {
    pub mod parser {
        pub fn parse(text: &str) -> usize {
            text.len()
        }
    }
}

pub use engine::parser::parse as parse_len;

fn main() {
    println!("len = {}", parse_len("hello"));
}
```

> [!WARNING]
> `use` 路径不是「从 crate 根开始」的，这是 2018 之后仍然常踩的坑。在子模块里写 `use net::http;`（`net` 是 crate 根下的兄弟模块）会报 E0432：

```rust,compile_fail
mod net {
    pub mod http {
        pub fn get() -> &'static str {
            "ok"
        }
    }
}

mod client {
    use net::http;

    pub fn ping() -> &'static str {
        http::get()
    }
}

fn main() {
    println!("{}", client::ping());
}
```

```text
error[E0432]: unresolved import `net`
   |
10 |     use net::http;
   |         ^^^
   |
   = note: `use` statements changed in Rust 2018; read more at
           <https://doc.rust-lang.org/edition-guide/rust-2018/module-system/path-clarity.html>
help: a similar path exists
   |
10 |     use crate::net::http;
   |         +++++++
```

改法是 `use crate::net::http;`（或 `use super::net::http;`）。经验规则：**`use` 写绝对路径（`crate::` 开头），表达式写相对路径 + `super::`**。

另外两点常被误解：

- `use` 只是编译期的名字绑定，不在运行时「加载」任何东西，也不产生机器码。删掉它、改成全路径写，行为完全一样，只是更啰嗦。
- 2015 时代的 `extern crate foo;` 声明在 2018 之后基本可以删掉，crate 名直接可用（迁移细节见第 33 章）。

### Cargo.toml 与依赖

`Cargo.toml` 是包的清单。开工只需要这几项：

```toml
[package]
name = "dep-demo"
version = "0.1.0"
edition = "2024"

[dependencies]
itoa = "1.0.18"
```

最后一行是下面 `cargo add` 写进去的。逐项看：

- `name`：包名，发布到 crates.io 时必须唯一；同时决定库 crate 名（`-` 换成 `_`）。
- `version`：用语义化版本（SemVer）三段式 `主.次.补`；`1.0.0-alpha` 这类预发布段也允许。
- `edition`：语言版本，不是编译器版本。本书基线 2024；`cargo new` 会写当前最新 edition，上面这份就是 1.98 的 `cargo new` 生成的。它影响包内所有 target，不写则按 2015 处理（第 33 章展开）。
- `[dependencies]`：键是依赖在 crates.io 上的包名，值是版本要求（`cargo add` 会替你写对名字）。

版本要求默认是 caret（`^`）语义：`itoa = "1.0.18"` 等价 `^1.0.18`，表示 `>=1.0.18, <2.0.0`。所以它允许 `1.0.19` 甚至 `1.9.0`，但不会被 `2.0.0` 满足；要更紧就写 `=1.0.18`（精确）、`~1.0.18`（只允许补丁位变化）。crates.io 拒绝 `*` 这种「什么版本都行」的写法（Cargo FAQ）。

`cargo build` 还会生成 `Cargo.lock`，记录这次构建实际用到的**确切**版本：

```toml
# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "dep-demo"
version = "0.1.0"
dependencies = [
 "itoa",
]

[[package]]
name = "itoa"
version = "1.0.18"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682"
```

它的作用是把「这一次成功的构建」冻结下来：换台机器、换个时间，装到的依赖版本完全相同，CI 不会因为上游发新版而莫名变红（Cargo FAQ「Why have Cargo.lock in version control?」）。`cargo new` 默认把它纳入版本控制，提交与否取决于你的需要 —— 关键事实是 **`Cargo.lock` 只影响你自己这个包的构建，不影响依赖你的下游**：下游用自己那份 lockfile 和你的 `Cargo.toml` 重新解析。升级依赖用 `cargo update`，它改的是 lockfile，不是 `Cargo.toml`。

依赖的来源默认是 crates.io；`[dependencies]` 也支持 `path`（本地目录）、`git`（仓库）等来源（Cargo 参考「Specifying Dependencies」）。

两个大主题这里只预告：**features**（用 `[features]` 把可选依赖和可选功能做成开关）和 **workspace**（一个仓库里放多个包，共享 `target/`、统一 `cargo build`）—— 都在第 30 章。

### 语言对照角

- Java：包是命名空间加一层弱隔离（`package-private`），成员默认包内可见。Rust 反过来：默认私有，并且能用 `pub(in crate::a)` 这种细粒度授权把可见性限制到具体子树。
- Python：`_name` 只是约定，没人拦你；Rust 的私有是编译错误。另外 Python 的 `import` 在运行时执行模块代码，Rust 的 `use` 纯粹是编译期名字绑定。
- C++20：modules 主要解决头文件包含带来的编译时间与 ODR 问题；Rust 的模块负责命名空间与可见性，编译单元仍然是 crate 这一层。

## 动手看看

### 一个真正的多文件程序

目录结构（crate 根是 `main.rs`）：

```text
shop/
├── main.rs
├── catalog.rs
└── catalog/
    └── items.rs
```

`main.rs` —— 声明 `catalog` 并使用它：

```rust,ignore
{{#rustdoc_include ../snippets/ch08/shop/main.rs}}
```

`catalog.rs` —— 模块 `catalog`，用 `pub mod items;` 声明子模块，并保证数据结构只能通过公开 API 修改：

```rust,ignore
{{#rustdoc_include ../snippets/ch08/shop/catalog.rs}}
```

`catalog/items.rs` —— 子模块 `items`：

```rust,ignore
{{#rustdoc_include ../snippets/ch08/shop/catalog/items.rs}}
```

注意 `catalog.rs` 里的 `use self::items::Product;`：`items` 是 `catalog` 的子模块，用 `self::` 明确「从当前模块出发」；写 `use crate::catalog::items::Product;` 等价。

编译运行（入口是 `main.rs`，rustc 顺着 `mod` 声明去找相邻位置的 `catalog.rs`）：

```console
$ rustc --edition 2024 main.rs -o shop
$ ./shop        # Windows 下是 shop.exe
2 件商品，合计 288 元
- 键盘：199 元
- 鼠标：89 元
```

完整代码在仓库的 `snippets/ch08/shop/`，可以照上面两条命令直接跑。

### 加上一个依赖

```console
$ cargo new dep-demo --vcs none
    Creating binary (application) `dep-demo` package
$ cd dep-demo
$ cargo add itoa --offline
      Adding itoa v1.0.18 to dependencies
             Features:
             - no-panic
     Locking 1 package to latest Rust 1.98.0 compatible version
```

> [!NOTE]
> 本书写作环境没有外网，这条 `cargo add` 带了 `--offline`（依赖来自本机缓存）。联网时去掉该参数即可，关键行相同。注意最后一行：1.84 起 Cargo 的解析器（resolver v3）会把当前 Rust 版本纳入考量，挑「与该工具链兼容的最新版本」（MSRV 相关，第 30 章）。

`cargo add` 改的是 `Cargo.toml`（就是本节开头那份），同时把该 crate 可用的 feature 列给你。接着构建：

```console
$ cargo build --offline
   Compiling itoa v1.0.18
   Compiling dep-demo v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.25s
```

（第二行本机输出还带包目录路径，这里省去。）用起来和标准库没有区别，crate 名就是路径首段：

```rust,ignore
fn main() {
    let mut buf = itoa::Buffer::new();
    println!("{}", buf.format(2026_u32));
}
```

```console
$ cargo run --offline --quiet
2026
```

上面这个块标了 `ignore`，因为它要用到外部 crate。本书正文里参与校验的代码块只依赖标准库，这样你在任何机器上都能直接编译，不用先配依赖。

## 报错现场

下面三条都是模块系统里最常见的错。输出取自 rustc 1.98.0（Windows），只保留关键行；本机路径用 `<crate 目录>` 替代。

### 1. 私有模块：E0603

```rust,compile_fail
mod outer {
    mod inner {
        pub fn greet() {
            println!("hi");
        }
    }
}

fn main() {
    outer::inner::greet();
}
```

```text
error[E0603]: module `inner` is private
   |
10 |     outer::inner::greet();
   |            ^^^^^  ----- function `greet` is not publicly re-exported
   |            |
   |            private module
   |
note: the module `inner` is defined here
   |
 2 |     mod inner {
   |     ^^^^^^^^^
```

定位路径：错误指向的 `inner` 就是断掉的那一环 —— `greet` 标了 `pub`，但上一层的 `inner` 没标，所以整个项对外不可见。编译器甚至把「函数不是公开重导出的」也点了出来。

修法二选一：确实要对外暴露，就给 `mod inner` 加 `pub`（然后继续检查它上面每一层）；只是本模块内部用，就把调用挪进 `outer` 里 —— 私有项的可见范围是「本模块及其后代」。

限定可见性的变体报同一类错。把函数改成 `pub(in crate::a)` 后从 crate 根调用：

```rust,compile_fail
mod a {
    pub mod b {
        pub mod c {
            pub(in crate::a) fn secret() -> u32 {
                42
            }
        }
    }
}

fn main() {
    println!("{}", a::b::c::secret());
}
```

```text
error[E0603]: function `secret` is private
   |
12 |     println!("{}", a::b::c::secret());
   |                             ^^^^^^ private function
   |
note: the function `secret` is defined here
   |
 4 |             pub(in crate::a) fn secret() -> u32 {
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

### 2. 路径首段不存在：E0433

```rust,compile_fail
fn main() {
    helpers::run();
}
```

```text
error[E0433]: cannot find module or crate `helpers` in this scope
   |
 2 |     helpers::run();
   |     ^^^^^^^ use of unresolved module or unlinked crate `helpers`
   |
   = help: you might be missing a crate named `helpers`
```

定位路径：`helpers` 既不在当前作用域里，也不是外部 crate 名。最常见的两种起因是拼错了模块名，以及**忘了 `mod helpers;` 声明**（文件写好了但没告诉编译器它的存在）。

修法：补上声明，或改成正确的名字。这里要分清「文件存在」和「模块存在」是两件事 —— 模块只由 `mod` 声明产生。

### 3. 声明了模块却没有文件：E0583

```rust,compile_fail
mod stats;

fn main() {
    stats::mean(&[1, 2, 3]);
}
```

```text
error[E0583]: file not found for module `stats`
   |
 1 | mod stats;
   | ^^^^^^^^^^
   |
   = help: to create the module `stats`, create file "<crate 目录>\stats.rs" or "<crate 目录>\stats\mod.rs"
   = note: if there is a `mod stats` elsewhere in the crate already, import it with `use crate::...` instead
```

定位路径：编译器根据 `mod stats;` 的位置算出应该在哪找文件，报错里的路径就是答案。它列了两个候选：`stats.rs` 和 `stats/mod.rs`（上一节的两种布局）。

嵌套模块更容易搞错位置。`main.rs` 里写 `mod util;`、`util.rs` 里再写 `mod config;`，那么 `config` 的文件必须在 `util/` 目录下：

```text
src/
├── main.rs      // mod util;
├── util.rs      // mod config;  → 找 src/util/config.rs
└── util/
    └── config.rs
```

```text
error[E0583]: file not found for module `config`
   |
 1 | mod config;
   | ^^^^^^^^^^^
   |
   = help: to create the module `config`, create file "<crate 目录>\util\config.rs" or "<crate 目录>\util\config\mod.rs"
```

如果文件已放在 `src/config.rs`（crate 根下），那就说明它被当成了根模块：要么移动文件，要么在 `main.rs` 里写 `mod config;`，再在 `util.rs` 里用 `use crate::config;` 引入。（如果 crate 里还有其他地方访问 `util::config::…`，还会连带报一条 E0603 —— 那是上一条规则在起作用。）

> [!TIP]
> 这三条报错都能用 `rustc --explain E0603` 之类的命令查官方解释与更多例子（第 2 章讲过怎么用）。

## 小结

- **三个层次各司其职**：crate 是编译单元（`main.rs` / `lib.rs` 是根），包是 `Cargo.toml` 描述的发布单元，模块是 crate 内的命名空间与可见性节点。
- **默认私有，`pub` 要显式写**。`pub` 项要对 crate 外可见，路径上所有祖先模块都必须可见；`pub(crate)` / `pub(super)` / `pub(in crate::a)` 用来做折中。
- **`mod` 决定文件位置**：祖先模块名是目录，模块名加 `.rs`；`foo.rs` + `foo/bar.rs` 与 `foo/mod.rs` 布局等价，新代码用前者。
- **路径**：`crate::` / `self::` / `super::` 加裸名（外部 crate）。`use` 是编译期改名，`pub use` 是重导出；`use` 的裸首段从当前模块开始找，不是从 crate 根。
- **Cargo.toml 三项就够开工**：`name` / `version` / `edition`（基线 2024），依赖写在 `[dependencies]`，版本要求默认按 caret 语义。
- **`Cargo.lock` 冻结确切版本**，让构建可复现；它不影响下游，是否提交看你这个包的性质。

## 练习

1. 下面三个调用哪些编译不过？为什么？

   ```rust,compile_fail
   mod util {
       pub mod math {
           pub fn double(x: u32) -> u32 {
               x * 2
           }
           pub(crate) fn triple(x: u32) -> u32 {
               x * 3
           }
       }

       mod hidden {
           pub fn secret() -> u32 {
               7
           }
       }
   }

   fn main() {
       println!("{}", util::math::double(2));  // A
       println!("{}", util::math::triple(2));  // B
       println!("{}", util::hidden::secret()); // C
   }
   ```

2. 有人把文件放成了这样，`cargo build` 报 E0583（找不到 `config`）。错在哪，怎么改？

   ```text
   src/
   ├── main.rs      // mod util;
   ├── util.rs      // mod config;
   └── config.rs
   ```

3. 下面的代码报 E0432（`unresolved import`，指 `net` 这一名称）。解释原因，并给出两种改法。

   ```rust,compile_fail
   mod net {
       pub mod http {
           pub fn get() -> &'static str {
               "ok"
           }
       }
   }

   mod client {
       use net::http;

       pub fn ping() -> &'static str {
           http::get()
       }
   }
   ```

<details><summary>参考答案</summary>

**1.** 只有 C 编译不过。

- A 通过：`double` 是 `pub`，`util::math` 每一层从 crate 根过去都可见（`util` 虽然是私有模块，但 `main` 就在 crate 根，私有项对它可见）。
- B 通过：`triple` 是 `pub(crate)`，整个 crate 内都能调用。
- C 失败：`hidden` 是 `util` 的私有子模块，`main` 不在 `util` 的后代里，从 crate 根看不到它 —— 报 E0603（提示 `hidden` 这个模块是私有的）。注意 `secret` 自己标了 `pub`，但那救不了它：祖先链断了一环。

**2.** `util.rs` 里的 `mod config;` 表示 `config` 是模块 `util` 的子模块，编译器据此去找 `src/util/config.rs`（祖先模块名当目录），而文件被放在 crate 根下，于是报 E0583。两种改法：

- 把文件挪到 `src/util/config.rs`（`mod` 声明不动）；
- 或者让 `config` 成为 crate 根的模块：在 `main.rs` 里写 `mod config;`，`util.rs` 里改成 `use crate::config;`。

**3.** 2018 起 `use` 是「统一路径」：首段在当前模块的作用域里解析，找不到才去外部 crate 里找。`client` 模块里没有叫 `net` 的名字（`net` 是 crate 根下的兄弟模块），外部也没有 `net` 这个 crate，于是报 E0432。改法：

- `use crate::net::http;`（推荐，绝对路径）；
- `use super::net::http;`（`client` 的父模块就是 crate 根）。

</details>

## 延伸阅读

- The Book 第 7 章「Managing Growing Projects with Packages, Crates, and Modules」：<https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html>
- Reference「Visibility and privacy」：<https://doc.rust-lang.org/reference/visibility-and-privacy.html>
- Reference「Paths」：<https://doc.rust-lang.org/reference/paths.html>
- Reference「Modules」（文件查找规则）：<https://doc.rust-lang.org/reference/items/modules.html>
- Reference「Use declarations」（`use` 的全部写法）：<https://doc.rust-lang.org/reference/items/use-declarations.html>
- Cargo Book「The Manifest Format」：<https://doc.rust-lang.org/cargo/reference/manifest.html>
- Cargo Book「Specifying Dependencies」（版本要求语法）：<https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>
- Cargo Book「Cargo Targets」（库目标名与连字符）：<https://doc.rust-lang.org/cargo/reference/cargo-targets.html>
- Cargo Book FAQ（`Cargo.lock` 与版本控制）：<https://doc.rust-lang.org/cargo/faq.html>
- Edition Guide「Path clarity」（`use` 路径的 2018 变化）：<https://doc.rust-lang.org/edition-guide/rust-2018/module-system/path-clarity.html>
