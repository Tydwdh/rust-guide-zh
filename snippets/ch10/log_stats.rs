// 日志格式：<级别> <时间> <消息>
const LOG: &str = "\
INFO 09:01:12 启动完成
WARN 09:01:15 配置项 timeout 缺失，使用默认值
ERROR 09:02:03 连接数据库失败：timeout
ERROR 09:02:04 连接数据库失败：timeout
INFO 09:03:41 重试成功
WARN 09:05:00 队列积压 120 条
";

fn parse(line: &str) -> (&str, &str) {
    let mut fields = line.splitn(3, ' ');
    let level = fields.next().unwrap();
    let time = fields.next().unwrap();
    (level, time)
}

// 命令式：手写循环挑出 ERROR
fn imperative(log: &str) -> (usize, Vec<&str>) {
    let mut errors = 0;
    let mut times = Vec::new();
    for line in log.lines() {
        let (level, time) = parse(line);
        if level == "ERROR" {
            errors += 1;
            times.push(time);
        }
    }
    (errors, times)
}

// 链式：同一套 parse，改成适配器
fn chained(log: &str) -> (usize, Vec<&str>) {
    let errors: Vec<(&str, &str)> = log
        .lines()
        .map(parse)
        .filter(|(level, _)| *level == "ERROR")
        .collect();
    (errors.len(), errors.into_iter().map(|(_, time)| time).collect())
}

fn main() {
    let result = imperative(LOG);
    assert_eq!(result, chained(LOG), "两种写法必须给出同一份结果");
    let (errors, times) = result;
    println!("ERROR {errors} 条，时间：{}", times.join("、"));
}
