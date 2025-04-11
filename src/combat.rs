use bevy::prelude::*;
use rand::Rng;

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
        // let mut rng = rand::thread_rng();
        let enemy_positions: Vec<_> = query.iter().map(|t| t.translation).collect();
        for (i, mut enemy_transform) in query.iter_mut().enumerate() {
            let distance_to_player = enemy_transform.translation.distance(player_translation);

            if distance_to_player > 20.0 {
                // Move towards the player
                let direction = (player_translation - enemy_transform.translation).normalize();
                // let random_offset = Vec3::new(
                //     rng.gen_range(-0.1..0.1),
                //     rng.gen_range(-0.1..0.1),
                //     0.0,
                // );
                // let parabolic_direction = Vec3::new(
                //     direction.x + random_offset.x,
                //     direction.y + random_offset.y,
                //     direction.z,
                // ).normalize();
                // transform.translation += parabolic_direction * 0.1; // Move towards the player with a speed of 100 units per second
                enemy_transform.translation += direction * 0.1; // Move towards the player with a speed of 100 units per second
            } else {
                // Orbit the player
                // let angle = time.elapsed_secs() as f32;
                // let orbit_radius = 10.0;
                // enemy_transform.translation = Vec3::new(
                //     player_translation.x + orbit_radius * angle.cos(),
                //     player_translation.y + orbit_radius * angle.sin(),
                //     player_translation.z,
                // );
            }

            // Avoid other enemies
            for (j, other_translation) in enemy_positions.iter().enumerate() {
                if i != j {
                    let distance = enemy_transform.translation.distance(*other_translation);
                    if distance < 10.0 {
                        let avoid_direction = (enemy_transform.translation - *other_translation).normalize();
                        enemy_transform.translation += avoid_direction * 0.05; // Adjust position to avoid collision
                    }
                }
            }
        }
    }
}
