use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_mod_raycast::{prelude::*, print_intersections};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultRaycastingPlugin::<MyRaycastSet>::default(),
        ))
        .add_systems(
            First,
            update_raycast_with_cursor.before(RaycastSystem::BuildRays::<MyRaycastSet>),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, print_intersections::<MyRaycastSet>)
        .run();
}

#[derive(Reflect)]
struct MyRaycastSet;

// Update our `RaycastSource` with the current cursor position every frame.
fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RaycastSource<MyRaycastSet>>,
) {
    // Grab the most recent cursor event if it exists:
    let Some(cursor_moved) = cursor.iter().last() else {
        return;
    };
    for mut pick_source in &mut query {
        pick_source.cast_method = RaycastMethod::Screenspace(cursor_moved.position);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        .spawn(Camera2dBundle { ..default() })
        .insert(RaycastSource::<MyRaycastSet>::new()); // Designate the camera as our source;
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
            transform: Transform::default().with_scale(Vec3::splat(128.)),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            ..default()
        })
        .insert(RaycastMesh::<MyRaycastSet>::default()); // Make this mesh ray cast-able;
}
