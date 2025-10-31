use log::*;
use rand::Rng;
use screeps::{
    action_error_codes::CreepMoveToErrorCode, constants::Part, enums::StructureObject, find, game,
    local::ObjectId, objects::Creep, HasPosition, SharedCreepProperties,
};
use std::sync::Once;

use crate::types::CreepTarget;

/// 辅助函数：从delta计算方向
pub fn direction_from_delta(dx: i8, dy: i8) -> screeps::Direction {
    match (dx, dy) {
        (0, -1) => screeps::Direction::Top,
        (1, -1) => screeps::Direction::TopRight,
        (1, 0) => screeps::Direction::Right,
        (1, 1) => screeps::Direction::BottomRight,
        (0, 1) => screeps::Direction::Bottom,
        (-1, 1) => screeps::Direction::BottomLeft,
        (-1, 0) => screeps::Direction::Left,
        (-1, -1) => screeps::Direction::TopLeft,
        _ => screeps::Direction::Top, // 默认向上
    }
}

/// 辅助函数：检查并记录creep的身体部件状态
pub fn log_creep_body_status(creep: &Creep) {
    let name = creep.name();
    let body = creep.body();
    let body_count = body.len();

    debug!(
        "log_creep_body_status({}): time={}, Total body parts:{} ",
        name,
        game::time(),
        body_count
    );
}

