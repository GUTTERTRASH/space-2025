use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct ReticulePlugin;

impl Plugin for ReticulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, draw_reticule);
    }
}

const VERTICAL_OFFSET: f32 = 20.0;
const RETICULE_SIZE: f32 = 9.0;
const RETICULE_HALF_SIZE: f32 = RETICULE_SIZE / 2.0;

fn draw_reticule(
    mut commands: Commands,
    assets: Res<AssetServer>,
    primary_window_query: Single<&Window, With<PrimaryWindow>>,
) {
    let primary_window = *primary_window_query;

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::FlexStart,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                ImageNode {
                    image: assets.load("images/reticule.png"),
                    ..default()
                },
                Node {
                    width: Val::Px(RETICULE_SIZE),
                    height: Val::Px(RETICULE_SIZE),
                    top: Val::Px(
                        primary_window.height() / 2.0 + RETICULE_HALF_SIZE - VERTICAL_OFFSET,
                    ),
                    right: Val::Px(primary_window.width() / 2.0 + RETICULE_HALF_SIZE),
                    bottom: Val::Px(primary_window.height() / 2.0 - RETICULE_HALF_SIZE),
                    left: Val::Px(primary_window.width() / 2.0 - RETICULE_HALF_SIZE),
                    ..default()
                },
            ));
        });
}

// fn draw_reticule(
//     mut gizmos: Gizmos,
//     camera_query: Single<
//         (&Camera, &Transform, &GlobalTransform),
//         (With<ThirdPersonCamera>, Without<ThirdPersonCameraTarget>),
//     >,
// ) {
//
//     let (camera, camera_transform, camera_global_transform) = *camera_query;
//
//     let xz = Vec3::new(1.0, 0.0, 1.0);
//     let (forward, right, up) = (
//         (*camera_transform.forward() * xz).normalize(),
//         (*camera_transform.right() * xz).normalize(),
//         Vec3::Y,
//     );
//
//     let starting_point = Vec2::new(forward.x, forward.y);
//
//     let Ok(point) = camera.viewport_to_world_2d(camera_global_transform, starting_point) else { return };
//
//     gizmos.circle_2d(point, 10., Color::WHITE);
//
// }
