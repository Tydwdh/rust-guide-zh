# 第 6 章 集合与字符串

## 这一章解决什么问题

前五章处理的都是「一个值」：一个整数、一个结构体、一个枚举。真实程序几乎不会只处理一个值——它要读入一列数字、一段文本、一批键值对。这一章讲三种最常用的集合（collection）：

- `Vec<T>`：可增长的连续序列；
- `String`：可增长的 UTF-8 字符串；
- `HashMap<K, V>`：键值映射。

读完这一章，你能：

- 用 `Vec` 增删、读取元素，并说清 `v[i]` 与 `v.get(i)` 的区别；
- 说清 `String`、`&str`、`str` 三者的关系，知道什么时候用哪个；
- 理解为什么 `s[0]` 写不出来、`&s[0..1]` 会 panic，以及正确的「按字符」遍历方式；
- 用 `HashMap` 完成计数、词频这类小统计。

## 为什么这样设计

很多语言里，越界访问要么返回垃圾、要么抛异常；Rust 把「可能取不到」这件事**写进类型**。于是同一个「取第 i 个元素」给了你两个 API：

- `v[i]`：你确信不会越界时用，越界就 panic（不可恢复错误，第 7 章讨论什么时候接受它）；
- `v.get(i)`：返回 `Option<&T>`，把「可能没有」交给类型系统，你必须用模式匹配处理（第 5 章）。

另一个贯穿全章的心智模型：**集合拥有它的元素**（所有权，第 4 章）。所以每一次集合操作都在回答「元素的所有权怎么了」：

| 你想做的事 | 写法 | 元素所有权 |
|---|---|---|
| 看一眼 | `&v[i]`、`v.get(i)`、`for x in &v` | 借出，集合还在 |
| 取出一个 | `v.pop()`、`v.remove(i)` | 交给你，集合少一个 |
| 消耗整个集合 | `for x in v`、`v.into_iter()` | 集合被转移，之后不能再用 |

迭代器的完整规则在第 10 章展开；这里先建立「借出 / 取出 / 消耗」三种形态的直觉。

再补一句和其他语言的对照：Python 的 `list` 只是指向堆上缓冲区的引用，`String` 在 Java 里也是引用。Rust 的 `Vec<T>`/`String` 是**值**——它自己拥有那块堆内存，离开作用域时自动释放（第 4 章讲过的 `Drop`）。所以把它们赋给别人时是移动，要复制得显式说 `clone()`。

字符串还多了一层决定：Rust 的字符串是 **UTF-8 字节序列**，不是「字符数组」。选 UTF-8 是为了和现实世界对齐——文件、网络、系统调用交给你的都是字节流，而 UTF-8 是 ASCII 的严格超集：纯英文文本时，它和 C 的字符串逐字节相同，不需要任何转换。代价是「取第 i 个字符」不再是免费的 O(1) 操作，你必须显式选择按字节还是按字符来数。下面「字符串是字节序列」一节就是围着这个取舍展开的。

## 机制与规则

### Vec：一条会长的连续表

```rust
fn main() {
    let mut v: Vec<i32> = Vec::new(); // 空表
    v.push(10);
    v.push(20);
    v.push(30);
    println!("{v:?}");
    println!("len = {}", v.len());

    let last = v.pop(); // 取出末尾元素，所有权给你
    println!("pop -> {last:?}, v = {v:?}");

    let a = vec![1, 2, 3]; // 宏：带初值的表
    println!("{a:?}");
}
```

`vec![1, 2, 3]` 是标准库宏，用来写「带初值的表」——内容上等价于建一个空 `Vec` 再依次 `push`，但容量轨迹不同：`vec![1, 2, 3].capacity()` 是 3，而空 `Vec` push 三次后容量是 4。真跑输出：

```text
[10, 20, 30]
len = 3
pop -> Some(30), v = [10, 20]
[1, 2, 3]
```

注意 `pop()` 返回的是 `Option<T>`——表可能本来就是空的，这个类型把「空」也覆盖了，不需要额外判断。

