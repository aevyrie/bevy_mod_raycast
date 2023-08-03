use std::sync::{Arc, RwLock};

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
            Read<Aabb>,
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
    /// Casts the `ray` into the world and returns a sorted list of intersections, nearest first.
    ///
    /// Setting `should_frustum_cull` to true will prevent raycasting anything that is not visible
    /// to a camera. This is a useful optimization when doing a screenspace raycast.
    pub fn cast_ray(
        &self,
        ray: Ray3d,
        should_frustum_cull: bool,
        should_early_exit: bool,
    ) -> Vec<(Entity, IntersectionData)> {
        let ray_cull = info_span!("ray culling");
        let ray_cull_guard = ray_cull.enter();
        // Check all entities to see if the ray intersects the AABB, use this to build a short list
        // of entities that are in the path of the ray.
        let max_hits = self.culling_query.iter().len();
        let (aabb_hits_tx, aabb_hits_rx) =
            crossbeam_channel::bounded::<(FloatOrd, Entity)>(max_hits);

        self.culling_query
            .par_iter()
            .for_each(|(comp_visibility, aabb, transform, entity)| {
                let should_raycast = match should_frustum_cull {
                    true => comp_visibility.is_visible(),
                    false => comp_visibility.is_visible_in_hierarchy(),
                };
                if should_raycast {
                    if let Some([near, _]) = ray
                        .intersects_aabb(aabb, &transform.compute_matrix())
                        .filter(|[_, far]| *far >= 0.0)
                    {
                        aabb_hits_tx.send((FloatOrd(near), entity)).ok();
                    }
                }
            });
        let mut culled_list: Vec<(FloatOrd, Entity)> = aabb_hits_rx.try_iter().collect();
        culled_list.sort_by_key(|(aabb_near, _)| *aabb_near);
        drop(ray_cull_guard);

        let nearest_hit_lock = Arc::new(RwLock::new(FloatOrd(f32::INFINITY)));

        let (hits_tx, hits_rx) =
            crossbeam_channel::bounded::<(FloatOrd, (Entity, IntersectionData))>(culled_list.len());
        let raycast_guard = info_span!("raycast");
        let raycast_mesh =
            |(mesh_handle, simplified_mesh, no_backface_culling, transform, entity): (
                &Handle<Mesh>,
                Option<&SimplifiedMesh>,
                Option<&NoBackfaceCulling>,
                &GlobalTransform,
                Entity,
            )| {
                // Is the entity in the list of culled entities?
                let culled_list = culled_list.iter().find(|(_, v)| *v == entity);
                let Some(&(aabb_near, entity)) = culled_list else {
                    return
                };

                if should_early_exit {
                    // Is it even possible the mesh could be closer than the current best?
                    let Some(&nearest_hit) = nearest_hit_lock.read().as_deref().ok() else {
                        return
                    };
                    if aabb_near > nearest_hit {
                        return;
                    }
                }

                let mesh_handle = simplified_mesh.map(|m| &m.mesh).unwrap_or(mesh_handle);
                let Some(mesh) = self.meshes.get(mesh_handle) else {
                    return
                };

                let _raycast_guard = raycast_guard.enter();
                let backfaces = match no_backface_culling {
                    Some(_) => Backfaces::Include,
                    None => Backfaces::Cull,
                };
                let transform = transform.compute_matrix();
                let intersection = ray_intersection_over_mesh(mesh, &transform, &ray, backfaces);
                if let Some(intersection) = intersection {
                    let distance = FloatOrd(intersection.distance());
                    if should_early_exit {
                        if let Ok(nearest_hit) = nearest_hit_lock.write().as_deref_mut() {
                            *nearest_hit = distance.min(*nearest_hit);
                        }
                    }
                    hits_tx.send((distance, (entity, intersection))).ok();
                };
            };

        self.mesh_query.par_iter().for_each(raycast_mesh);
        #[cfg(feature = "2d")]
        self.mesh2d_query.par_iter().for_each(
            |(mesh_handle, simplified_mesh, transform, entity)| {
                raycast_mesh((
                    &mesh_handle.0,
                    simplified_mesh,
                    Some(&NoBackfaceCulling),
                    transform,
                    entity,
                ))
            },
        );
        let mut hits: Vec<_> = hits_rx.try_iter().collect();
        hits.sort_by_key(|(k, _)| *k);
        if should_early_exit {
            hits.first()
                .map(|(_, (e, i))| vec![(*e, i.clone())])
                .unwrap_or_default()
        } else {
            hits.drain(..).map(|(_, v)| v).collect()
        }
    }
}
