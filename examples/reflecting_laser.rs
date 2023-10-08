//! This example demonstrates how to use the [`Raycast`] system param to chain multiple raycasts and
//! bounce off of surfaces.

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, bouncing_raycast)
        .insert_resource(ClearColor(Color::BLACK))
        .run();
}

const MAX_BOUNCES: usize = 64;
const LASER_MOVE_SPEED: f32 = 0.03;

#[derive(Reflect)]
struct Laser;

fn bouncing_raycast(mut raycast: Raycast, mut gizmos: Gizmos, time: Res<Time>) {
    let t =
        ((time.elapsed_seconds() - 4.0).max(0.0) * LASER_MOVE_SPEED).cos() * std::f32::consts::PI;
    let mut ray_pos = Vec3::new(t.sin(), (3.0 * t).cos() * 0.5, t.cos()) * 0.5;
    let mut ray_dir = (-ray_pos).normalize();
    gizmos.sphere(ray_pos, Quat::IDENTITY, 0.1, Color::WHITE);

    let mut intersections = Vec::with_capacity(MAX_BOUNCES + 1);
    intersections.push((ray_pos, Color::rgb(30.0, 0.0, 0.0)));

    for i in 0..MAX_BOUNCES {
        let ray = Ray3d::new(ray_pos, ray_dir);
        if let Some((_, hit)) = raycast.cast_ray(ray, &RaycastSettings::default()).first() {
            let r = 1.0 + 10.0 * (1.0 - i as f32 / MAX_BOUNCES as f32);
            intersections.push((hit.position(), Color::rgb(r, 0.0, 0.0)));
            gizmos.sphere(hit.position(), Quat::IDENTITY, 0.005, Color::RED * r * 2.0);
            // reflect the ray
            let proj = (ray_dir.dot(hit.normal()) / hit.normal().dot(hit.normal())) * hit.normal();
            ray_dir = (ray_dir - 2.0 * proj).normalize();
            ray_pos = hit.position() + ray_dir * 1e-6;
        } else {
            break;
        }
    }
    gizmos.linestrip_gradient(intersections);
}

// Set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.1, 0.2, 0.0)),
        ..Default::default()
    });
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(1.5, 1.5, 1.5).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                hdr: true,
                ..default()
            },
            tonemapping: bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
            ..default()
        },
        BloomSettings::default(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::GRAY.with_a(0.05).into()),
            ..default()
        },
        // Without this, raycasts would shoot straight out from the inside of the cube.
        bevy_mod_raycast::NoBackfaceCulling,
    ));
}