**索引还是 get**，这是 `Vec` 唯一的「危险点」：

```rust
fn main() {
    let v = vec![10, 20, 30];

    println!("v[1] = {}", v[1]); // 确定不越界：拿到值本身（这里会复制）

    match v.get(1) {
        Some(x) => println!("v.get(1) = Some({x})"),
        None => println!("v.get(1) = None"),
    }
    println!("v.get(9) = {:?}", v.get(9)); // 越界：None，不 panic

    let mut sum = 0;
    for x in &v {
        sum += x;
    }
    println!("sum = {sum}");
}
```

```text
v[1] = 20
v.get(1) = Some(20)
v.get(9) = None
sum = 60
```

`v[1]` 之所以能直接打印，是因为 `i32` 是 `Copy` 的：`v[1]` 取出的是值的一份拷贝。如果元素是 `String` 这类不可 `Copy` 的类型，`v[1]` 会尝试移动，编译器不允许——那时要写 `&v[1]` 借用，或者 `v[1].clone()` 复制（第 4 章）。

**容量（capacity）**：`len()` 是已有元素数，`capacity()` 是已分配、不重新分配内存就能放下的元素数。两者通常不等：一旦 `push` 超出容量，`Vec` 会分配一块更大的内存、把元素搬过去（reallocation）。实测 `Vec<i32>` 的扩容轨迹：

```text
len = 1, capacity = 4
len = 5, capacity = 8
len = 9, capacity = 16
```

即首次 `push` 后容量是 4，之后每次翻倍。

下图把「len 追上 capacity」的那一步摊开 —— 申请更大的新块、搬移元素、释放旧块：

<svg viewBox="0 0 640 170" width="100%" role="img" aria-label="Vec 容量不足时的重新分配：申请更大的新块、把元素搬过去、释放旧块" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">Vec&lt;i32&gt; 容量不够时：申请更大的新块 → 搬移元素 → 释放旧块</text>
<g font-size="12" fill="currentColor">
<text x="40" y="48">旧块：capacity = 4，len = 4（已满）</text>
<text x="272" y="48">新块：capacity = 8（搬来 4 个，第 5 个刚 push）</text>
</g>
<g font-size="13" text-anchor="middle" fill="currentColor" font-family="monospace">
<rect x="40" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="59" y="76">10</text>
<rect x="80" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="99" y="76">20</text>
<rect x="120" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="139" y="76">30</text>
<rect x="160" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="179" y="76">40</text>
<rect x="272" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="291" y="76">10</text>
<rect x="312" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="331" y="76">20</text>
<rect x="352" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="371" y="76">30</text>
<rect x="392" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="411" y="76">40</text>
<rect x="432" y="56" width="38" height="30" fill="none" stroke="var(--links)" /><text x="451" y="76" fill="var(--links)">50</text>
<rect x="472" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.35" stroke-dasharray="3 3" />
<rect x="512" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.35" stroke-dasharray="3 3" />
<rect x="552" y="56" width="38" height="30" fill="none" stroke="currentColor" stroke-opacity="0.35" stroke-dasharray="3 3" />
</g>
<g stroke="currentColor" stroke-width="1.5" fill="none">
<line x1="206" y1="71" x2="256" y2="71" />
<polygon points="266,71 254,66 254,76" fill="currentColor" stroke="none" />
</g>
<text x="232" y="100" font-size="12" text-anchor="middle" fill="currentColor">push 时容量不够</text>
<text x="0" y="132" font-size="12" fill="currentColor">容量轨迹：4 → 8 → 16 —— len 每次追上 capacity 就重新分配一次（把 n 个元素搬到新块）</text>
<text x="0" y="152" font-size="12" fill="currentColor" opacity="0.7">规模已知时先 Vec::with_capacity(n) 预分配，就能免掉这些搬移（下面就是它的实测）</text>
</svg>

所以如果你知道最终大概有多少元素（读文件、按行收集），一句 `Vec::with_capacity(n)` 预分配能省掉反复搬移：

