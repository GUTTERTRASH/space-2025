use avian3d::math::*;
use avian3d::prelude::*;
use bevy::prelude::*;
// use bevy_third_person_camera::{ThirdPersonCamera, ThirdPersonCameraTarget};
use std::ops::Deref;

use crate::common::Player;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TranslationEvent>()
            // .add_event::<RotationEvent>()
            .add_systems(Update, handle_keyboard_input)
            .add_systems(FixedUpdate, (translate_player, dampen_movement).chain());
    }
}

fn handle_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, Without<Player>>,
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

// #[derive(Event, Debug, Default)]
// pub struct RotationEvent {
//     value: Quat,
// }

// impl RotationEvent {
//     pub fn new(value: &Quat) -> Self {
//         Self { value: *value }
//     }
// }

// impl Deref for RotationEvent {
//     type Target = Quat;

//     fn deref(&self) -> &Self::Target {
//         &self.value
//     }
// }

// // fn rotate_player(
// //     time: Res<Time>,
// //     mut events: EventReader<RotationEvent>,
// //     mut player_query: Query<&mut Transform, With<ThirdPersonCameraTarget>>,
// // ) {
// //     let Ok(mut player_transform) = player_query.get_single_mut() else {
// //         return;
// //     };
// //     for event in events.read() {
// //         player_transform.rotation = player_transform
// //             .rotation
// //             .slerp(**event, 10.0 * time.delta_secs());
// //     }
// // }
