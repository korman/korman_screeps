use hecs::World;
use log::*;
use screeps::{game, objects::Creep, SharedCreepProperties};
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

// 初始化日志的Once实例
static INIT_LOGGING: Once = Once::new();

/// 游戏主循环函数
///
/// 这是游戏的主要入口点，每帧被调用一次。
/// 负责初始化日志、创建世界、运行各种游戏系统。
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    // 强制使用Debug级别日志以便捕获所有调试信息
    INIT_LOGGING.call_once(|| logging::setup_logging(logging::Debug));

    let mut world = World::new();
    let mut creep_targets: HashMap<String, types::CreepTarget> = HashMap::new();

    debug!(
        "loop starting! CPU: {}, Time: {}",
        game::cpu::get_used(),
        game::time()
    );

    // 记录当前房间和creep数量信息
    let creep_count = game::creeps().keys().collect::<Vec<_>>().len();
    let room_count = game::rooms().keys().collect::<Vec<_>>().len();
    info!(
        "Current game state: {} creeps in {} rooms",
        creep_count, room_count
    );

    // 记录所有房间信息
    for room in game::rooms().values() {
        info!(
            "Room: {:?}, Energy available: {}, Controllers: {:?}",
            room.name(),
            room.energy_available(),
            room.controller().map(|c| c.level()).unwrap_or(0)
        );
    }

    // 添加实体
    for creep in game::creeps().values() {
        world.spawn((types::CreepId(creep.name()),));
    }

    // 运行系统
    systems::run_creep_system(&mut world, &mut creep_targets);
    systems::spawn_creep_system();
    systems::memory_cleanup_system();

    info!("done! cpu: {}", game::cpu::get_used())
}
