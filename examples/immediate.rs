//! This example demonstrates how to use the [`Raycast`] system param to run raycasts on-demand, in
//! an immediate mode style. This is unlike using a [`RaycastSource`] which runs a raycast and
//! stores the result once per frame.

use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, raycast)
        .run();
}

fn raycast(mut raycast: Raycast, mut gizmos: Gizmos, time: Res<Time>) {
    let t = time.elapsed_seconds();
    let ray_pos = Vec3::new(t.sin(), (t * 1.5).cos(), t.cos()) * 2.5;
    let ray_dir = -ray_pos.normalize();

    // This is all that is needed to raycast into the world!
    let hits = raycast.cast_ray(Ray3d::new(ray_pos, ray_dir), &RaycastSettings::default());

    gizmos.ray(ray_pos, ray_dir, Color::YELLOW);
    gizmos.sphere(ray_pos, Quat::IDENTITY, 0.1, Color::YELLOW);
    if let Some((_, hit)) = hits.first() {
        gizmos.sphere(hit.position(), Quat::IDENTITY, 0.2, Color::PINK);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(Camera3dBundle::default())
        .insert(Transform::from_xyz(0.0, 0.0, 5.0));
    commands
        .spawn(PointLightBundle::default())
        .insert(Transform::from_xyz(2.0, 2.0, 5.0));
    commands
        .spawn(PbrBundle::default())
        .insert(meshes.add(Mesh::try_from(shape::Icosphere::default()).unwrap()))
        .insert(materials.add(Color::rgb(1.0, 1.0, 1.0).into()));
}
