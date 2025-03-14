use bevy_asset::Handle;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::Reflect;

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct SimplifiedMesh {
    pub mesh: Handle<bevy_render::mesh::Mesh>,
}

#[derive(Component)]
pub struct NoBackfaceCulling;
