//! Turret bullets, homing missiles, and lasers.
//!
//! All three are the same thing mechanically: a physics projectile spawned
//! with a `LinearVelocity` and a lifetime, differing only in speed, turning
//! (missiles home, the others fly straight), damage, and visuals. Lasers are
//! not hitscan — they're just very fast — so travel time only becomes
//! gameplay-relevant (dodgeable) at long range.
//!
//! Firing is on mouse buttons (left = turret, right = laser, middle =
//! missile) since the keyboard is fully committed to flight in
//! `controller`. Hit detection uses avian3d sensor collisions instead of a
//! per-frame distance scan.

use bevy::prelude::*;

use avian3d::prelude::{
    Collider, CollisionEventsEnabled, CollisionStart, LinearVelocity, PhysicsSystems, RigidBody,
    Rotation, Sensor,
};

use crate::combat::{AiMarker, CombatSettings, ShipDestroyed, Ship, Staggered};
use crate::common::Player;
use crate::vfx::{HitFlash, VfxSettings};

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponSettings>()
            .add_systems(Startup, setup_weapon_assets)
            .add_systems(
                Update,
                (
                    fire_turret,
                    fire_laser,
                    fire_missile,
                    tick_projectile_lifetime,
                    handle_projectile_hits.in_set(HitDetectionSet),
                ),
            )
            .add_systems(
                FixedPostUpdate,
                steer_missiles.in_set(PhysicsSystems::Prepare),
            );
    }
}

/// Runs after a frame's projectile hits have been resolved. `combat::action_system`
/// orders `.after(HitDetectionSet)` so a `Staggered` ship never gets one stray
/// frame of AI control before the exclusion takes effect.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HitDetectionSet;

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct WeaponSettings {
    pub turret_speed: f32,
    pub turret_damage: f32,
    pub turret_cooldown: f32,
    /// Used with the projectile's current velocity to compute knockback
    /// impulse on hit: `impulse = velocity * mass`.
    pub turret_mass: f32,

    pub laser_speed: f32,
    pub laser_damage: f32,
    pub laser_cooldown: f32,
    pub laser_mass: f32,

    pub missile_speed: f32,
    pub missile_turn_rate: f32,
    pub missile_damage: f32,
    pub missile_cooldown: f32,
    pub missile_lock_range: f32,
    pub missile_mass: f32,
    /// Half-angle, in radians, of the lock-on cone in front of the ship.
    pub missile_lock_cone: f32,

    /// Shared despawn timer for every projectile kind; this is what caps
    /// each weapon's effective range (speed * lifetime).
    pub projectile_lifetime: f32,
}

impl Default for WeaponSettings {
    fn default() -> Self {
        Self {
            turret_speed: 120.0,
            turret_damage: 5.0,
            turret_cooldown: 0.1,
            turret_mass: 0.2,

            laser_speed: 600.0,
            laser_damage: 15.0,
            laser_cooldown: 0.6,
            laser_mass: 0.1,

            missile_speed: 40.0,
            missile_turn_rate: 2.0,
            missile_damage: 40.0,
            missile_cooldown: 1.5,
            missile_lock_range: 150.0,
            missile_mass: 3.0,
            missile_lock_cone: 30f32.to_radians(),

            projectile_lifetime: 6.0,
        }
    }
}

#[derive(Resource)]
struct WeaponAssets {
    turret_mesh: Handle<Mesh>,
    turret_material: Handle<StandardMaterial>,
    laser_mesh: Handle<Mesh>,
    laser_material: Handle<StandardMaterial>,
    missile_mesh: Handle<Mesh>,
    missile_material: Handle<StandardMaterial>,
}

// ── Components ───────────────────────────────────────────────────────────────

/// Per-weapon cooldowns, in seconds remaining until the next shot is allowed.
#[derive(Component, Default)]
pub struct WeaponLoadout {
    turret_timer: f32,
    laser_timer: f32,
    missile_timer: f32,
}

#[derive(Component)]
pub struct Projectile {
    pub damage: f32,
    pub owner: Entity,
    pub mass: f32,
}

#[derive(Component)]
struct ProjectileLifetime(Timer);

#[derive(Component)]
pub struct Homing {
    pub target: Entity,
    pub turn_rate: f32,
    pub speed: f32,
}

// ── Startup ──────────────────────────────────────────────────────────────────

