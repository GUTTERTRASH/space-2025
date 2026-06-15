//! Third-person camera and 6DOF ship controller.
//!
//! - Mouse freely orbits the camera around the player (independent of ship orientation).
//! - WASD + Q/E provide 6DOF thrust along camera axes.
//! - W/S commit heading to the camera look direction, slerp the ship, and thrust forward/back.
//! - A/D commit the same heading, slerp the ship, then strafe once aligned.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::common::{MainCamera, Player};
use avian3d::prelude::{AngularVelocity, LinearVelocity, PhysicsSystems, Rotation};

const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
const HEADING_ALIGNED_DOT: f32 = 0.999;

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ControllerSettings>()
            .init_resource::<CameraState>()
            .init_resource::<ShipInput>()
            .init_resource::<CommittedHeading>()
            .add_systems(Startup, setup_cursor)
            .add_systems(
                PreUpdate,
                (mouse_look, gather_input).chain(),
            )
            .add_systems(
                FixedPostUpdate,
                apply_ship_motion.in_set(PhysicsSystems::Prepare),
            )
            .add_systems(PostUpdate, update_third_person_camera);
    }
}

// ── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct ControllerSettings {
    pub follow_distance: f32,
    pub follow_height: f32,
    pub mouse_sensitivity: f32,
    pub move_speed: f32,
    pub sprint_multiplier: f32,
    /// Ship heading slerp rate (`slerp(target, rate * dt)`).
    pub turn_slerp_rate: f32,
    /// Per physics-tick velocity decay when no thrust keys are held.
    pub velocity_damping: f32,
}

impl Default for ControllerSettings {
    fn default() -> Self {
        Self {
            follow_distance: 6.0,
            follow_height: 1.5,
            mouse_sensitivity: 0.003,
            move_speed: 12.0,
            sprint_multiplier: 2.5,
            turn_slerp_rate: 6.0,
            velocity_damping: 0.88,
        }
    }
}

/// Camera yaw/pitch — fully independent of the ship.
#[derive(Resource)]
pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

/// Written in PreUpdate, consumed in FixedPostUpdate.
#[derive(Resource, Default)]
struct ShipInput {
    desired_velocity: Vec3,
    heading_target: Option<Quat>,
    thrusting: bool,
}

/// Camera heading captured when W, S, A, or D is first pressed.
#[derive(Resource, Default)]
struct CommittedHeading {
    forward: Option<Vec3>,
    right: Option<Vec3>,
    rotation: Option<Quat>,
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

fn mouse_look(
    settings: Res<ControllerSettings>,
    mut camera_state: ResMut<CameraState>,
    mut motion: MessageReader<MouseMotion>,
) {
    let mut delta = Vec2::ZERO;
    for event in motion.read() {
        delta -= event.delta;
    }
    if delta.length_squared() < 1e-8 {
        return;
    }

    delta *= settings.mouse_sensitivity;
    camera_state.yaw += delta.x;
    camera_state.pitch = (camera_state.pitch + delta.y).clamp(-PITCH_LIMIT, PITCH_LIMIT);
}

fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<ControllerSettings>,
    camera_state: Res<CameraState>,
    player: Query<&Transform, With<Player>>,
    mut committed: ResMut<CommittedHeading>,
    mut input: ResMut<ShipInput>,
) {
    input.desired_velocity = Vec3::ZERO;
    input.heading_target = None;
    input.thrusting = false;

    let forward_back = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::KeyS);
    let strafe = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::KeyD);
    let heading_keys = forward_back || strafe;

    if !heading_keys {
        committed.forward = None;
        committed.right = None;
        committed.rotation = None;
    }

    if !keys.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyQ,
        KeyCode::KeyE,
    ]) {
        return;
    }

    let camera_basis = camera_rotation(&camera_state);
    let cam_forward = camera_basis * Vec3::NEG_Z;
    let cam_right = camera_basis * Vec3::X;
    let cam_up = Vec3::Y;

    let just_committed = keys.just_pressed(KeyCode::KeyW)
        || keys.just_pressed(KeyCode::KeyS)
        || keys.just_pressed(KeyCode::KeyA)
        || keys.just_pressed(KeyCode::KeyD);

    if heading_keys && just_committed {
        let heading = heading_from_forward(cam_forward);
        committed.forward = Some(cam_forward);
        committed.right = Some(heading * Vec3::X);
        committed.rotation = Some(heading);
    }

    let forward = committed.forward.unwrap_or(cam_forward);
    let right = committed.right.unwrap_or(cam_right);
    let up = cam_up;

    let aligned = committed
        .rotation
        .and_then(|target| player.single().ok().map(|p| is_heading_aligned(p.rotation, target)))
        .unwrap_or(false);

    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    // Strafe only after the ship has turned to face the committed camera heading.
    if strafe && aligned {
        if keys.pressed(KeyCode::KeyD) {
            direction += right;
        }
        if keys.pressed(KeyCode::KeyA) {
            direction -= right;
        }
    }
    if keys.pressed(KeyCode::KeyE) {
        direction += up;
    }
    if keys.pressed(KeyCode::KeyQ) {
        direction -= up;
    }

    let sprint = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let speed = settings.move_speed * if sprint { settings.sprint_multiplier } else { 1.0 };

    if direction.length_squared() > 0.0 {
        input.desired_velocity = direction.normalize() * speed;
        input.thrusting = true;
    }

    if heading_keys {
        input.heading_target = committed.rotation;
    }
}