```rust
fn main() {
    let mut v = Vec::with_capacity(4);
    println!("len = {}, capacity = {}", v.len(), v.capacity()); // 0, 4
    v.push(1);
    println!("len = {}, capacity = {}", v.len(), v.capacity()); // 1, 4
}
```

> [!NOTE]
> 「`push` 可能重新分配」正是你不能一边持有 `&v[0]`、一边 `v.push(...)` 的原因：借用规则（第 4 章）在这里顺手救了你一把。

遍历 `Vec` 有三种写法，产出不同（迭代器细节第 10 章展开）：

```rust
fn main() {
    let mut v = vec![1, 2, 3];
    for x in &v {
        println!("{x}"); // x: &i32，借出，v 还能用
    }
    for x in &mut v {
        *x *= 10; // x: &mut i32，可变借用
    }
    println!("{v:?}");
    for x in v {
        println!("{x}"); // x: i32，消耗：v 之后不能再用
    }
}
```

```text
1
2
3
[10, 20, 30]
10
20
30
```

顺带记住成本：`push`/`pop` 作用在末尾，是摊还 O(1)（偶尔那一次会触发扩容、搬 n 个元素）；`insert(i, x)` 和 `remove(i)` 作用在中间，后面的元素要整体前移或后移，是 O(n)。中间插删频繁时，`Vec` 是错误的容器。

**`Vec` 还是数组？**长度在编译期已知、也不需要增长，用 `[T; N]`（第 3 章）；长度运行时才知道，或者需要增删，用 `Vec<T>`。

### String：UTF-8 字符串的拥有者

第 4 章的三件套在字符串上就是：

| 类型 | 是什么 | 对应 |
|---|---|---|
| `str` | 字符串内容本身（长度不定的 DST） | `[T]` |
| `&str` | 对一段 UTF-8 字节的借用视图 | `&[T]` |
| `String` | 拥有、可增长的字符串 | `Vec<T>` |

下图把这三对摊开对照 —— 左列是字符串，右列是泛型序列，同一行对应同一种角色：

<svg viewBox="0 0 640 250" width="100%" role="img" aria-label="String、&amp;str、str 与 Vec、&amp;[T]、[T] 的一一对应：拥有、借用、内容" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">字符串三件套 ↔ 泛型序列三件套：谁拥有、谁借用、谁是内容</text>
<g fill="none" stroke="currentColor" stroke-opacity="0.55">
<rect x="24" y="34" width="276" height="52" rx="4" />
<rect x="340" y="34" width="276" height="52" rx="4" />
<rect x="24" y="104" width="276" height="52" rx="4" />
<rect x="340" y="104" width="276" height="52" rx="4" />
<rect x="24" y="174" width="276" height="52" rx="4" />
<rect x="340" y="174" width="276" height="52" rx="4" />
</g>
<g font-size="13" fill="currentColor" font-family="monospace">
<text x="36" y="56">String</text>
<text x="352" y="56">Vec&lt;T&gt;</text>
<text x="36" y="126">&amp;str</text>
<text x="352" y="126">&amp;[T]</text>
<text x="36" y="196">str</text>
<text x="352" y="196">[T]</text>
</g>
<g font-size="12" fill="currentColor">
<text x="36" y="76">ptr · len · cap：拥有堆上的 UTF-8 字节</text>
<text x="352" y="76">ptr · len · cap：拥有堆上的连续元素</text>
<text x="36" y="146">ptr + 长度：借看别人的字节（胖指针）</text>
<text x="352" y="146">ptr + 个数：借看别人的元素（胖指针）</text>
<text x="36" y="216">内容本身：长度不定的 DST（借来才能用）</text>
<text x="352" y="216">内容本身：长度不定的 DST（借来才能用）</text>
</g>
<g font-size="14" text-anchor="middle" fill="currentColor" opacity="0.7">
<text x="320" y="66">≡</text><text x="320" y="136">≡</text><text x="320" y="206">≡</text>
</g>
<text x="0" y="246" font-size="12" fill="currentColor" opacity="0.7">（64 位实测：String 与 Vec 各 24 字节 = ptr + len + cap，&amp;str 与 &amp;[T] 各 16 字节 = ptr + 长度 —— 第 4 章量过）</text>
</svg>

