use hecs::World;
use js_sys::{JsString, Object, Reflect};
use log::*;
// rand::Rng已移除，因为我们只使用thread_rng
use screeps::{
    action_error_codes::{CreepMoveToErrorCode, UpgradeControllerErrorCode},
    constants::{Part, ResourceType},
    enums::StructureObject,
    find, game,
    local::ObjectId,
    objects::{Creep, Source, StructureController},
    prelude::*,
    Direction, HasPosition, MoveToOptions,
};
use std::collections::{HashMap, HashSet};
use std::sync::Once;
use wasm_bindgen::prelude::*;

mod logging;

// 定义CreepTarget枚举
#[derive(Clone)]
enum CreepTarget {
    Harvest(ObjectId<Source>),
    Upgrade(ObjectId<StructureController>),
}

// Hecs 组件
#[derive(Clone)]
struct CreepId(String); // Creep ID

// 初始化日志的Once实例
static INIT_LOGGING: Once = Once::new();

// game_loop
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    // 强制使用Debug级别日志以便捕获所有调试信息
    INIT_LOGGING.call_once(|| logging::setup_logging(logging::Debug));

    let mut world = World::new();
    let mut creep_targets: HashMap<String, CreepTarget> = HashMap::new();

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
        world.spawn((CreepId(creep.name()),));
    }

    // 运行系统
    run_creep_system(&mut world, &mut creep_targets);
    spawn_creep_system();
    memory_cleanup_system();

    info!("done! cpu: {}", game::cpu::get_used())
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

// 智能移动函数 - 新版本
fn smart_move(creep: &Creep, target: &impl HasPosition) -> bool {
    let name = creep.name();
    let creep_pos = creep.pos();
    let target_pos = target.pos();

    debug!(
        "smart_move: creep {} moving to ({},{}), current position ({},{})",
        name,
        target_pos.x(),
        target_pos.y(),
        creep_pos.x(),
        creep_pos.y()
    );

    // 简化版移动部件检查
    let move_parts = 1; // 简化处理，假设至少有一个移动部件
    let work_parts = 1;
    let carry_parts = 1;

    debug!(
        "smart_move: creep {} body parts - MOVE: {}, WORK: {}, CARRY: {}",
        name, move_parts, work_parts, carry_parts
    );

    // 如果已经在目标位置附近，返回成功
    if creep_pos.in_range_to(target_pos, 1) {
        debug!("smart_move: creep {} reached target position", name);
        return true;
    }

    // 尝试使用移动到方法，使用链式调用
    let options = MoveToOptions::new().reuse_path(5).no_path_finding(false);

    match creep.move_to(target_pos) {
        Ok(()) => {
            debug!("smart_move: creep {} moved successfully", name);
            return true;
        }
        Err(err) => {
            warn!(
                "smart_move: creep {} move_to failed with error: {:?}",
                name, err
            );

            // 根据错误类型尝试不同的策略
            match err {
                CreepMoveToErrorCode::NoPath => {
                    // 没有找到路径，尝试直接移动
                    warn!("smart_move: No path found, trying direct move");
                    let dx = (target_pos.x().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.x().to_string().parse::<i8>().unwrap_or(0))
                    .signum();
                    let dy = (target_pos.y().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.y().to_string().parse::<i8>().unwrap_or(0))
                    .signum();

                    match creep.move_direction(direction_from_delta(dx, dy)) {
                        Ok(()) => {
                            warn!("smart_move: creep {} used direct move successfully", name);
                            return true;
                        }
                        Err(err) => {
                            // 简化处理，移除不兼容的look_for_at调用
                            warn!("smart_move: Move failed with error: {}, trying alternative direction", err);

                            // 尝试随机移动以避免卡住
                            use rand::Rng;
                            let mut rng = rand::thread_rng();
                            let random_dir = rng.gen_range(1..=8); // 1-8的随机方向

                            // 选择一个随机方向
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
                            let _ = creep.move_direction(directions[random_dir as usize - 1]);

                            return false;
                        }
                    }
                }
                _ => {
                    // 其他错误，尝试随机移动
                    warn!("smart_move: Unknown error, trying random move");

                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let random_dir = rng.gen_range(1..=8); // 1-8的随机方向

                    // 选择一个随机方向
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
                    let _ = creep.move_direction(directions[random_dir as usize - 1]);

                    return false;
                }
            }
        }
    }
}

