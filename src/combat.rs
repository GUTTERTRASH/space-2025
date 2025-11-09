use std::f32::MIN;
use std::collections::HashMap;

use bevy::{color::palettes::css::RED, prelude::*};
use big_brain::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

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
            .insert_resource(SeparationGrid::new(GRID_CELL_SIZE))
            .add_systems(
                Update,
                (
                    approaching_system,
                    // orbit_target,
                    fire_missile,
                    missile_approach,
                    // build the spatial grid for separation neighbor queries
                    build_grid_system,
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

pub const MIN_DISTANCE: f32 = 10.0;

// Separation tuning
// Separation tuning
// Increased separation and strength for clearer spacing with many agents
pub const SEPARATION_DISTANCE: f32 = 4.0;
pub const SEPARATION_STRENGTH: f32 = 3.0;

// Steering smoothing (lerp factor for velocity blending; lower -> smoother)
pub const SEPARATION_SMOOTHING: f32 = 0.18;

// Grid cell size used for spatial partitioning. Slightly larger than separation distance
pub const GRID_CELL_SIZE: f32 = SEPARATION_DISTANCE * 1.5;
// Movement scale to convert previous per-frame speeds to per-second velocities.
// The original code applied `transform.translation += movement` (per-frame). After
// switching to velocity * delta_seconds, we scale by ~60 to maintain similar feel.
pub const MOVEMENT_SPEED_SCALE: f32 = 60.0;

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
    mut commands: Commands,
    mut approachings: Query<(
        Entity,
        &mut Approaching,
        &mut Transform,
        Option<&ApproachOffset>,
        Option<&mut Velocity>,
    )>,
    mut query: Query<(&Actor, &mut ActionState, &Approach, &ActionSpan)>,
    targets: Query<&Transform, Without<Approaching>>,
    time: Res<Time>,
    grid: Res<SeparationGrid>,
) {
    for (Actor(actor), mut state, approach, span) in query.iter_mut() {
        let _guard = span.span().enter();

        if let Ok((entity, mut approaching, mut transform, maybe_offset, maybe_velocity)) =
            approachings.get_mut(*actor)
        {
            let target = targets.get(approaching.target).unwrap();

            // Ensure each approaching enemy has a small random offset from the target so
            // multiple enemies don't all converge to the exact same position.
            // We lazily insert `ApproachOffset` the first time we need it to avoid changing spawn sites.
            let mut offset_vec = if let Some(offset) = maybe_offset {
                offset.0
            } else {
                // Generate a random position on the XZ plane around the target
                let mut rng = rand::thread_rng();
                let angle = rng.gen_range(0.0..TAU);
                let radius = rng.gen_range(1.5..4.0);
                let off = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
                // Insert the offset component so future frames reuse it
                commands.entity(entity).insert(ApproachOffset(off));
                off
            };

            let aim = target.translation + offset_vec;

            match *state {
                ActionState::Requested => {
                    info!("Begining approach to target...");
                    *state = ActionState::Executing;
                }
                ActionState::Executing => {
                    if approaching.distance > approach.until_distance {
                        // Base approach velocity toward the aim point
                        let my_pos = transform.translation;
                        let direction = (aim - my_pos).normalize();
                        let base_vel = direction * approaching.speed;

                        // Gather nearby neighbors using the spatial grid and compute separation
                        let mut neighbors: Vec<&(Entity, Vec3)> = Vec::with_capacity(16);
                        grid.neighbors(&my_pos, &mut neighbors);

                        let mut sep = Vec3::ZERO;
                        for item in neighbors.iter() {
                            let (other_entity, other_pos) = item;
                            if *other_entity == entity {
                                continue;
                            }

                            let dist = my_pos.distance(*other_pos);
                            if dist < SEPARATION_DISTANCE && dist > 0.0 {
                                let push = (my_pos - *other_pos).normalize()
                                    * SEPARATION_STRENGTH
                                    * ((SEPARATION_DISTANCE - dist) / SEPARATION_DISTANCE);
                                sep += push;
                            }
                        }

                        // Clamp separation to avoid explosive pushes
                        let max_sep = SEPARATION_STRENGTH;
                        if sep.length() > max_sep {
                            sep = sep.normalize() * max_sep;
                        }

                        // Desired (unscaled) velocity blends approach + separation
                        let desired_vel = base_vel + sep;

                        // Convert to per-second velocity using global scale to preserve prior
                        // per-frame movement feel from the original code.
                        let desired_vel_per_sec = desired_vel * MOVEMENT_SPEED_SCALE;

                        // Smooth velocity via component (insert on first use)
                        if let Some(mut vel_comp) = maybe_velocity {
                            vel_comp.0 = vel_comp.0.lerp(desired_vel_per_sec, SEPARATION_SMOOTHING);
                            transform.translation += vel_comp.0 * time.delta_secs();
                        } else {
                            // First time, insert a velocity component (store per-second velocity)
                            commands
                                .entity(entity)
                                .insert(Velocity(desired_vel_per_sec));
                            transform.translation += desired_vel_per_sec * time.delta_secs();
                        }

                        // Update facing and distance
                        transform.look_at(aim, Vec3::Y);
                        approaching.distance = transform.translation.distance(target.translation);
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

#[derive(Component)]
pub struct ApproachOffset(pub Vec3);

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

/// Velocity component used for smooth steering
#[derive(Component)]
pub struct Velocity(pub Vec3);

/// A simple uniform grid spatial partition for neighbor queries.
#[derive(Resource)]
pub struct SeparationGrid {
    pub cell_size: f32,
    pub buckets: HashMap<(i32, i32), Vec<(Entity, Vec3)>>,
}

impl SeparationGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            buckets: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
    }

    pub fn insert(&mut self, entity: Entity, pos: Vec3) {
        let key = (
            (pos.x / self.cell_size).floor() as i32,
            (pos.z / self.cell_size).floor() as i32,
        );
        self.buckets.entry(key).or_default().push((entity, pos));
    }

    /// Collect neighbors from the 3x3 cells around position (including the cell itself)
    pub fn neighbors<'a>(&'a self, pos: &Vec3, out: &mut Vec<&'a (Entity, Vec3)>) {
        out.clear();
        let cx = (pos.x / self.cell_size).floor() as i32;
        let cz = (pos.z / self.cell_size).floor() as i32;

        for dx in -1..=1 {
            for dz in -1..=1 {
                let key = (cx + dx, cz + dz);
                if let Some(bucket) = self.buckets.get(&key) {
                    for item in bucket.iter() {
                        out.push(item);
                    }
                }
            }
        }
    }
}

/// Build the spatial grid from current approaching entity positions. This runs each Update.
fn build_grid_system(
    mut grid: ResMut<SeparationGrid>,
    query: Query<(Entity, &Transform), With<Approaching>>,
) {
    grid.clear();
    for (e, t) in query.iter() {
        grid.insert(e, t.translation);
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
