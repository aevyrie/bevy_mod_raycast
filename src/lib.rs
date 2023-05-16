#![allow(clippy::type_complexity)]

#[cfg(feature = "debug")]
// pub mod debug;
pub mod octree;
pub mod primitives;
pub mod raycast;

use std::marker::PhantomData;

use bevy::prelude::*;

#[cfg(feature = "debug")]
// pub use debug::*;

pub struct DefaultRaycastingPlugin<T>(pub PhantomData<fn() -> T>);
impl<T: 'static + Send + Sync + Reflect + Clone> Plugin for DefaultRaycastingPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugin(octree::plugin::MeshOctreePlugin);
    }
}
impl<T> Default for DefaultRaycastingPlugin<T> {
    fn default() -> Self {
        DefaultRaycastingPlugin(PhantomData)
    }
}

#[derive(Component)]
pub struct SimplifiedMesh {
    pub mesh: Handle<Mesh>,
}

#[derive(Component)]
pub struct NoBackfaceCulling;
