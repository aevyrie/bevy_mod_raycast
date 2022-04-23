use std::f32::consts::FRAC_PI_2;

use bevy::{prelude::*, window::PresentMode};
use bevy_mod_raycast::{
    ray_intersection_over_mesh, DefaultPluginState, DefaultRaycastingPlugin, Ray3d, RayCastMesh,
    RayCastMethod, RayCastSource, RaycastSystem,
};

// This example shows how to use `ray_intersection_over_mesh` to cast a ray over a mesh
// without waiting for a frame to get results. This can be useful if you want to cast several rays
// where each ray depends on the previous ray result for example.
// This example only check for an obstacle. To get the angle to turn to avoid the obstacle,
// you would need to rotate the ray and recast it until it doesn't intersect with the obstacle.

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate, // Reduces input latency
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(DefaultRaycastingPlugin::<Ground>::default())
        .add_startup_system(setup)
        .add_startup_system(setup_ui)
        .add_system_to_stage(
            CoreStage::PreUpdate,
            update_raycast_with_cursor.before(RaycastSystem::BuildRays::<Ground>),
        )
        .add_system(check_path)
        .add_system(move_origin)
        .run();
}

fn update_raycast_with_cursor(
    mut cursor: EventReader<CursorMoved>,
    mut query: Query<&mut RayCastSource<Ground>>,
) {
    for mut pick_source in &mut query.iter_mut() {
        if let Some(cursor_latest) = cursor.iter().last() {
            pick_source.cast_method = RayCastMethod::Screenspace(cursor_latest.position);
        }
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(UiCameraBundle::default());
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexStart,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Path between shooter and mouse cursor: ".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "Direct!".to_string(),
                        style: TextStyle {
                            font,
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PathStatus);
}

// Marker struct for the text
#[derive(Component)]
struct PathStatus;
// Marker struct for the ground, used to get cursor position
#[derive(Component)]
struct Ground;
// Marker struct for the path origin, shown by a cyan sphere
#[derive(Component)]
struct PathOrigin;

// Marker struct for the path pointer, shown by a cyan box
#[derive(Component)]
struct PathPointer;
// Marker struct for obstacles
#[derive(Component)]
struct PathObstacle;
// Marker struct for the intersection point
#[derive(Component)]
struct PathObstaclePoint;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Enable the debug cursor against the `Ground`
    commands.insert_resource(DefaultPluginState::<Ground>::default().with_debug_cursor());
    // Spawn the camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(RayCastSource::<Ground>::new());

    // Spawn a plane that will represent the ground. It will be used to pick the mouse location in 3D space
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 500.0 })),
            material: materials.add(Color::DARK_GRAY.into()),
            ..Default::default()
        })
        .insert(RayCastMesh::<Ground>::default());

    // Spawn obstacles
    for x in -2..=2 {
        for z in -2..=2 {
            commands
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube::default())),
                    material: materials.add(Color::BLACK.into()),
                    transform: Transform::from_xyz(x as f32 * 4.0, 0.0, z as f32 * 4.0),
                    ..Default::default()
                })
                .insert(PathObstacle);
        }
    }
    // Spawn the path origin
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube::new(0.5))),
            material: materials.add(Color::CYAN.into()),
            transform: Transform {
                translation: Vec3::new(-6.0, 0.0, -2.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PathOrigin)
        .with_children(|from| {
            // Spawn a visual indicator for the path direction
            from.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box::default())),
                material: materials.add(StandardMaterial {
                    unlit: true,
                    base_color: Color::RED,
                    ..Default::default()
                }),
                transform: Transform::from_scale(Vec3::ZERO),
                ..Default::default()
            })
            .insert(PathPointer);
        });

    // Spawn the intersection point, invisible by default until there is an intersection
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere::default())),
            material: materials.add(StandardMaterial {
                unlit: true,
                base_color: Color::RED,
                ..Default::default()
            }),
            transform: Transform::from_scale(Vec3::splat(0.1)),
            visibility: Visibility {
                is_visible: false,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PathObstaclePoint);

    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });
}

