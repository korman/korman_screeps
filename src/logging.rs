use log::LevelFilter;
use std::{fmt::Write, panic};

/// 设置基本的panic处理，移除所有日志功能
pub fn setup_logging(_verbosity: log::LevelFilter) {
    panic::set_hook(Box::new(panic_hook));
}

fn panic_hook(info: &panic::PanicHookInfo) {
    // 简单的panic处理，不再输出详细的堆栈信息
    let mut fmt_error = String::new();
    let _ = writeln!(fmt_error, "{}", info);
    // 移除错误日志输出
}
