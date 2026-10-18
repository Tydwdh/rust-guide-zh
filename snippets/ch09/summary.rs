// trait 定义：行为契约。
trait Summary {
    // 没有方法体：实现者必须提供。
    fn author(&self) -> String;

    // 有方法体：默认实现，实现者可以不写。
    fn summarize(&self) -> String {
        format!("（{} 的内容摘要待补充）", self.author())
    }
}

struct Article {
    title: String,
    author: String,
}

impl Summary for Article {
    fn author(&self) -> String {
        self.author.clone()
    }

    // 覆盖默认实现。
    fn summarize(&self) -> String {
        format!("《{}》，作者 {}", self.title, self.author)
    }
}

struct Comment {
    user: String,
}

impl Summary for Comment {
    fn author(&self) -> String {
        self.user.clone()
    }
    // summarize 不写，直接用默认实现。
}

// 调用方视角：不关心具体类型，只要求「实现了 Summary」。
fn notify(item: &impl Summary) {
    println!("新内容：{}", item.summarize());
}

fn main() {
    let article = Article {
        title: String::from("RISC-V 的十年"),
        author: String::from("林一"),
    };
    let comment = Comment {
        user: String::from("qiao"),
    };

    notify(&article);
    notify(&comment);
}
