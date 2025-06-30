use bevy::color::palettes::tailwind::{GRAY_500, PINK_100, RED_500};
use bevy::log::LogPlugin;
use bevy::picking::pointer::PointerInteraction;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};
use bevy_third_person_camera::{
    Offset, ThirdPersonCamera, ThirdPersonCameraPlugin, ThirdPersonCameraTarget, Zoom,
};
use big_brain::prelude::{FirstToScore, HighestToScore};
use big_brain::scorers::{FixedScore, Score};
use big_brain::thinker::Thinker;
use space::combat::{
    Approach, Approaching, Approachy, Attack, Attacking, Attacky, CombatPlugin, MissileAttack,
    MissileLoadout, Missily, MIN_DISTANCE,
};
use space::common::{Enemy, Player};
use space::movement::MovementPlugin;
use space::projectile::ProjectilePlugin;
use space::reticule::ReticulePlugin;
use space::utils::generate_targets;

use avian3d::prelude::*;

#[derive(Component)]
struct Target;

const NUM_TARGETS: usize = 1;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(LogPlugin {
                filter: "big_brain=debug,space=debug".to_string(),
                ..default()
            }),
            ThirdPersonCameraPlugin,
            PhysicsPlugins::default(),
            PhysicsPickingPlugin,
            ReticulePlugin,
            MovementPlugin,
            ProjectilePlugin,
            CombatPlugin,
            WorldInspectorPlugin::default(),
        ))
        // .insert_resource(ClearColor(Color::from(BLACK)))
        .insert_resource(ClearColor(Color::from(GRAY_500)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 10.0,
        })
        .insert_resource(Gravity(Vec3::ZERO))
        .add_systems(
            Startup,
            (
                spawn_camera,
                spawn_lights,
                spawn_player,
                spawn_targets,
                spawn_scorecard,
            )
                .chain(),
        )
        .add_systems(Update, (draw_mesh_intersections, update_ui))
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        ThirdPersonCamera {
            offset_enabled: true,
            offset: Offset::new(0.0, 0.2),
            zoom: Zoom::new(0.2, 10.0),
            ..default()
        },
        Camera3d::default(),
    ));
}

fn spawn_player(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        reflectance: 1.0,
        ..default()
    });

    commands.spawn((
        Name::new("Player"),
        Mesh3d(assets.load("models/spaceship.gltf#Mesh0/Primitive0")),
        MeshMaterial3d(material_handle.clone()),
        Transform::from_scale(Vec3::new(0.1, 0.1, 0.5)),
        ThirdPersonCameraTarget,
        PickingBehavior::IGNORE,
        Player,
        RigidBody::Dynamic,
        ColliderConstructor::TrimeshFromMesh,
        LockedAxes::ROTATION_LOCKED,
    ));
}

#[derive(Component)]
pub struct Nameplate;

// Spawns n number of random targets
fn spawn_targets(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // actions: Res<ActionIds>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    assets: Res<AssetServer>,
) {
    let mut spawn_cube = |position, color, name: String| {
        let material = materials.add(StandardMaterial {
            base_color: color,
            reflectance: 1.0,
            unlit: false,
            ..Default::default()
        });

        let name_clone = name.clone();

        let (player, player_transform) = player_query.single();

        commands
            .spawn((
                Name::new(name),
                Mesh3d(assets.load("models/spaceship.gltf#Mesh0/Primitive0")),
                MeshMaterial3d(material),
                Transform::from_translation(position).with_scale(Vec3::new(0.1, 0.1, 0.5)),
                Target,
                Enemy::default(),
                Approaching {
                    target: player,
                    distance: player_transform.translation.distance(position),
                    speed: 0.5,
                },
                Attacking(player),
                MissileLoadout { ammo: 20 },
                Thinker::build()
                    .label("My Thinker")
                    // .picker(FirstToScore { threshold: 0.3 })
                    // .picker(HighestToScore::new(0.3)
                    .picker(HighestToScore::default())
                    .when(
                        Approachy,
                        Approach {
                            until_distance: MIN_DISTANCE,
                        },
                    )
                    .when(Attacky, Attack { min_distance: 30.0 })
                    .when(Missily, MissileAttack { min_distance: 60.0 }),
                RigidBody::Dynamic,
                ColliderConstructor::TrimeshFromMesh,
            ))
            .observe(move |_over: Trigger<Pointer<Over>>| {
                info!("YOOO {name_clone}!");
            });
    };

    for (position, color, name) in generate_targets(NUM_TARGETS) {
        spawn_cube(
            position
                - Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 200.0,
                },
            color,
            name,
        );
    }
}

