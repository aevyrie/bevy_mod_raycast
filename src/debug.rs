use bevy::prelude::*;

use crate::PickSource;

pub struct DebugPickingPlugin;
impl Plugin for DebugPickingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_debug_cursor.system());
    }
}

pub struct DebugCursor;

pub struct DebugCursorMesh;

/// Updates the 3d cursor to be in the pointed world coordinates
pub fn update_debug_cursor_position<T: 'static>(
    mut query: Query<&mut Transform, With<DebugCursor>>,
    mut visibility_query: Query<&mut Visible, With<DebugCursorMesh>>,
    pick_source_query: Query<&PickSource<T>>,
) {
    // Set the cursor translation to the top pick's world coordinates
    for pick_source in pick_source_query.iter() {
        match pick_source.intersect_list() {
            Some(intersection_list) => {
                for (_entity, intersection) in intersection_list {
                    let transform_new = intersection.normal_ray().to_transform();
                    for mut transform in &mut query.iter_mut() {
                        *transform = Transform::from_matrix(transform_new);
                    }
                    for mut visible in &mut visibility_query.iter_mut() {
                        visible.is_visible = true;
                    }
                }
            }
            None => {
                for mut visible in &mut visibility_query.iter_mut() {
                    visible.is_visible = false;
                }
            }
        }
    }
}

/// Start up system to create 3d Debug cursor
fn setup_debug_cursor(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_matl = materials.add(StandardMaterial {
        albedo: Color::rgb(0.0, 1.0, 0.0),
        shaded: false,
        ..Default::default()
    });
    let cube_size = 0.04;
    let cube_tail_scale = 20.0;
    let ball_size = 0.08;
    commands
        // cursor
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 4,
                radius: ball_size,
            })),
            material: debug_matl.clone(),
            ..Default::default()
        })
        .with_children(|parent| {
            let mut transform = Transform::from_translation(Vec3::new(
                0.0,
                (cube_size * cube_tail_scale) / 2.0,
                0.0,
            ));
            transform.apply_non_uniform_scale(Vec3::from([1.0, cube_tail_scale, 1.0]));

            // child cube
            parent
                .spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: cube_size })),
                    material: debug_matl,
                    transform,
                    ..Default::default()
                })
                .with(DebugCursorMesh);
        })
        .with(DebugCursor)
        .with(DebugCursorMesh);
}
