//! This example demonstrates how to use the [`Raycast`] system param in queries to run raycasts
//! on-demand, in an immediate mode style. Unlike using a [`RaycastSource`]

use bevy::prelude::*;
use bevy_mod_raycast::{prelude::*, NoBackfaceCulling};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (immediate_mode_raycast, make_scene_pickable))
        .run();
}

const MAX_BOUNCES: usize = 16;

#[derive(Reflect, Clone)]
struct MyRaycastSet;

fn immediate_mode_raycast(raycast: Raycast<MyRaycastSet>, mut gizmos: Gizmos, time: Res<Time>) {
    let t = time.elapsed_seconds() * 0.1;
    let mut ray_pos = Vec3::new(t.sin(), (3.0 * t).cos() * 0.5, t.cos()) * 4.0;
    let mut ray_dir = (-ray_pos).normalize();
    gizmos.sphere(ray_pos, Quat::IDENTITY, 0.1, Color::YELLOW);

    for _ in 0..MAX_BOUNCES {
        let ray = Ray3d::new(ray_pos, ray_dir);
        if let Some((_, hit)) = raycast.cast_ray(ray, false).values().next() {
            gizmos.line(ray_pos, hit.position(), Color::RED);
            let proj = (ray_dir.dot(hit.normal()) / hit.normal().dot(hit.normal())) * hit.normal();
            ray_dir = (ray_dir - 2.0 * proj).normalize();
            ray_pos = hit.position() + ray_dir * 1e-6;
        } else {
            gizmos.ray(ray_pos, ray_dir, Color::RED);
            break;
        }
    }
}

// Set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RaycastPluginState::<MyRaycastSet>::default().with_debug_cursor());
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 20.0, 20.0, 0.0)),
        directional_light: DirectionalLight {
            illuminance: 5000.0,
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(7.0, 3.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        RaycastSource::<MyRaycastSet>::new(), // Designate the camera as our source
    ));

    commands.spawn((PbrBundle {
        mesh: meshes.add(shape::Cube::default().into()),
        material: materials.add(Color::GRAY.into()),
        ..default()
    },));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Cube::default().into()),
            material: materials.add(Color::rgba_u8(255, 255, 255, 100).into()),
            transform: Transform::from_scale(Vec3::splat(8.0)),
            ..default()
        },
        NoBackfaceCulling,
    ));
}

#[allow(clippy::type_complexity)]
fn make_scene_pickable(
    mut commands: Commands,
    mesh_query: Query<Entity, (With<Handle<Mesh>>, Without<RaycastMesh<MyRaycastSet>>)>,
) {
    for entity in &mesh_query {
        commands
            .entity(entity)
            .insert(RaycastMesh::<MyRaycastSet>::default()); // Make this mesh ray cast-able
    }
}