创建与增长：

```rust
fn main() {
    let mut s = String::new(); // 空串
    s.push_str("hello");
    s.push(' '); // 追加一个 char
    s.push_str("world");
    println!("{s}");

    let t = String::from("abc");
    let u = "abc".to_string(); // 和上一行等价
    println!("{}", t == u);
}
```

```text
hello world
true
```

**`+` 和 `format!`** 的区别在所有权：

```rust
fn main() {
    let a = String::from("foo");
    let b = String::from("bar");
    let c = a + &b; // a 被移动（消费），b 只是借出
    println!("{c}");
    // println!("{a}"); // 取消注释：borrow of moved value: `a`

    let d = format!("{b}-{c}-{}", 42); // 谁都不动，返回新 String
    println!("{d}");
}
```

```text
foobar
bar-foobar-42
```

记忆点：`+` 的实质是 `fn add(self, other: &str) -> String`——左边被吃掉，右边必须是 `&str`，所以要写 `&b` 而不是 `b`（`String + String` 写不出来）。要拼三段以上就用 `format!`，它一个都不消费，可读性也好。

**`&String → &str` 自动转换**：把 `&String` 传给要 `&str` 的参数时不用写 `.as_str()`，编译器会自动完成这一步：

```rust
fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

fn main() {
    let owned = String::from("hello world");
    println!("{}", first_word(&owned)); // &String 自动变成 &str
    println!("{}", first_word("hello world")); // 字面量也直接传
}
```

```text
hello
hello
```

这种自动转换叫解引用强制转换（deref coercion），第 17 章讲 `Deref` 时展开。现在只要记住结论：**函数参数要读字符串就写 `&str`，`String`、`&str`、字面量都能传进来**。

三者怎么选：

| 场景 | 选 | 理由 |
|---|---|---|
| 函数参数，只读 | `&str` | 谁都能传进来，不用转移所有权 |
| 需要拥有、需要增长、要返回新串 | `String` | 有所有权、可增长 |
| 字符串字面量 | `&'static str` | 编译期写进二进制，生命周期是 `'static` |

参数是一**串**字符串时，原则不变、往上抬一层：只读就用 `&[&str]`，要拥有、要存起来就用 `Vec<String>`。

### 字符串是字节序列：UTF-8 与索引陷阱（重点）

`String` 内部就是 `Vec<u8>`，并保证内容始终是合法的 UTF-8。也就是说，**字符数和字节数不是一回事**。`"你好"` 在内存里是 6 个字节：

```text
"你好"  ──▶  [228, 189, 160][229, 165, 189]
               ^──────^       ^──────^
               字符「你」      字符「好」
               字节 0..3       字节 3..6
```

由此推出三条必须记住的结论。

**一、`len()` 是字节数，不是字符数。**

```rust
fn main() {
    let s = "你好";
    println!("字节 = {}", s.len());
    println!("字符 = {}", s.chars().count());

    let e = "héllo🦀";
    println!("e 的字节 = {}, e 的字符 = {}", e.len(), e.chars().count());
}
```

```text
字节 = 6
字符 = 2
e 的字节 = 10, e 的字符 = 6
```

`é` 占 2 字节，`🦀` 占 4 字节。所以「限制用户名最多 20 个字」这样的需求，用 `len()` 判断是错的。

**二、`s[i]` 根本写不出来。**既然「第 i 个」既可能指字节也可能指字符，返回哪个都可能不合你的意，Rust 干脆不提供 `Index<usize>`，编译期就拦下（见「报错现场」现场二）。想要第 `i` 个字符就写 `s.chars().nth(i)`，返回的还是 `Option<char>`。The Book 还补了一条理由：Rust 约定索引操作是常数时间（O(1)），而 UTF-8 里定位第 i 个字符必须从头扫描、是 O(n)。与其给一个名不副实的 `[]` 语法，不如不给。

