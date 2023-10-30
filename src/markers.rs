use bevy::prelude::*;

#[derive(Component)]
pub struct SimplifiedMesh {
    pub mesh: Handle<bevy::render::mesh::Mesh>,
}

#[derive(Component)]
pub struct NoBackfaceCulling;
