use rand::Rng;
use screeps::{
    action_error_codes::CreepMoveToErrorCode, objects::Creep, HasPosition, SharedCreepProperties,
};

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

/// 智能移动函数
pub fn smart_move(creep: &Creep, target: &impl HasPosition) -> bool {
    let creep_pos = creep.pos();
    let target_pos = target.pos();

    // 检查基本状态但不分析具体部件
    let body = creep.body();
    let has_move_parts = body.len() > 0; // 简化判断

    // 检查是否有疲劳值
    if creep.fatigue() > 0 {
        return false;
    }

    // 如果没有移动部件，不能移动
    if !has_move_parts {
        return false;
    }

    // 如果已经在目标位置附近，返回成功
    if creep_pos.in_range_to(target_pos, 1) {
        return true;
    }

    // 尝试标准移动
    let result = creep.move_to(target_pos);

    match result {
        Ok(()) => {
            // 检查移动后位置是否变化
            let new_pos = creep.pos();
            if new_pos.x() != creep_pos.x() || new_pos.y() != creep_pos.y() {
                return true;
            } else {
                // 移动返回成功但位置未变，尝试备选策略
            }
        }
        Err(err) => {
            // 根据错误类型尝试不同的策略
            match err {
                CreepMoveToErrorCode::NoPath => {
                    // 计算直接方向
                    let dx = (target_pos.x().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.x().to_string().parse::<i8>().unwrap_or(0))
                    .signum();
                    let dy = (target_pos.y().to_string().parse::<i8>().unwrap_or(0)
                        - creep_pos.y().to_string().parse::<i8>().unwrap_or(0))
                    .signum();

                    let direction = direction_from_delta(dx, dy);
                    match creep.move_direction(direction) {
                        Ok(()) => {
                            return true;
                        }
                        Err(_dir_err) => {
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
                            match creep.move_direction(rand_direction) {
                                Ok(()) => {
                                    return true;
                                }
                                Err(_rand_err) => {
                                    return false;
                                }
                            }
                        }
                    }
                }
                _ => {
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
                    let _random_move_result = creep.move_direction(rand_direction);
                }
            }
        }
    }

    return false;
}
