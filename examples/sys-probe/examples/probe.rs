//! 把三个 API 各跑一遍，看看它们给出的到底是什么数。
//!
//! ```console
//! $ cargo run --manifest-path examples/Cargo.toml -p sys-probe --example probe
//! ```
//!
//! Windows 上这些值都是真的（时间随运行时刻变）；别的平台上第一个调用就会
//! 以 [`sys_probe::ProbeError::Unsupported`] 结束，`main` 把它打印出来。

use std::time::{SystemTime, UNIX_EPOCH};

use sys_probe::{ProbeError, watch_ctrl_events};

fn main() -> Result<(), ProbeError> {
    let uptime = sys_probe::uptime_millis()?;
    println!("开机时长        ：{uptime} ms");

    let file_time = sys_probe::system_file_time()?;
    println!("FILETIME        ：{} 个 100 纳秒", file_time.ticks());
    println!("其中的 Unix 毫秒 ：{}", file_time.unix_millis()?);

    // 和标准库对一下：两条路算出来的应该是同一个瞬间。
    let std_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间早于 Unix 纪元")
        .as_millis();
    println!("std::time 对照   ：{std_ms} ms");

    // 处理器：注册、看看句柄在不在、注销。真按 Ctrl+C 时，这个闭包会在
    // 系统新起的线程上被调用 —— 那一行只有你自己按下去才看得到。
    let guard = watch_ctrl_events(|event| {
        println!("  收到控制事件：{event}（在系统起的线程上）");
        true
    })?;
    println!("控制事件处理器   ：已注册");

    // 再注册一个：被拒。一个进程一个槽位，「谁处理这个事件」才有唯一答案。
    if let Err(err) = watch_ctrl_events(|_| false) {
        println!("第二次注册       ：被拒（{err}）");
    }

    drop(guard);
    println!("控制事件处理器   ：已注销");

    Ok(())
}
