#![allow(clippy::type_complexity)]

#[cfg(feature = "debug")]
mod debug;
mod primitives;
mod raycast;

use std::{
    collections::BTreeMap,
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[cfg(feature = "2d")]
use bevy::sprite::Mesh2dHandle;
use bevy::{
    ecs::schedule::ShouldRun,
    math::Vec3A,
    prelude::*,
    render::{
        camera::Camera,
        mesh::{Indices, Mesh, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    utils::FloatOrd,
};

pub use crate::{primitives::*, raycast::*};
#[cfg(feature = "debug")]
pub use debug::*;

pub struct DefaultRaycastingPlugin<T>(pub PhantomData<fn() -> T>);
impl<T: 'static + Send + Sync> Plugin for DefaultRaycastingPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<DefaultPluginState<T>>()
            .add_system_set_to_stage(
                CoreStage::First,
                SystemSet::new()
                    .with_system(
                        build_rays::<T>
                            .label(RaycastSystem::BuildRays::<T>)
                            .with_run_criteria(|state: Res<DefaultPluginState<T>>| {
                                bool_criteria(state.build_rays)
                            }),
                    )
                    .with_system(
                        update_raycast::<T>
                            .label(RaycastSystem::UpdateRaycast::<T>)
                            .with_run_criteria(|state: Res<DefaultPluginState<T>>| {
                                bool_criteria(state.update_raycast)
                            })
                            .after(RaycastSystem::BuildRays::<T>),
                    )
                    .with_system(
                        update_intersections::<T>
                            .label(RaycastSystem::UpdateIntersections::<T>)
                            .with_run_criteria(|state: Res<DefaultPluginState<T>>| {
                                bool_criteria(state.update_raycast)
                            })
                            .after(RaycastSystem::UpdateRaycast::<T>),
                    ),
            );

        #[cfg(feature = "debug")]
        app.add_system_to_stage(
            CoreStage::First,
            update_debug_cursor::<T>
                .label(RaycastSystem::UpdateDebugCursor::<T>)
                .with_run_criteria(|state: Res<DefaultPluginState<T>>| {
                    bool_criteria(state.update_debug_cursor)
                })
                .after(RaycastSystem::UpdateIntersections::<T>),
        );
    }
}
impl<T> Default for DefaultRaycastingPlugin<T> {
    fn default() -> Self {
        DefaultRaycastingPlugin(PhantomData)
    }
}

fn bool_criteria(flag: bool) -> ShouldRun {
    if flag {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

#[derive(SystemLabel)]
pub enum RaycastSystem<T> {
    BuildRays,
    UpdateRaycast,
    UpdateIntersections,
    #[cfg(feature = "debug")]
    UpdateDebugCursor,
    #[system_label(ignore_fields)]
    _Phantom(PhantomData<fn() -> T>),
}
impl<T> PartialEq for RaycastSystem<T> {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}
impl<T> Eq for RaycastSystem<T> {}
impl<T> Debug for RaycastSystem<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let set = std::any::type_name::<T>();
        match self {
            Self::BuildRays => write!(f, "BuildRays ({})", set),
            Self::UpdateRaycast => write!(f, "UpdateRaycast ({})", set),
            Self::UpdateIntersections => write!(f, "UpdateIntersections ({})", set),
            #[cfg(feature = "debug")]
            Self::UpdateDebugCursor => write!(f, "UpdateDebugCursor ({})", set),
            Self::_Phantom(_) => write!(f, "PhantomData<{}>", set),
        }
    }
}
impl<T> Hash for RaycastSystem<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let set = std::any::type_name::<T>();
        (core::mem::discriminant(self), set).hash(state);
    }
}
impl<T> Clone for RaycastSystem<T> {
    fn clone(&self) -> Self {
        match self {
            Self::BuildRays => Self::BuildRays,
            Self::UpdateRaycast => Self::UpdateRaycast,
            Self::UpdateIntersections => Self::UpdateIntersections,
            #[cfg(feature = "debug")]
            Self::UpdateDebugCursor => Self::UpdateDebugCursor,
            Self::_Phantom(_) => Self::_Phantom(PhantomData),
        }
    }
}

