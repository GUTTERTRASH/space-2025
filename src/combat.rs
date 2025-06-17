use avian3d::prelude::*;
use bevy::prelude::*;
use big_brain::prelude::*;

use crate::projectile::{BULLET_SPEED, Projectile, ProjectileTimer};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BigBrainPlugin::new(PreUpdate))
            .register_type::<Approaching>()
            .register_type::<Attacking>()
            .register_type::<Approachy>()
            .register_type::<Attacky>()
            .register_type::<Score>()
            .add_systems(Update, (approaching_system, attacking_system))
            .add_systems(
                PreUpdate,
                (
                    approach_action_system.in_set(BigBrainSet::Actions),
                    approachy_scorer_system.in_set(BigBrainSet::Scorers),
                    attack_action_system.in_set(BigBrainSet::Actions),
                    attacky_scorer_system.in_set(BigBrainSet::Scorers),
                ),
            );
    }
}

#[derive(Component, Reflect)]
pub struct Approaching {
    pub distance: f32,
    pub target: Entity,
    pub speed: f32,
}

pub fn approaching_system(
    mut query: Query<(&Transform, &mut Approaching)>,
    targets: Query<&Transform>,
) {
    for (transform, mut approaching) in query.iter_mut() {
        if let Ok(target_transform) = targets.get(approaching.target) {
            approaching.distance = transform.translation.distance(target_transform.translation);
            // debug!("Current distance to target: {}", approaching.distance);
        }
    }
}

#[derive(Clone, Component, Reflect, Debug, ActionBuilder)]
pub struct Approach {
    pub until_distance: f32,
}

fn approach_action_system(
    mut approachings: Query<(&mut Approaching, &mut Transform)>,
    mut query: Query<(&Actor, &mut ActionState, &Approach, &ActionSpan)>,
    targets: Query<&Transform, Without<Approaching>>,
) {
    for (Actor(actor), mut state, approach, span) in query.iter_mut() {
        let _guard = span.span().enter();

        if let Ok((mut approaching, mut transform)) = approachings.get_mut(*actor) {
            let target = targets.get(approaching.target).unwrap();

            match *state {
                ActionState::Requested => {
                    info!("Begining approach to target...");
                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if approaching.distance > approach.until_distance {
                        // info!("Approaching target...");
                        let direction = (target.translation - transform.translation).normalize();
                        let movement = direction * approaching.speed;
                        approaching.distance -= movement.length();
                        transform.translation += movement;
                    } else {
                        info!("Reached target distance, ending approach...");
                        *state = ActionState::Success;
                    }
                }
                ActionState::Cancelled => {
                    info!("Cancelling approach to target...");
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Reflect, Component, Debug, ScorerBuilder)]
pub struct Approachy;

pub fn approachy_scorer_system(
    approachings: Query<&Approaching>,
    // Same dance with the Actor here, but now we use look up Score instead of ActionState.
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<Approachy>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(approaching) = approachings.get(*actor) {
            let score_value = (approaching.distance / 100.0).clamp(0.0, 1.0);
            score.set(score_value);
            // span.span().in_scope(|| {
            //     info!("Aproach score is Score: {}", score_value);
            // });
        }
    }
}

#[derive(Component, Reflect)]
pub struct Attacking {
    pub distance: f32,
    pub target: Entity,
}

pub fn attacking_system(
    mut query: Query<(&Transform, &mut Attacking)>,
    targets: Query<&Transform>,
) {
    for (transform, mut attacking) in query.iter_mut() {
        if let Ok(target_transform) = targets.get(attacking.target) {
            attacking.distance = transform.translation.distance(target_transform.translation);
            // debug!("Current distance to target: {}", attacking.distance);
        }
    }
}

#[derive(Clone, Reflect, Component, Debug, ScorerBuilder)]
pub struct Attacky;

pub fn attacky_scorer_system(
    attackings: Query<&Attacking>,
    // Same dance with the Actor here, but now we use look up Score instead of ActionState.
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<Attacky>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(attacking) = attackings.get(*actor) {
            let score_value = (10.0 / attacking.distance).clamp(0.0, 1.0);
            score.set(score_value);
            // span.span().in_scope(|| {
            //     info!("Attack score is Score: {}", score_value);
            // });
        }
    }
}

#[derive(Clone, Component, Reflect, Debug, ActionBuilder)]
pub struct Attack {
    pub min_distance: f32,
}

fn attack_action_system(
    mut attackings: Query<(&mut Attacking, &Transform)>,
    mut query: Query<(&Actor, &mut ActionState, &Attack, &ActionSpan)>,
    targets: Query<(&Name, &Transform), Without<Attacking>>,

    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ProjectileTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (Actor(actor), mut state, attack, span) in query.iter_mut() {
        let _guard = span.span().enter();

        if let Ok((mut attacking, transform)) = attackings.get_mut(*actor) {
            let (target_name, target_transform) = targets.get(attacking.target).unwrap();

            match *state {
                ActionState::Requested => {
                    info!("Begining attack to {:?}...", target_name);
                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if attacking.distance > attack.min_distance {
                        info!("Too far! Stopping attack");
                        *state = ActionState::Success;
                    } else {
                        if timer.0.tick(time.delta()).just_finished() {
                            let bullet_material = materials.add(StandardMaterial {
                                base_color: Color::WHITE,
                                unlit: true,
                                ..Default::default()
                            });

                            let direction =
                                (target_transform.translation - transform.translation).normalize();
                            let rotation = Quat::from_rotation_arc(Vec3::Z, direction);

                            commands.spawn((
                                Mesh3d(meshes.add(Cuboid::default())),
                                MeshMaterial3d(bullet_material),
                                Transform::from_scale(Vec3::new(0.1, 0.1, 1.0))
                                    .with_translation(transform.translation + direction * 1.5)
                                    .with_rotation(rotation),
                                Name::new("Bullet"),
                                Projectile {
                                    velocity: direction * BULLET_SPEED,
                                    power: 1.0,
                                },
                                PickingBehavior::IGNORE,
                                // RigidBody::Dynamic,
                                // ColliderConstructor::TrimeshFromMesh,
                            ));
                        }

                        // info!("Attacking {:?}!", target);
                    }
                }
                ActionState::Cancelled => {
                    info!("Cancelling approach to target...");
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}