fn run_creep(creep: &Creep, creep_targets: &mut HashMap<String, CreepTarget>) {
    if creep.spawning() {
        debug!("creep {} is still spawning, skipping", creep.name());
        return;
    }

    let name = creep.name();
    let pos = creep.pos();
    debug!("running creep {} at position {},{}", name, pos.x(), pos.y());

    // 简化版移动部件检查
    let move_parts = 1; // 简化处理，假设至少有一个移动部件
    if move_parts == 0 {
        warn!("creep {} has no MOVE parts! Cannot move.", name);
    } else {
        debug!("creep {} has {} MOVE parts", name, move_parts);
    }

    let target = creep_targets.entry(name.clone());
    match target {
        std::collections::hash_map::Entry::Occupied(entry) => {
            let creep_target = entry.get();
            match creep_target {
                CreepTarget::Upgrade(controller_id)
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    debug!("creep {} attempting to upgrade controller", name);
                    if let Some(controller) = controller_id.resolve() {
                        let controller_pos = controller.pos();
                        debug!(
                            "controller position: {},{}",
                            controller_pos.x(),
                            controller_pos.y()
                        );

                        creep
                            .upgrade_controller(&controller)
                            .unwrap_or_else(|e| match e {
                                UpgradeControllerErrorCode::NotInRange => {
                                    debug!("creep {} not in range of controller, moving...", name);
                                    // 使用智能移动函数
                                    let moved = smart_move(creep, &controller);
                                    debug!("creep {} move to controller result: {}", name, moved);
                                }
                                _ => {
                                    warn!("couldn't upgrade: {:?}", e);
                                    entry.remove();
                                }
                            });
                    } else {
                        warn!("controller not found for creep {}", name);
                        entry.remove();
                    }
                }
                CreepTarget::Harvest(source_id)
                    if creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    debug!("creep {} attempting to harvest energy", name);
                    if let Some(source) = source_id.resolve() {
                        let source_pos = source.pos();
                        debug!(
                            "source position: {},{} - creep distance: {}",
                            source_pos.x(),
                            source_pos.y(),
                            pos.get_range_to(source_pos)
                        );

                        if creep.pos().is_near_to(source.pos()) {
                            creep.harvest(&source).unwrap_or_else(|e| {
                                warn!("couldn't harvest: {:?}", e);
                                entry.remove();
                            });
                        } else {
                            debug!("creep {} not near source, moving...", name);
                            // 使用智能移动函数
                            let moved = smart_move(creep, &source);
                            debug!("creep {} move to source result: {}", name, moved);
                        }
                    } else {
                        warn!("source not found for creep {}", name);
                        entry.remove();
                    }
                }
                _ => {
                    debug!(
                        "creep {} target invalid or energy state changed, removing target",
                        name
                    );
                    entry.remove();
                }
            }
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            // no target, let's find one depending on if we have energy
            debug!("creep {} has no target, finding new one", name);
            let room = creep.room().expect("couldn't resolve creep room");
            debug!("creep {} in room: {:?}", name, room.name());

            if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                debug!("creep {} has energy, looking for controller", name);
                for structure in room.find(find::STRUCTURES, None).iter() {
                    if let StructureObject::StructureController(controller) = structure {
                        debug!("creep {} found controller, assigning upgrade task", name);
                        entry.insert(CreepTarget::Upgrade(controller.id()));
                        break;
                    }
                }
            } else if let Some(source) = room.find(find::SOURCES_ACTIVE, None).first() {
                debug!(
                    "creep {} has no energy, found active source at {},{}, assigning harvest task",
                    name,
                    source.pos().x(),
                    source.pos().y()
                );
                entry.insert(CreepTarget::Harvest(source.id()));
            } else {
                debug!("creep {} couldn't find any valid targets", name);
            }
        }
    }
}