/// Global plugin state used to enable or disable all ray casting for a given type T.
#[derive(Component, Resource)]
pub struct DefaultPluginState<T> {
    pub build_rays: bool,
    pub update_raycast: bool,
    #[cfg(feature = "debug")]
    pub update_debug_cursor: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for DefaultPluginState<T> {
    fn default() -> Self {
        DefaultPluginState {
            build_rays: true,
            update_raycast: true,
            #[cfg(feature = "debug")]
            update_debug_cursor: false,
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "debug")]
impl<T> DefaultPluginState<T> {
    pub fn with_debug_cursor(self) -> Self {
        DefaultPluginState {
            update_debug_cursor: true,
            ..self
        }
    }
}

/// Marks an entity as pickable, with type T.
///
/// # Requirements
///
/// The marked entity must also have a [Mesh] component.
#[derive(Component, Debug)]
pub struct RaycastMesh<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for RaycastMesh<T> {
    fn default() -> Self {
        RaycastMesh {
            _marker: PhantomData,
        }
    }
}

/// The `RaycastSource` component is used to generate rays with the specified `cast_method`. A `ray`
/// is generated when the RaycastSource is initialized, either by waiting for update_raycast system
/// to process the ray, or by using a `with_ray` function.
#[derive(Component)]
pub struct RaycastSource<T> {
    pub cast_method: RaycastMethod,
    pub ray: Option<Ray3d>,
    intersections: Vec<(Entity, IntersectionData)>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for RaycastSource<T> {
    fn default() -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Screenspace(Vec2::ZERO),
            ray: None,
            intersections: Vec::new(),
            _marker: PhantomData::default(),
        }
    }
}

impl<T> RaycastSource<T> {
    /// Instantiates a [RaycastSource]. It will not be initialized until the update_raycast system
    /// runs, or one of the `with_ray` functions is run.
    pub fn new() -> RaycastSource<T> {
        RaycastSource::default()
    }
    /// Initializes a [RaycastSource] with a valid screenspace ray.
    pub fn with_ray_screenspace(
        self,
        cursor_pos_screen: Vec2,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Screenspace(cursor_pos_screen),
            ray: Ray3d::from_screenspace(cursor_pos_screen, camera, camera_transform),
            intersections: self.intersections,
            _marker: self._marker,
        }
    }
    /// Initializes a [RaycastSource] with a valid ray derived from a transform.
    pub fn with_ray_transform(self, transform: Mat4) -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Transform,
            ray: Some(Ray3d::from_transform(transform)),
            intersections: self.intersections,
            _marker: self._marker,
        }
    }
    /// Instantiates and initializes a [RaycastSource] with a valid screenspace ray.
    pub fn new_screenspace(
        cursor_pos_screen: Vec2,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        RaycastSource::new().with_ray_screenspace(cursor_pos_screen, camera, camera_transform)
    }
    /// Initializes a [RaycastSource] with a valid ray derived from a transform.
    pub fn new_transform(transform: Mat4) -> Self {
        RaycastSource::new().with_ray_transform(transform)
    }
    /// Instantiates a [RaycastSource] with [RaycastMethod::Transform], and an empty ray. It will
    /// not be initialized until the [update_raycast] system is run and a [GlobalTransform] is
    /// present on this entity.
    ///
    /// # Warning
    /// Only use this if the entity this is associated with will have its [Transform] or
    /// [GlobalTransform] specified elsewhere. If the [GlobalTransform] is not set, this ray casting
    /// source will never be able to generate a raycast.
    pub fn new_transform_empty() -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Transform,
            ..Default::default()
        }
    }
    /// Get a reference to the ray cast source's intersections, if one exists.
    pub fn get_intersections(&self) -> Option<&[(Entity, IntersectionData)]> {
        if self.intersections.is_empty() {
            None
        } else {
            Some(&self.intersections)
        }
    }
    /// Get a reference to the ray cast source's intersections. Returns an empty list if there are
    /// no intersections.
    pub fn intersections(&self) -> &[(Entity, IntersectionData)] {
        &self.intersections
    }
    /// Get a reference to the nearest intersection point, if there is one.
    pub fn get_nearest_intersection(&self) -> Option<(Entity, &IntersectionData)> {
        if self.intersections.is_empty() {
            None
        } else {
            self.intersections.first().map(|(e, i)| (*e, i))
        }
    }
    /// Run an intersection check between this [`RaycastSource`] and a 3D primitive [`Primitive3d`].
    pub fn intersect_primitive(&self, shape: Primitive3d) -> Option<IntersectionData> {
        Some(self.ray?.intersects_primitive(shape)?.into())
    }
    /// Get a copy of the ray cast source's ray.
    pub fn get_ray(&self) -> Option<Ray3d> {
        self.ray
    }

    /// Get a mutable reference to the ray cast source's intersections.
    pub fn intersections_mut(&mut self) -> &mut Vec<(Entity, IntersectionData)> {
        &mut self.intersections
    }
}

