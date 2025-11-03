use log::info;
use screeps::{
    constants::ResourceType,
    find,
    local::ObjectId,
    objects::{Creep, Source, StructureController},
    SharedCreepProperties,
};

use std::collections::{hash_map::Entry, HashMap};

// Creep目标类型定义
#[derive(Clone, Debug)]
pub enum CreepTarget {
    Upgrade(ObjectId<StructureController>),
    Harvest(ObjectId<Source>),
}

/// 处理单个creep的行为逻辑
///
/// # 参数
/// - `creep`: 当前要处理的creep对象
/// - `creep_targets`: creep目标映射表
pub fn run_creep(creep: &Creep, creep_targets: &mut HashMap<String, CreepTarget>) {
    if creep.spawning() {
        info!("creep {} is spawning", creep.name());
    }

    let name: String = creep.name().to_string();

    let target: Entry<'_, String, CreepTarget> = creep_targets.entry(name);

    match target {
        Entry::Occupied(entry) => {
            let creep_target = entry.get();

            info!("creep {} is working on {:?}", creep.name(), creep_target);

            match creep_target {
                CreepTarget::Upgrade(controller_id) => {}
                CreepTarget::Harvest(source_id) => {}
            }
        }
        Entry::Vacant(entry) => {
            info!("creep {} has no target", creep.name());

            let room = creep.room().expect("creep has no room");

            if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
            } else if let Some(source) = room.find(find::SOURCES_ACTIVE, None).first() {
            }
        }
    }
}
