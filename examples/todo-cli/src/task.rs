//! 任务的数据表示，以及「一行文本 ↔ 一条任务」的转换。

use std::fmt;
use std::str::FromStr;

/// 任务状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pending,
    Done,
}

impl Status {
    /// 数据文件里用的标记字符。
    fn marker(self) -> char {
        match self {
            Status::Pending => 'o',
            Status::Done => 'x',
        }
    }

    /// 展示用的中文名。
    pub fn label(self) -> &'static str {
        match self {
            Status::Pending => "待办",
            Status::Done => "已完成",
        }
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "o" => Ok(Status::Pending),
            "x" => Ok(Status::Done),
            other => Err(format!("未知的状态标记 `{other}`")),
        }
    }
}

/// 一条待办任务。字段私有：状态只能通过 `mark_done` 改变。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    id: u32,
    status: Status,
    text: String,
}

impl Task {
    /// 新建一条「待办」状态的任务。
    pub fn new(id: u32, text: &str) -> Self {
        Task {
            id,
            status: Status::Pending,
            text: text.to_owned(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn status(&self) -> Status {
        self.status
    }

    /// 借出任务文本，不复制（第 4 章：借用）。
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 标记为完成，返回这是不是第一次标记。
    pub fn mark_done(&mut self) -> bool {
        let first = self.status == Status::Pending;
        self.status = Status::Done;
        first
    }
}

/// 文件格式：一行文本与一条任务之间的转换。
impl Task {
    /// 序列化成数据文件里的一行（不含换行符）。
    pub fn to_record(&self) -> String {
        format!("{}\t{}\t{}", self.id, self.status.marker(), self.text)
    }

    /// 解析数据文件的一行；失败时给一句可以直接展示的原因。
    pub fn parse_record(line: &str) -> Result<Task, String> {
        let mut fields = line.split('\t');
        let id = fields.next().ok_or("缺少编号")?;
        let status = fields.next().ok_or("缺少状态标记")?;
        let text = fields.next().ok_or("缺少文本")?;
        if fields.next().is_some() {
            return Err("字段多于三个".to_owned());
        }
        Ok(Task {
            id: id.parse().map_err(|_| format!("编号 `{id}` 不是数字"))?,
            status: status.parse()?,
            text: text.to_owned(),
        })
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 中文字符在终端里占两列，用 `{:<3}` 对齐列宽反而会错位：空格分隔就好。
        write!(f, "#{} {} {}", self.id, self.status.label(), self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_round_trip_keeps_id_status_and_text() {
        let task = Task::new(3, "买牛奶");
        assert_eq!(task.to_record(), "3\to\t买牛奶");

        let mut done = task.clone();
        assert!(done.mark_done(), "第一次标记应返回 true");
        assert!(!done.mark_done(), "重复标记应返回 false");
        assert_eq!(done.to_record(), "3\tx\t买牛奶");
        assert_eq!(Task::parse_record(&done.to_record()).unwrap(), done);
    }

    #[test]
    fn broken_records_report_a_reason() {
        assert_eq!(Task::parse_record("1\tx").unwrap_err(), "缺少文本");
        assert_eq!(
            Task::parse_record("abc\to\t写").unwrap_err(),
            "编号 `abc` 不是数字"
        );
        assert_eq!(
            Task::parse_record("1\t?\t写").unwrap_err(),
            "未知的状态标记 `?`"
        );
    }

    #[test]
    fn status_parses_from_its_marker() {
        assert_eq!("o".parse::<Status>().unwrap(), Status::Pending);
        assert_eq!("x".parse::<Status>().unwrap(), Status::Done);
    }
}
