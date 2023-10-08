//! This example will show you how to setup bounding volume to optimize when raycasting over a scene
//! with many meshes. The bounding volume will be used to check faster for which mesh to actually
//! raycast on.

use std::ops::Sub;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    math::Vec3A,
    prelude::*,
    render::primitives::Aabb,
};

use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(low_latency_window_plugin()),
            FrameTimeDiagnosticsPlugin,
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(
            First,
            (
                update_status,
                update_raycast_pos.before(RaycastSystem::BuildRays::<MyRaycastSet>),
            ),
        )
        .add_systems(Update, (update_fps, make_scene_pickable))
        .run();
}

// This is a unit struct we will use to mark our generic `RaycastMesh`s and `RaycastSource` as part
// of the same group, or "RaycastSet". For more complex use cases, you might use this to associate
// some meshes with one ray casting source, and other meshes with a different ray casting source."
#[derive(Reflect)]
struct MyRaycastSet;

// Update our `RaycastSource` with the current cursor position every frame.
fn update_raycast_pos(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    for mut pick_source in &mut query {
        if let Some(cursor_latest) = cursor.iter().last() {
            pick_source.cast_method = RaycastMethod::Screenspace(cursor_latest.position);
        }
    }
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
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
        Camera3dBundle::default(),
        RaycastSource::<MyRaycastSet>::default(), // Camera as source
    ));

    let mut i = 0;
    for x in -2..=2 {
        for k in -210..-10 {
            commands.spawn((bevy::prelude::SceneBundle {
                scene: asset_server.load("models/monkey/Monkey.gltf#Scene0"),
                transform: Transform::from_translation(Vec3::new(
                    x as f32 * k as f32 * -2.0,
                    0.0,
                    k as f32 * 3.0,
                ))
                .with_scale(Vec3::splat((k as f32).abs().sub(5.0)) * 0.6),
                ..default()
            },));
            i += 1;
        }
    }
    info!("Raycasting against {i} meshes");
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
fn setup_ui(mut commands: Commands) {
    let text_section = |text: &'static str| TextSection {
        value: text.into(),
        style: TextStyle {
            font_size: 40.0,
            color: Color::WHITE,
            ..default()
        },
    };

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
            ui.spawn(TextBundle::from_sections([text_section(
                "Toggle with number keys",
            )]));
            ui.spawn(TextBundle::from_sections([
                text_section("(1) AABB Culling: "),
                text_section(""),
            ]))
            .insert(BoundVolStatus);
            ui.spawn(TextBundle::from_sections([
                text_section("(2) Early Exit: "),
                text_section(""),
            ]))
            .insert(EarlyExitStatus);
            ui.spawn(TextBundle::from_sections([
                text_section("FPS: "),
                text_section(""),
            ]))
            .insert(FpsText);
        });
}

#[derive(Component)]
struct BoundVolStatus;

#[derive(Component)]
struct EarlyExitStatus;

#[derive(Component)]
struct FpsText;

// Insert or remove aabb components from the meshes being raycasted on.
fn update_status(
    mut commands: Commands,
    keyboard: Res<Input<KeyCode>>,
    mut enabled: Local<Option<(bool, bool)>>,
    // Bounding toggle
    mut bound_status: Query<&mut Text, (With<BoundVolStatus>, Without<EarlyExitStatus>)>,
    mut aabbs: Query<(Entity, &mut Aabb), With<RaycastMesh<MyRaycastSet>>>,
    // Early exit toggle
    mut exit_status: Query<&mut Text, (Without<BoundVolStatus>, With<EarlyExitStatus>)>,
    mut sources: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    if enabled.is_none() {
        *enabled = Some((true, true));
    }
    let enabled = enabled.as_mut().unwrap();

    let bool_to_text = |is_enabled: bool, text: &mut Text| {
        if is_enabled {
            text.sections[1].value = "ON".to_string();
            text.sections[1].style.color = Color::GREEN;
        } else {
            text.sections[1].value = "OFF".to_string();
            text.sections[1].style.color = Color::RED;
        }
    };

    if keyboard.just_pressed(KeyCode::Key1) {
        enabled.0 = !enabled.0;
        for (entity, mut aabb) in &mut aabbs {
            if enabled.0 {
                // bevy's built in systems will see that the Aabb is missing and make a valid one
                commands.entity(entity).remove::<Aabb>();
            } else {
                // infinite AABB to make AABB useless
                aabb.half_extents = Vec3A::ONE * f32::MAX;
            }
        }
    }
    bool_to_text(enabled.0, bound_status.single_mut().as_mut());

    if keyboard.just_pressed(KeyCode::Key2) {
        enabled.1 = !enabled.1;
        for mut source in &mut sources {
            source.should_early_exit = enabled.1;
        }
    }
    bool_to_text(enabled.1, exit_status.single_mut().as_mut());
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
