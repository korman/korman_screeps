use log::{debug, warn};
use screeps::{constants::Part, game};

/// 生成新creep的函数
///
/// 检查所有可用的spawn，并在能量充足时生成新的creep
pub fn spawn_creep() {
    let mut additional: i32 = 0;

    for spawn in game::spawns().values() {
        debug!("running spawn {}", spawn.name());

        let body = [Part::Move, Part::Move, Part::Carry, Part::Work];

        if spawn.room().unwrap().energy_available() >= body.iter().map(|p: &Part| p.cost()).sum() {
            let name_base = game::time();
            let name: String = format!("{}-{}", name_base, additional);

            match spawn.spawn_creep(&body, &name) {
                Ok(()) => additional += 1,
                Err(e) => warn!("Can't Spawn: {:?}", e),
            }
        }
    }
}