/// 智能移动函数 - 详细调试版本
pub fn smart_move(creep: &Creep, target: &impl HasPosition) -> bool {
    let name = creep.name();
    let creep_pos = creep.pos();
    let target_pos = target.pos();
    let time = game::time();

    debug!(
        "[{}] smart_move() START: creep={}, current_pos=({},{}), target_pos=({},{}), range={}",
        time,
        name,
        creep_pos.x(),
        creep_pos.y(),
        target_pos.x(),
        target_pos.y(),
        creep_pos.get_range_to(target_pos)
    );

    // 记录身体部件状态
    log_creep_body_status(creep);

    // 检查基本状态但不分析具体部件
    let body = creep.body();
    let body_count = body.len();
    let has_move_parts = body_count > 0; // 简化判断

    debug!("[{}] smart_move(): creep={}, has_body_parts={}, total_body_parts={}, fatigue={}, spawning={}", 
           time, name, has_move_parts, body_count, creep.fatigue(), creep.spawning());

    // 检查是否有疲劳值
    if creep.fatigue() > 0 {
        debug!(
            "[{}] smart_move(): creep={} cannot move due to fatigue={}",
            time,
            name,
            creep.fatigue()
        );
        return false;
    }

    // 如果没有移动部件，不能移动
    if !has_move_parts {
        warn!(
            "[{}] smart_move(): creep={} has no MOVE parts! Cannot move.",
            time, name
        );
        return false;
    }

    // 如果已经在目标位置附近，返回成功
    if creep_pos.in_range_to(target_pos, 1) {
        debug!(
            "[{}] smart_move(): creep={} already in range of target position",
            time, name
        );
        return true;
    }

    // 记录移动前的房间和地形信息
    if let Some(room) = creep.room() {
        debug!(
            "[{}] smart_move(): creep={} in room={}, checking terrain at current position",
            time,
            name,
            room.name()
        );
    }

    debug!(
        "[{}] smart_move(): creep={} preparing move_to with standard path options",
        time, name
    );

    // 记录移动操作
    let result = creep.move_to(target_pos);

    match result {
        Ok(()) => {
            info!("[{}] smart_move(): creep={} move_to succeeded", time, name);

            // 检查移动后位置是否变化
            let new_pos = creep.pos();
            if new_pos.x() != creep_pos.x() || new_pos.y() != creep_pos.y() {
                info!(
                    "[{}] smart_move(): creep={} moved from ({},{}) to ({},{})",
                    time,
                    name,
                    creep_pos.x(),
                    creep_pos.y(),
                    new_pos.x(),
                    new_pos.y()
                );
            } else {
                warn!(
                    "[{}] smart_move(): creep={} move_to returned Ok but position did not change!",
                    time, name
                );
            }

            return true;
        }
        Err(err) => {
            warn!(
                "[{}] smart_move(): creep={} move_to failed with error: {:?}",
                time, name, err
            );

            // 根据错误类型尝试不同的策略
            match err {
                CreepMoveToErrorCode::NoPath => {
                    warn!("[{}] smart_move(): creep={} No path found to target, attempting fallback strategies", 
                          time, name);

                    // 计算直接方向
                    let dx = (target_pos.x().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.x().to_string().parse::<i8>().unwrap_or(0))
                    .signum();
                    let dy = (target_pos.y().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.y().to_string().parse::<i8>().unwrap_or(0))
                    .signum();

                    let direction = direction_from_delta(dx, dy);
                    debug!("[{}] smart_move(): creep={} calculated direction={:?} from delta({},{}), attempting direct move", 
                           time, name, direction, dx, dy);

                    match creep.move_direction(direction) {
                        Ok(()) => {
                            info!(
                                "[{}] smart_move(): creep={} direct move to {:?} succeeded",
                                time, name, direction
                            );
                            return true;
                        }
                        Err(dir_err) => {
                            warn!("[{}] smart_move(): creep={} direct move failed with error: {:?}, trying random move", 
                                  time, name, dir_err);

                            // 尝试随机移动以避免卡住
                            let mut rng = rand::thread_rng();
                            let random_dir = rng.gen_range(1..=8);

                            let directions = [
                                screeps::Direction::Top,
                                screeps::Direction::TopRight,
                                screeps::Direction::Right,
                                screeps::Direction::BottomRight,
                                screeps::Direction::Bottom,
                                screeps::Direction::BottomLeft,
                                screeps::Direction::Left,
                                screeps::Direction::TopLeft,
                            ];

                            let rand_direction = directions[random_dir as usize - 1];
                            debug!("[{}] smart_move(): creep={} attempting random move in direction {:?}", 
                                  time, name, rand_direction);

                            match creep.move_direction(rand_direction) {
                                Ok(()) => {
                                    info!(
                                        "[{}] smart_move(): creep={} random move succeeded",
                                        time, name
                                    );
                                    return true;
                                }
                                Err(rand_err) => {
                                    error!("[{}] smart_move(): creep={} ALL movement attempts failed. Last error: {:?}", 
                                           time, name, rand_err);
                                    return false;
                                }
                            }
                        }
                    }
                }
                _ => {
                    warn!("[{}] smart_move(): creep={} encountered error: {:?}, attempting alternative approaches", 
                          time, name, err);

                    // 检查是否有障碍物
                    if let Some(room) = creep.room() {
                        // 移除不支持的look_for_at调用，使用更简单的方式记录
                        debug!("[{}] smart_move(): creep={} in room={}, checking for obstacles at current position", 
                               time, name, room.name());
                    }

                    // 尝试随机移动
                    let mut rng = rand::thread_rng();
                    let random_dir = rng.gen_range(1..=8);

                    let directions = [
                        screeps::Direction::Top,
                        screeps::Direction::TopRight,
                        screeps::Direction::Right,
                        screeps::Direction::BottomRight,
                        screeps::Direction::Bottom,
                        screeps::Direction::BottomLeft,
                        screeps::Direction::Left,
                        screeps::Direction::TopLeft,
                    ];

                    let rand_direction = directions[random_dir as usize - 1];
                    debug!("[{}] smart_move(): creep={} trying random direction {:?} due to error {:?}", 
                           time, name, rand_direction, err);

                    let random_move_result = creep.move_direction(rand_direction);
                    debug!(
                        "[{}] smart_move(): creep={} random move result: {:?}",
                        time, name, random_move_result
                    );
                }
            }

            debug!(
                "[{}] smart_move() END: creep={}, movement failed, returning false",
                time, name
            );
            return false;
        }
    }
}
