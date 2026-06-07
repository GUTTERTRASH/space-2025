use avian3d::math::*;
use avian3d::prelude::*;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
// use bevy_third_person_camera::{ThirdPersonCamera, ThirdPersonCameraTarget};
use std::ops::Deref;

use crate::common::MainCamera;
use crate::common::Player;

#[derive(Resource)]
pub struct CameraSettings {
    pub mouse_sensitivity: f32,
    pub follow_distance: f32,
    pub follow_height: f32,
    pub rotation_slerp_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.0025,
            follow_distance: 10.0,   // further back for better third-person space view
            follow_height: 3.5,
            rotation_slerp_speed: 8.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct CameraOrbit {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .init_resource::<CameraOrbit>()
            .add_message::<TranslationEvent>()
            .add_systems(Startup, setup_cursor)
            .add_systems(Update, handle_keyboard_input)
            .add_systems(Update, handle_mouse_look)
            .add_systems(Update, update_chase_camera.after(handle_mouse_look))
            .add_systems(Update, orient_player_to_camera_view.after(update_chase_camera))
            .add_systems(FixedUpdate, (translate_player, dampen_movement).chain());
    }
}

fn setup_cursor(mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.single_mut() {
        // Cursor grab/visibility API varies slightly by Bevy patch and platform.
        // These are the most common names in 0.18 era. If it doesn't compile,
        // comment this out — MouseMotion events will still work.
        // window.cursor_options.visible = false;
        // window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Confined;
    }
}



fn handle_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, (With<MainCamera>, Without<Player>)>,
    mut translations: MessageWriter<TranslationEvent>,
    // mut rotations: EventWriter<RotationEvent>,
) {
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

    let Ok(camera_transform) = camera_query.single() else {
        warn!("Camera not found");
        return;
    };

    let forward_input = keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let backward_input = keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left_input = keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right_input = keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    let up_input = keys.pressed(KeyCode::KeyE);
    let down_input = keys.pressed(KeyCode::KeyQ);

    let forward_signal = forward_input as i8 - backward_input as i8;
    let right_signal = right_input as i8 - left_input as i8;
    let up_signal = up_input as i8 - down_input as i8;

    let xz = Vec3::new(1.0, 0.0, 1.0);
    let (forward, right, up) = (
        (*camera_transform.forward() * xz).normalize(),
        (*camera_transform.right() * xz).normalize(),
        Vec3::Y,
    );

    let direction = ((forward_signal as Scalar * forward)
        + (right_signal as Scalar * right)
        + (up_signal as Scalar * up))
        .clamp_length_max(1.0);

    if direction != Vector3::ZERO {
        translations.write(TranslationEvent::new(&direction));
    }
}

#[derive(Message, Event, Debug, Default)]
pub struct TranslationEvent {
    value: Vec3,
}

impl Deref for TranslationEvent {
    type Target = Vec3;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl TranslationEvent {
    pub fn new(value: &Vec3) -> Self {
        Self { value: *value }
    }
}

fn translate_player(
    time: Res<Time>,
    mut events: MessageReader<TranslationEvent>,
    mut query: Query<&mut LinearVelocity, With<Player>>,
) {
    let delta_time = time.delta_secs_f64().adjust_precision();
    let acceleration = 30.0;

    let Ok(mut linear_velocity) = query.single_mut() else {
        return;
    };

    for event in events.read() {
        **linear_velocity += **event * acceleration * delta_time;
    }
}

fn dampen_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut LinearVelocity, With<Player>>,
) {
    if keys.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyQ,
        KeyCode::KeyE,
    ]) {
        return;
    }

    let damping_factor = 0.9; // Adjust this value to control the damping speed

    let Ok(mut linear_velocity) = query.single_mut() else {
        return;
    };

    if **linear_velocity != Vec3::ZERO {
        **linear_velocity *= damping_factor;
    }
}

fn handle_mouse_look(
    mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mut orbit: ResMut<CameraOrbit>,
    settings: Res<CameraSettings>,
) {
    let delta = mouse_motion.delta;

    if delta.length_squared() < 0.0001 {
        return;
    }

    orbit.yaw -= delta.x * settings.mouse_sensitivity;
    orbit.pitch -= delta.y * settings.mouse_sensitivity;

    // Clamp pitch to avoid flipping upside down
    orbit.pitch = orbit.pitch.clamp(-1.5, 1.5);
}

/// Only when the player is pressing forward (W), orient the ship to face the current camera view direction.
/// Mouse look alone does **not** turn the ship. The ship only turns its facing when you thrust forward.
fn orient_player_to_camera_view(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, (With<MainCamera>, Without<Player>)>,
    mut player_query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
    settings: Res<CameraSettings>,
) {
    if !keys.pressed(KeyCode::KeyW) {
        return;
    }

    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };

    // Orient the ship to the exact direction the camera is currently pointed.
    // We copy the full rotation so the ship faces "forward" in the view.
    let target_rotation = camera_transform.rotation;

    player_transform.rotation = player_transform.rotation.slerp(
        target_rotation,
        settings.rotation_slerp_speed * time.delta_secs(),
    );
}

fn update_chase_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
    orbit: Res<CameraOrbit>,
    settings: Res<CameraSettings>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    // Snappy constant distance chase camera.
    // Mouse orbit (yaw/pitch) controls the camera's position around the player at fixed distance.
    // Direct set (no lerp) for snappy response.
    let yaw = orbit.yaw;
    let pitch = orbit.pitch;
    let orbit_rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

    // Base position is always exactly behind the *player's current facing* for "follow the player".
    // Mouse adds a small deviation to "move the camera" a bit for look around (keeps it roughly behind).
    // Direct set (no lerp) for snappy constant distance.
    let ship_rot = player_transform.rotation;
    let base = ship_rot * Vec3::new(0.0, settings.follow_height, -settings.follow_distance);
    let deviation = Vec3::new(orbit.yaw * 2.0, orbit.pitch * 1.5, 0.0); // small, tune the multipliers
    camera_transform.translation = player_transform.translation + base + deviation;

    // Always look at a point near the player. This guarantees the player ship is always visible
    // in the camera, no matter how you move the camera with the mouse.
    let lead = player_transform.rotation * Vec3::new(0.0, 0.0, 4.0);
    camera_transform.look_at(player_transform.translation + lead, Vec3::Y);
}
