//! 第 28 章「过程宏」的示例 crate：三个过程宏，零依赖、手写解析。
//!
//! - `#[derive(Describe)]`：给具名字段结构体生成 `fn describe(&self) -> String`
//! - `#[repeat(N)]`：把被标注的项复制 N 份，另起名为 `<名字>_0` … `<名字>_{N-1}`
//! - `sum!(1, 2, 3)`：编译期把一串整数字面量加成常量
//!
//! 全部只用 `proc_macro` 提供的类型与 `std`，不引 `syn`/`quote`。
//! 消费端在 `examples/macro-demo-use`（trait 与测试住在那里）。

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

// ---------------------------------------------------------------------------
// 造 token 的小工具：生成端不写字符串拼接，而是逐 token 拼装
// ---------------------------------------------------------------------------

/// 造一个标识符 token。
fn ident(name: &str, span: Span) -> TokenTree {
    TokenTree::Ident(Ident::new(name, span))
}

/// 造一个标点 token（`Alone` = 不与后一个字符连成多字符运算符）。
fn punct(ch: char) -> TokenTree {
    TokenTree::Punct(Punct::new(ch, Spacing::Alone))
}

/// 造一个多字符运算符，如 `->`、`::`：除最后一个字符外都标 `Joint`。
fn joint(chars: &str) -> Vec<TokenTree> {
    let mut out = Vec::new();
    let mut iter = chars.chars().peekable();
    while let Some(ch) = iter.next() {
        let spacing = if iter.peek().is_some() {
            Spacing::Joint
        } else {
            Spacing::Alone
        };
        out.push(TokenTree::Punct(Punct::new(ch, spacing)));
    }
    out
}

/// 造一个带定界符的分组（`{}`/`()`/`[]`）。
fn group(delim: Delimiter, tokens: impl IntoIterator<Item = TokenTree>) -> TokenTree {
    TokenTree::Group(Group::new(delim, tokens.into_iter().collect()))
}

/// 造一条绝对路径 `::std::string::String`（过程宏不卫生，路径要写全）。
fn path(segments: &[&str]) -> Vec<TokenTree> {
    let mut out = Vec::new();
    for seg in segments {
        out.extend(joint("::"));
        out.push(ident(seg, Span::call_site()));
    }
    out
}

/// 造 `compile_error!("…")`；整段调用落在 `span` 上，报错就指到那里。
fn compile_error(message: &str, span: Span) -> TokenStream {
    let mut literal = Literal::string(message);
    literal.set_span(span);
    let mut call = Group::new(Delimiter::Brace, TokenTree::Literal(literal).into());
    call.set_span(span);
    [ident("compile_error", span), punct('!'), TokenTree::Group(call)]
        .into_iter()
        .collect()
}

/// 把 token 流收进 `Vec`，方便按下标前后看。
fn to_vec(stream: TokenStream) -> Vec<TokenTree> {
    stream.into_iter().collect()
}

/// 跳过前缀：`#[…]` 属性与 `pub` / `pub(crate)` 这类可见性。
fn skip_attrs_and_visibility(tokens: &[TokenTree], mut i: usize) -> usize {
    while i < tokens.len() {
        match &tokens[i] {
            // `#` 后面一定跟着一个分组（括号里是属性内容）
            TokenTree::Punct(p) if p.as_char() == '#' => i += 2,
            TokenTree::Ident(kw) if kw.to_string() == "pub" => {
                i += 1;
                if let Some(TokenTree::Group(_)) = tokens.get(i) {
                    i += 1;
                }
            }
            _ => break,
        }
    }
    i
}

// ---------------------------------------------------------------------------
// 形态一：derive 宏
// ---------------------------------------------------------------------------

