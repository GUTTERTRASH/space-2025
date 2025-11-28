use bevy::prelude::*;
use bevy::color::palettes::css::*;
use bevy::color::palettes::tailwind::*;
use bevy_egui::egui::Color32;
use bevy_egui::*;
use bevy_egui::egui;

#[derive(Component, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Component)]
pub enum AiAction {
    Idle,
    SeekTarget,
    Evade,
    Fire
}


impl Default for AiAction {
    fn default() -> Self {
        AiAction::Idle
    }
}



#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct Thinker {
    pub scores: Vec<f32>,
    pub current_action: AiAction,
    pub threshold: f32,
}

#[derive(Component, Default)]
pub struct ThreatScore(f32);


#[derive(Component, Default)]
pub struct RangeScore(f32);


#[derive(Component)]
pub struct AiMarker;


#[derive(Component)]
pub struct Ship {
    pub position: Vec3,
    pub health: f32,
    pub max_health: f32,
}


#[derive(Component)]
pub struct AiEnemy {
    pub position: Vec3,
}


#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum AiSet {
    Scorers,
    Pickers,
    Actions
}

#[derive(Component)]
struct AiDebugLabel;


#[derive(Resource, Default, PartialEq)]
pub struct DebugAiViz(pub bool);

pub struct CombatPlugin;


impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DebugAiViz>()
            .register_type::<AiAction>()
            .register_type::<Thinker>()
            .add_systems(PreUpdate, (
                threat_scorer_system,
                range_scorer_system,
            ).in_set(AiSet::Scorers))
            .add_systems(PreUpdate, picker_system.in_set(AiSet::Pickers))
            .add_systems(Update, action_system.in_set(AiSet::Actions))
            .add_systems(Update, toggle_ai_viz)
            .add_systems(EguiPrimaryContextPass, ai_debug_dashboard)
            .add_systems(Update, ai_gizmos_system.run_if(resource_equals(DebugAiViz(true))));
    }
}


fn threat_scorer_system(
    mut query: Query<(&Ship, &mut ThreatScore), With<AiMarker>>,
    enemies: Query<&AiEnemy>,
) {

    let enemy_positions: Vec<Vec3> = enemies.iter().map(|e| e.position).collect();

    query.par_iter_mut().for_each(|(ship, mut score)| {

        if enemy_positions.is_empty() {

            info!("No enemies - setting threat score to zero");
            score.0 = 0.0;

        } else {

            let closest_dist = enemy_positions
                .iter()
                .map(|&pos| ship.position.distance(pos))
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(f32::MAX);


            let dist_norm = 1.0 - (closest_dist / 100.0).clamp(0.0, 1.0);  // 1.0 = close
            let health_norm = 1.0 - (ship.health / ship.max_health);  // 1.0 = low HP
            score.0 = (dist_norm * health_norm * 0.6).clamp(0.0, 1.0);

        }

  
    });

}


// Range Scorer: Similar parallel pattern
fn range_scorer_system(
    mut query: Query<(&Ship, &mut RangeScore), With<AiMarker>>,
    enemies: Query<&AiEnemy>,
) {
    let enemy_positions: Vec<Vec3> = enemies.iter().map(|e| e.position).collect();
    query.par_iter_mut().for_each(|(ship, mut score)| {
        let in_range = enemy_positions.iter().any(|&pos| ship.position.distance(pos) <= 50.0);
        score.0 = if in_range {
            0.4
        } else {
            0.0
        }
    });
}


