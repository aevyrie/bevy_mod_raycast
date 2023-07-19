use bevy::prelude::*;
use bevy_mod_raycast::{
    DefaultPluginState, DefaultRaycastingPlugin, RaycastMesh, RaycastMethod, RaycastSource,
    RaycastSystem,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        .insert_resource(DefaultPluginState::<MyRaycastSet>::default().with_debug_cursor())
        .add_startup_system(setup)
        .add_system(
            update_raycast_with_cursor
                .in_base_set(CoreSet::First)
                .before(RaycastSystem::BuildRays::<MyRaycastSet>),
        )
        .add_system(print_intersection)
        .run();
}

#[derive(Reflect, Clone)]
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
                near: -20.,
                ..default()
            }),
            ..default()
        },
        RaycastSource::<MyRaycastSet>::new(),
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
        print!("Ray origin: {:?}", source.ray.map(|r| r.origin()));
        source.intersections().iter().for_each(|i| {
            print!(", Intersection: {:?}", i.1.position().z);
        });
        println!("");
    }
}
