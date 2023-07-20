use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use bevy::{
    ecs::system::{lifetimeless::Read, SystemParam},
    prelude::*,
    reflect::TypePath,
    render::primitives::Aabb,
    sprite::Mesh2dHandle,
    utils::FloatOrd,
};

use crate::{
    ray_intersection_over_mesh, Backfaces, IntersectionData, NoBackfaceCulling, Ray3d, RaycastMesh,
    SimplifiedMesh,
};

/// A [`SystemParam`] that allows you to raycast into the world.
#[derive(SystemParam)]
pub struct Raycast<'w, 's, T: TypePath + Send + Sync> {
    pub meshes: Res<'w, Assets<Mesh>>,
    pub culling_query: Query<
        'w,
        's,
        (
            Read<ComputedVisibility>,
            Option<Read<Aabb>>,
            Read<GlobalTransform>,
            Entity,
        ),
        With<RaycastMesh<T>>,
    >,
    pub mesh_query: Query<
        'w,
        's,
        (
            Read<Handle<Mesh>>,
            Option<Read<SimplifiedMesh>>,
            Option<Read<NoBackfaceCulling>>,
            Read<GlobalTransform>,
            Entity,
        ),
        With<RaycastMesh<T>>,
    >,
    #[cfg(feature = "2d")]
    pub mesh2d_query: Query<
        'w,
        's,
        (
            Read<Mesh2dHandle>,
            Option<Read<SimplifiedMesh>>,
            Read<GlobalTransform>,
            Entity,
        ),
        With<RaycastMesh<T>>,
    >,
}

impl<'w, 's, T: TypePath + Send + Sync> Raycast<'w, 's, T> {
    pub fn cast_ray(
        &self,
        ray: Ray3d,
        should_frustum_cull: bool,
    ) -> BTreeMap<FloatOrd, (Entity, IntersectionData)> {
        let ray_cull = info_span!("ray culling");
        let raycast = info_span!("raycast");
        let ray_cull_guard = ray_cull.enter();
        // Check all entities to see if the source ray intersects the AABB, use this
        // to build a short list of entities that are in the path of the ray.
        let culled_list: Vec<Entity> = self
            .culling_query
            .iter()
            .filter_map(|(comp_visibility, bound_vol, transform, entity)| {
                let should_raycast = match should_frustum_cull {
                    true => comp_visibility.is_visible(),
                    false => comp_visibility.is_visible_in_hierarchy(),
                };
                if !should_raycast {
                    return None;
                }
                if let Some(aabb) = bound_vol {
                    ray.intersects_aabb(aabb, &transform.compute_matrix())
                        .filter(|[_, far]| *far >= 0.0)
                        .map(|_| entity)
                } else {
                    Some(entity)
                }
            })
            .collect();
        drop(ray_cull_guard);

        let picks = Arc::new(Mutex::new(BTreeMap::new()));

        let pick_mesh =
            |(mesh_handle, simplified_mesh, no_backface_culling, transform, entity): (
                &Handle<Mesh>,
                Option<&SimplifiedMesh>,
                Option<&NoBackfaceCulling>,
                &GlobalTransform,
                Entity,
            )| {
                if culled_list.contains(&entity) {
                    let _raycast_guard = raycast.enter();
                    // Use the mesh handle to get a reference to a mesh asset
                    if let Some(mesh) = self
                        .meshes
                        .get(simplified_mesh.map(|bm| &bm.mesh).unwrap_or(mesh_handle))
                    {
                        if let Some(intersection) = ray_intersection_over_mesh(
                            mesh,
                            &transform.compute_matrix(),
                            &ray,
                            if no_backface_culling.is_some() {
                                Backfaces::Include
                            } else {
                                Backfaces::Cull
                            },
                        ) {
                            picks
                                .lock()
                                .unwrap()
                                .insert(FloatOrd(intersection.distance()), (entity, intersection));
                        }
                    }
                }
            };

        self.mesh_query.par_iter().for_each(pick_mesh);
        #[cfg(feature = "2d")]
        self.mesh2d_query.par_iter().for_each(
            |(mesh_handle, simplified_mesh, transform, entity)| {
                pick_mesh((
                    &mesh_handle.0,
                    simplified_mesh,
                    Some(&NoBackfaceCulling),
                    transform,
                    entity,
                ))
            },
        );
        Arc::try_unwrap(picks).unwrap().into_inner().unwrap()
    }
}
