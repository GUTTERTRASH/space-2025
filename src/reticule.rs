//! Aim reticule: projects the ship's true firing axis into screen space.
//!
//! The chase camera sits behind and above the ship (see `controller::update_chase_camera`),
//! so the camera's own line of sight is parallel to — but offset from — the ship's actual
//! forward axis that `weapons` fires projectiles along. A reticule pinned to the literal
//! screen center would therefore not track where shots actually go except by coincidence.
//! Instead we project a point out along the ship's forward axis into viewport space each
//! frame, so the reticule always sits over the ship's true boresight.

use bevy::prelude::*;

use crate::common::{MainCamera, Player};
use crate::controller::CameraUpdateSet;

pub struct ReticulePlugin;

impl Plugin for ReticulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_reticule)
            .add_systems(PostUpdate, update_reticule_position.after(CameraUpdateSet));
    }
}

const RETICULE_SIZE: f32 = 18.0;
/// Distance along the ship's forward axis used to project the aim point onto screen.
const AIM_DISTANCE: f32 = 500.0;

#[derive(Component)]
struct Reticule;

fn spawn_reticule(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        Name::new("Reticule"),
        Reticule,
        ImageNode {
            image: assets.load("images/reticule.png"),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(RETICULE_SIZE),
            height: Val::Px(RETICULE_SIZE),
            ..default()
        },
        Visibility::Hidden,
    ));
}

fn update_reticule_position(
    player: Query<&Transform, With<Player>>,
    camera: Query<(&Camera, &Transform), With<MainCamera>>,
    mut reticule: Query<(&mut Node, &mut Visibility), With<Reticule>>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok((mut node, mut visibility)) = reticule.single_mut() else {
        return;
    };

    let aim_point = player_transform.translation + player_transform.forward() * AIM_DISTANCE;
    let camera_global = GlobalTransform::from(*camera_transform);

    match camera.world_to_viewport(&camera_global, aim_point) {
        Ok(screen_pos) => {
            *visibility = Visibility::Visible;
            node.left = Val::Px(screen_pos.x - RETICULE_SIZE / 2.0);
            node.top = Val::Px(screen_pos.y - RETICULE_SIZE / 2.0);
        }
        Err(_) => {
            *visibility = Visibility::Hidden;
        }
    }
}
