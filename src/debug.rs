use crate::{PluginState, RayCastSource};
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

pub struct DebugCursorTail<T> {
    _phantom: PhantomData<T>,
}
impl<T> Default for DebugCursorTail<T> {
    fn default() -> Self {
        DebugCursorTail {
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
    mut commands: Commands,
    state: Res<PluginState<T>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    added_sources_query: Query<&RayCastSource<T>, Added<RayCastSource<T>>>,
    mut cursor_query: Query<&mut GlobalTransform, With<DebugCursor<T>>>,
    mut cursor_tail_query: Query<
        &mut GlobalTransform,
        (With<DebugCursorTail<T>>, Without<DebugCursor<T>>),
    >,
    mut visibility_query: Query<&mut Visible, With<DebugCursorMesh<T>>>,
    raycast_source_query: Query<&RayCastSource<T>>,
) {
    if !state.enabled {
        return;
    }

    let cube_size = 0.04;
    let cube_tail_scale = 20.0;
    let ball_size = 0.08;

    for _source in added_sources_query.iter() {
        let debug_material = &materials.add(StandardMaterial {
            albedo: Color::rgb(0.0, 1.0, 0.0),
            unlit: true,
            ..Default::default()
        });
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
                    .with(DebugCursorTail::<T>::default())
                    .with(DebugCursorMesh::<T>::default());
            })
            .with(DebugCursor::<T>::default())
            .with(DebugCursorMesh::<T>::default());
    }

    // Set the cursor translation to the top pick's world coordinates
    for raycast_source in raycast_source_query.iter() {
        match raycast_source.intersect_top() {
            Some(top_intersection) => {
                let transform_new = top_intersection.1.normal_ray().to_transform();
                for mut transform in cursor_query.iter_mut() {
                    *transform = GlobalTransform::from_matrix(transform_new);
                }
                for mut transform in cursor_tail_query.iter_mut() {
                    let scale = Vec3::from([1.0, cube_tail_scale, 1.0]);
                    let rotation = Quat::default();
                    let translation = Vec3::new(0.0, (cube_size * cube_tail_scale) / 2.0, 0.0);
                    let transform_move =
                        Mat4::from_scale_rotation_translation(scale, rotation, translation);
                    *transform = GlobalTransform::from_matrix(transform_new * transform_move)
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
