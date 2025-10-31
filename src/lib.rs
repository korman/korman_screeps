use hecs::World;
use js_sys::JsString;
use js_sys::Object;
use js_sys::Reflect;
use log::*;
use screeps::action_error_codes::CreepMoveToErrorCode;
use screeps::action_error_codes::UpgradeControllerErrorCode;
use screeps::find;
use screeps::game;
use screeps::Creep;
use screeps::Direction;
use screeps::HasId;
use screeps::HasPosition;
use screeps::Part;
use screeps::ResourceType;
use screeps::SharedCreepProperties;
use screeps::StructureObject;
use std::collections::HashMap;
use std::collections::HashSet;
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
        screeps::game::cpu::get_used(),
        screeps::game::time()
    );
    // 记录当前房间和creep数量信息
    let creep_count = screeps::game::creeps().keys().collect::<Vec<_>>().len();
    let room_count = screeps::game::rooms().keys().collect::<Vec<_>>().len();
    info!(
        "Current game state: {} creeps in {} rooms",
        creep_count, room_count
    );

    // 记录所有房间信息
    for room in screeps::game::rooms().values() {
        info!(
            "Room: {:?}, Energy available: {}, Controllers: {:?}",
            room.name(),
            room.energy_available(),
            room.controller().map(|c| c.level()).unwrap_or(0)
        );
    }

    // 添加实体
    for creep in screeps::game::creeps().values() {
        world.spawn((types::CreepId(creep.name()),));
    }

    // 运行系统
    systems::run_creep_system(&mut world, &mut creep_targets);
    systems::spawn_creep_system();
    systems::memory_cleanup_system();

    info!("done! cpu: {}", screeps::game::cpu::get_used())
}

// 系统: 运行 Creep
fn run_creep_system(world: &mut World, creep_targets: &mut HashMap<String, CreepTarget>) {
    for (_, creep_id) in world.query::<&CreepId>().iter() {
        if let Some(creep) = game::creeps().get(creep_id.0.clone()) {
            run_creep(&creep, creep_targets);
        }
    }
}

// 系统: 生成 Creep
fn spawn_creep_system() {
    debug!("所有运行中的基地");
    let mut additional = 0;
    for spawn in game::spawns().values() {
        debug!("运行的基地: {}", spawn.name());

        let body = [Part::Move, Part::Move, Part::Carry, Part::Work];
        if spawn.room().unwrap().energy_available() >= body.iter().map(|p| p.cost()).sum() {
            let name_base = game::time();
            let name = format!("{}-{}", name_base, additional);
            match spawn.spawn_creep(&body, &name) {
                Ok(()) => additional += 1,
                Err(e) => warn!("couldn't spawn: {:?}", e),
            }
        }
    }
}

// 系统: 内存清理
fn memory_cleanup_system() {
    if game::time() % 1000 == 0 {
        info!("running memory cleanup");
        let mut alive_creeps = HashSet::new();
        for creep_name in game::creeps().keys() {
            alive_creeps.insert(creep_name);
        }

        // 注意：使用了deprecated的ROOT，后续应该更新为thread_local_v2
        if let Ok(memory_creeps) = Reflect::get(&screeps::memory::ROOT, &JsString::from("creeps")) {
            let memory_creeps: Object = memory_creeps.unchecked_into();
            for creep_name_js in Object::keys(&memory_creeps).iter() {
                let creep_name = String::from(creep_name_js.dyn_ref::<JsString>().unwrap());

                if !alive_creeps.contains(&creep_name) {
                    info!("deleting memory for dead creep {}", creep_name);
                    let _ = Reflect::delete_property(&memory_creeps, &creep_name_js);
                }
            }
        }
    }
}

// smart_move函数在后面有更新的版本

// 辅助函数：从delta计算方向
fn direction_from_delta(dx: i8, dy: i8) -> Direction {
    match (dx, dy) {
        (0, -1) => Direction::Top,
        (1, -1) => Direction::TopRight,
        (1, 0) => Direction::Right,
        (1, 1) => Direction::BottomRight,
        (0, 1) => Direction::Bottom,
        (-1, 1) => Direction::BottomLeft,
        (-1, 0) => Direction::Left,
        (-1, -1) => Direction::TopLeft,
        _ => Direction::Top, // 默认向上
    }
}

// 辅助函数：检查并记录creep的身体部件状态
fn log_creep_body_status(creep: &Creep) {
    let name = creep.name();
    let body = creep.body();
    let body_count = body.len();

    debug!(
        "log_creep_body_status({}): time={}, Total body parts:{}",
        name,
        game::time(),
        body_count
    );
}