/// Specifies the method used to generate rays.
pub enum RaycastMethod {
    /// Specify screen coordinates relative to the camera component associated with this entity.
    ///
    /// # Component Requirements
    ///
    /// This requires a [Windows] resource to convert the cursor coordinates to NDC, and a [Camera]
    /// component associated with this [RaycastSource]'s entity, to determine where the screenspace
    /// ray is firing from in the world.
    Screenspace(Vec2),
    /// Use a transform in world space to define a pick ray. This transform is applied to a vector
    /// at the origin pointing up to generate a ray.
    ///
    /// # Component Requirements
    ///
    /// Requires a [GlobalTransform] component associated with this [RaycastSource]'s entity.
    Transform,
}

pub fn build_rays<T: 'static>(
    mut pick_source_query: Query<(
        &mut RaycastSource<T>,
        Option<&GlobalTransform>,
        Option<&Camera>,
    )>,
) {
    for (mut pick_source, transform, camera) in &mut pick_source_query {
        pick_source.ray = match &mut pick_source.cast_method {
            RaycastMethod::Screenspace(cursor_pos_screen) => {
                // Get all the info we need from the camera and window
                let camera = match camera {
                    Some(camera) => camera,
                    None => {
                        error!(
                        "The PickingSource is a CameraScreenSpace but has no associated Camera component"
                    );
                        return;
                    }
                };
                let camera_transform = match transform {
                    Some(transform) => transform,
                    None => {
                        error!(
                        "The PickingSource is a CameraScreenSpace but has no associated GlobalTransform component"
                    );
                        return;
                    }
                };
                Ray3d::from_screenspace(*cursor_pos_screen, camera, camera_transform)
            }
            // Use the specified transform as the origin and direction of the ray
            RaycastMethod::Transform => {
                let transform = match transform {
                    Some(matrix) => matrix,
                    None => {
                        error!(
                        "The PickingSource is a Transform but has no associated GlobalTransform component"
                    );
                        return
                    }
                }
                .compute_matrix();
                Some(Ray3d::from_transform(transform))
            }
        };
    }
}

