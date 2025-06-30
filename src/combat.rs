use std::f32::MIN;

use bevy::{color::palettes::css::RED, prelude::*};
use big_brain::prelude::*;
use rand::Rng;

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
            .register_type::<Missily>()
            .insert_resource(MissileTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
            .add_systems(
                Update,
                (
                    approaching_system,
                    // orbit_target,
                    fire_missile,
                    missile_approach,
                ),
            )
            .add_systems(
                PreUpdate,
                (
                    approach_action_system.in_set(BigBrainSet::Actions),
                    approachy_scorer_system.in_set(BigBrainSet::Scorers),
                    attack_action_system.in_set(BigBrainSet::Actions),
                    attacky_scorer_system.in_set(BigBrainSet::Scorers),
                    missile_action_system.in_set(BigBrainSet::Actions),
                    missile_scorer_system.in_set(BigBrainSet::Scorers),
                ),
            );
    }
}

pub const MIN_DISTANCE: f32 = 30.0;

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
                        transform.look_at(target.translation, Vec3::Y);
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

            if approaching.distance > MIN_DISTANCE {
                score.set(0.5);
            } else {
                score.set(0.0);
            }

            // let score_value = (approaching.distance / MIN_DISTANCE).clamp(0.0, 1.0);
            // score.set(score_value);
            // span.span().in_scope(|| {
            //     info!("Approach score is Score: {}", score_value);
            // });
        }
    }
}

#[derive(Component, Reflect)]
pub struct Attacking(pub Entity);

#[derive(Clone, Reflect, Component, Debug, ScorerBuilder)]
pub struct Attacky;

