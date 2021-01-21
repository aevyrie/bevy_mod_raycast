use crate::RayCastSource;
use bevy::prelude::*;
use std::marker::PhantomData;

pub struct DebugCursor<T> {
    _phantom: PhantomData<T>,
}
impl<T> Default for DebugCursor<T> {
    fn default() -> Self {
        DebugCursor {
            _phantom: PhantomData::default(),
        }
    }
}

pub struct DebugCursorMesh<T> {
    _phantom: PhantomData<T>,
}
impl<T> Default for DebugCursorMesh<T> {
    fn default() -> Self {
        DebugCursorMesh {
            _phantom: PhantomData::default(),
        }
    }
}

/// Updates the 3d cursor to be in the pointed world coordinates
pub fn update_debug_cursor<T: 'static + Send + Sync>(
    mut query: Query<&mut Transform, With<DebugCursor<T>>>,
    mut visibility_query: Query<&mut Visible, With<DebugCursorMesh<T>>>,
    pick_source_query: Query<&RayCastSource<T>>,
) {
    // Set the cursor translation to the top pick's world coordinates
    for pick_source in pick_source_query.iter() {
        match pick_source.intersect_top() {
            Some(top_intersection) => {
                let transform_new = top_intersection.1.normal_ray().to_transform();
                for mut transform in &mut query.iter_mut() {
                    *transform = Transform::from_matrix(transform_new);
                }
                for mut visible in &mut visibility_query.iter_mut() {
                    visible.is_visible = true;
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
pub fn setup_debug_cursor<T: 'static + Send + Sync>(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&RayCastSource<T>>,
) {
    let debug_material = &materials.add(StandardMaterial {
        albedo: Color::rgb(0.0, 1.0, 0.0),
        shaded: false,
        ..Default::default()
    });
    let cube_size = 0.04;
    let cube_tail_scale = 20.0;
    let ball_size = 0.08;

    for _source in query.iter() {
        println!("spawning debug");
        commands
            // cursor
            .spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    subdivisions: 4,
                    radius: ball_size,
                })),
                material: debug_material.clone(),
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
                        material: debug_material.clone(),
                        transform,
                        ..Default::default()
                    })
                    .with(DebugCursorMesh::<T>::default());
            })
            .with(DebugCursor::<T>::default())
            .with(DebugCursorMesh::<T>::default());
    }
}
