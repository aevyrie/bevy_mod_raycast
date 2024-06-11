//! This example is similar to `mouse_picking` which uses the immediate mode `Raycast` system
//! parameter. By contrast this example instead uses the deferred API, where raycasts are declared
//! using components, and the plugin handles the raycasting. Note we use `()` as the raycasting set.

use bevy::{color::palettes::css, prelude::*};
use bevy_mod_raycast::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(bevy_mod_raycast::low_latency_window_plugin()),
            DeferredRaycastingPlugin::<()>::default(),
        ))
        .insert_resource(RaycastPluginState::<()>::default().with_debug_cursor())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3dBundle::default(),
        RaycastSource::<()>::new_cursor(), // Set this camera as a raycaster using the mouse cursor
    ));
    commands.spawn(PointLightBundle::default());
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::default()),
            material: materials.add(Color::from(css::GRAY)),
            transform: Transform::from_xyz(0.0, 0.0, -5.0),
            ..default()
        },
        RaycastMesh::<()>::default(), // Make this mesh ray cast-able;
    ));
}
