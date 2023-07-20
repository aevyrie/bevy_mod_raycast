//! This example demonstrates how to use the [`Raycast`] system param in queries to run raycasts
//! on-demand, in an immediate mode style. Unlike using a [`RaycastSource`]

use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, immediate_mode_raycast)
        .run();
}

const MAX_BOUNCES: usize = 128;
const LASER_MOVE_SPEED: f32 = 0.05;

#[derive(Reflect, Clone)]
struct MyRaycastSet;

fn immediate_mode_raycast(raycast: Raycast<MyRaycastSet>, mut gizmos: Gizmos, time: Res<Time>) {
    let t = (time.elapsed_seconds() * LASER_MOVE_SPEED).sin() * std::f32::consts::PI;
    let mut ray_pos = Vec3::new(t.sin(), (3.0 * t).cos() * 0.5, t.cos()) * 3.0;
    let mut ray_dir = (-ray_pos).normalize();
    gizmos.sphere(ray_pos, Quat::IDENTITY, 0.1, Color::YELLOW);

    let mut intersections = Vec::with_capacity(MAX_BOUNCES + 1);
    intersections.push((ray_pos, Color::RED));

    for i in 0..MAX_BOUNCES {
        let ray = Ray3d::new(ray_pos, ray_dir);
        if let Some((_, hit)) = raycast.cast_ray(ray, false).values().next() {
            intersections.push((
                hit.position(),
                Color::rgba(1.0, 0.0, 0.0, 1.0 - i as f32 / MAX_BOUNCES as f32),
            ));
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
    commands.insert_resource(RaycastPluginState::<MyRaycastSet>::default().with_debug_cursor());
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 1.0, 1.0)),
        ..Default::default()
    });
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(7.0, 5.0, 9.0).looking_at(Vec3::Y * -1.0, Vec3::Y),
            ..default()
        },
        RaycastSource::<MyRaycastSet>::new(), // Designate the camera as our source
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::WHITE.with_a(0.1).into()),
            transform: Transform::from_scale(Vec3::splat(6.0)),
            ..default()
        },
        RaycastMesh::<MyRaycastSet>::default(),
        bevy_mod_raycast::NoBackfaceCulling,
    ));
}
