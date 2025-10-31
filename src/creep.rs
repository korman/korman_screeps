use log::*;
use screeps::{
    constants::ResourceType, enums::StructureObject, find, game, objects::Creep, HasId,
    HasPosition, SharedCreepProperties,
};
use std::collections::HashMap;

use crate::movement::smart_move;
use crate::types::CreepTarget;

/// 运行单个creep的逻辑
pub fn run_creep(creep: &Creep, creep_targets: &mut HashMap<String, CreepTarget>) {
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
                            "[{}] run_creep(): creep={} upgrade result: {:?} ",
                            time, name, upgrade_result
                        );

                        match upgrade_result {
                            Ok(_amount) => {
                                debug!(
                                    "[{}] run_creep(): creep={} successfully upgraded controller ",
                                    time, name
                                );
                            }
                            Err(e) => match e {
                                screeps::action_error_codes::UpgradeControllerErrorCode::NotInRange => {
                                    debug!("[{}] run_creep(): creep={} not in range of controller, initiating movement ", 
                                           time, name);

                                    // 记录移动前的详细信息
                                    debug!("[{}] run_creep(): creep={} BEFORE MOVE TO controller - position: ({},{}), range: {} ", 
                                           time, name, pos.x(), pos.y(), range);

                                    // 使用智能移动函数
                                    let moved = smart_move(creep, &controller);

                                    // 记录移动后的位置
                                    let new_pos = creep.pos();
                                    debug!("[{}] run_creep(): creep={} AFTER MOVE TO controller - moved: {}, new position: ({},{}) - position changed: {} ", 
                                           time, name, moved, new_pos.x(), new_pos.y(),
                                           (new_pos.x() != pos.x()) || (new_pos.y() != pos.y()));
                                }
                                _ => {
                                    warn!("[{}] run_creep(): creep={} could 't upgrade controller: {:?}, removing target ", 
                                          time, name, e);
                                    entry.remove();
                                }
                            },
                        }
                    } else {
                        warn!(
                            "[{}] run_creep(): creep={} controller not found, removing target ",
                            time, name
                        );
                        entry.remove();
                    }
                }
                CreepTarget::Harvest(source_id)
                    if creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    debug!(
                        "[{}] run_creep(): creep={} has space for energy, attempting to harvest ",
                        time, name
                    );

                    if let Some(source) = source_id.resolve() {
                        let source_pos = source.pos();
                        let range = pos.get_range_to(source_pos);
                        let in_range = creep.pos().is_near_to(source_pos);

                        debug!(
                            "[{}] run_creep(): creep={} source at ({},{}), range={}, in_range={} ",
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
                                "[{}] run_creep(): creep={} harvest result: {:?} ",
                                time, name, harvest_result
                            );

                            match harvest_result {
                                Ok(_amount) => {
                                    debug!(
                                        "[{}] run_creep(): creep={} successfully harvested energy ",
                                        time, name
                                    );
                                }
                                Err(e) => {
                                    warn!("[{}] run_creep(): creep={} couldn 't harvest: {:?}, removing target ", 
                                          time, name, e);
                                    entry.remove();
                                }
                            }
                        } else {
                            debug!(
                                "[{}] run_creep(): creep={} not near source, initiating movement ",
                                time, name
                            );

                            // 记录移动前的详细信息
                            debug!("[{}] run_creep(): creep={} BEFORE MOVE TO source - position: ({},{}), range: {} ", 
                                   time, name, pos.x(), pos.y(), range);

                            // 使用智能移动函数
                            let moved = smart_move(creep, &source);

                            // 记录移动后的位置
                            let new_pos = creep.pos();
                            debug!("[{}] run_creep(): creep={} AFTER MOVE TO source - moved: {}, new position: ({},{}) - position changed: {} ", 
                                   time, name, moved, new_pos.x(), new_pos.y(),
                                   (new_pos.x() != pos.x()) || (new_pos.y() != pos.y()));
                        }
                    } else {
                        warn!(
                            "[{}] run_creep(): creep={} source not found, removing target ",
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
                            debug!("[{}] run_creep(): creep={} target is Upgrade but energy={}, removing target ", 
                                   time, name, energy);
                        }
                        CreepTarget::Harvest(_source_id) => {
                            let free_space =
                                creep.store().get_free_capacity(Some(ResourceType::Energy));
                            debug!("[{}] run_creep(): creep={} target is Harvest but free_space={}, removing target ", 
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
                "[{}] run_creep(): creep={} has no target, initiating target selection process ",
                time, name
            );

            if let Some(room) = creep.room() {
                debug!(
                    "[{}] run_creep(): creep={} in room={}, finding appropriate targets ",
                    time,
                    name,
                    room.name()
                );

                if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                    debug!("[{}] run_creep(): creep={} has {} energy, looking for controller to upgrade ", 
                           time, name, creep.store().get_used_capacity(Some(ResourceType::Energy)));

                    let mut found_controller = false;
                    for structure in room.find(find::STRUCTURES, None).iter() {
                        if let StructureObject::StructureController(controller) = structure {
                            let controller_pos = controller.pos();
                            debug!("[{}] run_creep(): creep={} found controller at ({},{}) with level={}, assigning upgrade task ", 
                                   time, name, controller_pos.x(), controller_pos.y(), controller.level());
                            entry.insert(CreepTarget::Upgrade(controller.id()));
                            found_controller = true;

                            // 立即尝试移动到控制器，而不是等到下一个tick
                            let range = pos.get_range_to(controller_pos);
                            if !creep.pos().is_near_to(controller_pos) && creep.fatigue() == 0 {
                                debug!("[{}] run_creep(): creep={} immediately moving to controller after target assignment ", 
                                       time, name);

                                // 记录移动前的详细信息
                                debug!("[{}] run_creep(): creep={} BEFORE MOVE TO controller - position: ({},{}), range: {} ", 
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
                            "[{}] run_creep(): creep={} couldn 't find any controller in room ",
                            time, name
                        );
                    }
                } else {
                    debug!(
                        "[{}] run_creep(): creep={} has no energy, looking for active sources ",
                        time, name
                    );

                    let sources = room.find(find::SOURCES_ACTIVE, None);
                    debug!(
                        "[{}] run_creep(): creep={} found {} active sources in room ",
                        time,
                        name,
                        sources.len()
                    );

                    if let Some(source) = sources.first() {
                        let source_pos = source.pos();
                        debug!("[{}] run_creep(): creep={} found active source at ({},{}) with energy={}, assigning harvest task ", 
                               time, name, source_pos.x(), source_pos.y(), source.energy());
                        entry.insert(CreepTarget::Harvest(source.id()));

                        // 立即尝试移动到资源，而不是等到下一个tick
                        let range = pos.get_range_to(source_pos);
                        if !creep.pos().is_near_to(source_pos) && creep.fatigue() == 0 {
                            debug!("[{}] run_creep(): creep={} immediately moving to source after target assignment ", 
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
                            "[{}] run_creep(): creep={} couldn 't find any active sources in room ",
                            time, name
                        );
                    }
                }
            } else {
                warn!(
                    "[{}] run_creep(): creep={} couldn 't resolve room! Cannot find targets. ",
                    time, name
                );
            }
        }
    }

    debug!(
        "[{}] run_creep() END: creep={} - final position: ({},{}) ",
        time,
        name,
        creep.pos().x(),
        creep.pos().y()
    );
}