**三、按字节切片可能 panic。**`&s[a..b]` 里的 `a`、`b` 必须是**字符边界**，否则运行期 panic：

```rust,no_run
fn main() {
    let s = "你好";
    let ok = &s[0..3]; // 「你」正好 3 字节，合法
    println!("{ok}");
    let bad = &s[0..1]; // 切在「你」中间 —— 运行期 panic
    println!("{bad}");
}
```

真的想要「前 n 个字节，但如果 n 不是边界就返回 `None`」，用 `s.get(..n)`，它返回 `Option<&str>`：

```rust
fn main() {
    let s = "你好";
    println!("{:?}", s.get(..1)); // None：1 不是字符边界
    println!("{:?}", s.get(..3)); // Some("你")
}
```

```text
None
Some("你")
```

反过来也一样：`Vec<u8>` 不会自动变成 `String`。`String::from_utf8(bytes)` 会校验合法性并返回 `Result`（第 7 章）：对 `[228, 189, 160, 229, 165, 189]` 返回 `Ok("你好")`，对含非法字节的 `[0xff]` 返回 `Err(FromUtf8Error { .. })`。这就是「合法 UTF-8」这条保证的入口。

下图把这 6 个字节摊开 —— 每个汉字 3 个字节，字符边界只在 0、3、6 处：

<svg viewBox="0 0 640 212" width="100%" role="img" aria-label="“你好”的 UTF-8 字节布局、字符边界与非法切点" xmlns="http://www.w3.org/2000/svg">
<text x="0" y="16" font-size="14" fill="currentColor">“你好”的 UTF-8 字节布局：共 6 字节，字符边界只在 0、3、6</text>
<g font-size="12" text-anchor="middle" fill="currentColor">
<text x="100" y="44">0</text><text x="144" y="44">1</text><text x="188" y="44">2</text><text x="232" y="44">3</text><text x="276" y="44">4</text><text x="320" y="44">5</text>
</g>
<g font-size="13" text-anchor="middle" fill="currentColor" font-family="monospace">
<rect x="80" y="50" width="40" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="100" y="70">e4</text>
<rect x="124" y="50" width="40" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="144" y="70">bd</text>
<rect x="168" y="50" width="40" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="188" y="70">a0</text>
<rect x="212" y="50" width="40" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="232" y="70">e5</text>
<rect x="256" y="50" width="40" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="276" y="70">a5</text>
<rect x="300" y="50" width="40" height="30" fill="none" stroke="currentColor" stroke-opacity="0.55" /><text x="320" y="70">bd</text>
</g>
<g font-size="13" text-anchor="middle" fill="currentColor">
<text x="144" y="102">你（3 字节）</text>
<text x="276" y="102">好（3 字节）</text>
</g>
<g stroke="var(--links)" stroke-width="2">
<line x1="80" y1="112" x2="80" y2="124" /><line x1="210" y1="112" x2="210" y2="124" /><line x1="340" y1="112" x2="340" y2="124" />
</g>
<g font-size="12" text-anchor="middle" fill="var(--links)">
<text x="80" y="140">边界 0</text><text x="210" y="140">边界 3</text><text x="340" y="140">边界 6</text>
</g>
<g stroke="var(--warning-border)" stroke-width="2" stroke-dasharray="4 3">
<line x1="122" y1="38" x2="122" y2="92" />
</g>
<g font-size="12" fill="var(--warning-border)">
<text x="132" y="178">✗ &amp;s[0..1] 在字节 1 处切开 —— 不是字符边界，运行时会 panic</text>
</g>
<text x="0" y="200" font-size="12" fill="currentColor" opacity="0.7">（每个汉字占 3 个字节；&amp;s[0..3]、&amp;s[3..6] 才是完整字符）</text>
</svg>

三个遍历 API 的语义：