/// Iterates through all entities with the [RaycastMesh] component, checking for
/// intersections. If these entities have bounding volumes, these will be checked first, greatly
/// accelerating the process.
pub fn update_raycast<T: 'static>(
    // Resources
    meshes: Res<Assets<Mesh>>,
    // Queries
    mut pick_source_query: Query<&mut RaycastSource<T>>,
    culling_query: Query<
        (
            &Visibility,
            &ComputedVisibility,
            Option<&bevy::render::primitives::Aabb>,
            &GlobalTransform,
            Entity,
        ),
        With<RaycastMesh<T>>,
    >,
    #[cfg(feature = "debug")] mesh_query: Query<
        (
            &Handle<Mesh>,
            Option<&SimplifiedMesh>,
            Option<&NoBackfaceCulling>,
            &GlobalTransform,
            Entity,
        ),
        (With<RaycastMesh<T>>, Without<DebugCursorMesh<T>>),
    >,
    #[cfg(not(feature = "debug"))] mesh_query: Query<
        (
            &Handle<Mesh>,
            Option<&SimplifiedMesh>,
            Option<&NoBackfaceCulling>,
            &GlobalTransform,
            Entity,
        ),
        With<RaycastMesh<T>>,
    >,
    #[cfg(feature = "2d")] mesh2d_query: Query<
        (
            &Mesh2dHandle,
            Option<&SimplifiedMesh>,
            &GlobalTransform,
            Entity,
        ),
        With<RaycastMesh<T>>,
    >,
) {
    for mut pick_source in &mut pick_source_query {
        if let Some(ray) = pick_source.ray {
            pick_source.intersections.clear();
            // Create spans for tracing
            let ray_cull = info_span!("ray culling");
            let raycast = info_span!("raycast");
            let ray_cull_guard = ray_cull.enter();
            // Check all entities to see if the source ray intersects the AABB, use this
            // to build a short list of entities that are in the path of the ray.
            let culled_list: Vec<Entity> = culling_query
                .iter()
                .filter_map(
                    |(visibility, comp_visibility, bound_vol, transform, entity)| {
                        let should_raycast =
                            if let RaycastMethod::Screenspace(_) = pick_source.cast_method {
                                visibility.is_visible && comp_visibility.is_visible()
                            } else {
                                visibility.is_visible
                            };
                        if should_raycast {
                            if let Some(aabb) = bound_vol {
                                if let Some([_, far]) =
                                    ray.intersects_aabb(aabb, &transform.compute_matrix())
                                {
                                    if far >= 0.0 {
                                        Some(entity)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                Some(entity)
                            }
                        } else {
                            None
                        }
                    },
                )
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
                        if let Some(mesh) =
                            meshes.get(simplified_mesh.map(|bm| &bm.mesh).unwrap_or(mesh_handle))
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
                                picks.lock().unwrap().insert(
                                    FloatOrd(intersection.distance()),
                                    (entity, intersection),
                                );
                            }
                        }
                    }
                };

            mesh_query.par_for_each(32, pick_mesh);
            #[cfg(feature = "2d")]
            mesh2d_query.par_for_each(32, |(mesh_handle, simplified_mesh, transform, entity)| {
                pick_mesh((
                    &mesh_handle.0,
                    simplified_mesh,
                    Some(&NoBackfaceCulling),
                    transform,
                    entity,
                ))
            });

            let picks = Arc::try_unwrap(picks).unwrap().into_inner().unwrap();
            pick_source.intersections = picks.into_values().map(|(e, i)| (e, i)).collect();
        }
    }
}
pub fn update_intersections<T: 'static>(
    mut commands: Commands,
    mut old_intersections: Query<(Entity, &mut Intersection<T>)>,
    sources: Query<&RaycastSource<T>>,
) {
    let new_intersections = sources
        .iter()
        .filter_map(|source| source.get_nearest_intersection())
        .collect::<BTreeMap<_, _>>();

    for (entity, _) in old_intersections.iter() {
        if !new_intersections.contains_key(&entity) {
            // Remove Intersection components that have no intersection this frame
            commands.entity(entity).remove::<Intersection<T>>();
        }
    }
    for (entity, new_intersect) in new_intersections.into_iter() {
        match old_intersections.get_mut(entity) {
            // Update Intersection components that already exist
            Ok((_, mut old_intersect)) => old_intersect.data = Some(new_intersect.to_owned()),
            // Add Intersection components to entities that did not have them already
            Err(_) => {
                commands
                    .entity(entity)
                    .insert(Intersection::<T>::new(new_intersect.to_owned()));
            }
        }
    }
}

