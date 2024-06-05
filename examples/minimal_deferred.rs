//! This example demonstrates how to use the [`bevy_mod_raycast::deferred`] API. Unlike the
//! [`Raycast`] system param, this API is declarative, and does not return a result immediately.
//! Instead, behavior is defined using components, and raycasting is done once per frame.

use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DeferredRaycastingPlugin::<MyRaycastSet>::default())
        // Overrides default settings and enables the debug cursor
        .insert_resource(RaycastPluginState::<MyRaycastSet>::default().with_debug_cursor())
        .add_systems(Startup, setup)
        .add_systems(Update, move_ray)
        .run();
}

const RAY_DIST: Vec3 = Vec3::new(0.0, 0.0, -7.0);

#[derive(Reflect)]
struct MyRaycastSet; // Groups raycast sources with meshes, can use `()` instead.

#[derive(Component)]
struct MovingRaycaster;

fn move_ray(time: Res<Time>, mut query: Query<&mut Transform, With<MovingRaycaster>>) {
    let t = time.elapsed_seconds();
    let pos = Vec3::new(t.sin(), (t * 1.5).cos() * 2.0, t.cos()) * 1.5 + RAY_DIST;
    let dir = (RAY_DIST - pos).normalize();
    *query.single_mut() = Transform::from_translation(pos).looking_to(dir, Vec3::Y);
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle::default());
    commands.spawn(PointLightBundle::default());
    // Unlike the immediate mode API where the raycast is built every frame in a system, instead we
    // spawn an entity and mark it as a raycasting source, using its `GlobalTransform`.
    commands.spawn((
        MovingRaycaster,
        SpatialBundle::default(),
        RaycastSource::<MyRaycastSet>::new_transform_empty(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d::default()),
            material: materials.add(Color::srgb(1.0, 1.0, 1.0)),
            transform: Transform::from_translation(RAY_DIST),
            ..default()
        },
        RaycastMesh::<MyRaycastSet>::default(), // Make this mesh ray cast-able
    ));
}
