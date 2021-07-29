use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_mod_raycast::{
    DefaultRaycastingPlugin, RayCastMesh, RayCastMethod, RayCastSource, RaycastSystem,
    SimplifiedMesh,
};

// This example will show you how to setup simplified mesh to optimise when raycasting over a
// scene with a complicated mesh. The simplified mesh will be used to check faster for intersection
// with the mesh.

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            vsync: false, // We'll turn off vsync for this example, as it's a source of input lag.
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        // You will need to pay attention to what order you add systems! Putting them in the wrong
        // order can result in multiple frames of latency. Ray casting should probably happen after
        // the positions of your meshes have been updated in the UPDATE stage.
        .add_system_to_stage(
            CoreStage::PreUpdate,
            update_raycast_with_cursor
                .system()
                .before(RaycastSystem::BuildRays),
        )
        .add_startup_system(setup_scene.system())
        .add_startup_system(setup_ui.system())
        .add_system(update_fps.system())
        .add_system(manage_simplified_mesh.system())
        .run();
}

// This is a unit struct we will use to mark our generic `RayCastMesh`s and `RayCastSource` as part
// of the same group, or "RayCastSet". For more complex use cases, you might use this to associate
// some meshes with one ray casting source, and other meshes with a different ray casting source."
struct MyRaycastSet;

// Update our `RayCastSource` with the current cursor position every frame.
fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RayCastSource<MyRaycastSet>>,
) {
    for mut pick_source in &mut query.iter_mut() {
        // Grab the most recent cursor event if it exists:
        if let Some(cursor_latest) = cursor.iter().last() {
            pick_source.cast_method = RayCastMethod::Screenspace(cursor_latest.position);
        }
    }
}

// Set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PerspectiveCameraBundle::default())
        .insert(RayCastSource::<MyRaycastSet>::new()); // Designate the camera as our source
    commands
        .spawn_bundle(PbrBundle {
            // This is a very complex mesh that will be hard to raycast on
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 1.0,
                sectors: 1000,
                stacks: 1000,
            })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
            ..Default::default()
        })
        .insert(RayCastMesh::<MyRaycastSet>::default()); // Make this mesh ray cast-able
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}

// Set up UI to show status of simplified mesh
fn setup_ui(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(UiCameraBundle::default());
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                align_self: AlignSelf::FlexStart,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|ui| {
            ui.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "Press spacebar to toggle - FPS: ".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        },
                        TextSection {
                            value: "".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(FpsText);
            ui.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![
                        TextSection {
                            value: "Simplified Mesh: ".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        },
                        TextSection {
                            value: "OFF".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: 30.0,
                                color: Color::RED,
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

struct SimplifiedStatus;

struct FpsText;

// Insert or remove SimplifiedMesh component from the mesh being raycasted on.
fn manage_simplified_mesh(
    mut commands: Commands,
    query: Query<(Entity, Option<&SimplifiedMesh>), With<RayCastMesh<MyRaycastSet>>>,
    mut status_query: Query<&mut Text, With<SimplifiedStatus>>,
    keyboard: Res<Input<KeyCode>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        if let Ok((entity, ray)) = query.single() {
            if let Ok(mut text) = status_query.single_mut() {
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

fn update_fps(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}
