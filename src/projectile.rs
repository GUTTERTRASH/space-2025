use crate::common::Player;
use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (fire_bullet, update_bullet));
    }
}

const BULLET_SPEED: f32 = 10.0;

#[derive(Component)]
pub struct Projectile {
    pub ballistic: bool,
    pub velocity: Vec3,
    pub direction: Vec3,
    // pub ray: Ray3d,
    pub speed: f32,
}

fn fire_bullet(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&Transform, With<Camera3d>>,
    player_query: Query<&GlobalTransform, With<Player>>,
) {
    if keys.pressed(KeyCode::ControlLeft) {
        let bullet_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            reflectance: 0.02,
            unlit: true,
            ..Default::default()
        });

        let camera_transform = camera_query.single();
        let player_global_transform = player_query.single();
        // let ray = Ray3d::from(camera_transform.compute_matrix());

        let direction = Vec3::X;
        // let dir: Vec3 = target
        //     .and_then(|intersection| {
        //         let target_transform = Transform::from_translation(intersection.position);
        //         let d =
        //             player_global_transform.looking_at(target_transform.translation, Vec3::Y);
        //         let d_ray = Ray3d::from(d.compute_matrix());
        //         Some(d_ray.direction.into())
        //     })
        //     .unwrap_or(ray.direction.into());

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(bullet_material),
            Transform::from_scale(Vec3::new(0.1, 0.1, 0.8))
                .with_translation(player_global_transform.translation())
                .with_rotation(camera_transform.rotation),
            Name::new("Bullet"),
            Projectile {
                ballistic: false,
                direction,
                velocity: direction * BULLET_SPEED,
                // ray,
                speed: BULLET_SPEED,
            },
        ));
    }
}

fn update_bullet(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Projectile, Entity)>,
    // player_query: Query<&GlobalTransform, With<PlayerModelTag>>,
) {
    for (mut transform, projectile, _entity) in query.iter_mut() {
        transform.translation += BULLET_SPEED * projectile.direction * time.delta_secs();
    }

    // let player_global_translation = player_query.single().translation;
    // for (mut transform, projectile, entity) in query.iter_mut() {
    //     if (transform.translation - player_global_translation).length_squared()
    //         > MAX_DISTANCE_SQUARED
    //     {
    //         commands.entity(entity).despawn();
    //     } else {
    //         transform.translation += BULLET_SPEED * projectile.direction * time.delta_seconds();
    //     }
    // }
}
