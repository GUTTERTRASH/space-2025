use bevy::prelude::Component;

#[derive(Component)]
pub struct Player;

// TODO Move to combat.rs?
#[derive(Component)]
pub struct Enemy {
    pub health: f32,
}

impl Default for Enemy {
    fn default() -> Self {
        Self { health: 100.0 }
    }
}
