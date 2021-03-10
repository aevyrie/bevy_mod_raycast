use bevy::prelude::*;
use bevy_mod_raycast::*;

// This example will show you how to use your mouse cursor as a ray casting source, cast into the
// scene, intersect a mesh, and mark the intersection with the built in debug cursor. If you are
// looking for a more fully-featured mouse picking plugin, try out bevy_mod_picking.

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            vsync: false, // We'll turn off vsync for this example, as it's a source of input lag.
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .init_resource::<PluginState<MyRaycastSet>>() // We can use this state to enable/disable picking
        // You will need to pay attention to what order you add systems! Putting them in the wrong
        // order can result in multiple frames of latency. Ray casting should probably happen after
        // the positions of your meshes have been updated in the UPDATE stage.
        .add_system(update_raycast_with_cursor.system()) // Update our ray casting source in the UPDATE stage
        .add_system_to_stage(
            stage::POST_UPDATE, // We want this system to run after we've updated our ray casting source
            update_raycast::<MyRaycastSet>.system(), // This provided system does the ray casting
        )
        .add_system_to_stage(
            stage::POST_UPDATE, // We want this system to run after ray casting has been computed
            update_debug_cursor::<MyRaycastSet>.system(), // Update the debug cursor location
        )
        .add_startup_system(setup.system())
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
fn setup(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
        .with(RayCastSource::<MyRaycastSet>::new()) // Designate the camera as our source
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 20,
                radius: 2.0,
            })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::ZERO),
            ..Default::default()
        })
        .with(RayCastMesh::<MyRaycastSet>::default()) // Make this mesh ray cast-able
        .spawn(LightBundle {
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        });
}
