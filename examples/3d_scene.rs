use bevy::prelude::*;
use bevy_mod_raycast::*;

fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_resource(WindowDescriptor {
            vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_system_to_stage(stage::POST_UPDATE, update_raycast::<MyRaycastSet>.system())
        .add_system_to_stage(
            stage::POST_UPDATE,
            update_debug_cursor::<MyRaycastSet>.system(),
        )
        .add_startup_system(setup.system())
        .run();
}

struct MyRaycastSet;

/// set up a simple 3D scene
fn setup(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    // camera
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
        .with(RayCastSource::<MyRaycastSet>::new(
            RayCastMethod::CameraCursor(UpdateOn::EveryFrame(Vec2::zero()), EventReader::default()),
        ))
        //plane
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            ..Default::default()
        })
        .with(RayCastMesh::<MyRaycastSet>::default())
        // cube
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            ..Default::default()
        })
        .with(RayCastMesh::<MyRaycastSet>::default())
        // sphere
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 20,
                radius: 0.5,
            })),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_translation(Vec3::new(1.5, 1.5, 1.5)),
            ..Default::default()
        })
        .with(RayCastMesh::<MyRaycastSet>::default())
        // light
        .spawn(LightBundle {
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        });
}