/// 给具名字段结构体生成 `impl Describe`。
///
/// 支持的输入：`struct Name { f1: T1, f2: T2 }`（允许属性与可见性前缀）。
/// 不支持的输入会生成 `compile_error!`，而不是 panic。
#[proc_macro_derive(Describe)]
pub fn derive_describe(input: TokenStream) -> TokenStream {
    let tokens = to_vec(input);
    let mut i = skip_attrs_and_visibility(&tokens, 0);

    // 1) 必须是 struct。
    let Some(TokenTree::Ident(keyword)) = tokens.get(i) else {
        return compile_error("Describe: 期望一个 struct 定义", Span::call_site());
    };
    if keyword.to_string() != "struct" {
        let message = format!("Describe: 只支持 struct，这里是 `{keyword}`");
        return compile_error(&message, keyword.span());
    }
    i += 1;

    // 2) 结构体名。
    let Some(TokenTree::Ident(name)) = tokens.get(i) else {
        return compile_error("Describe: 结构体缺少名字", keyword.span());
    };
    let name = name.clone();
    i += 1;

    // 3) 泛型一律拒绝（本示例的范围限制，错误落在 `<` 上）。
    if let Some(TokenTree::Punct(p)) = tokens.get(i) {
        if p.as_char() == '<' {
            return compile_error("Describe: 本示例不支持泛型结构体", p.span());
        }
    }

    // 4) 要一对花括号包起来的字段表。
    let Some(TokenTree::Group(body)) = tokens.get(i) else {
        return compile_error(
            "Describe: 只支持具名字段结构体，如 `struct Point { x: i32 }`",
            name.span(),
        );
    };
    if body.delimiter() != Delimiter::Brace {
        return compile_error(
            "Describe: 只支持具名字段结构体，如 `struct Point { x: i32 }`",
            body.span(),
        );
    }
    let fields = field_names(body.stream());

    // 5) 拼格式串：`Point {{ x = {:?}, y = {:?} }}`
    let mut fmt = name.to_string();
    if !fields.is_empty() {
        fmt.push_str(" {{ ");
        for (n, field) in fields.iter().enumerate() {
            if n > 0 {
                fmt.push_str(", ");
            }
            fmt.push_str(&field.to_string());
            fmt.push_str(" = {:?}");
        }
        fmt.push_str(" }}");
    }

    // 6) `::std::format!("…", &self.x, &self.y)` 的参数表
    let mut format_args = vec![TokenTree::Literal(Literal::string(&fmt))];
    for field in &fields {
        format_args.push(punct(','));
        format_args.extend([punct('&'), ident("self", Span::call_site()), punct('.')]);
        format_args.push(TokenTree::Ident(field.clone()));
    }

    let mut call = path(&["std", "format"]);
    call.push(punct('!'));
    call.push(group(Delimiter::Parenthesis, format_args));

    // 7) `fn describe(&self) -> ::std::string::String { … }`
    let mut method = vec![ident("fn", Span::call_site()), ident("describe", Span::call_site())];
    method.push(group(
        Delimiter::Parenthesis,
        [punct('&'), ident("self", Span::call_site())],
    ));
    method.extend(joint("->"));
    method.extend(path(&["std", "string", "String"]));
    method.push(group(Delimiter::Brace, call));

    // 8) `impl Describe for Name { … }`
    //    `Describe` 落在调用点（`call_site`），由调用处的 `use` 解析到 trait。
    [
        ident("impl", Span::call_site()),
        ident("Describe", Span::call_site()),
        ident("for", Span::call_site()),
        TokenTree::Ident(name),
        group(Delimiter::Brace, method),
    ]
    .into_iter()
    .collect()
}

/// 从字段表里挑出字段名：`ident` 后面紧跟一个孤立的 `:`（不是 `::` 里的那个）。
fn field_names(body: TokenStream) -> Vec<Ident> {
    let tokens = to_vec(body);
    let mut names = Vec::new();
    for pair in tokens.windows(2) {
        if let [TokenTree::Ident(name), TokenTree::Punct(colon)] = pair {
            if colon.as_char() == ':' && colon.spacing() == Spacing::Alone {
                names.push(name.clone());
            }
        }
    }
    names
}

// ---------------------------------------------------------------------------
// 形态二：属性宏
// ---------------------------------------------------------------------------

