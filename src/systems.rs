use hecs::World;
use js_sys::{JsString, Object, Reflect};
use screeps::{constants::Part, game};
use std::collections::{HashMap, HashSet};
use wasm_bindgen::JsCast;

use crate::creep::run_creep;
use crate::types::{CreepId, CreepTarget};

/// 系统: 运行 Creep
pub fn run_creep_system(world: &mut World, creep_targets: &mut HashMap<String, CreepTarget>) {
    for (_, creep_id) in world.query::<&CreepId>().iter() {
        if let Some(creep) = game::creeps().get(creep_id.0.clone()) {
            run_creep(&creep, creep_targets);
        }
    }
}

/// 系统: 生成 Creep
pub fn spawn_creep_system() {
    let mut additional = 0;
    for spawn in game::spawns().values() {
        let body = [Part::Move, Part::Move, Part::Carry, Part::Work];
        if spawn.room().unwrap().energy_available() >= body.iter().map(|p| p.cost()).sum() {
            let name_base = game::time();
            let name = format!("{}-{}", name_base, additional);
            match spawn.spawn_creep(&body, &name) {
                Ok(()) => additional += 1,
                Err(_) => {}
            }
        }
    }
}

/// 系统: 内存清理
pub fn memory_cleanup_system() {
    if game::time() % 1000 == 0 {
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
                    let _ = Reflect::delete_property(&memory_creeps, &creep_name_js);
                }
            }
        }
    }
}
