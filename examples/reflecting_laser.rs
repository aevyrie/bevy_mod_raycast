//! This example demonstrates how to use the [`Raycast`] system param to chain multiple raycasts and
//! bounce off of surfaces.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{color::palettes::css, core_pipeline::bloom::BloomSettings, math::vec3, prelude::*};
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(bevy_mod_raycast::low_latency_window_plugin()),
            CursorRayPlugin,
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, bouncing_raycast)
        .insert_resource(ClearColor(Color::BLACK))
        .run();
}

const MAX_BOUNCES: usize = 64;
const LASER_SPEED: f32 = 0.03;

#[derive(Reflect)]
struct Laser;

fn bouncing_raycast(
    mut raycast: Raycast,
    mut gizmos: Gizmos,
    time: Res<Time>,
    cursor_ray: Res<CursorRay>,
) {
    let t = ((time.elapsed_seconds() - 4.0).max(0.0) * LASER_SPEED).cos() * std::f32::consts::PI;
    let ray_pos = Vec3::new(t.sin(), (3.0 * t).cos() * 0.5, t.cos()) * 0.5;
    let ray_dir = (-ray_pos).normalize();
    let ray = Ray3d::new(ray_pos, ray_dir);
    gizmos.sphere(ray_pos, Quat::IDENTITY, 0.1, Color::WHITE);
    bounce_ray(ray, &mut raycast, &mut gizmos, Color::from(css::RED));

    if let Some(cursor_ray) = **cursor_ray {
        bounce_ray(
            cursor_ray,
            &mut raycast,
            &mut gizmos,
            Color::from(css::GREEN),
        )
    }
}

fn bounce_ray(mut ray: Ray3d, raycast: &mut Raycast, gizmos: &mut Gizmos, color: Color) {
    let mut intersections = Vec::with_capacity(MAX_BOUNCES + 1);
    intersections.push((ray.origin, Color::srgb(30.0, 0.0, 0.0)));

    for i in 0..MAX_BOUNCES {
        if let Some((_, hit)) = raycast.cast_ray(ray, &RaycastSettings::default()).first() {
            let bright = 1.0 + 10.0 * (1.0 - i as f32 / MAX_BOUNCES as f32);
            intersections.push((hit.position(), Color::BLACK.mix(&color, bright)));
            gizmos.sphere(
                hit.position(),
                Quat::IDENTITY,
                0.005,
                Color::BLACK.mix(&color, bright * 2.0),
            );
            let ray_dir = ray.direction;
            // reflect the ray
            let proj = (ray_dir.dot(hit.normal()) / hit.normal().dot(hit.normal())) * hit.normal();
            ray.direction = Dir3::new(*ray_dir - 2.0 * proj).unwrap();
            ray.origin = hit.position() + ray.direction * 1e-6;
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
        ..default()
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
    // Make a box of planes facing inward so the laser gets trapped inside:
    let plane = PbrBundle {
        mesh: meshes.add(Plane3d::default()),
        material: materials.add(Color::from(css::GRAY).with_alpha(0.01)),
        ..default()
    };
    let pbr_bundle = move |translation, rotation| PbrBundle {
        transform: Transform::from_translation(translation)
            .with_rotation(Quat::from_scaled_axis(rotation)),
        ..plane.clone()
    };
    commands.spawn(pbr_bundle(vec3(0.0, 0.5, 0.0), Vec3::X * PI));
    commands.spawn(pbr_bundle(vec3(0.0, -0.5, 0.0), Vec3::ZERO));
    commands.spawn(pbr_bundle(vec3(0.5, 0.0, 0.0), Vec3::Z * FRAC_PI_2));
    commands.spawn(pbr_bundle(vec3(-0.5, 0.0, 0.0), Vec3::Z * -FRAC_PI_2));
    commands.spawn(pbr_bundle(vec3(0.0, 0.0, 0.5), Vec3::X * -FRAC_PI_2));
    commands.spawn(pbr_bundle(vec3(0.0, 0.0, -0.5), Vec3::X * FRAC_PI_2));
}
