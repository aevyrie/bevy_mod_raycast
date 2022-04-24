use crate::Intersection;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Component)]
pub struct DebugCursor<T> {
    _phantom: PhantomData<fn() -> T>,
}
impl<T> Default for DebugCursor<T> {
    fn default() -> Self {
        DebugCursor {
            _phantom: PhantomData::default(),
        }
    }
}

#[derive(Component)]
pub struct DebugCursorTail<T> {
    _phantom: PhantomData<fn() -> T>,
}
impl<T> Default for DebugCursorTail<T> {
    fn default() -> Self {
        DebugCursorTail {
            _phantom: PhantomData::default(),
        }
    }
}

#[derive(Component)]
pub struct DebugCursorMesh<T> {
    _phantom: PhantomData<fn() -> T>,
}
impl<T> Default for DebugCursorMesh<T> {
    fn default() -> Self {
        DebugCursorMesh {
            _phantom: PhantomData::default(),
        }
    }
}

/// Updates the 3d cursor to be in the pointed world coordinates
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn update_debug_cursor<T: 'static>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cursors: Query<Entity, With<DebugCursor<T>>>,
    intersections: Query<&Intersection<T>>,
) {
    for entity in cursors.iter() {
        commands.entity(entity).despawn_recursive();
    }
    // Set the cursor translation to the top pick's world coordinates
    for intersection in intersections.iter() {
        let transform_new = intersection.normal_ray().to_transform();
        spawn_cursor::<T>(
            &mut commands,
            Transform::from_matrix(transform_new),
            &mut meshes,
            &mut materials,
        );
    }
}

fn spawn_cursor<T: 'static>(
    commands: &mut Commands,
    transform: Transform,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let cube_size = 0.04;
    let cube_tail_scale = 20.0;
    let ball_size = 0.08;
    let debug_material = &materials.add(StandardMaterial {
        base_color: Color::rgb(0.0, 1.0, 0.0),
        unlit: true,
        ..Default::default()
    });
    commands
        // cursor
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 4,
                radius: ball_size,
            })),
            material: debug_material.clone(),
            transform,
            ..Default::default()
        })
        .with_children(|parent| {
            let mut tail_transform = Transform::from_translation(Vec3::new(
                0.0,
                (cube_size * cube_tail_scale) / 2.0,
                0.0,
            ));
            tail_transform.apply_non_uniform_scale(Vec3::from([1.0, cube_tail_scale, 1.0]));

            // child cube
            parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: cube_size })),
                    material: debug_material.clone(),
                    transform: tail_transform,
                    ..Default::default()
                })
                .insert(DebugCursorTail::<T>::default())
                .insert(DebugCursorMesh::<T>::default());
        })
        .insert(DebugCursor::<T>::default())
        .insert(DebugCursorMesh::<T>::default());
}
