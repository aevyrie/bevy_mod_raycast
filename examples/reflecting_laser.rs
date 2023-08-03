//! This example demonstrates how to use the [`Raycast`] system param to chain multiple raycasts and
//! bounce off of surfaces.

use bevy::prelude::*;
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, DefaultRaycastingPlugin::<Laser>::default()))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, bouncing_raycast)
        .run();
}

const MAX_BOUNCES: usize = 128;
const LASER_MOVE_SPEED: f32 = 0.005;

#[derive(Reflect)]
struct Laser;

fn bouncing_raycast(raycast: Raycast<Laser>, mut gizmos: Gizmos, time: Res<Time>) {
    let t = (time.elapsed_seconds() * LASER_MOVE_SPEED).sin() * std::f32::consts::PI;
    let mut ray_pos = Vec3::new(t.sin(), (3.0 * t).cos() * 0.5, t.cos()) * 3.0;
    let mut ray_dir = (-ray_pos).normalize();
    gizmos.sphere(ray_pos, Quat::IDENTITY, 0.1, Color::YELLOW);

    let mut intersections = Vec::with_capacity(MAX_BOUNCES + 1);
    intersections.push((ray_pos, Color::RED));

    for i in 0..MAX_BOUNCES {
        let ray = Ray3d::new(ray_pos, ray_dir);
        if let Some((_, hit)) = raycast.cast_ray(ray, false).first() {
            let a = 0.2 + 0.8 * (1.0 - i as f32 / MAX_BOUNCES as f32);
            let color = Color::rgba(1.0, 0.0, 0.0, a);
            intersections.push((hit.position(), color));
            gizmos.sphere(hit.position(), Quat::IDENTITY, 0.02, color);
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
    commands.insert_resource(RaycastPluginState::<Laser>::default().with_debug_cursor());
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 1.0, 1.0, 1.0)),
        ..Default::default()
    });
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(7.0, 5.0, 9.0).looking_at(Vec3::Y * -1.0, Vec3::Y),
            ..default()
        },
        RaycastSource::<Laser>::new(), // Designate the camera as our source
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::WHITE.with_a(0.1).into()),
            transform: Transform::from_scale(Vec3::splat(6.0)),
            ..default()
        },
        RaycastMesh::<Laser>::default(),
        // Without this, raycasts would shoot straight out from the inside of the cube.
        bevy_mod_raycast::NoBackfaceCulling,
    ));
}
