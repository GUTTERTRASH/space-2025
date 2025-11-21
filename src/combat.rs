use bevy::prelude::*;


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


pub struct CombatPlugin;


impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<AiAction>()
            .register_type::<Thinker>()
            .add_systems(PreUpdate, (
                threat_scorer_system,
                range_scorer_system,
            ).in_set(AiSet::Scorers))
            .add_systems(PreUpdate, picker_system.in_set(AiSet::Pickers))
            .add_systems(Update, action_system.in_set(AiSet::Actions));
    }
}


pub fn threat_scorer_system(
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
                    info!("Seeking target...");
                    // Parallel-safe steering (assume closest enemy query cached elsewhere)
                    // let target_dir = Vec3::X;  // Placeholder: Compute from enemies
                    // ship.velocity += target_dir.normalize_or_zero() * 50.0 * time.delta_seconds();
                }
                AiAction::Evade => {
                    info!("Evading...");
                    // ship.velocity += ship.velocity.any_orthogonal().normalize_or_zero() * 30.0 * time.delta_seconds();
                }
                AiAction::Fire => {
                    info!("Firing...");
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