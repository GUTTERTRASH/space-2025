use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use std::ops::Deref;

use crate::common::{MainCamera, Player};

/// Consistent forward axis for the spaceship.
/// We standardize on Bevy's convention: an entity's "forward" is its local -Z axis.
/// (This matches Transform::forward(), default cameras, and most glTF authoring expectations in Bevy.)
/// All ship orientation, camera chase "behind", and LookDirection must use this.
const PLAYER_FORWARD_LOCAL: Vec3 = Vec3::NEG_Z;
const PLAYER_RIGHT_LOCAL: Vec3 = Vec3::X;
const PLAYER_UP_LOCAL: Vec3 = Vec3::Y;

pub struct ControllerPlugin;

const PITCH_BOUND: f32 = std::f32::consts::FRAC_PI_2 - 1E-3;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LockOnState>()
            .init_resource::<MouseSettings>()
            .init_resource::<CameraSettings>()
            .init_resource::<ShipAlignTarget>()
            .add_message::<RotationEvent>()
            .add_message::<ControllerRotationEvent>()
            .add_message::<TranslationEvent>()
            .add_systems(Update, update_camera)
            .add_systems(
                PreUpdate,
                (handle_mouse_input, handle_keyboard_input)
                    .chain()
                    .in_set(InputSystemSet::HandleInput),
            )
            .add_systems(Update, handle_translation_events)
            .add_systems(Update, handle_rotation_events)
            .add_systems(Update, update_look_direction)
            ;
            // handle_camera_rotation_events removed: camera rotation is now derived via look_at in update_camera
            // (tied to ship's orientation). Mouse only drives orbit for camera position around the ship.
            // handle_controller_rotation_events removed (was for lock-on position, conflicts with chase).
            // .add_systems(Update, handle_lock_on_events);
    }
}

fn handle_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, (With<MainCamera>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut mouse: ResMut<MouseSettings>,
    mut align_target: ResMut<ShipAlignTarget>,
    mut translation_events: MessageWriter<TranslationEvent>,
    mut rotation_events: MessageWriter<ControllerRotationEvent>,
) {
    // Only keep feeding a turn target (and thus continuing the slerp) while the player is
    // actively using forward/back thrust. Clear on release so old commits don't stick forever.
    if !keys.pressed(KeyCode::KeyW) && !keys.pressed(KeyCode::KeyS) {
        align_target.0 = None;
    }

    if keys.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyQ,
        KeyCode::KeyE,
    ]) {

        let Ok(cam) = camera_query.single() else {
            return;
        };

        let xz = Vec3::new(1.0, 0.0, 1.0);

        // Camera facing methods return Dir3 in current Bevy; convert to Vec3 for the xz mask + normalize.
        let cam_forward = cam.forward().as_vec3();
        let cam_right = cam.right().as_vec3();

        let (forward, right, up) = (
            (cam_forward * xz).normalize(),
            (cam_right * xz).normalize(),
            Vec3::Y,
        );

        let mut clamp_direction = false;

        let mut desired_velocity = Vec3::ZERO;

        if keys.pressed(KeyCode::KeyW) {
            desired_velocity += forward;
            clamp_direction = true;
        }
        if keys.pressed(KeyCode::KeyS) {
            desired_velocity -= forward;
            clamp_direction = true;
        }
        if keys.pressed(KeyCode::KeyD) {
            desired_velocity += right;
            // clamp_direction = true;
        }
        if keys.pressed(KeyCode::KeyA) {
            desired_velocity -= right;
            // clamp_direction = true;
        }
        if keys.pressed(KeyCode::KeyQ) {
            desired_velocity += up;
        }
        if keys.pressed(KeyCode::KeyE) {
            desired_velocity -= up;
        }

        let speed = if keys.pressed(KeyCode::ShiftLeft) {
            2.0
        } else {
            0.5
        };

        desired_velocity *= speed;

        // On the frame the player presses W or S (after having moused the camera to a new view),
        // capture a *fixed* target orientation: current ship rotation composed with the current
        // mouse orbit deviation. This is "turn the ship to face the direction the camera view
        // was pointing".
        // We immediately zero the mouse accum so that update_camera will compute the chase
        // position as "straight behind" (in ship local) + any new mouse. As the ship slerps
        // toward the captured target, the camera will swing from its previous offset position
        // to being directly behind the new heading. This breaks the previous circular feedback.
        if clamp_direction && (keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::KeyS)) {
            if let Ok(player_transform) = player_query.single() {
                let ship_rot = player_transform.rotation;
                let yaw = mouse.yaw_pitch_roll.x;
                let pitch = mouse.yaw_pitch_roll.y;
                let orbit_rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
                let target_rot = ship_rot * orbit_rot;
                align_target.0 = Some(target_rot);
                mouse.yaw_pitch_roll = Vec3::ZERO;
            }
        }

        // While W or S is held and we have a captured align target, keep emitting the *same*
        // fixed target every frame. This lets handle_rotation_events continue slerping the
        // ship toward it over multiple frames until it arrives (instead of a one-frame nudge).
        if clamp_direction && (keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::KeyS)) {
            if let Some(target) = align_target.0 {
                rotation_events.write(ControllerRotationEvent::new(&target));
            }
        }

        translation_events.write(TranslationEvent::new(&desired_velocity));

    }

}


