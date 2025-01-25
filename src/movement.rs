use std::ops::Deref;
use bevy::prelude::*;
use bevy_third_person_camera::{ThirdPersonCamera, ThirdPersonCameraTarget};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<TranslationEvent>()
            .add_event::<RotationEvent>()
            .add_systems(PreUpdate, move_player)
            .add_systems(Update, translate_player)
            .add_systems(Update, rotate_player);
    }
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<
        (&Transform),
        (With<ThirdPersonCamera>, Without<ThirdPersonCameraTarget>),
    >,
    mut translations: EventWriter<TranslationEvent>,
    mut rotations: EventWriter<RotationEvent>,
) {

    if keys.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyQ,
        KeyCode::KeyE,
    ]) {

        let Ok(camera_transform) = camera_query.get_single_mut() else { return };

        let xz = Vec3::new(1.0, 0.0, 1.0);
        let (forward, right, up) = (
            (*camera_transform.forward() * xz).normalize(),
            (*camera_transform.right() * xz).normalize(),
            Vec3::Y,
        );

        let mut desired_velocity = Vec3::ZERO;
        let mut clamp_direction = false;

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
        }
        if keys.pressed(KeyCode::KeyA) {
            desired_velocity -= right;
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
            0.1
        };

        desired_velocity *= speed;

        if clamp_direction {
            let rotation = Transform::default().looking_at(forward, up).rotation;
            rotations.send(RotationEvent::new(&rotation));
        }

        translations.send(TranslationEvent::new(&desired_velocity));

    }

}

#[derive(Event, Debug, Default)]
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
    mut events: EventReader<TranslationEvent>,
    mut query: Query<&mut Transform, With<ThirdPersonCameraTarget>>,
) {
    for event in events.read() {
        for mut transform in query.iter_mut() {
            transform.translation += **event;
        }
    }
}

#[derive(Event, Debug, Default)]
pub struct RotationEvent {
    value: Quat,
}

impl RotationEvent {
    pub fn new(value: &Quat) -> Self {
        Self { value: *value }
    }
}

impl Deref for RotationEvent {
    type Target = Quat;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}


fn rotate_player(
    time: Res<Time>,
    mut events: EventReader<RotationEvent>,
    mut player_query: Query<&mut Transform, With<ThirdPersonCameraTarget>>,
) {
    let Ok(mut player_transform) = player_query.get_single_mut() else { return };
    for event in events.read() {
        player_transform.rotation = player_transform
            .rotation
            .slerp(**event, 10.0 * time.delta_secs());
    }
}