/// Cast a ray on a mesh, and returns the intersection
pub fn ray_intersection_over_mesh(
    mesh: &Mesh,
    mesh_to_world: &Mat4,
    ray: &Ray3d,
    backface_culling: Backfaces,
) -> Option<IntersectionData> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        error!(
            "Invalid intersection check: `TriangleList` is the only supported `PrimitiveTopology`"
        );
        return None;
    }
    // Get the vertex positions from the mesh reference resolved from the mesh handle
    let vertex_positions: &Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        None => panic!("Mesh does not contain vertex positions"),
        Some(vertex_values) => match &vertex_values {
            VertexAttributeValues::Float32x3(positions) => positions,
            _ => panic!("Unexpected types in {:?}", Mesh::ATTRIBUTE_POSITION),
        },
    };
    let vertex_normals: Option<&[[f32; 3]]> =
        if let Some(normal_values) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            match &normal_values {
                VertexAttributeValues::Float32x3(normals) => Some(normals),
                _ => None,
            }
        } else {
            None
        };

    if let Some(indices) = &mesh.indices() {
        // Iterate over the list of pick rays that belong to the same group as this mesh
        match indices {
            Indices::U16(vertex_indices) => ray_mesh_intersection(
                mesh_to_world,
                vertex_positions,
                vertex_normals,
                ray,
                Some(vertex_indices),
                backface_culling,
            ),
            Indices::U32(vertex_indices) => ray_mesh_intersection(
                mesh_to_world,
                vertex_positions,
                vertex_normals,
                ray,
                Some(vertex_indices),
                backface_culling,
            ),
        }
    } else {
        ray_mesh_intersection(
            mesh_to_world,
            vertex_positions,
            vertex_normals,
            ray,
            None::<&Vec<u32>>,
            backface_culling,
        )
    }
}

pub trait IntoUsize: Copy {
    fn into_usize(self) -> usize;
}
impl IntoUsize for u16 {
    fn into_usize(self) -> usize {
        self as usize
    }
}
impl IntoUsize for u32 {
    fn into_usize(self) -> usize {
        self as usize
    }
}

/// Checks if a ray intersects a mesh, and returns the nearest intersection if one exists.
pub fn ray_mesh_intersection(
    mesh_to_world: &Mat4,
    vertex_positions: &[[f32; 3]],
    vertex_normals: Option<&[[f32; 3]]>,
    pick_ray: &Ray3d,
    indices: Option<&Vec<impl IntoUsize>>,
    backface_culling: Backfaces,
) -> Option<IntersectionData> {
    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut min_pick_distance = f32::MAX;
    let mut pick_intersection = None;

    let world_to_mesh = mesh_to_world.inverse();

    let mesh_space_ray = Ray3d::new(
        world_to_mesh.transform_point3(pick_ray.origin()),
        world_to_mesh.transform_vector3(pick_ray.direction()),
    );

    if let Some(indices) = indices {
        // Make sure this chunk has 3 vertices to avoid a panic.
        if indices.len() % 3 != 0 {
            warn!("Index list not a multiple of 3");
            return None;
        }
        // Now that we're in the vector of vertex indices, we want to look at the vertex
        // positions for each triangle, so we'll take indices in chunks of three, where each
        // chunk of three indices are references to the three vertices of a triangle.
        for index in indices.chunks(3) {
            let tri_vertex_positions = [
                Vec3A::from(vertex_positions[index[0].into_usize()]),
                Vec3A::from(vertex_positions[index[1].into_usize()]),
                Vec3A::from(vertex_positions[index[2].into_usize()]),
            ];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3A::from(normals[index[0].into_usize()]),
                    Vec3A::from(normals[index[1].into_usize()]),
                    Vec3A::from(normals[index[2].into_usize()]),
                ]
            });
            let intersection = triangle_intersection(
                tri_vertex_positions,
                tri_normals,
                min_pick_distance,
                mesh_space_ray,
                backface_culling,
            );
            if let Some(i) = intersection {
                pick_intersection = Some(IntersectionData::new(
                    mesh_to_world.transform_point3(i.position()),
                    mesh_to_world.transform_vector3(i.normal()),
                    mesh_to_world
                        .transform_vector3(mesh_space_ray.direction() * i.distance())
                        .length(),
                    i.triangle().map(|tri| {
                        Triangle::from([
                            mesh_to_world.transform_point3a(tri.v0),
                            mesh_to_world.transform_point3a(tri.v1),
                            mesh_to_world.transform_point3a(tri.v2),
                        ])
                    }),
                ));
                min_pick_distance = i.distance();
            }
        }
    } else {
        for vertex in vertex_positions.chunks(3) {
            let tri_vertex_positions = [
                Vec3A::from(vertex[0]),
                Vec3A::from(vertex[1]),
                Vec3A::from(vertex[2]),
            ];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3A::from(normals[0]),
                    Vec3A::from(normals[1]),
                    Vec3A::from(normals[2]),
                ]
            });
            let intersection = triangle_intersection(
                tri_vertex_positions,
                tri_normals,
                min_pick_distance,
                mesh_space_ray,
                backface_culling,
            );
            if let Some(i) = intersection {
                pick_intersection = Some(IntersectionData::new(
                    mesh_to_world.transform_point3(i.position()),
                    mesh_to_world.transform_vector3(i.normal()),
                    mesh_to_world
                        .transform_vector3(mesh_space_ray.direction() * i.distance())
                        .length(),
                    i.triangle().map(|tri| {
                        Triangle::from([
                            mesh_to_world.transform_point3a(tri.v0),
                            mesh_to_world.transform_point3a(tri.v1),
                            mesh_to_world.transform_point3a(tri.v2),
                        ])
                    }),
                ));
                min_pick_distance = i.distance();
            }
        }
    }
    pick_intersection
}

