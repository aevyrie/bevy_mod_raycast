//! This example will show you how to use your mouse cursor as a ray casting source, cast into the
//! scene, intersect a mesh, and mark the intersection with the built in debug cursor. If you are
//! looking for a more fully-featured mouse picking plugin, try out bevy_mod_picking.

use bevy::{color::palettes::css, prelude::*};
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy_mod_raycast::low_latency_window_plugin()))
        .add_plugins(CursorRayPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, raycast)
        .run();
}

fn raycast(cursor_ray: Res<CursorRay>, mut raycast: Raycast, mut gizmos: Gizmos) {
    if let Some(cursor_ray) = **cursor_ray {
        raycast.debug_cast_ray(cursor_ray, &default(), &mut gizmos);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle::default());
    commands.spawn(PointLightBundle::default());
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::default()),
        material: materials.add(Color::from(css::GRAY)),
        transform: Transform::from_xyz(0.0, 0.0, -5.0),
        ..default()
    });
}
