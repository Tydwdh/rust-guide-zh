use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// 一个目录节点。子目录由父目录强引用；父目录对子目录是弱引用，
/// 这样「向上」的边不会构成环。
struct Dir {
    name: String,
    parent: Weak<Dir>,
    children: RefCell<Vec<Rc<Dir>>>,
    files: Cell<usize>,
}

impl Dir {
    fn root(name: &str) -> Rc<Dir> {
        Rc::new(Dir {
            name: name.to_string(),
            parent: Weak::new(),
            children: RefCell::new(Vec::new()),
            files: Cell::new(0),
        })
    }

    fn mkdir(self: &Rc<Dir>, name: &str) -> Rc<Dir> {
        let child = Rc::new(Dir {
            name: name.to_string(),
            parent: Rc::downgrade(self),
            children: RefCell::new(Vec::new()),
            files: Cell::new(0),
        });
        self.children.borrow_mut().push(Rc::clone(&child));
        child
    }

    fn add_files(&self, n: usize) {
        self.files.set(self.files.get() + n);
    }

    /// 从根到自己的路径。根节点直接借用一个字面量，其余情况才拼字符串。
    fn path(&self) -> Cow<'_, str> {
        match self.parent.upgrade() {
            None => Cow::Borrowed("/"),
            Some(parent) => {
                let mut s = parent.path().into_owned();
                if !s.ends_with('/') {
                    s.push('/');
                }
                s.push_str(&self.name);
                Cow::Owned(s)
            }
        }
    }

    fn total_files(&self) -> usize {
        self.files.get() + self.children.borrow().iter().map(|c| c.total_files()).sum::<usize>()
    }
}

fn main() {
    let root = Dir::root("project");
    let src = root.mkdir("src");
    let util = src.mkdir("util");
    let docs = root.mkdir("docs");

    util.add_files(3);
    src.add_files(2);
    docs.add_files(5);
    root.add_files(1);

    println!("util.path()     = {}", util.path());
    println!("root.path()     = {}", root.path());
    println!("root.total_files() = {}", root.total_files());
    println!("util 的父目录   = {:?}", util.parent.upgrade().map(|d| d.name.clone()));

    for child in root.children.borrow().iter() {
        println!("{} 下有 {} 个文件", child.name, child.total_files());
    }

    let same_src = Rc::clone(&src);
    println!("src 的强引用数 = {}", Rc::strong_count(&src));

    drop(src);
    drop(same_src);
    drop(root);
    println!("释放 root 与 src 之后，util 仍被本地变量持有");
    println!("  父目录还在吗：{:?}", util.parent.upgrade().map(|d| d.name.clone()));
    println!("  这时 path() 退化成 {:?}", util.path());
}
