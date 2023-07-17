use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*};
use bevy_mod_raycast::{
    DefaultPluginState, DefaultRaycastingPlugin, Intersection, RaycastMesh, RaycastSource,
};

// This example casts a ray from the camera using its transform, intersects a mesh, displays
// the debug cursor at the intersection, and reports the intersection.
//
// It also demonstrates how normals are interpolated. Notice the debug cursor doesn't snap to the
// faces of the low-poly sphere's faces, but smoothly interpolates using the mesh's normals.

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotator, intersection))
        .run();
}

// Mark our generic `RaycastMesh`s and `RaycastSource`s as part of the same "RaycastSet". This
// plugin uses generics to distinguish between groups of raycasters.
#[derive(Reflect, Clone)]
struct MyRaycastSet;

// Set up a simple scene with a sphere, camera, and light.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Overwrite the default plugin state with one that enables the debug cursor. This line can be
    // removed if the debug cursor isn't needed as the state is set to default values when the
    // default plugin is added.
    commands.insert_resource(DefaultPluginState::<MyRaycastSet>::default().with_debug_cursor());
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 0.01,
                ..default()
            }),
            tonemapping: Tonemapping::ReinhardLuminance,
            ..default()
        },
        // Designate the camera as our ray casting source. Using `new_transform_empty()` means that
        // the ray casting source will not be initialized with a valid ray. Instead, a ray will be
        // calculated the first time the update_raycast system runs. Because we are setting this as
        // a RaycastMethod::Transform source, the update_raycast system will look for a
        // GlobalTransform on the camera entity, and build a ray using this transform. In this
        // example, this means that as the camera rotates in the scene, the update_raycast system
        // will build a valid ray every frame using the camera's updated position.
        RaycastSource::<MyRaycastSet>::new_transform_empty(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::try_from(shape::Icosphere::default()).unwrap()),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
            ..Default::default()
        },
        RaycastMesh::<MyRaycastSet>::default(), // Make this mesh ray cast-able
    ));
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}

/// Report intersections
fn intersection(query: Query<&Intersection<MyRaycastSet>>) {
    for intersection in &query {
        info!(
            "Distance {:?}, Position {:?}",
            intersection.distance(),
            intersection.position()
        );
    }
}

/// Rotate the camera up and down to show that the raycast intersection is updated every frame.
fn rotator(time: Res<Time>, mut query: Query<&mut Transform, With<RaycastSource<MyRaycastSet>>>) {
    for mut transform in &mut query {
        *transform = Transform::from_rotation(
            Quat::from_rotation_x(time.elapsed_seconds().sin() * 0.2)
                * Quat::from_rotation_y((time.elapsed_seconds() * 1.5).sin() * 0.1),
        );
    }
}
