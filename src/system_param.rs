use bevy::{
    ecs::system::{lifetimeless::Read, SystemParam},
    prelude::*,
    render::primitives::Aabb,
    sprite::Mesh2dHandle,
    utils::FloatOrd,
};

use crate::{
    ray_intersection_over_mesh, Backfaces, IntersectionData, NoBackfaceCulling, Ray3d,
    SimplifiedMesh,
};

/// How a raycast should handle visibility
#[derive(Clone, Copy, Reflect)]
pub enum RaycastVisibility {
    /// Completely ignore visibility checks. Hidden items can still be raycasted against.
    Ignore,
    /// Only raycast against entities that are visible in the hierarchy; see [`Visibility`].
    MustBeVisible,
    /// Only raycast against entities that are visible in the hierarchy and visible to a camera or
    /// light; see [`Visibility`].
    MustBeVisibleAndInView,
}

/// Settings for a raycast.
#[derive(Clone, Reflect)]
pub struct RaycastSettings<'a> {
    /// Determines how raycasting should consider entity visibility.
    pub visibility: RaycastVisibility,
    /// A filtering function that is applied to every entity that is raycasted. Only entities that
    /// return `true` will be considered.
    pub filter: &'a dyn Fn(Entity) -> bool,
    /// A function that is run every time a hit is found. Raycasting will continue to check for hits
    /// along the ray as long as this returns false.
    pub early_exit_test: &'a dyn Fn(Entity) -> bool,
}

impl<'a> RaycastSettings<'a> {
    /// Set the filter to apply to the raycast.
    pub fn with_filter(mut self, filter: &'a impl Fn(Entity) -> bool) -> Self {
        self.filter = filter;
        self
    }

    /// Set the early exit test to apply to the raycast.
    pub fn with_early_exit_test(mut self, early_exit_test: &'a impl Fn(Entity) -> bool) -> Self {
        self.early_exit_test = early_exit_test;
        self
    }

    /// This raycast should exit as soon as the nearest hit is found.
    pub fn always_early_exit(self) -> Self {
        self.with_early_exit_test(&|_| true)
    }

    /// This raycast should check all entities whose AABB intersects the ray and return all hits.
    pub fn never_early_exit(self) -> Self {
        self.with_early_exit_test(&|_| false)
    }
}

impl<'a> Default for RaycastSettings<'a> {
    fn default() -> Self {
        Self {
            visibility: RaycastVisibility::MustBeVisibleAndInView,
            filter: &|_| true,
            early_exit_test: &|_| true,
        }
    }
}

#[cfg(feature = "2d")]
type MeshFilter = Or<(With<Handle<Mesh>>, With<Mesh2dHandle>)>;
#[cfg(not(feature = "2d"))]
type MeshFilter = With<Handle<Mesh>>;

/// A [`SystemParam`] that allows you to raycast into the world.
#[derive(SystemParam)]
pub struct Raycast<'w, 's> {
    pub meshes: Res<'w, Assets<Mesh>>,
    pub hits: Local<'s, Vec<(FloatOrd, (Entity, IntersectionData))>>,
    pub output: Local<'s, Vec<(Entity, IntersectionData)>>,
    pub culled_list: Local<'s, Vec<(FloatOrd, Entity)>>,
    pub culling_query: Query<
        'w,
        's,
        (
            Read<ComputedVisibility>,
            Read<Aabb>,
            Read<GlobalTransform>,
            Entity,
        ),
        MeshFilter,
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
    >,
}

impl<'w, 's> Raycast<'w, 's> {
    /// Casts the `ray` into the world and returns a sorted list of intersections, nearest first.
    pub fn cast_ray(
        &mut self,
        ray: Ray3d,
        settings: &RaycastSettings,
    ) -> &[(Entity, IntersectionData)] {
        let ray_cull = info_span!("ray culling");
        let ray_cull_guard = ray_cull.enter();

        self.hits.clear();
        self.culled_list.clear();
        self.output.clear();

        // Check all entities to see if the ray intersects the AABB, use this to build a short list
        // of entities that are in the path of the ray.
        let (aabb_hits_tx, aabb_hits_rx) = crossbeam_channel::unbounded::<(FloatOrd, Entity)>();
        let visibility_setting = settings.visibility;
        self.culling_query
            .par_iter()
            .for_each(|(visibility, aabb, transform, entity)| {
                let should_raycast = match visibility_setting {
                    RaycastVisibility::Ignore => true,
                    RaycastVisibility::MustBeVisible => visibility.is_visible_in_hierarchy(),
                    RaycastVisibility::MustBeVisibleAndInView => visibility.is_visible_in_view(),
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
        *self.culled_list = aabb_hits_rx.try_iter().collect();
        self.culled_list.sort_by_key(|(aabb_near, _)| *aabb_near);
        drop(ray_cull_guard);

        let mut nearest_blocking_hit = FloatOrd(f32::INFINITY);
        let raycast_guard = info_span!("raycast");
        self.culled_list
            .iter()
            .filter(|(_, entity)| (settings.filter)(*entity))
            .for_each(|(aabb_near, entity)| {
                let mut raycast_mesh =
                    |mesh_handle: &Handle<Mesh>,
                     simplified_mesh: Option<&SimplifiedMesh>,
                     no_backface_culling: Option<&NoBackfaceCulling>,
                     transform: &GlobalTransform| {
                        // Is it even possible the mesh could be closer than the current best?
                        if *aabb_near > nearest_blocking_hit {
                            return;
                        }

                        // Does the mesh handle resolve?
                        let mesh_handle = simplified_mesh.map(|m| &m.mesh).unwrap_or(mesh_handle);
                        let Some(mesh) = self.meshes.get(mesh_handle) else {
                            return;
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
                            if (settings.early_exit_test)(*entity)
                                && distance < nearest_blocking_hit
                            {
                                // The reason we don't just return here is because right now we are
                                // going through the AABBs in order, but that doesn't mean that an
                                // AABB that starts further away cant end up with a closer hit than
                                // an AABB that starts closer. We need to keep checking AABBs that
                                // could possibly contain a nearer hit.
                                nearest_blocking_hit = distance.min(nearest_blocking_hit);
                            }
                            self.hits.push((distance, (*entity, intersection)));
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

        self.hits.retain(|(dist, _)| *dist <= nearest_blocking_hit);
        self.hits.sort_by_key(|(k, _)| *k);
        let hits = self.hits.iter().map(|(_, (e, i))| (*e, i.to_owned()));
        *self.output = hits.collect();
        self.output.as_ref()
    }
}