pub fn attacky_scorer_system(
    attackings: Query<&Approaching>,
    // Same dance with the Actor here, but now we use look up Score instead of ActionState.
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<Attacky>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok(attacking) = attackings.get(*actor) {

            if attacking.distance <= MIN_DISTANCE {
                score.set(0.6);
            } else {
                score.set(0.0);
            }

            // let score_value = (MIN_DISTANCE / attacking.distance).clamp(0.0, 1.0);
            // score.set(score_value);
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
    attackings: Query<(&Attacking, &Approaching, &Transform)>,
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

        if let Ok((Attacking(target), approaching, transform)) = attackings.get(*actor) {
            let (target_name, target_transform) = targets.get(*target).unwrap();

            match *state {
                ActionState::Requested => {
                    info!("Begining attack to {:?}...", target_name);
                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if approaching.distance > attack.min_distance {
                        info!("Too far! Stopping attack");
                        // *state = ActionState::Success;
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

#[derive(Clone, Reflect, Component, Debug, ScorerBuilder)]
pub struct Missily;

#[derive(Clone, Reflect, Component, Debug)]
pub struct MissileLoadout {
    pub ammo: i32,
}

pub fn missile_scorer_system(
    attackings: Query<(&Approaching, &MissileLoadout)>,
    mut query: Query<(&Actor, &mut Score, &ScorerSpan), With<Missily>>,
) {
    for (Actor(actor), mut score, span) in &mut query {
        if let Ok((
            Approaching {
                distance,
                target: _,
                speed: _,
            },
            MissileLoadout { ammo },
        )) = attackings.get(*actor)
        {
            if *ammo <= 0 {
                score.set(0.0);
            } else {
                let score_value = (2.0 * MIN_DISTANCE / distance).clamp(0.0, 1.0);
                score.set(score_value);
                // span.span().in_scope(|| {
                //     info!("Missile Attack score: {}", score_value);
                // });
            }
        }
    }
}

#[derive(Clone, Component, Reflect, Debug, ActionBuilder)]
pub struct MissileAttack {
    pub min_distance: f32,
}

#[derive(Resource)]
pub struct MissileTimer(pub Timer);

fn missile_action_system(
    mut actors: Query<(&Actor, &mut ActionState, &MissileAttack, &ActionSpan)>,
    attackers: Query<(&Attacking, &Approaching, &MissileLoadout)>,
    targets: Query<(&Name, &Transform), Without<Attacking>>,
    // mut commands: Commands,
    // time: Res<Time>,
    // mut timer: ResMut<ProjectileTimer>,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (Actor(actor), mut state, action, _) in actors.iter_mut() {
        let (Attacking(target), approaching, MissileLoadout { ammo }) =
            attackers.get(*actor).unwrap();

        let (target_name, _) = targets.get(*target).unwrap();

        match *state {
            ActionState::Requested => {
                info!("Beginning missile attack on {target_name}...");
                *state = ActionState::Executing;
            }
            ActionState::Executing => {
                if approaching.distance > action.min_distance {
                    info!("Too far! Stopping attack");
                    *state = ActionState::Success;
                }
                if *ammo <= 0 {
                    info!("Out of ammo!");
                    *state = ActionState::Success;
                }
            }
            ActionState::Cancelled => {
                info!("Cancelling missile attack on {target_name}...");
                *state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Clone, Component, Reflect, Debug)]
pub struct Missile {
    pub target: Entity,
}

fn fire_missile(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    mut timer: ResMut<MissileTimer>,
    actors: Query<&Actor, With<MissileAttack>>,
    mut attackers: Query<(&Name, &Attacking, &mut MissileLoadout, &Transform)>,
) {
    for Actor(actor) in actors.iter() {
        if timer.0.tick(time.delta()).just_finished() {
            let (name, Attacking(target), mut missile_loadout, attacker_transform) =
                attackers.get_mut(*actor).unwrap();

            let missile_material = materials.add(StandardMaterial {
                base_color: Color::from(RED),
                unlit: true,
                ..Default::default()
            });

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::default())),
                MeshMaterial3d(missile_material),
                Transform::from_scale(Vec3::new(0.2, 0.2, 1.0))
                    .with_translation(attacker_transform.translation),
                Name::new("Missile"),
                Missile { target: *target },
                PickingBehavior::IGNORE,
            ));

            missile_loadout.ammo -= 1;
            info!("{name} has {:?} missiles left", missile_loadout.ammo);
        }
    }
}

fn missile_approach(
    mut commands: Commands,
    mut missiles: Query<(&Missile, Entity, &mut Transform)>,
    targets: Query<(&Name, &Transform), Without<Missile>>,
) {
    for (Missile { target }, entity, mut transform) in missiles.iter_mut() {
        let (target_name, target_transform) = targets.get(*target).unwrap();

        let distance = transform.translation.distance(target_transform.translation);

        if distance < 1.0 {
            info!("Missile hit {target_name}!!");
            commands.entity(entity).despawn();
            continue;
        }

        let direction = (target_transform.translation - transform.translation).normalize();
        transform.translation += direction;
        transform.look_at(target_transform.translation, Vec3::Y);

        // // Calculate a quadratic Bezier curve: P0 (start), P1 (control), P2 (end)
        // let start = transform.translation;
        // let end = target_transform.translation;

        // // Choose a control point offset from the straight line for a curve
        // let mid = (start + end) * 0.5;
        // let up = Vec3::Y * 1.0; // You can randomize or adjust this for different curves
        // let control = mid + up;

        // // Progress along the curve based on distance (or you can use a timer for smoother motion)
        // let total_dist = start.distance(end);
        // let t = (1.0 - (distance / total_dist)).clamp(0.0, 1.0);

        // // Quadratic Bezier interpolation
        // let bezier = |t: f32, p0: Vec3, p1: Vec3, p2: Vec3| -> Vec3 {
        //     (1.0 - t).powi(2) * p0 + 2.0 * (1.0 - t) * t * p1 + t.powi(2) * p2
        // };

        // let next_pos = bezier((t + 0.03).clamp(0.0, 1.0), start, control, end); // 0.03 is the step size

        // transform.translation = next_pos;
        // //
    }
}

#[derive(Component)]
pub struct OrbitMotion {
    pub radius: f32,
    pub speed: f32,
    pub rotation_axis: Vec3,
    pub initial_angle: f32,
}

fn orbit_target(
    mut commands: Commands,
    mut attackings: Query<(&Attacking, &mut Transform, Option<&OrbitMotion>)>,
    query: Query<&Actor, With<Attack>>,
    targets: Query<&Transform, Without<Attacking>>,
    time: Res<Time>,
) {
    for Actor(entity) in query.iter() {
        let (Attacking(target), mut transform, orbit_motion) = attackings.get_mut(*entity).unwrap();
        let target_transform = targets.get(*target).unwrap();

        // If no OrbitMotion component exists, create one with random axis
        if orbit_motion.is_none() {
            let initial_radius = transform.translation.distance(target_transform.translation);

            let mut rng = rand::thread_rng();
            let random_axis = Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            )
            .normalize();

            let direction = (transform.translation - target_transform.translation).normalize();
            let initial_angle = Vec3::X.angle_between(direction);

            commands.entity(*entity).insert(OrbitMotion {
                radius: initial_radius,
                speed: 0.5,
                rotation_axis: random_axis,
                initial_angle,
            });

            continue;
        }

        let orbit = orbit_motion.unwrap();
        let angle = time.elapsed_secs() * orbit.speed + orbit.initial_angle;

        // Create rotation quaternion around the random axis
        let rotation = Quat::from_axis_angle(orbit.rotation_axis, angle);

        let base_offset = Vec3::X * orbit.radius; // Start with offset along X axis
        let offset = rotation * base_offset; // Rotate it

        transform.translation = target_transform.translation + offset;
        transform.look_at(target_transform.translation, Vec3::Y);
    }
}
