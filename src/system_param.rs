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

        let mut nearest_hit = FloatOrd(f32::INFINITY);

        let mut hits = Vec::<(FloatOrd, (Entity, IntersectionData))>::new();
        let raycast_guard = info_span!("raycast");

        culled_list.iter().for_each(|(aabb_near, entity)| {
            let mut raycast_mesh =
                |mesh_handle: &Handle<Mesh>,
                 simplified_mesh: Option<&SimplifiedMesh>,
                 no_backface_culling: Option<&NoBackfaceCulling>,
                 transform: &GlobalTransform| {
                    if should_early_exit {
                        // Is it even possible the mesh could be closer than the current best?
                        if *aabb_near > nearest_hit {
                            return;
                        }
                    }
                    // Does the mesh handle resolve?
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
                    let intersection =
                        ray_intersection_over_mesh(mesh, &transform, &ray, backfaces);
                    if let Some(intersection) = intersection {
                        let distance = FloatOrd(intersection.distance());
                        if should_early_exit && distance < nearest_hit {
                            nearest_hit = distance.min(nearest_hit);
                        }
                        hits.push((distance, (*entity, intersection)));
                    };
                };

            if let Ok((mesh, simp_mesh, culling, transform)) = self.mesh_query.get(*entity) {
                raycast_mesh(mesh, simp_mesh, culling, transform);
            }

            #[cfg(feature = "2d")]
            if let Ok((mesh, simp_mesh, transform)) = self.mesh2d_query.get(*entity) {
                raycast_mesh(&mesh.0, simp_mesh, Some(&NoBackfaceCulling), transform);
            }
        });

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
