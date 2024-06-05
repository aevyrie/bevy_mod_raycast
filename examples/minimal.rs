//! This example demonstrates how to use the [`Raycast`] system param to run raycasts on-demand, in
//! an immediate mode style. This is unlike using a deferred API, which runs a raycast based on
//! [`RaycastSource`] components once per frame.

use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, raycast)
        .run();
}

const RAY_DIST: Vec3 = Vec3::new(0.0, 0.0, -7.0);

fn raycast(mut raycast: Raycast, mut gizmos: Gizmos, time: Res<Time>) {
    let t = time.elapsed_seconds();
    let pos = Vec3::new(t.sin(), (t * 1.5).cos() * 2.0, t.cos()) * 1.5 + RAY_DIST;
    let dir = (RAY_DIST - pos).normalize();
    // This is all that is needed to raycast into the world! You can also use the normal, non-debug
    // version (raycast.cast_ray) when you don't need to visualize the ray or intersections.
    raycast.debug_cast_ray(Ray3d::new(pos, dir), &default(), &mut gizmos);
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle::default());
    commands.spawn(PointLightBundle::default());
    commands.spawn(PbrBundle {
        mesh: meshes.add(Capsule3d::default()),
        material: materials.add(Color::srgb(1.0, 1.0, 1.0)),
        transform: Transform::from_translation(RAY_DIST),
        ..default()
    });
}