| 调用 | 每次产出 | 单位 | 用途 |
|---|---|---|---|
| `s.bytes()` | `u8` | 1 字节 | 处理原始字节（协议、校验、哈希） |
| `s.chars()` | `char` | 一个 Unicode 标量值（Unicode scalar value，1–4 字节） | 按字符处理：计数、翻转、逐字判断 |
| `s.char_indices()` | `(usize, char)` | — | 同时要字符和它的字节下标 |

```rust
fn main() {
    for (i, c) in "你好".char_indices() {
        println!("byte {i} -> {c}");
    }
    for (i, c) in "héllo🦀".char_indices() {
        println!("byte {i} -> {c}");
    }
}
```

```text
byte 0 -> 你
byte 3 -> 好
byte 0 -> h
byte 1 -> é
byte 3 -> l
byte 4 -> l
byte 5 -> o
byte 6 -> 🦀
```

注意 `é` 也占 2 字节，所以第二个 `l` 的下标直接跳到 3。

> [!WARNING]
> `char` 是「一个 Unicode 标量值」，不等于「用户眼里的一个字」。像 `é` 可能是 `e` 加一个组合重音两个 `char`，`👨‍👩‍👧` 这种 emoji 是若干个 `char` 拼成的字素簇（grapheme cluster）。要按「用户看到的字」切分得用第三方库；标准库的 `chars()` 只到标量值这一层。

### HashMap：按 key 找 value

```rust
use std::collections::HashMap;

fn main() {
    let mut scores: HashMap<String, u32> = HashMap::new();
    scores.insert(String::from("蓝"), 10);

    let old = scores.insert(String::from("蓝"), 20); // 返回被覆盖的旧值
    println!("old = {old:?}, now = {:?}", scores.get("蓝"));

    println!("contains 红? {}", scores.contains_key("红"));

    // entry API：不存在就插入 0，再拿着可变引用加一
    *scores.entry(String::from("红")).or_insert(0) += 5;
    println!("红 = {:?}", scores.get("红"));
}
```

```text
old = Some(10), now = Some(20)
contains 红? false
红 = Some(5)
```

要点：

- `insert` 返回 `Option<V>`：如果原来就有，给你旧值（`Some(10)`）；没有则是 `None`。想「只在不存在时插入」用 `entry(k).or_insert(v)`，返回 `&mut V`；
- `get(&k)` 返回 `Option<&V>`，和 `Vec::get` 一个套路；只判断存在性用 `contains_key`；
- **遍历顺序不确定**。`for (k, v) in &map` 的顺序每次运行都可能不同，同一个二进制实测四次：

```text
["h", "f", "c", "a", "b", "g", "e", "d"]
["b", "a", "f", "h", "c", "d", "g", "e"]
["d", "f", "h", "c", "e", "a", "b", "g"]
["h", "g", "c", "a", "b", "d", "e", "f"]
```

需要输出稳定就得自己排序（把 `(k, v)` 收进 `Vec` 再 `sort`）；需要 map 自己按 key 有序，用 `BTreeMap`——API 几乎一样，代价是插入稍慢。

`entry` 的另一半价值在「一个 key 对应一组值」时。比如按长度给单词分组：

```rust
use std::collections::HashMap;

fn main() {
    let words = ["a", "bb", "c", "dd"];
    let mut by_len: HashMap<usize, Vec<&str>> = HashMap::new();
    for w in words {
        by_len.entry(w.len()).or_default().push(w);
    }
    println!("{:?}", by_len.get(&2));
}
```

```text
Some(["bb", "dd"])
```

`or_default()` 在 key 不存在时插入 `Vec::new()`，再把这个 `Vec` 的可变引用交给你，于是 `push` 直接接在后面写。Java 里对应 `computeIfAbsent`，Python 里对应 `setdefault`——同一个套路。

## 动手看看

下面是一个完整程序，把这一章的东西揉在一起：统计一段混杂英文、中文和 emoji 的文本，完整代码在 `snippets/ch06/word_count.rs`。

```rust
{{#rustdoc_include ../snippets/ch06/word_count.rs}}
```

真跑输出：

