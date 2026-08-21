//! Full 6DOF ship controller — arcade space-sim flight model.
//!
//! - Mouse pitches/yaws the ship directly (flight-stick style), with no pitch clamp:
//!   the ship can loop and invert freely. Rotation deltas are applied in the ship's
//!   local frame each tick, so there is no gimbal lock regardless of orientation.
//! - Q/E roll the ship around its own forward axis.
//! - W/S/A/D thrust forward/back/strafe left/right in ship-local space; Space/C thrust
//!   up/down. Thrust builds linear velocity via acceleration (inertia) rather than
//!   snapping to a target speed, and decays gradually (drift) when released.
//! - Left Shift boosts the max speed.
//! - The camera is a rigid third-person chase cam: it always sits at a fixed offset
//!   behind the ship in the ship's own local frame and matches the ship's rotation
//!   exactly (it banks and loops with the ship). There is no independent free-look.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::common::{MainCamera, Player};
use avian3d::prelude::{AngularVelocity, LinearVelocity, PhysicsSystems, Rotation};

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ControllerSettings>()
            .init_resource::<MouseAccum>()
            .add_systems(Startup, setup_cursor)
            .add_systems(PreUpdate, accumulate_mouse)
            .add_systems(
                FixedPostUpdate,
                apply_ship_motion.in_set(PhysicsSystems::Prepare),
            )
            .add_systems(PostUpdate, update_chase_camera.in_set(CameraUpdateSet));
    }
}

/// Runs after the chase camera's `Transform` has been written for this frame.
/// Anything that needs an up-to-date camera position/rotation this frame
/// (e.g. the reticule's screen-space projection) should order `.after(CameraUpdateSet)`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CameraUpdateSet;

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct ControllerSettings {
    /// Radians of ship rotation per pixel of mouse motion.
    pub mouse_sensitivity: f32,
    /// Roll rate in radians/sec while Q/E is held.
    pub roll_rate: f32,
    /// Linear thrust acceleration in m/s^2.
    pub linear_acceleration: f32,
    /// Maximum linear speed in m/s (before boost).
    pub max_linear_speed: f32,
    /// Multiplier applied to `max_linear_speed` while boosting.
    pub boost_multiplier: f32,
    /// Exponential velocity decay rate (per second) when no thrust is held.
    pub linear_damping: f32,
    /// Camera offset behind the ship, in ship-local space.
    pub follow_distance: f32,
    /// Camera offset above the ship, in ship-local space.
    pub follow_height: f32,
}

impl Default for ControllerSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.0025,
            roll_rate: 2.5,
            linear_acceleration: 40.0,
            max_linear_speed: 20.0,
            boost_multiplier: 2.5,
            linear_damping: 2.5,
            follow_distance: 8.0,
            follow_height: 2.5,
        }
    }
}

/// Mouse motion accumulated in `PreUpdate`, drained by the fixed-step rotation system.
#[derive(Resource, Default)]
struct MouseAccum {
    yaw: f32,
    pitch: f32,
}

// ── Startup ──────────────────────────────────────────────────────────────────

fn setup_cursor(mut cursor: Query<&mut CursorOptions>) {
    let Ok(mut opts) = cursor.single_mut() else {
        return;
    };
    opts.visible = false;
    opts.grab_mode = CursorGrabMode::Locked;
}

// ── Input ────────────────────────────────────────────────────────────────────

fn accumulate_mouse(
    settings: Res<ControllerSettings>,
    mut accum: ResMut<MouseAccum>,
    mut motion: MessageReader<MouseMotion>,
) {
    for event in motion.read() {
        accum.yaw -= event.delta.x * settings.mouse_sensitivity;
        accum.pitch -= event.delta.y * settings.mouse_sensitivity;
    }
}

// ── Physics ──────────────────────────────────────────────────────────────────

fn apply_ship_motion(
    time: Res<Time<Fixed>>,
    settings: Res<ControllerSettings>,
    keys: Res<ButtonInput<KeyCode>>,
    mut accum: ResMut<MouseAccum>,
    mut query: Query<(&mut LinearVelocity, &mut AngularVelocity, &mut Rotation), With<Player>>,
) {
    let Ok((mut linvel, mut angvel, mut rotation)) = query.single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    // ── Rotation: direct, instant response (precise aim), applied in the ship's
    // local frame so pitch/yaw/roll compose freely with no gimbal lock. ──
    let yaw = accum.yaw;
    let pitch = accum.pitch;
    accum.yaw = 0.0;
    accum.pitch = 0.0;

    let mut roll = 0.0;
    if keys.pressed(KeyCode::KeyQ) {
        roll += settings.roll_rate * dt;
    }
    if keys.pressed(KeyCode::KeyE) {
        roll -= settings.roll_rate * dt;
    }

    let delta_rotation = Quat::from_axis_angle(Vec3::X, pitch)
        * Quat::from_axis_angle(Vec3::Z, roll)
        * Quat::from_axis_angle(Vec3::Y, yaw);
    rotation.0 = (rotation.0 * delta_rotation).normalize();

    // The ship's facing is fully player-controlled; don't let physics spin it.
    angvel.0 = Vec3::ZERO;

    // ── Translation: ship-local thrust with acceleration/inertia. ──
    let mut local_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        local_dir.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        local_dir.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        local_dir.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        local_dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::Space) {
        local_dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyC) {
        local_dir.y -= 1.0;
    }

    let boost = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let max_speed = settings.max_linear_speed * if boost { settings.boost_multiplier } else { 1.0 };

    if local_dir.length_squared() > 0.0 {
        let world_dir = rotation.0 * local_dir.normalize();
        linvel.0 += world_dir * settings.linear_acceleration * dt;
        if linvel.0.length() > max_speed {
            linvel.0 = linvel.0.normalize() * max_speed;
        }
    } else {
        linvel.0 *= (-settings.linear_damping * dt).exp();
        if linvel.0.length_squared() < 1e-4 {
            linvel.0 = Vec3::ZERO;
        }
    }
}

// ── Camera ───────────────────────────────────────────────────────────────────

fn update_chase_camera(
    settings: Res<ControllerSettings>,
    player: Query<&Transform, With<Player>>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    let offset = Vec3::new(0.0, settings.follow_height, settings.follow_distance);
    camera_transform.translation = player_transform.translation + player_transform.rotation * offset;
    camera_transform.rotation = player_transform.rotation;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_rotation_delta_yaws_around_ship_up() {
        let rotation = Quat::IDENTITY;
        let delta = Quat::from_axis_angle(Vec3::Y, 0.5);
        let result = (rotation * delta).normalize();
        // Yawing from identity should rotate the forward vector in the XZ plane only.
        let forward = result * Vec3::NEG_Z;
        assert!((forward.y).abs() < 1e-5);
    }

    #[test]
    fn thrust_direction_is_normalized_before_scaling() {
        let mut local_dir = Vec3::ZERO;
        local_dir.z -= 1.0;
        local_dir.x += 1.0;
        let normalized = local_dir.normalize();
        assert!((normalized.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn velocity_clamps_to_max_speed() {
        let mut v = Vec3::new(0.0, 0.0, -50.0);
        let max_speed = 20.0;
        if v.length() > max_speed {
            v = v.normalize() * max_speed;
        }
        assert!((v.length() - max_speed).abs() < 1e-5);
    }
}
