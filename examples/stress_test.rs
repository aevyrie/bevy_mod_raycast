use std::ops::Sub;

use bevy::{
    color::palettes::css,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    math::Vec3A,
    prelude::*,
    render::primitives::Aabb,
};

use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(bevy_mod_raycast::low_latency_window_plugin()),
            FrameTimeDiagnosticsPlugin,
            DeferredRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(First, update_status)
        .add_systems(Update, (update_fps, make_scene_pickable))
        .run();
}

// This is a unit struct we will use to mark our generic `RaycastMesh`s and `RaycastSource` as part
// of the same group, or "RaycastSet". For more complex use cases, you might use this to associate
// some meshes with one ray casting source, and other meshes with a different ray casting source."
#[derive(Reflect)]
struct MyRaycastSet;

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RaycastPluginState::<MyRaycastSet>::default().with_debug_cursor());
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 20.0, 20.0, 0.0)),
        DirectionalLight::default(),
    ));

    commands.spawn((
        Camera3d::default(),
        RaycastSource::<MyRaycastSet>::new_cursor(),
    ));

    let mut i = 0;
    for x in -2..=2 {
        for k in -210..-10 {
            commands.spawn((
                SceneRoot(asset_server.load("models/monkey/Monkey.gltf#Scene0")),
                Transform::from_translation(Vec3::new(
                    x as f32 * k as f32 * -2.0,
                    0.0,
                    k as f32 * 3.0,
                ))
                .with_scale(Vec3::splat((k as f32).abs().sub(5.0)) * 0.6),
            ));
            i += 1;
        }
    }
    info!("Raycasting against {i} meshes");
}

#[allow(clippy::type_complexity)]
fn make_scene_pickable(
    mut commands: Commands,
    mesh_query: Query<Entity, (With<Mesh3d>, Without<RaycastMesh<MyRaycastSet>>)>,
) {
    for entity in &mesh_query {
        commands
            .entity(entity)
            .insert(RaycastMesh::<MyRaycastSet>::default()); // Make this mesh ray cast-able
    }
}

// Set up UI to show status of bounding volume
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
                Text::new("Toggle with number keys"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE.into()),
            ));

            ui.spawn((
                Text::new("(1) AABB Culling: "),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE.into()),
            ))
            .with_child((TextSpan::new(""), BoundVolStatus));

            ui.spawn((
                Text::new("(2) Early Exit: "),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE.into()),
            ))
            .with_child((TextSpan::new(""), EarlyExitStatus));

            ui.spawn((
                Text::new("FPS: "),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE.into()),
            ))
            .with_child((TextSpan::new(""), FpsText));
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
    keyboard: Res<ButtonInput<KeyCode>>,
    mut enabled: Local<Option<(bool, bool)>>,
    // Bounding toggle
    mut bound_status: Query<
        (&mut TextSpan, &mut TextColor),
        (With<BoundVolStatus>, Without<EarlyExitStatus>),
    >,
    mut aabbs: Query<(Entity, &mut Aabb), With<RaycastMesh<MyRaycastSet>>>,
    // Early exit toggle
    mut exit_status: Query<
        (&mut TextSpan, &mut TextColor),
        (Without<BoundVolStatus>, With<EarlyExitStatus>),
    >,
    mut sources: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    if enabled.is_none() {
        *enabled = Some((true, true));
    }
    let enabled = enabled.as_mut().unwrap();

    let bool_to_text = |is_enabled: bool, status: (Mut<'_, TextSpan>, Mut<'_, TextColor>)| {
        let (mut text, mut color) = status;
        if is_enabled {
            text.0 = "ON".to_string();
            color.0 = css::GREEN.into();
        } else {
            text.0 = "OFF".to_string();
            color.0 = css::RED.into();
        }
    };

    if keyboard.just_pressed(KeyCode::Digit1) {
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
    bool_to_text(enabled.0, bound_status.single_mut());

    if keyboard.just_pressed(KeyCode::Digit2) {
        enabled.1 = !enabled.1;
        for mut source in &mut sources {
            source.should_early_exit = enabled.1;
        }
    }
    bool_to_text(enabled.1, exit_status.single_mut());
}

fn update_fps(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut TextSpan, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.smoothed() {
                // Update the value of the second section
                text.0 = format!("{:.2}", average);
            }
        }
    }
}
