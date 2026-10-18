//! 集成测试：只用公开 API，把 crate 当黑盒用。
//!
//! 同一批断言在两种平台上都成立：Windows 上走真 FFI（`#[cfg(windows)]` 的那几条），
//! 别的平台上走降级分支（`#[cfg(not(windows))]` 的那条）。CI 上两个方向都会编译。

use sys_probe::{CtrlEvent, FileTime, ProbeError};

/// 纯算术，两边都一样：换算与边界与平台无关。
#[test]
fn file_time_arithmetic_is_platform_independent() {
    let epoch = FileTime::from_ticks(FileTime::EPOCH_OFFSET_TICKS);
    assert_eq!(epoch.unix_millis().unwrap(), 0);

    match FileTime::from_ticks(0).unix_millis() {
        Err(ProbeError::BeforeUnixEpoch { ticks }) => assert_eq!(ticks, 0),
        other => panic!("1601-01-01 应该报 BeforeUnixEpoch，实际是 {other:?}"),
    }
}

/// 不认识的码也要原样留着，`Display` 不能瞎猜。
#[test]
fn unknown_ctrl_codes_are_preserved() {
    let event = CtrlEvent::from_raw(9);
    assert_eq!(event.raw(), 9);
    assert_eq!(event.to_string(), "未知控制事件（码 9）");
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 开机时间是个「说得过去」的数，而且不会倒退。
    #[test]
    fn uptime_is_plausible_and_monotonic() {
        let first = sys_probe::uptime_millis().expect("Windows 上不该失败");
        let second = sys_probe::uptime_millis().expect("Windows 上不该失败");

        assert!(second >= first, "开机时间倒退了：{first} -> {second}");
        assert!(
            first < 10 * 365 * 24 * 60 * 60 * 1_000,
            "开机时间超过十年：{first} ms"
        );
    }

    /// 自己算的换算，和标准库的时钟对得上。
    #[test]
    fn unix_time_millis_agrees_with_the_standard_library() {
        let ours = sys_probe::unix_time_millis().expect("Windows 上不该失败");
        let std_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("系统时间不该早于 Unix 纪元")
            .as_millis() as u64;

        assert!(
            ours.abs_diff(std_ms) < 2_000,
            "和 std 差了 {} ms：ours={ours} std={std_ms}",
            ours.abs_diff(std_ms)
        );
    }

    /// `system_file_time` 与 `unix_time_millis` 描述的必须是同一个瞬间。
    #[test]
    fn both_time_apis_describe_the_same_instant() {
        let from_file_time = sys_probe::system_file_time()
            .expect("Windows 上不该失败")
            .unix_millis()
            .expect("现在的时间在 Unix 纪元之后");
        let from_api = sys_probe::unix_time_millis().expect("Windows 上不该失败");

        assert!(from_file_time.abs_diff(from_api) < 2_000);
    }

    /// 注册 → 拒绝第二个 → 释放 → 再注册：整条生命周期走一遍真 FFI。
    ///
    /// 这个用例能观察到的是**槽位**与注册调用的成败（真的调 `SetConsoleCtrlHandler`）；
    /// 「系统那边确实把处理器摘掉了」要发一次真信号才看得见（`GenerateConsoleCtrlEvent`
    /// 依赖控制台，不适合放进测试），所以不在这里覆盖 —— 注销传的是哪个指针由
    /// `CtrlGuard::drop` 保证（必须与注册时同一个，传 NULL 不是摘处理器）。
    #[test]
    fn ctrl_handler_lifecycle_works_end_to_end() {
        let guard = sys_probe::watch_ctrl_events(|_| true).expect("Windows 上不该失败");

        match sys_probe::watch_ctrl_events(|_| false) {
            Err(ProbeError::HandlerInUse) => {}
            other => panic!("第二个处理器应该被拒，实际是 {other:?}"),
        }

        drop(guard); // 注销
        let again = sys_probe::watch_ctrl_events(|_| true).expect("注销之后应该能再注册");
        drop(again);
    }
}

#[cfg(not(windows))]
mod not_windows {
    use super::*;

    /// 降级分支：每个入口都回 Unsupported，并且说得出缺的是哪个 API。
    #[test]
    fn every_entry_point_reports_which_api_is_missing() {
        let errors = [
            sys_probe::uptime_millis().unwrap_err(),
            sys_probe::system_file_time().unwrap_err(),
            sys_probe::unix_time_millis().unwrap_err(),
            sys_probe::watch_ctrl_events(|_| true).unwrap_err(),
        ];

        for err in errors {
            assert!(
                matches!(err, ProbeError::Unsupported { .. }),
                "实际是 {err:?}"
            );
            assert!(err.to_string().ends_with("在当前平台上不可用"));
        }
    }
}
