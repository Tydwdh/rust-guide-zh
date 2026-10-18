// 一个「下载任务」的状态机：状态只可能是这四种之一。
#[derive(Debug, Clone, PartialEq)]
enum Status {
    Pending,
    Downloading { progress: u8 },
    Done { path: String },
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
struct Job {
    id: u32,
    title: String,
    status: Status,
}

impl Job {
    // 关联函数：没有 self，按约定叫 new
    fn new(id: u32, title: &str) -> Self {
        Job {
            id,
            title: title.to_string(),
            status: Status::Pending,
        }
    }

    // &mut self：要改状态，就得拿到独占的可变借用
    fn start(&mut self) {
        self.status = Status::Downloading { progress: 0 };
    }

    fn set_progress(&mut self, percent: u8) {
        if let Status::Downloading { progress } = &mut self.status {
            *progress = percent;
        }
    }

    fn finish(&mut self, path: &str) {
        self.status = Status::Done {
            path: path.to_string(),
        };
    }

    // &self：只读
    fn describe(&self) -> String {
        match &self.status {
            Status::Pending => format!("[{}] {} 等待中", self.id, self.title),
            Status::Downloading { progress } => {
                format!("[{}] {} 下载中 {}%", self.id, self.title, progress)
            }
            Status::Done { path } => format!("[{}] {} 完成 -> {}", self.id, self.title, path),
            Status::Failed { reason } => format!("[{}] {} 失败：{}", self.id, self.title, reason),
        }
    }

    // 只有完成时才有路径，所以返回 Option
    fn saved_path(&self) -> Option<&str> {
        match &self.status {
            Status::Done { path } => Some(path),
            _ => None,
        }
    }
}

fn main() {
    let mut job = Job::new(1, "第 5 章草稿");
    println!("{}", job.describe());

    job.start();
    job.set_progress(42);
    println!("{}", job.describe());

    job.finish("ch05.md");
    println!("{}", job.describe());
    println!("saved_path = {:?}", job.saved_path());

    let failed = Job {
        id: 2,
        title: String::from("素材下载"),
        status: Status::Failed {
            reason: String::from("连接超时"),
        },
    };
    if let Some(path) = failed.saved_path() {
        println!("产出：{path}");
    } else {
        println!("{} 没有产出文件", failed.title);
    }
    println!(
        "失败了吗？{}",
        matches!(failed.status, Status::Failed { .. })
    );
}