fn setup_weapon_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(WeaponAssets {
        turret_mesh: meshes.add(Cuboid::new(0.08, 0.08, 1.5)),
        turret_material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            ..default()
        }),
        laser_mesh: meshes.add(Cuboid::new(0.05, 0.05, 6.0)),
        laser_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 1.0, 1.0),
            unlit: true,
            ..default()
        }),
        missile_mesh: meshes.add(Cylinder::new(0.1, 0.8)),
        missile_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.55, 0.1),
            unlit: true,
            ..default()
        }),
    });
}

// ── Firing ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn spawn_projectile(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    origin: Vec3,
    rotation: Quat,
    velocity: Vec3,
    damage: f32,
    mass: f32,
    owner: Entity,
    lifetime: f32,
) -> Entity {
    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(origin).with_rotation(rotation),
            RigidBody::Kinematic,
            Collider::sphere(0.12),
            Sensor,
            CollisionEventsEnabled,
            LinearVelocity(velocity),
            Projectile { damage, owner, mass },
            ProjectileLifetime(Timer::from_seconds(lifetime, TimerMode::Once)),
        ))
        .id()
}

fn fire_turret(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    settings: Res<WeaponSettings>,
    assets: Res<WeaponAssets>,
    mut player: Query<(Entity, &Transform, &mut WeaponLoadout), With<Player>>,
) {
    let Ok((entity, transform, mut loadout)) = player.single_mut() else {
        return;
    };
    loadout.turret_timer -= time.delta_secs();
    if !mouse.pressed(MouseButton::Left) || loadout.turret_timer > 0.0 {
        return;
    }
    loadout.turret_timer = settings.turret_cooldown;

    let forward = *transform.forward();
    spawn_projectile(
        &mut commands,
        assets.turret_mesh.clone(),
        assets.turret_material.clone(),
        transform.translation + forward,
        transform.rotation,
        forward * settings.turret_speed,
        settings.turret_damage,
        settings.turret_mass,
        entity,
        settings.projectile_lifetime,
    );
}

fn fire_laser(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    settings: Res<WeaponSettings>,
    assets: Res<WeaponAssets>,
    mut player: Query<(Entity, &Transform, &mut WeaponLoadout), With<Player>>,
) {
    let Ok((entity, transform, mut loadout)) = player.single_mut() else {
        return;
    };
    loadout.laser_timer -= time.delta_secs();
    if !mouse.pressed(MouseButton::Right) || loadout.laser_timer > 0.0 {
        return;
    }
    loadout.laser_timer = settings.laser_cooldown;

    let forward = *transform.forward();
    spawn_projectile(
        &mut commands,
        assets.laser_mesh.clone(),
        assets.laser_material.clone(),
        transform.translation + forward,
        transform.rotation,
        forward * settings.laser_speed,
        settings.laser_damage,
        settings.laser_mass,
        entity,
        settings.projectile_lifetime,
    );
}

