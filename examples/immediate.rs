//! This example demonstrates how to use the [`Raycast`] system param in queries to run raycasts
//! on-demand, in an immediate mode style. This is unlike using a [`RaycastSource`] which runs a
//! raycast and stores the result once per frame.

use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, immediate_mode_raycast)
        .run();
}

#[derive(Reflect)]
struct MyRaycastSet;

fn immediate_mode_raycast(raycast: Raycast<MyRaycastSet>, mut gizmos: Gizmos, time: Res<Time>) {
    // Animate the ray around the sphere mesh, always pointing to the center of the sphere
    let t = time.elapsed_seconds();
    let ray_pos = Vec3::new(t.sin(), (3.0 * t).cos() * 0.5, t.cos()) * 2.5;
    let ray_dir = -ray_pos.normalize();
    let ray = Ray3d::new(ray_pos, ray_dir);

    // Debug draw the ray
    gizmos.ray(ray_pos, ray_dir, Color::YELLOW);
    gizmos.sphere(ray_pos, Quat::IDENTITY, 0.1, Color::YELLOW);

    // This is all that is needed to raycast into the world!
    let hits = raycast.cast_ray(ray, false, true);

    // Go through the intersections and render them as a pink circle
    if let Some((_, hit)) = hits.first() {
        gizmos.sphere(
            hit.position(),
            Quat::from_rotation_arc(Vec3::Z, hit.normal()),
            0.2,
            Color::PINK,
        );
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 5.0),
        ..default()
    },));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::try_from(shape::Icosphere::default()).unwrap()),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            ..Default::default()
        },
        RaycastMesh::<MyRaycastSet>::default(), // Make this mesh ray cast-able
    ));
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}
