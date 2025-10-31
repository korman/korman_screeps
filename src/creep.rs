use screeps::{
    constants::ResourceType, enums::StructureObject, find, objects::Creep, HasId, HasPosition,
    SharedCreepProperties,
};
use std::collections::HashMap;

use crate::movement::smart_move;
use crate::types::CreepTarget;

/// 运行单个creep的逻辑
pub fn run_creep(creep: &Creep, creep_targets: &mut HashMap<String, CreepTarget>) {
    let name = creep.name();

    // 基本状态检查
    if creep.spawning() {
        return;
    }

    // 获取当前位置
    let pos = creep.pos();

    // 简化版身体部件分析
    let body = creep.body();
    let _total_parts = body.len();

    // 目标处理逻辑
    let target = creep_targets.entry(name.clone());
    match target {
        std::collections::hash_map::Entry::Occupied(entry) => {
            let creep_target = entry.get();

            match creep_target {
                CreepTarget::Upgrade(controller_id)
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    if let Some(controller) = controller_id.resolve() {
                        let controller_pos = controller.pos();
                        let _range = pos.get_range_to(controller_pos);

                        // 升级控制器逻辑
                        let upgrade_result = creep.upgrade_controller(&controller);

                        match upgrade_result {
                            Ok(_amount) => {
                                // 升级成功，继续执行
                            }
                            Err(e) => match e {
                                screeps::action_error_codes::UpgradeControllerErrorCode::NotInRange => {
                                    // 使用智能移动函数
                                    let _moved = smart_move(creep, &controller);
                                }
                                _ => {
                                    // 其他错误，移除目标
                                    entry.remove();
                                }
                            },
                        }
                    } else {
                        // 控制器未找到，移除目标
                        entry.remove();
                    }
                }
                CreepTarget::Harvest(source_id)
                    if creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    if let Some(source) = source_id.resolve() {
                        let source_pos = source.pos();
                        let _range = pos.get_range_to(source_pos);
                        let in_range = creep.pos().is_near_to(source_pos);

                        if in_range {
                            let harvest_result = creep.harvest(&source);

                            match harvest_result {
                                Ok(_amount) => {
                                    // 采集成功，继续执行
                                }
                                Err(_e) => {
                                    // 采集失败，移除目标
                                    entry.remove();
                                }
                            }
                        } else {
                            // 不在范围内，移动到资源
                            // 记录移动前的详细信息
                            let _range = pos.get_range_to(source_pos);

                            // 使用智能移动函数
                            let _moved = smart_move(creep, &source);

                            // 记录移动后的位置
                            let _new_pos = creep.pos();
                        }
                    } else {
                        // 资源未找到，移除目标
                        entry.remove();
                    }
                }
                _ => {
                    // 检查目标是否仍然有效
                    entry.remove();
                }
            }
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
            // 没有目标，根据能量状态寻找目标
            if let Some(room) = creep.room() {
                if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                    let mut found_controller = false;
                    for structure in room.find(find::STRUCTURES, None).iter() {
                        if let StructureObject::StructureController(controller) = structure {
                            let controller_pos = controller.pos();
                            entry.insert(CreepTarget::Upgrade(controller.id()));
                            found_controller = true;

                            // 立即尝试移动到控制器，而不是等到下一个tick
                            let _range = pos.get_range_to(controller_pos);
                            if !creep.pos().is_near_to(controller_pos) && creep.fatigue() == 0 {
                                // 使用智能移动函数
                                let _moved = smart_move(creep, &controller);
                                // 记录移动后的位置
                                let _new_pos = creep.pos();
                            }

                            break;
                        }
                    }

                    if !found_controller {
                        // 未找到控制器
                    }
                } else {
                    let sources = room.find(find::SOURCES_ACTIVE, None);
                    if let Some(source) = sources.first() {
                        let source_pos = source.pos();
                        entry.insert(CreepTarget::Harvest(source.id()));

                        // 立即尝试移动到资源，而不是等到下一个tick
                        let _range = pos.get_range_to(source_pos);
                        if !creep.pos().is_near_to(source_pos) && creep.fatigue() == 0 {
                            // 使用智能移动函数
                            let _moved = smart_move(creep, &source);
                            // 记录移动后的位置
                            let _new_pos = creep.pos();
                        }
                    } else {
                        // 未找到活动资源
                    }
                }
            } else {
                // 无法解析房间，无法寻找目标
            }
        }
    }
}
