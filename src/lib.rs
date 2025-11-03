use hecs::World;
use log::LevelFilter::Info;
use log::*;
use screeps::{game, SharedCreepProperties};
use std::collections::HashMap;
use std::sync::Once;
use wasm_bindgen::prelude::*;

// 导入子模块
mod logging;

// 导出公共API
// pub use types::*;

// 日志初始化静态变量
static INIT_LOGGING: Once = Once::new();

/// 游戏主循环函数
///
/// 这是游戏的主要入口点，每帧被调用一次。
/// 负责初始化日志、创建世界、运行各种游戏系统。
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    // 初始化日志系统
    INIT_LOGGING.call_once(|| {
        logging::setup_logging(Info);
    });

    // 添加指定内容的日志记录
    error!("呵呵呵呵呵");
}