fn triangle_intersection(
    tri_vertices: [Vec3A; 3],
    tri_normals: Option<[Vec3A; 3]>,
    max_distance: f32,
    ray: Ray3d,
    backface_culling: Backfaces,
) -> Option<IntersectionData> {
    if tri_vertices
        .iter()
        .any(|&vertex| (vertex - ray.origin).length_squared() < max_distance.powi(2))
    {
        // Run the raycast on the ray and triangle
        if let Some(ray_hit) = ray_triangle_intersection(&ray, &tri_vertices, backface_culling) {
            let distance = *ray_hit.distance();
            if distance > 0.0 && distance < max_distance {
                let position = ray.position(distance);
                let normal = if let Some(normals) = tri_normals {
                    let u = ray_hit.uv_coords().0;
                    let v = ray_hit.uv_coords().1;
                    let w = 1.0 - u - v;
                    normals[1] * u + normals[2] * v + normals[0] * w
                } else {
                    (tri_vertices.v1() - tri_vertices.v0())
                        .cross(tri_vertices.v2() - tri_vertices.v0())
                        .normalize()
                };
                let intersection = IntersectionData::new(
                    position,
                    normal.into(),
                    distance,
                    Some(tri_vertices.to_triangle()),
                );
                return Some(intersection);
            }
        }
    }
    None
}

pub trait TriangleTrait {
    fn v0(&self) -> Vec3A;
    fn v1(&self) -> Vec3A;
    fn v2(&self) -> Vec3A;
    fn to_triangle(self) -> Triangle;
}
impl TriangleTrait for [Vec3A; 3] {
    fn v0(&self) -> Vec3A {
        self[0]
    }
    fn v1(&self) -> Vec3A {
        self[1]
    }
    fn v2(&self) -> Vec3A {
        self[2]
    }

    fn to_triangle(self) -> Triangle {
        Triangle::from(self)
    }
}
impl TriangleTrait for Triangle {
    fn v0(&self) -> Vec3A {
        self.v0
    }

    fn v1(&self) -> Vec3A {
        self.v1
    }

    fn v2(&self) -> Vec3A {
        self.v2
    }

    fn to_triangle(self) -> Triangle {
        self
    }
}

#[derive(Component)]
pub struct SimplifiedMesh {
    pub mesh: Handle<Mesh>,
}

#[derive(Component)]
pub struct NoBackfaceCulling;
