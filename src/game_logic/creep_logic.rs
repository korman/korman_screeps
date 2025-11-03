use log::info;
use screeps::{
    local::ObjectId,
    objects::{Creep, Source, StructureController},
    SharedCreepProperties,
};

use std::{
    collections::{hash_map::Entry, HashMap},
};

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

    let name:String = creep.name().to_string();

    let target:Entry<'_,String,CreepTarget> = creep_targets.entry(name);
}