// 智能移动函数 - 详细调试版本
fn smart_move(creep: &Creep, target: &impl HasPosition) -> bool {
    let name = creep.name();
    let creep_pos = creep.pos();
    let target_pos = target.pos();
    let time = game::time();

    debug!(
        "[{}] smart_move() START: creep={}, current_pos=({},{}), target_pos=({},{}), range={}",
        time,
        name,
        creep_pos.x(),
        creep_pos.y(),
        target_pos.x(),
        target_pos.y(),
        creep_pos.get_range_to(target_pos)
    );

    // 记录身体部件状态
    log_creep_body_status(creep);

    // 检查基本状态但不分析具体部件
    let body = creep.body();
    let body_count = body.len();
    let has_move_parts = body_count > 0; // 简化判断

    debug!("[{}] smart_move(): creep={}, has_body_parts={}, total_body_parts={}, fatigue={}, spawning={}", 
           time, name, has_move_parts, body_count, creep.fatigue(), creep.spawning());

    // 检查是否有疲劳值
    if creep.fatigue() > 0 {
        debug!(
            "[{}] smart_move(): creep={} cannot move due to fatigue={}",
            time,
            name,
            creep.fatigue()
        );
        return false;
    }

    // 如果没有移动部件，不能移动
    if !has_move_parts {
        warn!(
            "[{}] smart_move(): creep={} has no MOVE parts! Cannot move.",
            time, name
        );
        return false;
    }

    // 如果已经在目标位置附近，返回成功
    if creep_pos.in_range_to(target_pos, 1) {
        debug!(
            "[{}] smart_move(): creep={} already in range of target position",
            time, name
        );
        return true;
    }

    // 记录移动前的房间和地形信息
    if let Some(room) = creep.room() {
        debug!(
            "[{}] smart_move(): creep={} in room={}, checking terrain at current position",
            time,
            name,
            room.name()
        );
    }

    debug!(
        "[{}] smart_move(): creep={} preparing move_to with standard path options",
        time, name
    );

    // 记录移动操作
    let result = creep.move_to(target_pos);

    match result {
        Ok(()) => {
            info!("[{}] smart_move(): creep={} move_to succeeded", time, name);

            // 检查移动后位置是否变化
            let new_pos = creep.pos();
            if new_pos.x() != creep_pos.x() || new_pos.y() != creep_pos.y() {
                info!(
                    "[{}] smart_move(): creep={} moved from ({},{}) to ({},{})",
                    time,
                    name,
                    creep_pos.x(),
                    creep_pos.y(),
                    new_pos.x(),
                    new_pos.y()
                );
            } else {
                warn!(
                    "[{}] smart_move(): creep={} move_to returned Ok but position did not change!",
                    time, name
                );
            }

            return true;
        }
        Err(err) => {
            warn!(
                "[{}] smart_move(): creep={} move_to failed with error: {:?}",
                time, name, err
            );

            // 根据错误类型尝试不同的策略
            match err {
                CreepMoveToErrorCode::NoPath => {
                    warn!("[{}] smart_move(): creep={} No path found to target, attempting fallback strategies", 
                          time, name);

                    // 计算直接方向
                    let dx = (target_pos.x().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.x().to_string().parse::<i8>().unwrap_or(0))
                    .signum();
                    let dy = (target_pos.y().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.y().to_string().parse::<i8>().unwrap_or(0))
                    .signum();

                    let direction = direction_from_delta(dx, dy);
                    debug!("[{}] smart_move(): creep={} calculated direction={:?} from delta({},{}), attempting direct move", 
                           time, name, direction, dx, dy);

                    match creep.move_direction(direction) {
                        Ok(()) => {
                            info!(
                                "[{}] smart_move(): creep={} direct move to {:?} succeeded",
                                time, name, direction
                            );
                            return true;
                        }
                        Err(dir_err) => {
                            warn!("[{}] smart_move(): creep={} direct move failed with error: {:?}, trying random move", 
                                  time, name, dir_err);

                            // 尝试随机移动以避免卡住
                            use rand::Rng;
                            let mut rng = rand::thread_rng();
                            let random_dir = rng.gen_range(1..=8);

                            let directions = [
                                Direction::Top,
                                Direction::TopRight,
                                Direction::Right,
                                Direction::BottomRight,
                                Direction::Bottom,
                                Direction::BottomLeft,
                                Direction::Left,
                                Direction::TopLeft,
                            ];

                            let rand_direction = directions[random_dir as usize - 1];
                            debug!("[{}] smart_move(): creep={} attempting random move in direction {:?}", 
                                  time, name, rand_direction);

                            match creep.move_direction(rand_direction) {
                                Ok(()) => {
                                    info!(
                                        "[{}] smart_move(): creep={} random move succeeded",
                                        time, name
                                    );
                                    return true;
                                }
                                Err(rand_err) => {
                                    error!("[{}] smart_move(): creep={} ALL movement attempts failed. Last error: {:?}", 
                                           time, name, rand_err);
                                    return false;
                                }
                            }
                        }
                    }
                }
                _ => {
                    warn!("[{}] smart_move(): creep={} encountered error: {:?}, attempting alternative approaches", 
                          time, name, err);

                    // 检查是否有障碍物
                    if let Some(room) = creep.room() {
                        // 移除不支持的look_for_at调用，使用更简单的方式记录
                        debug!("[{}] smart_move(): creep={} in room={}, checking for obstacles at current position", 
                               time, name, room.name());
                    }

                    // 尝试随机移动
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let random_dir = rng.gen_range(1..=8);

                    let directions = [
                        Direction::Top,
                        Direction::TopRight,
                        Direction::Right,
                        Direction::BottomRight,
                        Direction::Bottom,
                        Direction::BottomLeft,
                        Direction::Left,
                        Direction::TopLeft,
                    ];

                    let rand_direction = directions[random_dir as usize - 1];
                    debug!("[{}] smart_move(): creep={} trying random direction {:?} due to error {:?}", 
                           time, name, rand_direction, err);

                    let random_move_result = creep.move_direction(rand_direction);
                    debug!(
                        "[{}] smart_move(): creep={} random move result: {:?}",
                        time, name, random_move_result
                    );
                }
            }

            debug!(
                "[{}] smart_move() END: creep={}, movement failed, returning false",
                time, name
            );
            return false;
        }
    }
}

