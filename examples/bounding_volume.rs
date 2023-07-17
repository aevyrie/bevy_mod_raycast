use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    math::Vec3A,
    prelude::*,
    render::primitives::Aabb,
    window::PresentMode,
};

use bevy_mod_raycast::{
    DefaultPluginState, DefaultRaycastingPlugin, RaycastMesh, RaycastMethod, RaycastSource,
    RaycastSystem,
};

// This example will show you how to setup bounding volume to optimize when raycasting over a
// scene with many meshes. The bounding volume will be used to check faster for which mesh
// to actually raycast on.

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        // You will need to pay attention to what order you add systems! Putting them in the wrong
        // order can result in multiple frames of latency. Ray casting should probably happen after
        // the positions of your meshes have been updated in the UPDATE stage.
        .add_systems(
            First,
            update_raycast_with_cursor.before(RaycastSystem::BuildRays::<MyRaycastSet>),
        )
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(Update, (update_fps, make_scene_pickable))
        .add_systems(First, manage_aabb)
        .run();
}

// This is a unit struct we will use to mark our generic `RaycastMesh`s and `RaycastSource` as part
// of the same group, or "RaycastSet". For more complex use cases, you might use this to associate
// some meshes with one ray casting source, and other meshes with a different ray casting source."
#[derive(Clone, Reflect)]
struct MyRaycastSet;

// Update our `RaycastSource` with the current cursor position every frame.
fn update_raycast_with_cursor(
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
fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(DefaultPluginState::<MyRaycastSet>::default().with_debug_cursor());
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 20.0, 20.0, 0.0)),
        directional_light: DirectionalLight {
            illuminance: 5000.0,
            ..Default::default()
        },
        ..Default::default()
    });

    commands
        .spawn(Camera3dBundle {
            tonemapping: Tonemapping::ReinhardLuminance,
            ..default()
        })
        .insert(RaycastSource::<MyRaycastSet>::new()); // Designate the camera as our source

    // Spawn multiple mesh to raycast on
    let n = 8;
    for i in -n..=n {
        for j in -n..=n {
            for k in -n..=n {
                commands.spawn((bevy::prelude::SceneBundle {
                    scene: asset_server.load("models/monkey/Monkey.gltf#Scene0"),
                    transform: Transform::from_translation(Vec3::new(
                        i as f32 * 3.0,
                        j as f32 * 3.0,
                        k as f32 * 3.0 - n as f32 * 4.0,
                    )),
                    ..default()
                },));
            }
        }
    }
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

// Set up UI to show status of bounding volume
fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
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
                                font: font.clone(),
                                font_size: 40.0,
                                color: Color::WHITE,
                            },
                        },
                        TextSection {
                            value: "".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: 40.0,
                                color: Color::WHITE,
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
                            value: "AABB Culling: ".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: 40.0,
                                color: Color::WHITE,
                            },
                        },
                        TextSection {
                            value: "ON".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: 40.0,
                                color: Color::GREEN,
                            },
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(BoundVolStatus);
        });
}

#[derive(Component)]
struct BoundVolStatus;

#[derive(Component)]
struct FpsText;

struct Enabled(bool);

impl Default for Enabled {
    fn default() -> Self {
        Enabled(true)
    }
}

// Insert or remove aabb components from the meshes being raycasted on.
fn manage_aabb(
    mut commands: Commands,
    mut enabled: Local<Enabled>,
    mut query: Query<(Entity, &mut Aabb), With<RaycastMesh<MyRaycastSet>>>,
    mut status_query: Query<&mut Text, With<BoundVolStatus>>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        enabled.0 = !enabled.0;
        if let Ok(mut text) = status_query.get_single_mut() {
            if enabled.0 {
                text.sections[1].value = "ON".to_string();
                text.sections[1].style.color = Color::GREEN;
            } else {
                text.sections[1].value = "OFF".to_string();
                text.sections[1].style.color = Color::RED;
            }
        }

        for (entity, mut aabb) in &mut query {
            if enabled.0 {
                commands.entity(entity).remove::<Aabb>();
            } else {
                aabb.half_extents = Vec3A::ONE * f32::MAX;
            }
        }
    }
}

fn update_fps(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.smoothed() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}