/// `#[repeat(N)]`：把被标注的项复制 N 份（另起名字），原件照旧保留。
///
/// 作用在 `const` / `static` / `fn` 上，例如 `#[repeat(3)] const SIZE: usize = 8;`
/// 会额外出 `SIZE_0`、`SIZE_1`、`SIZE_2`。
#[proc_macro_attribute]
pub fn repeat(attr: TokenStream, item: TokenStream) -> TokenStream {
    // 括号里的那一小段就是 `attr`：这里只接受一个整数字面量。
    let count = match to_vec(attr).as_slice() {
        [] => return compile_error("repeat: 缺参数，写成 `#[repeat(3)]`", Span::call_site()),
        [TokenTree::Literal(literal)] => match literal.to_string().parse::<u32>() {
            Ok(count) => count,
            Err(_) => {
                let message = format!("repeat: 只接受无后缀整数，`{literal}` 不行");
                return compile_error(&message, literal.span());
            }
        },
        [other, ..] => return compile_error("repeat: 只接受一个整数参数", other.span()),
    };

    let tokens = to_vec(item.clone());
    let mut i = skip_attrs_and_visibility(&tokens, 0);

    // 找到项的种类与名字：`const NAME` / `static mut NAME` / `fn NAME`
    let Some(TokenTree::Ident(kind)) = tokens.get(i) else {
        return compile_error("repeat: 支持 const / static / fn", Span::call_site());
    };
    let kind_text = kind.to_string();
    if !matches!(kind_text.as_str(), "const" | "static" | "fn") {
        let message = format!("repeat: 不支持 `{kind_text}`，只支持 const / static / fn");
        return compile_error(&message, kind.span());
    }
    i += 1;
    if let Some(TokenTree::Ident(next)) = tokens.get(i) {
        if next.to_string() == "mut" {
            i += 1;
        }
    }
    let Some(TokenTree::Ident(name)) = tokens.get(i) else {
        return compile_error("repeat: 这项没有名字", kind.span());
    };
    let name_text = name.to_string();

    let mut output = TokenStream::new();
    for n in 0..count {
        // 复制一份，把名字换成 `NAME_n`；标点、字面量、其余标识符原样搬过去，
        // 新标识符沿用原名 ident 的 span，报错才会指向源码里的那一行。
        let renamed = tokens.iter().map(|token| match token {
            TokenTree::Ident(id) if id.to_string() == name_text => {
                TokenTree::Ident(Ident::new(&format!("{name_text}_{n}"), id.span()))
            }
            other => other.clone(),
        });
        output.extend(renamed);
    }
    output.extend(item);
    output
}

// ---------------------------------------------------------------------------
// 形态三：函数式宏
// ---------------------------------------------------------------------------

/// `sum!(1, 2, 3)` → 编译期算出的常量 `6`（无后缀，类型由上下文推断）。
///
/// 过程宏在编译期真的跑代码，所以这类「算出来」的活它干得了，
/// `macro_rules!` 只能做模式替换。
#[proc_macro]
pub fn sum(input: TokenStream) -> TokenStream {
    let mut total: i64 = 0;
    let mut seen = 0usize;
    let mut expect_value = true;
    let mut last_span = Span::call_site();

    for token in input {
        match token {
            TokenTree::Literal(literal) if expect_value => {
                let text = literal.to_string();
                match text.parse::<i64>() {
                    Ok(value) => total += value,
                    Err(_) => {
                        let message = format!("sum!: `{text}` 不是无后缀整数");
                        return compile_error(&message, literal.span());
                    }
                }
                last_span = literal.span();
                seen += 1;
                expect_value = false;
            }
            TokenTree::Punct(p) if p.as_char() == ',' && !expect_value => expect_value = true,
            other => {
                return compile_error(
                    "sum!: 参数得是逗号分隔的整数，如 `sum!(1, 2, 3)`",
                    other.span(),
                );
            }
        }
    }

    if seen == 0 {
        return compile_error("sum!: 至少要有一个整数参数", Span::call_site());
    }

    let mut literal = Literal::i64_unsuffixed(total);
    literal.set_span(last_span);
    TokenTree::Literal(literal).into()
}