```text
字节数 = 55
字符数 = 40
不同词数 = 8
Rust: 2
你好: 2
brown: 1
最长词（按字符数）= brown
```

几个值得注意的点：

- `text.len()` 是 55，`chars().count()` 是 40——差的 15 个字节全部来自中文和 emoji；
- 第 4 行起是排序后的词频前三名；因为排过序，这个输出是确定的；
- `counts.iter()` 借出 map（产出 `(&&str, &u32)`），`map(|(w, n)| (*w, *n))` 解引用成 `(&str, u32)`，然后 `sort_by` 先按次数降序、次数相同按词升序。`iter()` / `map()` / `collect()` / `sort_by` 属于迭代器与闭包，第 10 章展开。

## 报错现场

### 现场一：切片切在字符中间（运行期 panic）

```rust,no_run
fn main() {
    let s = String::from("你好");
    let head = &s[0..1];
    println!("{head}");
}
```

```text
thread 'main' (13728) panicked at src/main.rs:3:18:
end byte index 1 is not a char boundary; it is inside '你' (bytes 0..3 of string)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

定位路径：报错把话说得很直白——**字节 1 不是字符边界**，它落在字符「你」（占字节 0..3）内部。括号里是线程编号，每次运行不同（以 Rust 1.98 为例）。

修法：不要自己数字节。要第 0 个字符用 `s.chars().next()`；要切一段字节用 `s.get(0..1)`（返回 `Option<&str>`，不是边界就 `None`）；要按字符切用 `s.char_indices()` 得出的边界。

### 现场二：`s[0]`（编译期）

```rust,compile_fail
fn main() {
    let s = String::from("hello");
    let c = s[0];
    println!("{c}");
}
```

```text
error[E0277]: the type `str` cannot be indexed by `{integer}`
 --> src/main.rs:3:15
  |
3 |     let c = s[0];
  |               ^ string indices are ranges of `usize`
  |
  = help: the trait `SliceIndex<str>` is not implemented for `{integer}`
  = note: you can use `.chars().nth()` or `.bytes().nth()`
          for more information, see chapter 8 in The Book: <https://doc.rust-lang.org/book/ch08-02-strings.html#indexing-into-strings>