fn fire_missile(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    settings: Res<WeaponSettings>,
    assets: Res<WeaponAssets>,
    mut player: Query<(Entity, &Transform, &mut WeaponLoadout), With<Player>>,
    targets: Query<(Entity, &Transform), With<AiMarker>>,
) {
    let Ok((entity, transform, mut loadout)) = player.single_mut() else {
        return;
    };
    loadout.missile_timer -= time.delta_secs();
    if !mouse.just_pressed(MouseButton::Middle) || loadout.missile_timer > 0.0 {
        return;
    }

    let forward = *transform.forward();
    let lock = targets
        .iter()
        .filter_map(|(target, target_transform)| {
            let offset = target_transform.translation - transform.translation;
            let distance = offset.length();
            if distance < 1e-3 || distance > settings.missile_lock_range {
                return None;
            }
            let angle = forward.angle_between(offset / distance);
            (angle <= settings.missile_lock_cone).then_some((target, distance))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let Some((target, _)) = lock else {
        info!("Missile fire: no lock");
        return;
    };
    loadout.missile_timer = settings.missile_cooldown;

    let id = spawn_projectile(
        &mut commands,
        assets.missile_mesh.clone(),
        assets.missile_material.clone(),
        transform.translation + forward,
        transform.rotation,
        forward * settings.missile_speed,
        settings.missile_damage,
        settings.missile_mass,
        entity,
        settings.projectile_lifetime,
    );
    commands.entity(id).insert(Homing {
        target,
        turn_rate: settings.missile_turn_rate,
        speed: settings.missile_speed,
    });
}

// ── Flight ───────────────────────────────────────────────────────────────────

fn steer_missiles(
    time: Res<Time<Fixed>>,
    targets: Query<&Transform>,
    mut missiles: Query<(&Transform, &Homing, &mut LinearVelocity, &mut Rotation)>,
) {
    let dt = time.delta_secs();
    for (transform, homing, mut linvel, mut rotation) in &mut missiles {
        let Ok(target_transform) = targets.get(homing.target) else {
            // Target despawned — go ballistic rather than reacquiring; the
            // shared lifetime timer will clean this up regardless.
            continue;
        };

        let to_target = (target_transform.translation - transform.translation).normalize_or_zero();
        if to_target == Vec3::ZERO {
            continue;
        }

        let current_dir = {
            let d = linvel.0.normalize_or_zero();
            if d == Vec3::ZERO { *transform.forward() } else { d }
        };

        let rot_current = Quat::from_rotation_arc(Vec3::NEG_Z, current_dir);
        let rot_target = Quat::from_rotation_arc(Vec3::NEG_Z, to_target);
        let t = (homing.turn_rate * dt).min(1.0);
        let new_rot = rot_current.slerp(rot_target, t);
        let new_dir = new_rot * Vec3::NEG_Z;

        linvel.0 = new_dir * homing.speed;
        rotation.0 = new_rot;
    }
}

fn tick_projectile_lifetime(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ProjectileLifetime)>,
) {
    for (entity, mut lifetime) in &mut query {
        if lifetime.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// ── Hit detection ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn handle_projectile_hits(
    mut commands: Commands,
    mut events: MessageReader<CollisionStart>,
    mut destroyed: MessageWriter<ShipDestroyed>,
    combat_settings: Res<CombatSettings>,
    vfx_settings: Res<VfxSettings>,
    projectiles: Query<(&Projectile, &LinearVelocity)>,
    mut ships: Query<(&mut Ship, &mut LinearVelocity, &Transform), Without<Projectile>>,
) {
    for event in events.read() {
        let (a, b) = (event.collider1, event.collider2);

        let (proj_entity, other_entity, damage, owner, mass, proj_velocity) =
            if let Ok((p, v)) = projectiles.get(a) {
                (a, b, p.damage, p.owner, p.mass, v.0)
            } else if let Ok((p, v)) = projectiles.get(b) {
                (b, a, p.damage, p.owner, p.mass, v.0)
            } else {
                continue;
            };

        if other_entity == owner {
            continue;
        }

        if let Ok((mut ship, mut linvel, transform)) = ships.get_mut(other_entity) {
            ship.health -= damage;

            if ship.health <= 0.0 {
                destroyed.write(ShipDestroyed {
                    position: transform.translation,
                });
                commands.entity(other_entity).despawn();
            } else {
                let impulse = proj_velocity * mass;
                linvel.0 += impulse / ship.mass;
                commands
                    .entity(other_entity)
                    .insert(Staggered::new(
                        combat_settings.stagger_duration,
                        combat_settings.stagger_angular_kick,
                    ))
                    .insert(HitFlash::new(vfx_settings.flash_duration));
            }
        }

        commands.entity(proj_entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homing_slerp_reduces_angle_to_target() {
        let current_dir = Vec3::NEG_Z;
        let target_dir = Vec3::X;
        let rot_current = Quat::from_rotation_arc(Vec3::NEG_Z, current_dir);
        let rot_target = Quat::from_rotation_arc(Vec3::NEG_Z, target_dir);

        let before_angle = current_dir.angle_between(target_dir);
        let new_rot = rot_current.slerp(rot_target, 0.2);
        let after_angle = (new_rot * Vec3::NEG_Z).angle_between(target_dir);

        assert!(after_angle < before_angle);
    }

    #[test]
    fn lifetime_timer_despawns_on_finish() {
        let mut lifetime = ProjectileLifetime(Timer::from_seconds(1.0, TimerMode::Once));
        assert!(!lifetime
            .0
            .tick(std::time::Duration::from_secs_f32(0.5))
            .is_finished());
        assert!(lifetime
            .0
            .tick(std::time::Duration::from_secs_f32(0.6))
            .is_finished());
    }
}
