//! This example will show you how to use a simplified mesh to improve performance when raycasting
//! over a scene with a complicated mesh. The simplified mesh will be used to check faster for
//! intersection with the mesh.

use bevy::{
    color::palettes::css,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(bevy_mod_raycast::low_latency_window_plugin()),
            CursorRayPlugin,
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(Update, (raycast, update_fps, manage_simplified_mesh))
        .run();
}

fn raycast(cursor_ray: Res<CursorRay>, mut raycast: Raycast, mut gizmos: Gizmos) {
    if let Some(ray) = **cursor_ray {
        raycast.debug_cast_ray(ray, &default(), &mut gizmos);
    }
}

// Set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3d::default());
    commands.spawn((
        // This is a very complex mesh that will be hard to raycast on
        Mesh3d(meshes.add(Sphere::default().mesh().uv(1000, 1000))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 1.0, 1.0))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
        SimplifiedMesh {
            mesh: meshes.add(Sphere::default()),
        },
    ));
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
    ));
}

// Set up UI to show status of simplified mesh
fn setup_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                align_self: AlignSelf::FlexStart,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE.into()),
        ))
        .with_children(|ui| {
            ui.spawn((
                Text::new("Press spacebar to toggle - FPS: "),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE.into()),
            ))
            .with_child((FpsText, TextSpan::new("")));

            ui.spawn((
                Text::new("Simplified Mesh: "),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE.into()),
            ))
            .with_child((
                SimplifiedStatus,
                TextSpan::new("ON"),
                TextColor(css::GREEN.into()),
            ));
        });
}

#[derive(Component)]
struct SimplifiedStatus;

#[derive(Component)]
struct FpsText;

// Insert or remove SimplifiedMesh component from the mesh being raycasted on.
fn manage_simplified_mesh(
    mut commands: Commands,
    query: Query<(Entity, Option<&SimplifiedMesh>), With<Mesh3d>>,
    mut status_query: Query<(&mut TextSpan, &mut TextColor), With<SimplifiedStatus>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        if let Ok((entity, simplified_mesh)) = query.get_single() {
            if let Ok((mut text, mut color)) = status_query.get_single_mut() {
                if simplified_mesh.is_none() {
                    commands.entity(entity).insert(SimplifiedMesh {
                        mesh: meshes.add(Sphere::default()),
                    });
                    text.0 = "ON".to_string();
                    color.0 = css::GREEN.into();
                } else {
                    commands.entity(entity).remove::<SimplifiedMesh>();
                    text.0 = "OFF".to_string();
                    color.0 = css::RED.into();
                }
            }
        }
    }
}

fn update_fps(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut TextSpan, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.0 = format!("{:.2}", average);
            }
        }
    }
}
