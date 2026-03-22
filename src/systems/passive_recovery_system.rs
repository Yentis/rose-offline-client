use std::time::Duration;

use bevy::prelude::{Query, Res, Time};
use rose_game_common::{
    components::{AbilityValues, HealthPoints, ManaPoints},
    data::PassiveRecoveryState,
};

use crate::components::{Command, Dead, PassiveRecoveryTime};

const RECOVERY_INTERVAL: Duration = Duration::from_secs(4);

pub fn passive_recovery_system(
    mut query: Query<(
        &mut PassiveRecoveryTime,
        &AbilityValues,
        &Command,
        Option<&Dead>,
        &mut HealthPoints,
        &mut ManaPoints,
    )>,
    time: Res<Time>,
) {
    let delta = time.delta();

    for (
        mut passive_recovery_time,
        ability_values,
        command,
        dead,
        mut health_points,
        mut mana_points,
    ) in query.iter_mut()
    {
        passive_recovery_time.time += delta;

        if passive_recovery_time.time > RECOVERY_INTERVAL {
            passive_recovery_time.time -= RECOVERY_INTERVAL;

            if dead.is_some() {
                // No recovery whilst dead!
                continue;
            }

            let recovery_state = if command.is_sit() {
                PassiveRecoveryState::Sitting
            } else {
                PassiveRecoveryState::Normal
            };

            let recover_hp = ability_values.get_health_recovery(recovery_state);
            let recover_mp = ability_values.get_mana_recovery(recovery_state);

            health_points.hp = i32::min(
                health_points.hp + recover_hp,
                ability_values.get_max_health(),
            );
            mana_points.mp = i32::min(mana_points.mp + recover_mp, ability_values.get_max_mana());
        }
    }
}
