use bevy::{color::palettes::css, prelude::*, sprite::MaterialMesh2dBundle};
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
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2dBundle::default(), RaycastSource::<()>::new_cursor()));
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Circle::default()).into(),
            transform: Transform::default().with_scale(Vec3::splat(128.)),
            material: materials.add(ColorMaterial::from(Color::from(css::PURPLE))),
            ..default()
        },
        RaycastMesh::<()>::default(), // Make this mesh ray cast-able;
    ));
}