fn handle_translation_events(
    mut messages: MessageReader<TranslationEvent>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut lock_on_state: ResMut<LockOnState>,
) {
    for message in messages.read().by_ref() {
        for mut player_transform in player_query.iter_mut() {
            player_transform.translation += **message;
            lock_on_state.player_transform = *player_transform;
        }
    }
}

fn handle_rotation_events(
    time: Res<Time>,
    mut messages: MessageReader<ControllerRotationEvent>,
    mut query: Query<&mut Transform, With<Player>>,
    mut lock_on_state: ResMut<LockOnState>,
) {
    for event in messages.read().by_ref() {
        for mut transform in query.iter_mut() {
            transform.rotation = transform
                .rotation
                .slerp(**event, 10.0 * time.delta_secs());
            lock_on_state.player_transform = *transform;
        }
    }
}

// Removed to prevent position conflicts with update_camera (which drives the chase position).
// If you implement lock-on, re-add a version that only affects rotation or coordinates with the chase.
 /*
fn handle_controller_rotation_events(
    lock_on_state: Res<LockOnState>,
    mut events: MessageReader<ControllerRotationEvent>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    if let Some(_) = lock_on_state.target {
        if let Some(event) = events.read().next() {
            for mut transform in query.iter_mut() {
                transform.rotation = **event;
                let rotation_matrix = Mat3::from_quat(transform.rotation);
                transform.translation = rotation_matrix.mul_vec3(Vec3::new(0.0, 2.25, 15.0));
            }
        }
    }
}
*/

fn handle_mouse_input(
    lock_on_state: Res<LockOnState>,
    mut settings: ResMut<MouseSettings>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mut rotation_events: MessageWriter<RotationEvent>,
) {
    if let None = lock_on_state.target {
        let mut delta = Vec2::ZERO;
        for motion in mouse_motion_events.read() {
            delta -= motion.delta;
        }

        if delta.length_squared() > 1E-6 {
            delta *= settings.sensitivity;
            settings.yaw_pitch_roll += delta.extend(0.0);
            if settings.yaw_pitch_roll.y > PITCH_BOUND {
                settings.yaw_pitch_roll.y = PITCH_BOUND;
            }
            if settings.yaw_pitch_roll.y < -PITCH_BOUND {
                settings.yaw_pitch_roll.y = -PITCH_BOUND;
            }
            rotation_events.write(RotationEvent::new(Vec2::new(
                settings.yaw_pitch_roll.x,
                settings.yaw_pitch_roll.y,
            )));
        }
    }
}

