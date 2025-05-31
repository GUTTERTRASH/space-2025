use avian3d::prelude::{ExternalForce, RigidBody};
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
    mut query: Query<(&mut Transform, &mut ExternalForce), (With<Enemy>, Without<Player>)>,
    player: Query<&Transform, With<Player>>,
    mut timer: ResMut<CombatTimer>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let Ok(player_transform) = player.get_single() else {
            return;
        };

        let player_translation = player_transform.translation;
        // let mut rng = rand::thread_rng();
        let enemy_positions: Vec<_> = query.iter().map(|(t, _)| t.translation,).collect();
        for (i, (mut enemy_transform, mut enemy_rigidbody)) in query.iter_mut().enumerate() {
            let distance_to_player = enemy_transform.translation.distance(player_translation);

            if distance_to_player > 20.0 {
                
                // Move towards the player
                // let direction = (player_translation - enemy_transform.translation).normalize();

                // // Parabolic offset
                // let offset = Vec3::new(0.0, (time.elapsed_secs() * 2.0).sin() * 5.0, 0.0);

                // // Sinusoidal offset
                // let offset = Vec3::new(
                //     (time.elapsed_secs() * 2.0).sin() * 2.0,
                //     (time.elapsed_secs() * 2.0).cos() * 2.0,
                //     0.0,
                // );

                // let curved_direction = (direction + offset).normalize();
                // enemy_transform.translation += curved_direction * 0.1; // Move towards the player with a speed of 100 units per second

                // let curve_force = Vec3::new(0.0, (time.elapsed_secs() * 2.0).sin() * 5.0, 0.0);
                // let total_force = direction * 10.0 + curve_force;
                // enemy_rigidbody.apply_force(total_force);

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
                // enemy_transform.translation += direction * 0.5; // Move towards the player with a speed of 100 units per second


                // Bezier curve
                let control_point = (enemy_transform.translation + player_translation) / 2.0 + Vec3::Y * 100.0; // Control point above the midpoint for a curve
               
                let speed = 0.05;
                let t = ((time.elapsed_secs() * speed) % 1.0) as f32; // Loop through 0.0 to 1.0

                let new_position = bezier_curve(
                    enemy_transform.translation,
                    control_point,
                    player_translation,
                    t,
                );

                enemy_transform.translation = new_position;

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
                        let avoid_direction =
                            (enemy_transform.translation - *other_translation).normalize();
                        enemy_transform.translation += avoid_direction * 0.05; // Adjust position to avoid collision
                    }
                }
            }
        }
    }
}

fn bezier_curve(p0: Vec3, p1: Vec3, p2: Vec3, t: f32) -> Vec3 {
    let u = 1.0 - t;
    (u * u * p0) + (2.0 * u * t * p1) + (t * t * p2)
}