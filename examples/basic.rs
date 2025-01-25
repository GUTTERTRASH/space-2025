use bevy::prelude::*;
use bevy_third_person_camera::{ThirdPersonCamera, ThirdPersonCameraPlugin, ThirdPersonCameraTarget};
use space::movement::MovementPlugin;

#[derive(Component)]
struct Target;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ThirdPersonCameraPlugin)
        .add_plugins(MovementPlugin)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 10.0,
        })
        .add_systems(Startup, spawn_camera)
        .add_systems(Startup, spawn_player)
        .add_systems(Startup, spawn_targets)
        .add_systems(Startup, spawn_lights)
        .run();
}

fn spawn_camera(
    mut commands: Commands,
) {
    commands.spawn((
        ThirdPersonCamera::default(),
        Camera3d::default()
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
        Mesh3d(assets.load("models/spaceship.gltf#Mesh0/Primitive0")),
        MeshMaterial3d(material_handle.clone()),
        Transform::from_scale(Vec3::new(0.1, 0.1, 0.5)),
        ThirdPersonCameraTarget,
    ));

}

fn spawn_targets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {

    let mut spawn_cube = |position, color, name| {

        let material = materials.add(StandardMaterial {
            base_color: color,
            reflectance: 0.02,
            unlit: false,
            ..Default::default()
        });

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(material),
            Transform::from_translation(position),
            Name::new(name),
            Target,
        ));
    };

    spawn_cube(Vec3::new(-15.0, 0.0, -55.0), Color::srgb_u8(255, 100, 0), "Sara");
    spawn_cube(Vec3::new(-25.0, 0.0, -25.0), Color::srgb_u8(0, 240, 123), "Entity");

}

fn spawn_lights(mut commands: Commands) {
    commands.spawn(
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        }
    );
}