fn update_look_direction(
    mut events: MessageReader<RotationEvent>,
    mut query: Query<&mut LookDirection>,
) {
    if let Some(event) = events.read().next() {
        for mut look in query.iter_mut() {
            let rot = **event;
            // Rotate the canonical local axes so the ship's -Z (forward) points in the desired world direction.
            look.forward = rot * PLAYER_FORWARD_LOCAL;
            look.right   = rot * PLAYER_RIGHT_LOCAL;
            look.up      = rot * PLAYER_UP_LOCAL;
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum InputSystemSet {
    HandleInput,
}

#[derive(Clone, Copy, Component)]
pub struct LookDirection {
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

impl Default for LookDirection {
    fn default() -> Self {
        Self {
            forward: PLAYER_FORWARD_LOCAL,
            right: PLAYER_RIGHT_LOCAL,
            up: PLAYER_UP_LOCAL,
        }
    }
}

#[derive(Debug, Component)]
pub struct LookEntity(pub Entity);

#[derive(Resource, Default)]
pub struct LockOnState {
    pub target: Option<Entity>,
    pub player_transform: Transform,
}

/// Captured target orientation for the ship when the player presses W/S after mousing the camera.
/// We store a fixed target (instead of re-deriving every frame from live camera/ship) so the slerp
/// can complete a full turn toward the view direction that was active at commit time, without the
/// feedback loop that caused the ship+camera to circle each other.
#[derive(Resource, Default)]
pub struct ShipAlignTarget(pub Option<Quat>);

#[derive(Debug, Event, Message)]
pub struct TranslationEvent {
    value: Vec3,
}

impl TranslationEvent {
    pub fn new(value: &Vec3) -> Self {
        Self { value: *value }
    }
}

impl Deref for TranslationEvent {
    type Target = Vec3;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug, Event, Message)]
pub struct RotationEvent {
    value: Quat,
}

impl RotationEvent {
    pub fn new(v: Vec2) -> Self {
        // This quat, when applied to SHIP_FORWARD_LOCAL (-Z), produces the desired world forward.
        // It matches how we build LookDirection and how Bevy's looking_at / forward convention works.
        Self {
            value: Quat::from_rotation_y(v.x) * Quat::from_rotation_x(v.y),
        }
    }
}

impl From<Quat> for RotationEvent {
    fn from(value: Quat) -> Self {
        Self { value }
    }
}

impl Deref for RotationEvent {
    type Target = Quat;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug, Event, Message)]
pub struct ControllerRotationEvent {
    value: Quat,
}

impl ControllerRotationEvent {
    pub fn new(value: &Quat) -> Self {
        Self { value: *value }
    }
}

impl Deref for ControllerRotationEvent {
    type Target = Quat;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Resource)]
pub struct MouseSettings {
    pub sensitivity: f32,
    pub yaw_pitch_roll: Vec3,
}

impl Default for MouseSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.002,
            yaw_pitch_roll: Vec3::ZERO,
        }
    }
}

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

fn update_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
    settings: Res<CameraSettings>,
    mouse: Res<MouseSettings>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    // Snappy chase at constant predefined distance from the ship.
    // The mouse orbit (yaw/pitch) is used to rotate the local back offset, then the result is
    // transformed by the ship's rotation. This makes the camera *position* orbit around the
    // *ship's position* (the ship itself is the center/pivot of the orbit), with the orbit
    // angles relative to the ship's local axes ("from the ship's perspective").
    // Camera rotation is derived with look_at using the ship's up, tying it to the ship.
    let ship_rot = player_transform.rotation;

    let yaw = mouse.yaw_pitch_roll.x;
    let pitch = mouse.yaw_pitch_roll.y;

    let orbit_rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

    // local base "back" in ship local ( +Z back since forward local = -Z )
    let local_base = Vec3::new(0.0, settings.follow_height, settings.follow_distance);
    let local_offset = orbit_rot * local_base;
    let offset = ship_rot * local_offset;

    camera_transform.translation = player_transform.translation + offset;

    // Derive camera rotation tied to the ship by looking at the player with ship's up.
    camera_transform.look_at(player_transform.translation, ship_rot * Vec3::Y);
}
