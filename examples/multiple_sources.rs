//! This example will show you how to use multiple different ray cast sources.
//!
//! # Usage
//! - Move mouse: Move first ray cast.
//! - Left Click: Toggle plugins 'ActiveState'
//! - Right Click: Toggle plugins 'DebugState'
//! - Cursor Keys: Move second ray cast source.
//!
//! # Note
//! The 'debug' feature must be enabled (which it is by default) for this example to work correctly.

use bevy::prelude::*;

use bevy_mod_raycast::{
    ActiveState, DefaultRaycastingPlugin, PluginState, RayCastMesh, RayCastMethod, RayCastSource,
    RaycastSystem,
};
#[cfg(feature = "debug")]
use bevy_mod_raycast::DebugState;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            vsync: false, // We'll turn off vsync for this example, as it's a source of input lag.
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(DefaultRaycastingPlugin::<MyCursorRayCastSet>::default())
        .add_plugin(DefaultRaycastingPlugin::<MyKeyboardRayCastSet>::default())
        // You will need to pay attention to what order you add systems! Putting them in the wrong
        // order can result in multiple frames of latency. Ray casting should probably happen after
        // the positions of your meshes have been updated in the UPDATE stage.
        .add_system_to_stage(
            CoreStage::PreUpdate,
            update_raycast_with_cursor
                .system()
                .before(RaycastSystem::BuildRays),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            update_raycast_with_keyboard
                .system()
                .before(RaycastSystem::BuildRays),
        )
        .add_system(toggle_debug_cursor.system())
        .add_startup_system(setup.system())
        .run();
}

// This is a unit struct we will use to mark our generic `RayCastMesh`s and `RayCastSource` as part
// of the same group, or "RayCastSet". For more complex use cases, you might use this to associate
// some meshes with one ray casting source, and other meshes with a different ray casting source."
struct MyCursorRayCastSet;

struct MyKeyboardRayCastSet;

// Marker for our keyboard-controllable RayCastSource
#[derive(Default)]
struct MyKeyboardRayCastSource;

#[derive(Default)]
struct MyKeyboardRayCastTarget;

const ROTATION_SPEED: f32 = 5.0;

// Toggle ray cast states
fn toggle_debug_cursor(
    mut commands: Commands,
    mut cursor_ray_cast: ResMut<PluginState<MyCursorRayCastSet>>,
    mut keyboard_ray_cast: ResMut<PluginState<MyKeyboardRayCastSet>>,
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    source: Query<
        (Entity, Option<&RayCastSource<MyKeyboardRayCastSet>>),
        With<MyKeyboardRayCastSource>,
    >,
) {
    if buttons.just_pressed(MouseButton::Left) {
        if cursor_ray_cast.enabled == ActiveState::Enabled {
            println!("Ray casting disabled");
            cursor_ray_cast.enabled = ActiveState::Disabled;
            keyboard_ray_cast.enabled = ActiveState::Disabled;
        } else {
            println!("Ray casting enabled");
            cursor_ray_cast.enabled = ActiveState::Enabled;
            keyboard_ray_cast.enabled = ActiveState::Enabled;
        };
    }

    #[cfg(feature = "debug")]
    if buttons.just_pressed(MouseButton::Right) {
        if cursor_ray_cast.debug == DebugState::Cursor {
            println!("Debug cursor disabled");
            cursor_ray_cast.debug = DebugState::None;
            keyboard_ray_cast.debug = DebugState::None;
        } else {
            println!("Debug cursor enabled");
            cursor_ray_cast.debug = DebugState::Cursor;
            keyboard_ray_cast.debug = DebugState::Cursor;
        };
    }

    if keys.just_pressed(KeyCode::Space) {
        if let Ok((entity, source)) = source.single() {
            if let Some(_) = source {
                commands
                    .entity(entity)
                    .remove::<RayCastSource<MyKeyboardRayCastSet>>();
            } else {
                commands
                    .entity(entity)
                    .insert(RayCastSource::<MyKeyboardRayCastSet>::new_transform_empty());
            }
        }
    }
}

fn update_raycast_with_keyboard(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<MyKeyboardRayCastTarget>>,
) {
    let mut rotation_degrees = Vec3::ZERO;

    if input.pressed(KeyCode::Left) {
        rotation_degrees += Vec3::new(0.0, -ROTATION_SPEED, 0.0);
    }
    if input.pressed(KeyCode::Right) {
        rotation_degrees += Vec3::new(0.0, ROTATION_SPEED, 0.0);
    }
    if input.pressed(KeyCode::Up) {
        rotation_degrees += Vec3::new(-ROTATION_SPEED, 0.0, 0.0);
    }
    if input.pressed(KeyCode::Down) {
        rotation_degrees += Vec3::new(ROTATION_SPEED, 0.0, 0.0);
    }

    if rotation_degrees == Vec3::ZERO {
        return;
    }

    for mut pick_source in &mut query.iter_mut() {
        pick_source.rotate(Quat::from_rotation_x(
            rotation_degrees.normalize().x * time.delta_seconds(),
        ));
        pick_source.rotate(Quat::from_rotation_y(
            rotation_degrees.normalize().y * time.delta_seconds(),
        ));
    }
}

// Update our `RayCastSource` with the current cursor position every frame.
fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RayCastSource<MyCursorRayCastSet>>,
) {
    for mut pick_source in &mut query.iter_mut() {
        // Grab the most recent cursor event if it exists:
        if let Some(cursor_latest) = cursor.iter().last() {
            pick_source.cast_method = RayCastMethod::Screenspace(cursor_latest.position);
        }
    }
}

// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(PluginState::<MyCursorRayCastSet>::default());
    commands.insert_resource(PluginState::<MyKeyboardRayCastSet>::default());
    commands
        .spawn_bundle(PerspectiveCameraBundle::default())
        .insert(RayCastSource::<MyCursorRayCastSet>::new()); // Designate the camera as our source
    // Create a new RayCastSource which rotates around the sphere
    commands
        .spawn()
        .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)))
        .insert(GlobalTransform::default())
        .insert(MyKeyboardRayCastTarget::default())
        .with_children(|parent| {
            parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Icosphere {
                        radius: 0.1,
                        subdivisions: 5,
                    })),
                    material: materials.add(Color::rgba(1.0, 0.0, 0.0, 0.3).into()),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                    ..Default::default()
                })
                .insert(MyKeyboardRayCastSource::default())
                .insert(RayCastSource::<MyKeyboardRayCastSet>::new_transform_empty());
        });
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 1.0,
                subdivisions: 3,
            })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
            ..Default::default()
        })
        .insert(RayCastMesh::<MyCursorRayCastSet>::default()) // Make this mesh ray cast-able
        .insert(RayCastMesh::<MyKeyboardRayCastSet>::default());
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}
