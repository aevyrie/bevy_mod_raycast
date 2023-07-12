use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*, window::PresentMode};

use bevy_mod_raycast::{
    DefaultPluginState, DefaultRaycastingPlugin, RaycastMesh, RaycastMethod, RaycastSource,
    RaycastSystem,
};

// This example will show you how to use your mouse cursor as a ray casting source, cast into the
// scene, intersect a mesh, and mark the intersection with the built in debug cursor. If you are
// looking for a more fully-featured mouse picking plugin, try out bevy_mod_picking.

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync, // Reduces input lag.
                ..default()
            }),
            ..default()
        }))
        // The DefaultRaycastingPlugin bundles all the functionality you might need into a single
        // plugin. This includes building rays, casting them, and placing a debug cursor at the
        // intersection. For more advanced uses, you can compose the systems in this plugin however
        // you need. For example, you might exclude the debug cursor system.
        .add_plugins(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        // You will need to pay attention to what order you add systems! Putting them in the wrong
        // order can result in multiple frames of latency. Ray casting should probably happen near
        // start of the frame. For example, we want to be sure this system runs before we construct
        // any rays, hence the ".before(...)". You can use these provided RaycastSystem labels to
        // order your systems with the ones provided by the raycasting plugin.
        .add_systems(
            First,
            update_raycast_with_cursor.before(RaycastSystem::BuildRays::<MyRaycastSet>),
        )
        .add_systems(Startup, setup)
        .run();
}

/// This is a unit struct we will use to mark our generic `RaycastMesh`s and `RaycastSource` as part
/// of the same group, or "RaycastSet". For more complex use cases, you might use this to associate
/// some meshes with one ray casting source, and other meshes with a different ray casting source."
#[derive(Clone, Reflect)]
struct MyRaycastSet;

// Update our `RaycastSource` with the current cursor position every frame.
fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    // Grab the most recent cursor event if it exists:
    let cursor_position = match cursor.iter().last() {
        Some(cursor_moved) => cursor_moved.position,
        None => return,
    };

    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(cursor_position);
    }
}

// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Overwrite the default plugin state with one that enables the debug cursor. This line can be
    // removed if the debug cursor isn't needed as the state is set to default values when the
    // default plugin is added.
    commands.insert_resource(DefaultPluginState::<MyRaycastSet>::default().with_debug_cursor());
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            tonemapping: Tonemapping::ReinhardLuminance,
            ..Default::default()
        })
        .insert(RaycastSource::<MyRaycastSet>::new()); // Designate the camera as our source
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::try_from(shape::Icosphere::default()).unwrap()),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            ..Default::default()
        })
        .insert(RaycastMesh::<MyRaycastSet>::default()); // Make this mesh ray cast-able
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}
