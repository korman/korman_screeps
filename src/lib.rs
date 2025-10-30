use hecs::World;
use js_sys::{JsString, Object, Reflect};
use log::*;
use screeps::{
    action_error_codes::*,
    constants::{Part, ResourceType},
    enums::StructureObject,
    find, game,
    local::ObjectId,
    objects::{Creep, Source, StructureController},
    prelude::*,
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
    INIT_LOGGING.call_once(|| logging::setup_logging(logging::Info));

    let mut world = World::new();
    let mut creep_targets: HashMap<String, CreepTarget> = HashMap::new();

    debug!("loop starting! CPU: {}", game::cpu::get_used());

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

fn run_creep(creep: &Creep, creep_targets: &mut HashMap<String, CreepTarget>) {
    if creep.spawning() {
        return;
    }
    let name = creep.name();
    debug!("running creep {}", name);

    let target = creep_targets.entry(name.clone());
    match target {
        std::collections::hash_map::Entry::Occupied(entry) => {
            let creep_target = entry.get();
            match creep_target {
                CreepTarget::Upgrade(controller_id)
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    if let Some(controller) = controller_id.resolve() {
                        creep
                            .upgrade_controller(&controller)
                            .unwrap_or_else(|e| match e {
                                UpgradeControllerErrorCode::NotInRange => {
                                    let _ = creep.move_to(&controller);
                                }
                                _ => {
                                    warn!("couldn't upgrade: {:?}", e);
                                    entry.remove();
                                }
                            });
                    } else {
                        entry.remove();
                    }
                }
                CreepTarget::Harvest(source_id)
                    if creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    if let Some(source) = source_id.resolve() {
                        if creep.pos().is_near_to(source.pos()) {
                            creep.harvest(&source).unwrap_or_else(|e| {
                                warn!("couldn't harvest: {:?}", e);
                                entry.remove();
                            });
                        } else {
                            let _ = creep.move_to(&source);
                        }
                    } else {
                        entry.remove();
                    }
                }
                _ => {
                    entry.remove();
                }
            }
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            // no target, let's find one depending on if we have energy
            let room = creep.room().expect("couldn't resolve creep room");
            if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                for structure in room.find(find::STRUCTURES, None).iter() {
                    if let StructureObject::StructureController(controller) = structure {
                        entry.insert(CreepTarget::Upgrade(controller.id()));
                        break;
                    }
                }
            } else if let Some(source) = room.find(find::SOURCES_ACTIVE, None).first() {
                entry.insert(CreepTarget::Harvest(source.id()));
            }
        }
    }
}