// ── Physics ──────────────────────────────────────────────────────────────────

fn apply_ship_motion(
    time: Res<Time<Fixed>>,
    settings: Res<ControllerSettings>,
    input: Res<ShipInput>,
    mut query: Query<(&mut LinearVelocity, &mut Rotation, &mut AngularVelocity), With<Player>>,
) {
    let Ok((mut linvel, mut rotation, mut ang_vel)) = query.single_mut() else {
        return;
    };

    if input.thrusting {
        linvel.0 = input.desired_velocity;
    } else {
        linvel.0 *= settings.velocity_damping;
        if linvel.0.length_squared() < 1e-4 {
            linvel.0 = Vec3::ZERO;
        }
    }

    if let Some(target) = input.heading_target {
        if !is_heading_aligned(rotation.0, target) {
            let t = (settings.turn_slerp_rate * time.delta_secs()).min(1.0);
            rotation.0 = rotation.0.slerp(target, t);
            ang_vel.0 = Vec3::ZERO;
        }
    }
}

// ── Camera ───────────────────────────────────────────────────────────────────

fn update_third_person_camera(
    settings: Res<ControllerSettings>,
    camera_state: Res<CameraState>,
    player: Query<&Transform, With<Player>>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    let rot = camera_rotation(&camera_state);
    let forward = rot * Vec3::NEG_Z;

    camera_transform.translation = player_transform.translation
        - forward * settings.follow_distance
        + Vec3::Y * settings.follow_height;
    camera_transform.rotation = rot;
}

// ── Math helpers ─────────────────────────────────────────────────────────────

fn camera_rotation(state: &CameraState) -> Quat {
    Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0)
}

fn heading_from_forward(forward: Vec3) -> Quat {
    let dir = forward.normalize_or_zero();
    if dir.length_squared() < 1e-6 {
        return Quat::IDENTITY;
    }
    let up = if dir.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
    Transform::from_translation(Vec3::ZERO)
        .looking_to(dir, up)
        .rotation
}

fn is_heading_aligned(current: Quat, target: Quat) -> bool {
    current.dot(target).abs() >= HEADING_ALIGNED_DOT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_input_normalizes() {
        let rot = camera_rotation(&CameraState::default());
        let forward = rot * Vec3::NEG_Z;
        let right = rot * Vec3::X;
        let mut direction = forward + right;
        direction = direction.normalize();
        assert!((direction.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn heading_from_forward_faces_neg_z() {
        let heading = heading_from_forward(Vec3::NEG_Z);
        let expected = Transform::from_translation(Vec3::ZERO)
            .looking_to(Vec3::NEG_Z, Vec3::Y)
            .rotation;
        assert!(heading.dot(expected).abs() > 0.999);
    }
}