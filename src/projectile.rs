use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fire_bullet);
    }
}

fn fire_bullet(commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.pressed(KeyCode::ControlLeft) {
        info!("Firing projectile");
    }
}