fn picker_system(
    mut query: Query<(&ThreatScore, &RangeScore, &mut Thinker), With<AiMarker>>,
) {
    for (threat, range, mut thinker) in &mut query.iter_mut() {
        let num_actions = 4;  // AiAction count
        let mut scores = vec![0.2; num_actions];  // Baseline for Idle

        // Map scores to actions (tune weights/curves here)
        scores[AiAction::SeekTarget as usize] = threat.0 * 0.7 + range.0 * 0.3;
        scores[AiAction::Evade as usize] = threat.0 * 1.2;  // Boost urgency
        scores[AiAction::Fire as usize] = range.0;

        // Pick highest above threshold
        let mut best_idx = 0;
        let mut best_score = thinker.threshold;
        for (i, &score) in scores.iter().enumerate() {
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        let new_action = unsafe { std::mem::transmute(best_idx as u8) };
        if new_action != thinker.current_action {
            thinker.current_action = new_action;
            thinker.scores = scores;  // Cache for debug
            // Optional: Spawn event for action change
        }
    }
}


fn action_system(
    mut query: Query<(&mut Ship, &Thinker), With<AiMarker>>,
    time: Res<Time>,
) {
    query.par_iter_mut().for_each(|(mut ship, thinker)| {
        if thinker.current_action != AiAction::Idle {

            match thinker.current_action {
                AiAction::SeekTarget => {
                    // info!("Seeking target...");
                    // Parallel-safe steering (assume closest enemy query cached elsewhere)
                    // let target_dir = Vec3::X;  // Placeholder: Compute from enemies
                    // ship.velocity += target_dir.normalize_or_zero() * 50.0 * time.delta_seconds();
                }
                AiAction::Evade => {
                    // info!("Evading...");
                    // ship.velocity += ship.velocity.any_orthogonal().normalize_or_zero() * 30.0 * time.delta_seconds();
                }
                AiAction::Fire => {
                    // info!("Firing...");
                    // Spawn projectile (Bevy's spawn is thread-safe via commands)
                    // commands.spawn(Projectile { .. });
                }
                _ => {}
            }

            // Physics update (local)
            // ship.position += ship.velocity * time.delta_seconds();

        }
    });
}


fn toggle_ai_viz(mut viz: ResMut<DebugAiViz>, keys: Res<ButtonInput<KeyCode>>) {

    if keys.just_pressed(KeyCode::KeyF) {
        viz.0 = !viz.0;
    }

}

fn ai_gizmos_system(
    mut gizmos: Gizmos,
    ai_query: Query<(&Ship, &Thinker, &ThreatScore, &RangeScore), With<AiMarker>>,
    enemies: Query<&AiEnemy>,
    viz: Res<DebugAiViz>,
) {

    if !viz.0 { return; }

    let enemy_positions: Vec<Vec3> = enemies.iter().map(|e| e.position).collect();

    for (ship, thinker, threat, range) in &ai_query {

        let pos = ship.position;
        let radius = 3.0 + range.0 + 15.0;

        let ring_color = match thinker.current_action {
            AiAction::Idle => Color::from(GRAY),
            AiAction::SeekTarget => Color::from(TURQUOISE),
            AiAction::Evade => Color::from(RED),
            AiAction::Fire => Color::from(YELLOW),
        };

        gizmos.circle(pos, radius, ring_color);

        let threat_color = Color::hsl(0.0, 1.0, 0.5 * (1.0 - threat.0));
        // gizmos.circle(pos, 1.5, threat_color);

        gizmos.circle(
            Isometry3d::new(
                pos,
                Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
            ),
            0.2,
            threat_color,
        );

        // Seek/Fire: Line to closest enemy
        if matches!(thinker.current_action, AiAction::SeekTarget | AiAction::Fire) && !enemy_positions.is_empty() {
            if let Some(&closest) = enemy_positions.iter().min_by(|a, b| {
                pos.distance_squared(**a).partial_cmp(&pos.distance_squared(**b)).unwrap()
            }) {
                // gizmos.line(pos, closest, Color::WHITE);
                gizmos.arrow(pos, (pos - closest).normalize(), Color::from(WHITE));
            }
        }


        let label_pos = pos + Vec3::Y * (radius + 2.0);
        match thinker.current_action {
            AiAction::Idle => {
                // Gray dot for idle
                gizmos.circle(
                    Isometry3d::new(
                        label_pos,
                        Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
                    ),
                    0.5,
                    Color::from(GRAY),
                );
            }
            AiAction::SeekTarget => {
                // Cyan arrow pointing forward
                gizmos.arrow_2d(label_pos.xy(), label_pos.xy() + Vec2::X * 2.0, Color::from(TURQUOISE));
            }
            AiAction::Evade => {
                // Red zigzag line for evasion
                let zig = [label_pos + Vec3::X * -1.0 + Vec3::Y * 0.5, label_pos + Vec3::X * 1.0 - Vec3::Y * 0.5];
                gizmos.linestrip_2d([zig[0].xy(), label_pos.xy(), zig[1].xy()], Color::from(RED));
            }
            AiAction::Fire => {
                // Yellow burst (star-like cross)
                gizmos.cross_2d(label_pos.xy(), 1.0, Color::from(YELLOW));
            }
        }

    }

}


fn ai_debug_dashboard(
    mut contexts: EguiContexts,
    ai_query: Query<(Entity, &Thinker, &ThreatScore, &RangeScore), With<AiMarker>>,
) {
    let ctx = match contexts.ctx_mut() {
        Ok(c) => c,
        Err(_) => return,
    };
    egui::Window::new("🧠 Utility AI Debug").show(ctx, |ui| {
        let agents = ai_query.iter().collect::<Vec<_>>();
        ui.label(format!("{} AI Agents Active", agents.len()));

        let action_names = ["Idle", "Seek", "Evade", "Fire"];

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (entity, thinker, threat, range) in agents {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(format!("Entity #{}: {:?}", entity.index(), thinker.current_action));
                        if ui.button("📋 Inspect").clicked() {
                            // Optional: Integrate inspector focus (advanced)
                        }
                    });

                    // Threat/Range rows
                    ui.horizontal(|ui| {
                        ui.label("Threat:");
                        ui.add(egui::ProgressBar::new(threat.0).fill(Color32::from_rgba_unmultiplied(255, 77, 77, 255)));
                        ui.label(format!("{:.2}", threat.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Range:");
                        ui.add(egui::ProgressBar::new(range.0).fill(Color32::from_rgba_unmultiplied(77, 255, 77, 255)));
                        ui.label(format!("{:.2}", range.0));
                    });

                    // All action scores as bars
                    ui.horizontal_wrapped(|ui| {
                        for (i, score) in thinker.scores.iter().enumerate() {
                            let clamped = score.clamp(0.0, 1.0);
                            ui.vertical(|ui| {
                                ui.label(action_names[i]);
                                let bar = egui::ProgressBar::new(clamped);
                                ui.add(bar.fill(get_action_color(i as u8)));  // Define below
                            });
                        }
                    });

                    ui.label(format!("Threshold: {:.2}", thinker.threshold));
                    ui.separator();
                });
            }
        });
    });
}

fn get_action_color(action_idx: u8) -> egui::Color32 {
    match action_idx {
        0 => egui::Color32::GRAY,      // Idle
        1 => egui::Color32::CYAN,      // Seek
        2 => egui::Color32::RED,       // Evade
        3 => egui::Color32::YELLOW,    // Fire
        _ => egui::Color32::WHITE,
    }
}