fn spawn_lights(mut commands: Commands) {
    let theta = std::f32::consts::FRAC_PI_4;
    let light_transform = Mat4::from_euler(EulerRot::ZYX, 0.0, std::f32::consts::FRAC_PI_2, -theta);
    commands.spawn((
        DirectionalLight {
            illuminance: 9_999.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_matrix(light_transform),
    ));
}

// fn spawn_sun(
//     mut commands: Commands,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     mut meshes: ResMut<Assets<Mesh>>,
//     assets: Res<AssetServer>,
// ) {
//     // True-to-life scale (1 unit = 1,000 km)
//     let sun_radius = 0.696; // 696,340 km / 1,000,000
//     let sun_distance = 149.6; // 149,600,000 km / 1,000,000

//     let sun_material = materials.add(StandardMaterial {
//         base_color: Color::srgb(1.0, 0.95, 0.7),
//         emissive: Color::srgba(1.0, 0.95, 0.7, 1.0).into(),
//         unlit: true,
//         ..default()
//     });

//     let sun_transform = Transform::from_translation(Vec3::new(0.0, 0.0, -sun_distance));

//     commands.spawn((
//         PointLight {
//             intensity: 10_000.0,
//             shadows_enabled: true,
//             range: 100.0,
//             ..default()
//         },
//         sun_transform,
//     ));

//     commands.spawn((
//         Mesh3d(meshes.add(Sphere { radius: sun_radius })),
//         MeshMaterial3d(sun_material),
//         sun_transform,
//         Name::new("Sun"),
//     ));
// }

/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}

#[derive(Component)]
pub struct ScoreText;

fn spawn_scorecard(mut commands: Commands) {
    let font = TextFont {
        font_size: 15.0,
        ..default()
    };

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::End,
            align_items: AlignItems::FlexStart,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((Text::new("Score "), font.clone(), ScoreText));
            // builder.spawn((Text::new(""), font.clone(), FatigueText));
            // builder.spawn((Text::new(""), font.clone(), InventoryText));
        });
}

fn update_ui(
    actor_query: Query<(&Name, &Approaching)>,
    approach_score_query: Query<&Score, With<Approachy>>,
    attack_score_query: Query<&Score, With<Attacky>>,
    missile_score_query: Query<&Score, With<Missily>>,
    // Our queries must be "disjoint", so we use the `Without` component to
    // ensure that we do not query for the same entity twice.
    mut score_text_query: Query<&mut Text, (With<ScoreText>,)>,
) {

    let approach_score = approach_score_query.get_single()
            .map(|x| x.get())
            .unwrap_or(0.0);

    let attack_score = attack_score_query.get_single()
        .map(|x| x.get())
        .unwrap_or(0.0);

    let missile_score = missile_score_query.get_single()
        .map(|x| x.get())
        .unwrap_or(0.0);
        
    let mut score_text = score_text_query.single_mut();
    for (name, approaching) in &actor_query {
        score_text.0 = format!(
            "Name: {name}\nDistance: {}\nApproach:{}\nAttack:{}\nMissle:{}",
            approaching.distance, approach_score, attack_score, missile_score
        );
    }
}
