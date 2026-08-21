//! Cosmetic reactions to combat: hit flashes and death explosions/debris.
//!
//! Deliberately knows nothing about weapons or AI — it only reacts to the
//! `HitFlash` component it owns (inserted externally by
//! `weapons::handle_projectile_hits`) and one message,
//! `combat::ShipDestroyed`, that it listens for.

use bevy::prelude::*;
use rand::Rng;

use avian3d::prelude::{LinearVelocity, RigidBody};

use crate::combat::ShipDestroyed;

pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VfxSettings>()
            .add_systems(Startup, setup_vfx_assets)
            .add_systems(
                Update,
                (
                    apply_hit_flash,
                    spawn_death_explosion,
                    update_explosion_flash,
                    update_debris,
                ),
            );
    }
}

#[derive(Resource)]
pub struct VfxSettings {
    pub flash_duration: f32,
    pub flash_strength: f32,
    pub debris_count: usize,
    pub debris_speed_min: f32,
    pub debris_speed_max: f32,
    pub debris_lifetime_min: f32,
    pub debris_lifetime_max: f32,
    pub explosion_lifetime: f32,
    pub explosion_scale: f32,
}

impl Default for VfxSettings {
    fn default() -> Self {
        Self {
            flash_duration: 0.15,
            flash_strength: 6.0,
            debris_count: 8,
            debris_speed_min: 3.0,
            debris_speed_max: 9.0,
            debris_lifetime_min: 0.6,
            debris_lifetime_max: 1.0,
            explosion_lifetime: 0.25,
            explosion_scale: 1.5,
        }
    }
}

#[derive(Resource)]
struct VfxAssets {
    debris_mesh: Handle<Mesh>,
    explosion_mesh: Handle<Mesh>,
}

fn setup_vfx_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(VfxAssets {
        debris_mesh: meshes.add(Cuboid::new(0.08, 0.08, 0.08)),
        explosion_mesh: meshes.add(Sphere::new(0.3)),
    });
}

// ── Hit flash ────────────────────────────────────────────────────────────────

/// A brief emissive pulse on a target's own material, driven back down to
/// black over `duration`. Safe to mutate directly because every spawned
/// target already gets its own unique `StandardMaterial` instance
/// (`examples/basic.rs`'s `spawn_targets` gives each cube a distinct color).
#[derive(Component)]
pub struct HitFlash {
    timer: Timer,
}

impl HitFlash {
    pub fn new(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

fn apply_hit_flash(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<VfxSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Entity, &mut HitFlash, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (entity, mut flash, material_handle) in &mut query {
        let finished = flash.timer.tick(time.delta()).is_finished();
        let intensity = flash.timer.fraction_remaining() * settings.flash_strength;

        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            material.emissive = LinearRgba::WHITE * intensity;
        }

        if finished {
            if let Some(mut material) = materials.get_mut(&material_handle.0) {
                material.emissive = LinearRgba::BLACK;
            }
            commands.entity(entity).remove::<HitFlash>();
        }
    }
}

// ── Death explosion ──────────────────────────────────────────────────────────

#[derive(Component)]
struct ExplosionFlash {
    timer: Timer,
    max_scale: f32,
}

#[derive(Component)]
struct Debris {
    timer: Timer,
}

fn spawn_death_explosion(
    mut commands: Commands,
    mut events: MessageReader<ShipDestroyed>,
    settings: Res<VfxSettings>,
    assets: Res<VfxAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();

    for event in events.read() {
        let flash_material = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.8, 0.4, 1.0),
            emissive: LinearRgba::rgb(4.0, 2.5, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        commands.spawn((
            Mesh3d(assets.explosion_mesh.clone()),
            MeshMaterial3d(flash_material),
            Transform::from_translation(event.position).with_scale(Vec3::splat(0.1)),
            ExplosionFlash {
                timer: Timer::from_seconds(settings.explosion_lifetime, TimerMode::Once),
                max_scale: settings.explosion_scale,
            },
        ));

        for _ in 0..settings.debris_count {
            let direction = Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            )
            .normalize_or_zero();
            let speed = rng.gen_range(settings.debris_speed_min..settings.debris_speed_max);
            let lifetime = rng.gen_range(settings.debris_lifetime_min..settings.debris_lifetime_max);

            let debris_material = materials.add(StandardMaterial {
                base_color: Color::srgba(0.6, 0.6, 0.6, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            commands.spawn((
                Mesh3d(assets.debris_mesh.clone()),
                MeshMaterial3d(debris_material),
                Transform::from_translation(event.position),
                RigidBody::Kinematic,
                LinearVelocity(direction * speed),
                Debris {
                    timer: Timer::from_seconds(lifetime, TimerMode::Once),
                },
            ));
        }
    }
}

fn update_explosion_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(
        Entity,
        &mut ExplosionFlash,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for (entity, mut flash, mut transform, material_handle) in &mut query {
        let finished = flash.timer.tick(time.delta()).is_finished();
        let t = flash.timer.fraction();
        transform.scale = Vec3::splat(0.1 + flash.max_scale * t);

        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            material.base_color.set_alpha(1.0 - t);
        }

        if finished {
            commands.entity(entity).despawn();
        }
    }
}

fn update_debris(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Entity, &mut Debris, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (entity, mut debris, material_handle) in &mut query {
        let finished = debris.timer.tick(time.delta()).is_finished();
        let t = debris.timer.fraction();

        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            material.base_color.set_alpha(1.0 - t);
        }

        if finished {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_flash_intensity_decays_to_zero() {
        let mut flash = HitFlash::new(0.2);
        flash.timer.tick(std::time::Duration::from_secs_f32(0.1));
        let mid = flash.timer.fraction_remaining();
        flash.timer.tick(std::time::Duration::from_secs_f32(0.2));
        let end = flash.timer.fraction_remaining();
        assert!(end < mid);
        assert_eq!(end, 0.0);
    }
}
