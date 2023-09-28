//! This example will show you how to use a simplified mesh to improve performance when raycasting
//! over a scene with a complicated mesh. The simplified mesh will be used to check faster for
//! intersection with the mesh.

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(low_latency_window_plugin()),
            FrameTimeDiagnosticsPlugin,
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        // You will need to pay attention to what order you add systems! Putting them in the wrong
        // order can result in multiple frames of latency.
        .add_systems(
            First,
            update_raycast_with_cursor_position.before(RaycastSystem::BuildRays::<MyRaycastSet>),
        )
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(Update, (update_fps, manage_simplified_mesh))
        .run();
}

// This is a unit struct we will use to mark our generic `RaycastMesh`s and `RaycastSource` as part
// of the same group, or "RaycastSet". For more complex use cases, you might use this to associate
// some meshes with one ray casting source, and other meshes with a different ray casting source."
#[derive(Reflect)]
struct MyRaycastSet;

// Update our `RaycastSource` with the current cursor position every frame.
fn update_raycast_with_cursor_position(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    for mut pick_source in &mut query {
        // Grab the most recent cursor event if it exists:
        if let Some(cursor_latest) = cursor.iter().last() {
            pick_source.cast_method = RaycastMethod::Screenspace(cursor_latest.position);
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
    commands
        .spawn(Camera3dBundle {
            tonemapping: Tonemapping::ReinhardLuminance,
            ..default()
        })
        .insert(RaycastSource::<MyRaycastSet>::new()); // Designate the camera as our source
    commands.spawn((
        PbrBundle {
            // This is a very complex mesh that will be hard to raycast on
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 1.0,
                sectors: 1000,
                stacks: 1000,
            })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
            ..Default::default()
        },
        SimplifiedMesh {
            mesh: meshes.add(Mesh::from(shape::UVSphere::default())),
        },
        RaycastMesh::<MyRaycastSet>::default(), // Make this mesh ray cast-able
    ));
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}

// Set up UI to show status of simplified mesh
fn setup_ui(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_self: AlignSelf::FlexStart,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background_color: Color::NONE.into(),
            ..Default::default()
        })
        .with_children(|ui| {
            ui.spawn(TextBundle {
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "Press spacebar to toggle - FPS: ".to_string(),
                            style: TextStyle {
                                font_size: 30.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        },
                        TextSection {
                            value: "".to_string(),
                            style: TextStyle {
                                font_size: 30.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(FpsText);

            ui.spawn(TextBundle {
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "Simplified Mesh: ".to_string(),
                            style: TextStyle {
                                font_size: 30.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        },
                        TextSection {
                            value: "ON".to_string(),
                            style: TextStyle {
                                font_size: 30.0,
                                color: Color::GREEN,
                                ..default()
                            },
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(SimplifiedStatus);
        });
}

#[derive(Component)]
struct SimplifiedStatus;

#[derive(Component)]
struct FpsText;

// Insert or remove SimplifiedMesh component from the mesh being raycasted on.
fn manage_simplified_mesh(
    mut commands: Commands,
    query: Query<(Entity, Option<&SimplifiedMesh>), With<RaycastMesh<MyRaycastSet>>>,
    mut status_query: Query<&mut Text, With<SimplifiedStatus>>,
    keyboard: Res<Input<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        if let Ok((entity, ray)) = query.get_single() {
            if let Ok(mut text) = status_query.get_single_mut() {
                if ray.is_none() {
                    commands.entity(entity).insert(SimplifiedMesh {
                        mesh: meshes.add(Mesh::from(shape::UVSphere::default())),
                    });
                    text.sections[1].value = "ON".to_string();
                    text.sections[1].style.color = Color::GREEN;
                } else {
                    commands.entity(entity).remove::<SimplifiedMesh>();
                    text.sections[1].value = "OFF".to_string();
                    text.sections[1].style.color = Color::RED;
                }
            }
        }
    }
}

fn update_fps(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}
