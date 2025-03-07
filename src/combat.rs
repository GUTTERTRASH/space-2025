use bevy::prelude::*;

use crate::common::{Enemy, Player};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CombatTimer(Timer::from_seconds(
            FIRE_RATE,
            TimerMode::Repeating,
        )))
        .add_systems(Update, combat_system);
    }
}

const FIRE_RATE: f32 = 0.02;

#[derive(Resource)]
struct CombatTimer(Timer);

fn combat_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Enemy>, Without<Player>)>,
    player: Query<&Transform, With<Player>>,
    mut timer: ResMut<CombatTimer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let player_translation = player.get_single().unwrap().translation;
        for mut transform in query.iter_mut() {
            // Simple AI: move towards the player
            let direction = (player_translation - transform.translation).normalize();
            transform.translation += direction * 0.1; // Move towards the player with a speed of 100 units per second
        }
    }
}
