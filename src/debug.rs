use std::marker::PhantomData;

use bevy::prelude::*;

use crate::Intersection;

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
#[allow(clippy::too_many_arguments)]
pub fn update_debug_cursor<T: 'static>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursors: Query<(&Intersection<T>, &mut Transform), With<DebugCursor<T>>>,
    intersection_without_cursor: Query<(Entity, &Intersection<T>), Without<DebugCursor<T>>>,
) {
    // Set the cursor translation to the top pick's world coordinates
    for (intersection, mut transform) in &mut cursors {
        *transform = Transform::from_matrix(intersection.normal_ray().to_transform());
    }
    // Spawn a new debug cursor for intersections without one
    for (entity, intersection) in &intersection_without_cursor {
        spawn_cursor::<T>(
            &mut commands,
            entity,
            Transform::from_matrix(intersection.normal_ray().to_transform()),
            &mut meshes,
            &mut materials,
        );
    }
}

fn spawn_cursor<T: 'static>(
    commands: &mut Commands,
    entity: Entity,
    transform: Transform,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let cube_size = 0.01;
    let cube_tail_scale = 80.0;
    let matl_x = materials.add(StandardMaterial {
        base_color: Color::rgb(1e7, 0.0, 0.0),
        emissive: Color::rgb(1e7, 0.0, 0.0),
        unlit: true,
        ..Default::default()
    });
    let matl_y = materials.add(StandardMaterial {
        base_color: Color::rgb(0.0, 1e7, 0.0),
        emissive: Color::rgb(0.0, 1e7, 0.0),
        unlit: true,
        ..Default::default()
    });
    let matl_z = materials.add(StandardMaterial {
        base_color: Color::rgb(0.0, 0.0, 1e7),
        emissive: Color::rgb(0.0, 0.0, 1e7),
        unlit: true,
        ..Default::default()
    });
    commands
        .entity(entity)
        .insert(SpatialBundle {
            transform,
            ..default()
        })
        // cursor
        .with_children(|parent| {
            let tail_scale = (cube_size * cube_tail_scale) / 2.0;
            let t_x = Transform {
                translation: (Vec3::X * tail_scale),
                scale: Vec3::ONE + Vec3::X * cube_tail_scale,
                ..default()
            };
            let t_y = Transform {
                translation: (Vec3::Y * tail_scale),
                scale: Vec3::ONE + Vec3::Y * cube_tail_scale,
                ..default()
            };
            let t_z = Transform {
                translation: (Vec3::Z * tail_scale),
                scale: Vec3::ONE + Vec3::Z * cube_tail_scale,
                ..default()
            };
            parent.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: cube_size })),
                    material: matl_x,
                    transform: t_x,
                    ..Default::default()
                },
                DebugCursorMesh::<T>::default(),
            ));
            parent.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: cube_size })),
                    material: matl_y,
                    transform: t_y,
                    ..Default::default()
                },
                DebugCursorMesh::<T>::default(),
            ));
            parent.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: cube_size })),
                    material: matl_z,
                    transform: t_z,
                    ..Default::default()
                },
                DebugCursorMesh::<T>::default(),
            ));
        })
        .insert(DebugCursor::<T>::default());
}
