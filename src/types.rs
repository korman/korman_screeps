use screeps::local::ObjectId;
use screeps::objects::{Source, StructureController};

/// 定义CreepTarget枚举，用于表示creep的目标类型
#[derive(Clone, Debug)]
pub enum CreepTarget {
    /// 采集资源目标
    Harvest(ObjectId<Source>),
    /// 升级控制器目标
    Upgrade(ObjectId<StructureController>),
}

/// Hecs组件，用于存储creep ID
#[derive(Clone)]
pub struct CreepId(pub String); // Creep ID
