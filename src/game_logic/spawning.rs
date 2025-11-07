use log::{debug, warn};
use screeps::{constants::Part, game, SharedCreepProperties};

/// 生成新creep的函数
///
/// 检查所有可用的spawn，并在能量充足时生成新的creep（优先Worker，其次Hauler）
pub fn spawn_creep() {
    // 定义身体部件和成本
    let worker_body = [Part::Move, Part::Carry, Part::Work, Part::Work];
    let worker_cost: u32 = worker_body.iter().map(|p| p.cost()).sum();

    let hauler_body = [Part::Move, Part::Move, Part::Carry, Part::Carry];
    let hauler_cost: u32 = hauler_body.iter().map(|p| p.cost()).sum();

    // 统计当前存活creep数量（通过名称前缀）
    let worker_count = game::creeps()
        .values()
        .filter(|creep| creep.name().as_str().starts_with("Worker-"))
        .count() as i32;
    let hauler_count = game::creeps()
        .values()
        .filter(|creep| creep.name().as_str().starts_with("Hauler-"))
        .count() as i32;

    // 名称基础（当前游戏时间）和编号
    let name_base = game::time();
    let mut additional: i32 = 0;

    // 上限配置
    const MAX_WORKERS: i32 = 5;
    const MAX_HAULERS: i32 = 2;

    for spawn in game::spawns().values() {
        debug!("running spawn {}", spawn.name());

        // 跳过正在孵化的spawn
        if spawn.spawning().is_some() {
            continue;
        }

        let room = spawn.room().unwrap();
        let energy = room.energy_available();

        // 决定要孵化的角色
        let (role, body, _cost): (&str, &[Part], u32) =
            if worker_count < MAX_WORKERS && energy >= worker_cost {
                ("Worker", &worker_body, worker_cost)
            } else if hauler_count < MAX_HAULERS && energy >= hauler_cost {
                ("Hauler", &hauler_body, hauler_cost)
            } else {
                continue;
            };

        let name: String = format!("{}-{}-{}", role, name_base, additional);

        match spawn.spawn_creep(body, &name) {
            Ok(()) => {
                additional += 1;
                debug!("Spawned {}: {}", role, name);
            }
            Err(e) => warn!("Can't Spawn {:?}: {:?}", role, e),
        }
    }
}
