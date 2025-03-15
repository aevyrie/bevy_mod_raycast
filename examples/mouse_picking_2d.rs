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
        .add_systems(Update, print_intersections::<()>)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((Camera2d::default(), RaycastSource::<()>::new_cursor()));
    commands.spawn((
        Mesh3d(meshes.add(Circle::default()).into()),
        MeshMaterial3d(materials.add(Color::from(css::PURPLE))),
        Transform::default().with_scale(Vec3::splat(128.)),
        RaycastMesh::<()>::default(), // Make this mesh ray cast-able;
    ));
}
