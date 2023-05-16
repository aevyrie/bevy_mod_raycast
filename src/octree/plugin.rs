use bevy::prelude::*;

use super::MeshOctree;

pub struct MeshOctreePlugin;

impl Plugin for MeshOctreePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_octrees);
    }
}

pub fn update_octrees(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    mesh_handles: Query<(Entity, &Handle<Mesh>), Changed<Handle<Mesh>>>,
) {
    mesh_handles
        .iter()
        .filter_map(|(entity, handle)| Some(entity).zip(meshes.get(handle)))
        .filter_map(|(entity, mesh)| Some(entity).zip(MeshOctree::build(mesh).ok()))
        .for_each(|(entity, octree)| {
            commands.entity(entity).insert(octree);
        });
}
