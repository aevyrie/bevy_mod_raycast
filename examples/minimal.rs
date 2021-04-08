use bevy::prelude::*;
use bevy_mod_raycast::{DefaultRaycastingPlugin, RayCastMesh, RayCastSource};

// This example casts a ray from the camera using its transform, intersecting a mesh, and displays
// the debug cursor at the intersection.

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        .add_startup_system(setup.system())
        .add_system(rotator.system())
        .run();
}

// Mark our generic `RayCastMesh`s and `RayCastSource`s as part of the same group, or "RayCastSet".
struct MyRaycastSet;

// Set up a simple scene with a sphere, camera, and light.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PerspectiveCameraBundle::default())
        // Designate the camera as our ray casting source. Using `new_transform_empty()` means that
        // the ray casting source will not be initialized with a valid ray. Instead, a ray will be
        // calculated the first time the update_raycast system runs. Because we are setting this as
        // a RayCastMethod::Transform source, the update_raycast system will look for a
        // GlobalTransform on the camera entity, and build a ray using this transform. In this
        // example, this means that as the camera rotates in the scene, the update_raycast system
        // will build a valid ray every frame using the camera's updated position.
        .insert(RayCastSource::<MyRaycastSet>::new_transform_empty());
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere::default())),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
            ..Default::default()
        })
        .insert(RayCastMesh::<MyRaycastSet>::default()); // Make this mesh ray cast-able
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}

// Rotate the camera up and down to show that the raycast intersection is updated every frame.
fn rotator(time: Res<Time>, mut query: Query<&mut Transform, With<RayCastSource<MyRaycastSet>>>) {
    for mut transform in query.iter_mut() {
        *transform = Transform::from_rotation(Quat::from_rotation_x(
            time.seconds_since_startup().sin() as f32 * 0.2,
        ));
    }
}
