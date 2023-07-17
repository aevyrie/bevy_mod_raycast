use bevy::prelude::*;
use bevy_mod_raycast::{DefaultPluginState, DefaultRaycastingPlugin, RaycastMesh, RaycastSource};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        .insert_resource(DefaultPluginState::<MyRaycastSet>::default().with_debug_cursor())
        .add_startup_system(setup)
        .add_system(move_sphere)
        .add_system(print_intersection)
        .run();
}

#[derive(Reflect, Clone)]
struct MyRaycastSet;

// Set up a simple scene with a sphere, camera, and light.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 0.01,
                near: -10.,
                ..default()
            }),
            ..default()
        },
        RaycastSource::<MyRaycastSet>::new_transform_empty(),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::try_from(shape::Icosphere::default()).unwrap()),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        },
        RaycastMesh::<MyRaycastSet>::default(), // Make this mesh ray cast-able
    ));
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_axis_angle(Vec3::splat(0.5), 1.5)),
        ..default()
    });
}

fn print_intersection(query: Query<&RaycastSource<MyRaycastSet>>) {
    for source in &query {
        print!("Ray Z: {:?}", source.ray.map(|r| r.origin().z));
        source.intersections().iter().for_each(|i| {
            print!(", Intersection: {:?}", i.1.position().z);
        });
        println!("");
    }
}

fn move_sphere(time: Res<Time>, mut query: Query<&mut Transform, With<RaycastMesh<MyRaycastSet>>>) {
    for mut transform in &mut query {
        let s = (time.elapsed_seconds() * 0.5).sin();
        *transform = Transform::from_xyz(0.0, 0.0, s * 10.0);
    }
}
