use bevy::prelude::*;

pub struct ReticulePlugin;

impl Plugin for ReticulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, draw_reticule);
    }
}

const RETICULE_SIZE: f32 = 18.0; // Slightly larger for better visibility in space games

fn draw_reticule(mut commands: Commands, assets: Res<AssetServer>) {
    // Best practice: Use a full-screen centered container.
    // This automatically handles window resizes, different aspect ratios,
    // and is robust across Bevy UI changes. No manual pixel math or startup window query needed.
    commands
        .spawn((
            Name::new("Reticule Container"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                // Ensure it doesn't interfere with picking / other UI
                // PickingBehavior::IGNORE,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Reticule"),
                ImageNode {
                    image: assets.load("images/reticule.png"),
                    // You can add color tint or alpha here if desired:
                    // color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                    ..default()
                },
                Node {
                    width: Val::Px(RETICULE_SIZE),
                    height: Val::Px(RETICULE_SIZE),
                    // Optional: small negative margin if you want it perfectly pixel-centered
                    // margin: UiRect::all(Val::Px(-1.0)),
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
