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

/// 游戏主循环函数
///
/// 这是游戏的主要入口点，每帧被调用一次。
/// 负责初始化日志、创建世界、运行各种游戏系统。
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {}
