//! 任务的加载、保存与增删改查。

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::error::{AppError, Result};
use crate::task::{Status, Task};

/// `list` 要挑出哪一部分任务。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Pending,
    Done,
}

/// 任务集合 + 数据文件位置。
#[derive(Debug)]
pub struct Store {
    path: PathBuf,
    tasks: Vec<Task>,
    next_id: u32,
}

/// 文件进出：这一块会碰磁盘。
impl Store {
    /// 打开数据文件；文件不存在视为「还没有任何任务」，不是错误。
    pub fn open(path: impl Into<PathBuf>) -> Result<Store> {
        let path = path.into();
        let tasks = match fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        // 编号用满（出现过 `u32::MAX`）时把 next_id 钉在 `u32::MAX`：`add` 会返回错误，
        // 只读的 list / done / rm 照常可用。
        let next_id = match tasks.iter().map(Task::id).max() {
            Some(max) => max.checked_add(1).unwrap_or(u32::MAX),
            None => 1,
        };
        Ok(Store {
            path,
            tasks,
            next_id,
        })
    }

    /// 解析整个数据文件：忽略空行，其余每行必须是一条合法记录。
    fn parse(text: &str) -> Result<Vec<Task>> {
        text.lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(i, line)| {
                Task::parse_record(line).map_err(|reason| AppError::BadRecord {
                    line: i + 1,
                    reason,
                })
            })
            .collect()
    }

    /// 把全部任务写回数据文件（整文件覆盖）。
    pub fn save(&self) -> Result<()> {
        let mut text = self
            .tasks
            .iter()
            .map(Task::to_record)
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        fs::write(&self.path, text)?;
        Ok(())
    }
}

/// 只读查询：只借出数据，不改集合。
impl Store {
    /// 借出全部任务（只读视图）。
    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// 按筛选条件借出任务。
    pub fn select(&self, filter: Filter) -> Vec<&Task> {
        self.tasks()
            .iter()
            .filter(|task| match filter {
                Filter::All => true,
                Filter::Pending => task.status() == Status::Pending,
                Filter::Done => task.status() == Status::Done,
            })
            .collect()
    }
}

/// 改动：都会修改内存里的集合。
impl Store {
    /// 追加一条任务，返回它的编号。
    pub fn add(&mut self, text: &str) -> Result<u32> {
        if text.contains(['\t', '\n', '\r']) {
            return Err(AppError::UnstorableText);
        }
        let id = self.next_id;
        // 编号用满时返回错误，而不是让 `+ 1` 溢出 panic。
        self.next_id = id.checked_add(1).ok_or(AppError::IdsExhausted)?;
        self.tasks.push(Task::new(id, text));
        Ok(id)
    }

    /// 把某条任务标记为完成；返回它是不是刚被标记。
    pub fn mark_done(&mut self, id: u32) -> Result<bool> {
        let task = self
            .tasks
            .iter_mut()
            .find(|task| task.id() == id)
            .ok_or(AppError::NoSuchTask(id))?;
        Ok(task.mark_done())
    }

    /// 删除某条任务，返回被删掉的那条。
    pub fn remove(&mut self, id: u32) -> Result<Task> {
        let pos = self
            .tasks
            .iter()
            .position(|task| task.id() == id)
            .ok_or(AppError::NoSuchTask(id))?;
        Ok(self.tasks.remove(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个只活在内存里的 `Store`，不碰磁盘。
    fn store(tasks: Vec<Task>) -> Store {
        let next_id = match tasks.iter().map(Task::id).max() {
            Some(max) => max.checked_add(1).unwrap_or(u32::MAX),
            None => 1,
        };
        Store {
            path: PathBuf::from("不会被写入.txt"),
            tasks,
            next_id,
        }
    }

    #[test]
    fn parse_ignores_blank_lines_and_keeps_line_numbers_honest() {
        let tasks = Store::parse("1\to\t写测试\n\n2\tx\t删旧代码\n").unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].text(), "删旧代码");
    }

    #[test]
    fn bad_record_reports_the_physical_line_number() {
        let err = Store::parse("1\to\t写测试\n\n2\t?\t坏行").unwrap_err();
        assert!(err.to_string().contains("第 3 行"), "实际：{err}");
    }

    #[test]
    fn add_assigns_increasing_ids_and_rejects_multiline_text() {
        let mut store = store(vec![Task::new(4, "旧任务")]);
        assert_eq!(store.add("新任务").unwrap(), 5);
        assert_eq!(store.add("又一条").unwrap(), 6);
        assert!(matches!(
            store.add("带\t制表符"),
            Err(AppError::UnstorableText)
        ));
        assert_eq!(store.tasks().len(), 3);
    }

    #[test]
    fn exhausted_id_space_reports_an_error_instead_of_overflowing() {
        let mut store = store(vec![Task::new(u32::MAX, "最后一条")]);
        assert!(matches!(
            store.add("加不进去了"),
            Err(AppError::IdsExhausted)
        ));
        assert_eq!(store.tasks().len(), 1, "失败时不该改动集合");
    }

    #[test]
    fn select_filters_by_status() {
        let mut done = Task::new(2, "已完成的事");
        done.mark_done();
        let store = store(vec![Task::new(1, "待办的事"), done]);
        assert_eq!(store.select(Filter::All).len(), 2);
        assert_eq!(store.select(Filter::Pending)[0].id(), 1);
        assert_eq!(store.select(Filter::Done)[0].id(), 2);
    }

    #[test]
    fn operations_on_unknown_ids_are_errors() {
        let mut store = store(vec![Task::new(1, "唯一的任务")]);
        assert!(matches!(store.mark_done(9), Err(AppError::NoSuchTask(9))));
        assert!(matches!(store.remove(9), Err(AppError::NoSuchTask(9))));
    }
}