```

定位路径：`String` 借 `Deref` 解引用到 `str`，而 `str` 没有实现 `Index<usize>`（只有 `Index<Range<usize>>` 这类切片索引），所以编译器报 `str` 不能被整数索引。注意它是 `E0277`（trait 未实现），不是「越界」——越界是运行期的事。（以 Rust 1.98 为例；行号按你项目里的实际位置。）

修法：按字符取 `.chars().nth(0)`，按字节取 `.bytes().nth(0)`；按范围切片用 `&s[0..1]`（确保落在字符边界），更稳的写法是 `s.get(0..1)`——边界不合法时给你 `None` 而不是 panic。

## 小结

1. 集合拥有元素；每个操作先问「是借出、取出还是消耗」。
2. `v[i]` 越界 panic，`v.get(i)` 返回 `Option<&T>`——失败可预期时用后者。
3. `len()` 与 `capacity()` 是两回事；规模已知就 `Vec::with_capacity` 预分配。
4. `str`/`&str`/`String` 就是 `[T]`/`&[T]`/`Vec<T>` 的字符串版；参数写 `&str`，`+` 消费左边、借用右边，多段拼接用 `format!`。
5. 字符串是 UTF-8 字节序列：`len()` 数字节，`s[i]` 不存在，`&s[a..b]` 必须落在字符边界；按字符用 `chars()` / `char_indices()`。
6. `HashMap::insert` 返回旧值、遍历顺序不定；要顺序用 `BTreeMap`。

## 语言对照角

- **Python**：`len(s)` 是字符数，`s[0]` 给第一个字符（Python 3 的 `str` 是码点序列）。Rust 两个都不给你，因为 `str` 是 UTF-8 字节，你必须先声明「我按字节还是按字符」。
- **Java**：`String` 内部是 UTF-16 码元（code unit）序列，`s.charAt(0)` 给一个 `char`（UTF-16 码元），一个 emoji 要占两个。Rust 的 `char` 是完整的 Unicode 码点（code point，1–4 字节），比 Java 的 `char` 更「整」，但同样不等于「用户眼里的一个字」。
- **C++**：`std::string` 也是字节序列，`s[0]` 给首字节——字节视角与 Rust 相同。区别是 Rust 在类型上把「拥有」与「借用」分开（`String` 对 `&str`），切片还做边界检查（越界 panic）；C++ 的 `operator[]` 完全不检查，`substr` 只在 `pos > size()` 时抛 `std::out_of_range`，两者都不关心 UTF-8 字符边界。
- **Go**：`string` 也是 UTF-8 字节序列，`len(s)` 是字节数——与 Rust 一致，只有 `for range` 才按 rune 迭代。Rust 把这层区分做成了两个显式的 API：`bytes()` 与 `chars()`。
- **集合**：Python `list`、Java `ArrayList`、C++ `vector` 大致对应 `Vec<T>`；Rust 多给了一条路——越界可以 `[]` panic，也可以 `get` 拿 `Option`，把「可能失败」写进类型。

## 练习

1. 预测输出：

   ```rust
   fn main() {
       let s = String::from("héllo");
       println!("{}", s.len());
       println!("{}", s.chars().count());
   }
   ```

2. 下面这段代码能编译吗？如果不能，编译器会说什么？怎么改？

   ```rust,compile_fail
   fn main() {
       let mut v = vec![1, 2, 3];
       let first = &v[0];
       v.push(4);
       println!("{first}");
   }
   ```

3. 解释：为什么 `"你好"` 上 `&s[0..3]` 合法而 `&s[0..1]` 会 panic？如果你确实想让 `&s[0..1]` 这种情况「要么成功要么告诉我没有」，代码怎么写？

<details><summary>参考答案</summary>

**1.** 输出 `6` 和 `5`。`héllo` 有 5 个字符，但 `é` 是 2 字节，所以字节数是 6。这正是「`len()` 是字节数」的直接体现。

**2.** 不能编译。报错核心是：

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
  |
3 |     let first = &v[0];
  |                  - immutable borrow occurs here
4 |     v.push(4);
  |     ^^^^^^^^^ mutable borrow occurs here
5 |     println!("{first}");
  |                ----- immutable borrow later used here
```

（以 Rust 1.98 为例。）

`first` 借用了 `v` 的元素，而 `push` 需要可变借用（还可能重新分配内存），两者不能同时存在（第 4 章）。至少改一处：把 `println!("{first}")` 移到 `push` 之前（借用结束得早），或者把 `first` 改成拷贝 `let first = v[0];`（`i32` 是 `Copy`），或者改用 `first.clone()`（元素是 `String` 等类型时）。

**3.** `你好` 的字节布局是「你」占 0..3、「好」占 3..6。`0..3` 与 `3..6` 都是字符边界，所以 `&s[0..3]` 合法；`0..1` 把「你」截成一半，切出来的字节序列不是合法 UTF-8，Rust 只能 panic。要「失败可预期」就用 `s.get(0..1)`，返回 `Option<&str>`，不是边界时得到 `None`——这正是 `Vec::get` 相同的思路：把「可能失败」放进类型。

</details>

## 延伸阅读

- The Book 第 8 章《常用集合》：<https://doc.rust-lang.org/book/ch08-00-common-collections.html>
- 其中「字符串」一节（含「为什么不支持索引」）：<https://doc.rust-lang.org/book/ch08-02-strings.html>
- `std::vec::Vec`：<https://doc.rust-lang.org/std/vec/struct.Vec.html>
- `std::string::String`：<https://doc.rust-lang.org/std/string/struct.String.html>
- `str` 的 `char_indices`：<https://doc.rust-lang.org/std/primitive.str.html#method.char_indices>
- `std::collections::HashMap` / `BTreeMap`：<https://doc.rust-lang.org/std/collections/index.html>
