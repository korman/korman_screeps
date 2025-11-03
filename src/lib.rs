use hecs::World;
use log::LevelFilter::Info;
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

use js_sys::{JsString, Object, Reflect};

use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::Once,
};

use wasm_bindgen::prelude::*;

// 导入子模块
mod logging;

thread_local! {
    static CREEP_TARGETS: RefCell<HashMap<String, CreepTarget>> = RefCell::new(HashMap::new());
}

// 导出公共API
// pub use types::*;

// 日志初始化静态变量
static INIT_LOGGING: Once = Once::new();

#[derive(Clone)]
enum CreepTarget {
    Upgrade(ObjectId<StructureController>),
    Harvest(ObjectId<Source>),
}

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

    if game::time() % 1000 == 0 {
        info!("running memory cleanup");

        let mut alive_creeps: HashSet<String> = HashSet::new();

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

    info!(
        "done! cpu: {}, tick_limit: {}",
        game::cpu::get_used(),
        game::cpu::tick_limit()
    );
}
