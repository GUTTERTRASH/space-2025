use crate::common::Player;
use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ProjectileTimer(Timer::from_seconds(
            FIRE_RATE,
            TimerMode::Repeating,
        )))
        .add_systems(Update, (fire_bullet, update_bullet, detect_collisions));
    }
}

const MAX_DISTANCE: f32 = 10000.0;
const BULLET_SPEED: f32 = 200.0;
const FIRE_RATE: f32 = 0.02;

#[derive(Resource)]
struct ProjectileTimer(Timer);

#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
}

fn fire_bullet(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&Transform, With<Camera3d>>,
    player_query: Query<&GlobalTransform, With<Player>>,
    time: Res<Time>,
    mut timer: ResMut<ProjectileTimer>,
) {
    if keys.pressed(KeyCode::ControlLeft) {
        if timer.0.tick(time.delta()).just_finished() {
            let bullet_material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                ..Default::default()
            });

            let camera_transform = camera_query.single();
            let player_global_transform = player_query.single();
            let direction = camera_transform.forward().normalize();

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::default())),
                MeshMaterial3d(bullet_material),
                Transform::from_scale(Vec3::new(0.1, 0.1, 8.0))
                    .with_translation(player_global_transform.translation() + direction * 10.0)
                    .with_rotation(camera_transform.rotation),
                Name::new("Bullet"),
                Projectile {
                    velocity: direction * BULLET_SPEED,
                },
                PickingBehavior::IGNORE,
            ));
        }
    }
}

fn update_bullet(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Projectile, Entity), Without<Player>>,
    player: Query<&Transform, With<Player>>,
) {
    let player_translation = player.get_single().unwrap().translation;
    for (mut transform, projectile, entity) in query.iter_mut() {
        transform.translation += projectile.velocity * time.delta_secs();
        if (player_translation.distance(transform.translation)).abs() > MAX_DISTANCE {
            commands.entity(entity).despawn();
        }
    }
}

fn detect_collisions(
    mut commands: Commands,
    mut projectiles: Query<(&mut Transform, Entity), With<Projectile>>,
    meshes: Query<(&Transform, &Name, Entity), Without<Projectile>>,
) {
    for (projectile_transform, projectile_entity) in projectiles.iter_mut() {
        for (mesh_transform, name, mesh_entity) in meshes.iter() {
            let distance = projectile_transform
                .translation
                .distance(mesh_transform.translation);
            if distance < 1.0 {
                // TODO Fix bug where the projectile might already be despawned due to being too far
                commands.entity(projectile_entity).despawn();
                commands.entity(mesh_entity).despawn();
                info!("Hit {name}!");
                break;
            }
        }
    }
}
