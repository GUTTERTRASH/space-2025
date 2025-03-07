use bevy::prelude::Component;

#[derive(Component)]
pub struct Player;

// TODO Move to combat.rs?
#[derive(Component)]
pub struct Enemy {
    pub health: f32,
}