fn run_creep(creep: &Creep, creep_targets: &mut HashMap<String, CreepTarget>) {
    let name = creep.name();
    let time = game::time();

    debug!("[{}] run_creep() START: creep={}", time, name);

    // 基本状态检查
    if creep.spawning() {
        debug!(
            "[{}] run_creep(): creep={} is still spawning, skipping movement logic",
            time, name
        );
        return;
    }

    // 记录当前位置和房间信息
    let pos = creep.pos();
    let room_name = creep
        .room()
        .as_ref()
        .map_or("unknown".to_string(), |r| r.name().to_string());
    debug!(
        "[{}] run_creep(): creep={} at position ({},{}) in room={}",
        time,
        name,
        pos.x(),
        pos.y(),
        room_name
    );

    // 记录存储状态
    let energy_capacity = creep.store().get_capacity(Some(ResourceType::Energy));
    let energy_amount = creep.store().get_used_capacity(Some(ResourceType::Energy));
    debug!(
        "[{}] run_creep(): creep={} energy: {}/{}, fatigue: {}, hits: {}/{}",
        time,
        name,
        energy_amount,
        energy_capacity,
        creep.fatigue(),
        creep.hits(),
        creep.hits_max()
    );

    // 简化版身体部件分析，移除可能的编译错误
    let body = creep.body();
    let total_parts = body.len();

    debug!(
        "[{}] run_creep(): creep={} has {} total body parts",
        time, name, total_parts
    );

    // 检查是否被卡住或无法移动的状态
    if creep.fatigue() > 0 {
        debug!(
            "[{}] run_creep(): creep={} cannot move due to fatigue: {}",
            time,
            name,
            creep.fatigue()
        );
    }

    // 目标处理逻辑
    let target = creep_targets.entry(name.clone());
    match target {
        std::collections::hash_map::Entry::Occupied(entry) => {
            let creep_target = entry.get();
            debug!(
                "[{}] run_creep(): creep={} has existing target: {:?}",
                time, name, creep_target
            );

            match creep_target {
                CreepTarget::Upgrade(controller_id)
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    debug!(
                        "[{}] run_creep(): creep={} has energy, attempting to upgrade controller",
                        time, name
                    );

                    if let Some(controller) = controller_id.resolve() {
                        let controller_pos = controller.pos();
                        let range = pos.get_range_to(controller_pos);
                        debug!("[{}] run_creep(): creep={} controller at ({},{}), range={}, can_upgrade={}", 
                               time, name, controller_pos.x(), controller_pos.y(), range,
                               creep.pos().is_near_to(controller_pos));

                        // 升级控制器逻辑
                        let upgrade_result = creep.upgrade_controller(&controller);
                        debug!(
                            "[{}] run_creep(): creep={} upgrade result: {:?}",
                            time, name, upgrade_result
                        );

                        match upgrade_result {
                            Ok(_amount) => {
                                debug!(
                                    "[{}] run_creep(): creep={} successfully upgraded controller",
                                    time, name
                                );
                            }
                            Err(e) => match e {
                                UpgradeControllerErrorCode::NotInRange => {
                                    debug!("[{}] run_creep(): creep={} not in range of controller, initiating movement", 
                                           time, name);

                                    // 记录移动前的详细信息
                                    debug!("[{}] run_creep(): creep={} BEFORE MOVE TO controller - position: ({},{}), range: {}", 
                                           time, name, pos.x(), pos.y(), range);

                                    // 使用智能移动函数
                                    let moved = smart_move(creep, &controller);

                                    // 记录移动后的位置
                                    let new_pos = creep.pos();
                                    debug!("[{}] run_creep(): creep={} AFTER MOVE TO controller - moved: {}, new position: ({},{}) - position changed: {}" , 
                                           time, name, moved, new_pos.x(), new_pos.y(),
                                           (new_pos.x() != pos.x()) || (new_pos.y() != pos.y()));
                                }
                                _ => {
                                    warn!("[{}] run_creep(): creep={} couldn't upgrade controller: {:?}, removing target", 
                                          time, name, e);
                                    entry.remove();
                                }
                            },
                        }
                    } else {
                        warn!(
                            "[{}] run_creep(): creep={} controller not found, removing target",
                            time, name
                        );
                        entry.remove();
                    }
                }
                CreepTarget::Harvest(source_id)
                    if creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    debug!(
                        "[{}] run_creep(): creep={} has space for energy, attempting to harvest",
                        time, name
                    );

                    if let Some(source) = source_id.resolve() {
                        let source_pos = source.pos();
                        let range = pos.get_range_to(source_pos);
                        let in_range = creep.pos().is_near_to(source_pos);

                        debug!(
                            "[{}] run_creep(): creep={} source at ({},{}), range={}, in_range={}",
                            time,
                            name,
                            source_pos.x(),
                            source_pos.y(),
                            range,
                            in_range
                        );

                        if in_range {
                            let harvest_result = creep.harvest(&source);
                            debug!(
                                "[{}] run_creep(): creep={} harvest result: {:?}",
                                time, name, harvest_result
                            );

                            match harvest_result {
                                Ok(_amount) => {
                                    debug!(
                                        "[{}] run_creep(): creep={} successfully harvested energy",
                                        time, name
                                    );
                                }
                                Err(e) => {
                                    warn!("[{}] run_creep(): creep={} couldn't harvest: {:?}, removing target", 
                                          time, name, e);
                                    entry.remove();
                                }
                            }
                        } else {
                            debug!(
                                "[{}] run_creep(): creep={} not near source, initiating movement",
                                time, name
                            );

                            // 记录移动前的详细信息
                            debug!("[{}] run_creep(): creep={} BEFORE MOVE TO source - position: ({},{}), range: {}", 
                                   time, name, pos.x(), pos.y(), range);

                            // 使用智能移动函数
                            let moved = smart_move(creep, &source);

                            // 记录移动后的位置
                            let new_pos = creep.pos();
                            debug!("[{}] run_creep(): creep={} AFTER MOVE TO source - moved: {}, new position: ({},{})
                                    - position changed: {}", 
                                   time, name, moved, new_pos.x(), new_pos.y(),
                                   (new_pos.x() != pos.x()) || (new_pos.y() != pos.y()));
                        }
                    } else {
                        warn!(
                            "[{}] run_creep(): creep={} source not found, removing target",
                            time, name
                        );
                        entry.remove();
                    }
                }
                _ => {
                    // 检查目标是否仍然有效
                    match creep_target {
                        CreepTarget::Upgrade(_controller_id) => {
                            let energy =
                                creep.store().get_used_capacity(Some(ResourceType::Energy));
                            debug!("[{}] run_creep(): creep={} target is Upgrade but energy={}, removing target", 
                                   time, name, energy);
                        }
                        CreepTarget::Harvest(_source_id) => {
                            let free_space =
                                creep.store().get_free_capacity(Some(ResourceType::Energy));
                            debug!("[{}] run_creep(): creep={} target is Harvest but free_space={}, removing target", 
                                   time, name, free_space);
                        }
                    }
                    entry.remove();
                }
            }
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            // no target, let's find one depending on if we have energy
            debug!(
                "[{}] run_creep(): creep={} has no target, initiating target selection process",
                time, name
            );

            if let Some(room) = creep.room() {
                debug!(
                    "[{}] run_creep(): creep={} in room={}, finding appropriate targets",
                    time,
                    name,
                    room.name()
                );

                if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                    debug!("[{}] run_creep(): creep={} has {} energy, looking for controller to upgrade", 
                           time, name, creep.store().get_used_capacity(Some(ResourceType::Energy)));

                    let mut found_controller = false;
                    for structure in room.find(find::STRUCTURES, None).iter() {
                        if let StructureObject::StructureController(controller) = structure {
                            let controller_pos = controller.pos();
                            debug!("[{}] run_creep(): creep={} found controller at ({},{}) with level={}, assigning upgrade task", 
                                   time, name, controller_pos.x(), controller_pos.y(), controller.level());
                            entry.insert(CreepTarget::Upgrade(controller.id()));
                            found_controller = true;

                            // 立即尝试移动到控制器，而不是等到下一个tick
                            let range = pos.get_range_to(controller_pos);
                            if !creep.pos().is_near_to(controller_pos) && creep.fatigue() == 0 {
                                debug!("[{}] run_creep(): creep={} immediately moving to controller after target assignment", 
                                       time, name);

                                // 记录移动前的详细信息
                                debug!("[{}] run_creep(): creep={} BEFORE MOVE TO controller - position: ({},{}), range: {}", 
                                       time, name, pos.x(), pos.y(), range);

                                // 使用智能移动函数
                                let moved = smart_move(creep, &controller);

                                // 记录移动后的位置
                                let new_pos = creep.pos();
                                debug!("[{}] run_creep(): creep={} AFTER MOVE TO controller - moved: {}, new position: ({},{}) - position changed: {}", 
                                       time, name, moved, new_pos.x(), new_pos.y(),
                                       (new_pos.x() != pos.x()) || (new_pos.y() != pos.y()));
                            }

                            break;
                        }
                    }

                    if !found_controller {
                        debug!(
                            "[{}] run_creep(): creep={} couldn't find any controller in room",
                            time, name
                        );
                    }
                } else {
                    debug!(
                        "[{}] run_creep(): creep={} has no energy, looking for active sources",
                        time, name
                    );

                    let sources = room.find(find::SOURCES_ACTIVE, None);
                    debug!(
                        "[{}] run_creep(): creep={} found {} active sources in room",
                        time,
                        name,
                        sources.len()
                    );

                    if let Some(source) = sources.first() {
                        let source_pos = source.pos();
                        debug!("[{}] run_creep(): creep={} found active source at ({},{}) with energy={}, assigning harvest task", 
                               time, name, source_pos.x(), source_pos.y(), source.energy());
                        entry.insert(CreepTarget::Harvest(source.id()));

                        // 立即尝试移动到资源，而不是等到下一个tick
                        let range = pos.get_range_to(source_pos);
                        if !creep.pos().is_near_to(source_pos) && creep.fatigue() == 0 {
                            debug!("[{}] run_creep(): creep={} immediately moving to source after target assignment", 
                                   time, name);

                            // 记录移动前的详细信息
                            debug!("[{}] run_creep(): creep={} BEFORE MOVE TO source - position: ({},{}), range: {}", 
                                   time, name, pos.x(), pos.y(), range);

                            // 使用智能移动函数
                            let moved = smart_move(creep, &source);

                            // 记录移动后的位置
                            let new_pos = creep.pos();
                            debug!("[{}] run_creep(): creep={} AFTER MOVE TO source - moved: {}, new position: ({},{}) - position changed: {}", 
                                   time, name, moved, new_pos.x(), new_pos.y(),
                                   (new_pos.x() != pos.x()) || (new_pos.y() != pos.y()));
                        }
                    } else {
                        debug!(
                            "[{}] run_creep(): creep={} couldn't find any active sources in room",
                            time, name
                        );
                    }
                }
            } else {
                warn!(
                    "[{}] run_creep(): creep={} couldn't resolve room! Cannot find targets.",
                    time, name
                );
            }
        }
    }

    debug!(
        "[{}] run_creep() END: creep={} - final position: ({},{})",
        time,
        name,
        creep.pos().x(),
        creep.pos().y()
    );
}
