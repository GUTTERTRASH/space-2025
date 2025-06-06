//! A simple 3D scene to demonstrate physics picking for colliders.
//!
//! By default, the [`PhysicsPickingPlugin`] will test intersections with the pointer against all colliders.
//! If you want physics picking to be opt-in, you can set [`PhysicsPickingSettings::require_markers`] to `true`
//! and add a [`PhysicsPickable`] component to the desired camera and target entities.
//!
//! Cameras can further filter which entities are pickable with the [`PhysicsPickingFilter`] component.

use core::f32::consts::PI;

use avian3d::{math::Vector, prelude::*};
use bevy::{color::palettes::tailwind::*, picking::pointer::PointerInteraction, prelude::*};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            // `PhysicsPickingPlugin` is not a default plugin
            PhysicsPickingPlugin,
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, draw_pointer_intersections)
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 12.0;
const Z_EXTENT: f32 = 5.0;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Set up the materials.
    let white_matl = materials.add(Color::WHITE);
    let ground_matl = materials.add(Color::from(GRAY_300));
    let hover_matl = materials.add(Color::from(CYAN_300));
    let pressed_matl = materials.add(Color::from(YELLOW_300));

    // Meshes and colliders for the shapes.
    let shapes = [
        (
            meshes.add(Cuboid::default()),
            Collider::from(Cuboid::default()),
        ),
        (
            meshes.add(Tetrahedron::default()),
            Collider::convex_hull_from_mesh(&Tetrahedron::default().mesh().build()).unwrap(),
        ),
        (
            meshes.add(Capsule3d::default()),
            Collider::from(Capsule3d::default()),
        ),
        (
            meshes.add(Torus::default()),
            Collider::trimesh_from_mesh(&Torus::default().mesh().build()).unwrap(),
        ),
        (
            meshes.add(Cylinder::default()),
            Collider::from(Cylinder::default()),
        ),
        (meshes.add(Cone::default()), Collider::from(Cone::default())),
        (
            meshes.add(ConicalFrustum::default()),
            Collider::trimesh_from_mesh(&ConicalFrustum::default().mesh().build()).unwrap(),
        ),
        (
            meshes.add(Sphere::default().mesh().ico(5).unwrap()),
            Collider::from(Sphere::default()),
        ),
    ];

    let num_shapes = shapes.len();

    // Spawn the shapes. The colliders will be pickable by default.
    for (i, (mesh, collider)) in shapes.into_iter().enumerate() {
        commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(white_matl.clone()),
                RigidBody::Kinematic,
                collider,
                AngularVelocity(Vector::new(0.0, 0.5, 0.0)),
                Transform::from_xyz(
                    -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                    2.0,
                    Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
            ))
            .observe(update_material_on::<Pointer<Over>>(hover_matl.clone()))
            .observe(update_material_on::<Pointer<Out>>(white_matl.clone()))
            .observe(update_material_on::<Pointer<Down>>(pressed_matl.clone()))
            .observe(update_material_on::<Pointer<Up>>(hover_matl.clone()))
            .observe(rotate_on_drag);
    }

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(ground_matl.clone()),
        RigidBody::Static,
        Collider::cuboid(50.0, 0.1, 50.0),
        PickingBehavior::IGNORE, // Disable picking for the ground plane.
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    // Instructions
    commands.spawn((
        Text::new("Hover over the shapes to pick them\nDrag to rotate"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Returns an observer that updates the entity's material to the one specified.
fn update_material_on<E>(
    new_material: Handle<StandardMaterial>,
) -> impl Fn(Trigger<E>, Query<&mut MeshMaterial3d<StandardMaterial>>) {
    // An observer closure that captures `new_material`. We do this to avoid needing to write four
    // versions of this observer, each triggered by a different event and with a different hardcoded
    // material. Instead, the event type is a generic, and the material is passed in.
    move |trigger, mut query| {
        if let Ok(mut material) = query.get_mut(trigger.entity()) {
            material.0 = new_material.clone();
        }
    }
}

/// A system that draws hit indicators for every pointer.
fn draw_pointer_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}

/// An observer to rotate an entity when it is dragged.
fn rotate_on_drag(drag: Trigger<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    let mut transform = transforms.get_mut(drag.target).unwrap();
    transform.rotate_y(drag.delta.x * 0.02);
    transform.rotate_x(drag.delta.y * 0.02);
}