use log::info;
use log::{debug, warn};
use screeps::HasPosition;
use screeps::{
    action_error_codes::*,
    constants::ResourceType,
    enums::StructureObject,
    find,
    local::ObjectId,
    objects::{Creep, Source, StructureController},
    prelude::*,
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
                CreepTarget::Upgrade(controller_id) => {
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                        if let Some(controller) = controller_id.resolve() {
                            creep
                                .upgrade_controller(&controller)
                                .unwrap_or_else(|e| match e {
                                    UpgradeControllerErrorCode::NotInRange => {
                                        let _ = creep.move_to(&controller);
                                    }
                                    _ => {
                                        warn!("can't upgrade controller {:?}", e);
                                        entry.remove();
                                    }
                                });
                        } else {
                            entry.remove();
                        }
                    } else {
                        // 能量耗尽，移除目标以便重新分配任务
                        debug!(
                            "creep {} energy depleted, removing upgrade target",
                            creep.name()
                        );
                        entry.remove();
                    }
                }
                CreepTarget::Harvest(source_id) => {
                    if creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0 {
                        if let Some(source) = source_id.resolve() {
                            if creep.pos().is_near_to(source.pos()) {
                                creep.harvest(&source).unwrap_or_else(|e| {
                                    warn!("can't harvest source {:?}", e);
                                    entry.remove();
                                });
                            } else {
                                let _ = creep.move_to(&source);
                            }
                        }
                    } else {
                        entry.remove();
                    }
                }
            }
        }
        Entry::Vacant(entry) => {
            info!("creep {} has no target", creep.name());

            let room = creep.room().expect("creep has no room");

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
