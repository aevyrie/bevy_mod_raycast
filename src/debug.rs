#![cfg(feature = "debug")]

use std::marker::PhantomData;

use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::{ActiveState, DebugState, PluginState, RayCastSource};

pub struct DebugResource {
    pub cube_size: f32,
    pub cube_tail_scale: f32,
    pub material: Option<Handle<StandardMaterial>>,
    pub tip_mesh: Option<Handle<Mesh>>,
    pub tail_mesh: Option<Handle<Mesh>>,
}

impl Default for DebugResource {
    fn default() -> Self {
        Self {
            cube_size: 0.04,
            cube_tail_scale: 20.0,
            material: None,
            tip_mesh: None,
            tail_mesh: None,
        }
    }
}

pub(crate) struct SourceToCursorMap<T>(HashMap<Entity, Entity>, PhantomData<T>);

impl<T> Default for SourceToCursorMap<T> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

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

fn add_debug_cursor<T: 'static + Send + Sync>(
    commands: &mut Commands,
    source_entity: Entity,
    mut source: &mut RayCastSource<T>,
    sources_to_cursors: &mut ResMut<SourceToCursorMap<T>>,
    res: &Res<DebugResource>,
) {
    if let Some(current_entity) = source.debug_entity.as_ref() {
        if let Some(stored_entity) = sources_to_cursors.0.get(&source_entity) {
            if current_entity == stored_entity {
                return;
            } else {
                remove_debug_cursor(commands, source_entity, sources_to_cursors);
            }
        }
    }

    trace!(target: "debug", "Adding debug cursor to {:?}", source_entity);

    let mut cursor = commands.spawn();

    cursor
        .insert(GlobalTransform::default())
        .insert(Transform::default())
        .with_children(|parent| {
            // Child cursor
            if let Some((tip, material)) = res.tip_mesh.as_ref().zip(res.material.as_ref()) {
                parent.spawn_bundle(PbrBundle {
                    mesh: tip.clone(),
                    material: material.clone(),
                    ..Default::default()
                });
            }

            // child cube
            if let Some((tail, material)) = res.tail_mesh.as_ref().zip(res.material.as_ref()) {
                let mut transform = Transform::from_translation(Vec3::new(
                    0.0,
                    (res.cube_size * res.cube_tail_scale) / 2.0,
                    0.0,
                ));
                transform.apply_non_uniform_scale(Vec3::from([1.0, res.cube_tail_scale, 1.0]));

                parent.spawn_bundle(PbrBundle {
                    mesh: tail.clone(),
                    material: material.clone(),
                    transform,
                    ..Default::default()
                });
            }
        })
        .insert(DebugCursor::<T>::default());

    sources_to_cursors.0.insert(source_entity, cursor.id());

    source.debug_entity = Some(cursor.id());
}

fn remove_debug_cursor<T: 'static + Send + Sync>(
    commands: &mut Commands,
    source_entity: Entity,
    sources_to_cursors: &mut ResMut<SourceToCursorMap<T>>,
) {
    if let Some(cursor) = sources_to_cursors.0.remove(&source_entity) {
        trace!(target: "debug", "Removing debug cursor from {:?}", source_entity);

        commands.entity(cursor).despawn_recursive();
    }
}

pub(crate) fn startup_debug_global(
    mut res: ResMut<DebugResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if res.material.is_none() {
        res.material = Some(materials.add(StandardMaterial {
            base_color: Color::rgb(0.0, 1.0, 0.0),
            unlit: true,
            ..Default::default()
        }));
    }

    let cube_size = 0.04;
    let ball_size = 0.08;

    if res.tip_mesh.is_none() {
        res.tip_mesh = Some(meshes.add(Mesh::from(shape::Icosphere {
            subdivisions: 4,
            radius: ball_size,
        })));
    }

    if res.tail_mesh.is_none() {
        res.tail_mesh = Some(meshes.add(Mesh::from(shape::Cube { size: cube_size })));
    }
}

pub(crate) fn add_debug_cursors_to_new_sources<T: 'static + Send + Sync>(
    mut commands: Commands,
    res: Res<DebugResource>,
    mut sources_to_cursors: ResMut<SourceToCursorMap<T>>,
    mut added_sources: Query<(Entity, &mut RayCastSource<T>), Added<RayCastSource<T>>>,
) {
    added_sources.for_each_mut(|(source_entity, mut source)| {
        add_debug_cursor::<T>(
            &mut commands,
            source_entity,
            &mut source,
            &mut sources_to_cursors,
            &res,
        );
    });
}

pub(crate) fn remove_debug_cursors_of_removed_sources<T: 'static + Send + Sync>(
    mut commands: Commands,
    mut sources_to_cursors: ResMut<SourceToCursorMap<T>>,
    removed_sources: RemovedComponents<RayCastSource<T>>,
) {
    removed_sources.iter().for_each(|source_entity| {
        remove_debug_cursor::<T>(&mut commands, source_entity, &mut sources_to_cursors);
    });
}

pub(crate) fn change_cursors_by_changed_state<T: 'static + Send + Sync>(
    mut commands: Commands,
    res: Res<DebugResource>,
    state: Res<PluginState<T>>,
    mut sources_to_cursors: ResMut<SourceToCursorMap<T>>,
    mut sources: Query<(Entity, &mut RayCastSource<T>)>,
) {
    if !state.is_changed() && !state.is_added() {
        return;
    }

    if state.enabled == ActiveState::Enabled && state.debug == DebugState::Cursor {
        sources.for_each_mut(|(source_entity, mut source)| {
            add_debug_cursor::<T>(
                &mut commands,
                source_entity,
                &mut source,
                &mut sources_to_cursors,
                &res,
            );
        });
    } else {
        sources.for_each_mut(|(source_entity, _)| {
            remove_debug_cursor::<T>(&mut commands, source_entity, &mut sources_to_cursors);
        });
    }
}

pub(crate) fn run_if_debug_enabled<T: 'static + Send + Sync>(
    state: Res<PluginState<T>>,
) -> ShouldRun {
    if state.enabled == ActiveState::Enabled && state.debug == DebugState::Cursor {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

/// Updates the 3d cursor to be in the pointed world coordinates
pub(crate) fn update_debug_cursor_position<T: 'static + Send + Sync>(
    child_query: Query<&Children, With<DebugCursor<T>>>,
    mut cursor_query: Query<(&mut Transform, &Children), With<DebugCursor<T>>>,
    mut visibility_query: Query<&mut Visible>,
    raycast_source_query: Query<&RayCastSource<T>>,
) {
    // Set the cursor translation to the top pick's world coordinates
    for raycast_source in raycast_source_query.iter() {
        match (raycast_source.debug_entity, raycast_source.intersect_top()) {
            (Some(cursor), Some(top_intersection)) => {
                if let Ok((mut cursor, children)) = cursor_query.get_mut(cursor) {
                    let transform_new = top_intersection.1.normal_ray().to_transform();
                    *cursor = Transform::from_matrix(transform_new);

                    for &child in children.iter() {
                        if let Ok(mut visible) = visibility_query.get_mut(child) {
                            visible.is_visible = true;
                        }
                    }
                }
            }
            (Some(cursor), None) => {
                if let Ok(children) = child_query.get(cursor) {
                    for &child in children.iter() {
                        if let Ok(mut visible) = visibility_query.get_mut(child) {
                            visible.is_visible = false;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
