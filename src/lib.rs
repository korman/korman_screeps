use hecs::World;
use log::LevelFilter::Info;
use log::*;
use screeps::{game, SharedCreepProperties};
use std::collections::HashMap;
use std::sync::Once;
use wasm_bindgen::prelude::*;

// 导入子模块
mod creep;
mod logging;
mod movement;
mod systems;
mod types;

// 导出公共API
pub use types::*;

// 移除日志初始化
static INIT_LOGGING: Once = Once::new();

/// 游戏主循环函数
///
/// 这是游戏的主要入口点，每帧被调用一次。
/// 负责初始化日志、创建世界、运行各种游戏系统。
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    // 初始化（移除日志初始化）
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(Info);
    });
    let limit = screeps::game::cpu::limit();

    debug!("CPU Limit: {}", limit);

    let mut world = World::new();
    let mut creep_targets: HashMap<String, types::CreepTarget> = HashMap::new();

    // 移除所有调试日志

    // 添加实体
    for creep in game::creeps().values() {
        world.spawn((types::CreepId(creep.name()),));
    }

    // 运行系统
    systems::run_creep_system(&mut world, &mut creep_targets);
    systems::spawn_creep_system();
    systems::memory_cleanup_system();

    // 主循环完成
}