// Move the path origin on mouse click
fn move_origin(
    mut from: Query<&mut Transform, With<PathOrigin>>,
    to: Query<&RayCastSource<Ground>>,
    mouse_event: Res<Input<MouseButton>>,
) {
    if let Ok(raycast_source) = to.get_single() {
        if let Some(top_intersection) = raycast_source.intersect_top() {
            let mut new_position = top_intersection.1.position();
            new_position.y = 0.0;

            if mouse_event.just_pressed(MouseButton::Left) {
                if let Ok(mut transform) = from.get_single_mut() {
                    transform.translation = new_position;
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
// Check the path between origin and mouse cursor position
fn check_path(
    mut from: Query<
        &mut Transform,
        (
            With<PathOrigin>,
            Without<PathObstacle>,
            Without<PathObstaclePoint>,
        ),
    >,
    mut pointer: Query<
        &mut Transform,
        (
            With<PathPointer>,
            Without<PathOrigin>,
            Without<PathObstacle>,
        ),
    >,
    to: Query<&RayCastSource<Ground>>,
    obstacles: Query<(&Handle<Mesh>, &Transform), With<PathObstacle>>,
    meshes: Res<Assets<Mesh>>,
    mut status_query: Query<&mut Text, With<PathStatus>>,
    mut intersection_point: Query<
        (&mut Transform, &mut Visibility),
        (
            With<PathObstaclePoint>,
            Without<PathObstacle>,
            Without<PathOrigin>,
            Without<PathPointer>,
        ),
    >,
) {
    if let Ok(mut origin_transform) = from.get_single_mut() {
        let raycast_source = to.single();
        let mut pointer = pointer.single_mut();
        if let Some(top_intersection) = raycast_source.intersect_top() {
            let from = origin_transform.translation;
            let to = top_intersection.1.position();
            let ray_direction = (to - from).normalize();

            // Rotate the direction indicator
            if Vec3::Z.angle_between(ray_direction) > FRAC_PI_2 {
                origin_transform.rotation =
                    Quat::from_rotation_y(Vec3::X.angle_between(ray_direction));
            } else {
                origin_transform.rotation =
                    Quat::from_rotation_y(-Vec3::X.angle_between(ray_direction));
            }

            let ray = Ray3d::new(from, ray_direction);
            if let Ok(mut text) = status_query.get_single_mut() {
                if let Ok((mut intersection_transform, mut visible)) =
                    intersection_point.get_single_mut()
                {
                    // Set everything as OK in case there are no obstacle in path
                    text.sections[1].value = "Direct!".to_string();
                    text.sections[1].style.color = Color::GREEN;
                    visible.is_visible = false;

                    let mut closest_hit = f32::MAX;

                    // Check for an obstacle on path
                    for (mesh_handle, transform) in obstacles.iter() {
                        if let Some(mesh) = meshes.get(mesh_handle) {
                            let mesh_to_world = transform.compute_matrix();

                            // Check for intersection with this obstacle
                            if let Some(intersection) =
                                ray_intersection_over_mesh(mesh, &mesh_to_world, &ray)
                            {
                                // There was an intersection, check if it is before the cursor
                                // on the ray
                                let hit_distance = intersection.distance();
                                let cursor_distance = from.distance(to);
                                if hit_distance < cursor_distance && hit_distance < closest_hit {
                                    text.sections[1].value = "Obstructed!".to_string();
                                    text.sections[1].style.color = Color::RED;
                                    intersection_transform.translation = intersection.position();
                                    visible.is_visible = true;
                                    closest_hit = hit_distance;
                                }
                            }
                        }
                    }

                    pointer.scale = Vec3::new(closest_hit / 2.0, 0.05, 0.05);
                    pointer.translation = Vec3::new(closest_hit / 2.0, 0.0, 0.0);
                }
            }
        }
    }
}
