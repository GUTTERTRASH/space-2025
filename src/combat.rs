use avian3d::prelude::{ExternalForce, RigidBody};
use bevy::ecs::component::ComponentId;
use bevy::prelude::*;
use bevy_observed_utility::prelude::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ObservedUtilityPlugins::RealTime)
            .init_resource::<ActionIds>()
            .init_resource::<Approaching>()
            .add_systems(FixedUpdate, (recalculate_distance, approach_target).chain())
            .add_observer(score_ancestor::<CombatState, Combaty>)
            .add_observer(on_action_initiated_insert_from_resource::<Approaching>)
            .add_observer(on_action_ended_remove::<Approaching>);
    }
}

#[derive(Component)]
pub struct CombatState {
    pub target: Entity,
    pub distance: f32,
}

impl From<&CombatState> for Score {
    fn from(combat_state: &CombatState) -> Self {
        let score = (combat_state.distance / 100.).clamp(0.0, 1.0);
        info!("Calculating score based on distance: {}", score);
        Score::new(score)
    }
}

#[derive(Component)]
pub struct Combaty;

#[derive(Component, Resource, Reflect, Clone, Copy, PartialEq, Debug)]
pub struct Approaching {
    pub minimum_distance: f32,
    pub speed: f32,
}

impl Default for Approaching {
    fn default() -> Self {
        Self {
            minimum_distance: 20.0,
            speed: 0.50,
        }
    }
}

#[derive(Component, Reflect, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Idle;

#[derive(Resource)]
pub struct ActionIds {
    pub idle: ComponentId,
    pub approach: ComponentId,
}

impl FromWorld for ActionIds {
    fn from_world(world: &mut World) -> Self {
        Self {
            idle: world.register_component::<Idle>(),
            approach: world.register_component::<Approaching>(),
        }
    }
}

pub fn recalculate_distance(
    mut query: Query<(&mut CombatState, &Transform)>,
    targets: Query<&Transform>,
) {
    for (mut combat_state, transform) in query.iter_mut() {
        if let Ok(target_transform) = targets.get(combat_state.target) {
            combat_state.distance = transform.translation.distance(target_transform.translation);
            // info!(
            //     "Recalculating distance to target: {:?}",
            //     combat_state.distance
            // );
        }
    }
}

pub fn approach_target(
    mut commands: Commands,
    actions: Res<ActionIds>,
    mut query: Query<(
        Entity,
        &mut CombatState,
        &mut ExternalForce,
        &mut Transform,
        &Approaching,
    )>,
    targets: Query<&Transform, Without<Approaching>>,
) {
    for (actor, mut combat_state, mut external_force, mut transform, approaching) in
        query.iter_mut()
    {
        if let Ok(target_transform) = targets.get(combat_state.target) {
            let distance = combat_state.distance;
            info!("Current distance to target: {}", distance);
            if distance > approaching.minimum_distance {
                let direction = (target_transform.translation - transform.translation).normalize();
                let movement = direction * approaching.speed;
                combat_state.distance -= movement.length();
                // external_force.apply_force(movement);
                transform.translation += movement;
                info!(
                    "Approaching target: {:?}, new distance: {}",
                    combat_state.target, combat_state.distance
                );
            } else {
                info!("Reached minimum distance to target: {}", distance);
                commands.trigger_targets(
                    OnActionEnded {
                        action: actions.approach,
                        reason: ActionEndReason::Completed,
                    },
                    TargetedAction(actor, actions.approach),
                );
            }
        }
    }
}
