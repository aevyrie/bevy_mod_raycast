mod debug;
mod primitives;
mod raycast;

use crate::raycast::*;
pub use crate::{debug::*, primitives::*};
use bevy::{
    ecs::schedule::ShouldRun,
    math::Vec3A,
    prelude::*,
    render::{
        camera::Camera,
        mesh::{Indices, Mesh, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    tasks::ComputeTaskPool,
    utils::FloatOrd,
};
use std::{
    collections::BTreeMap,
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

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
                    )
                    .with_system(
                        update_debug_cursor::<T>
                            .label(RaycastSystem::UpdateDebugCursor::<T>)
                            .with_run_criteria(|state: Res<DefaultPluginState<T>>| {
                                bool_criteria(state.update_debug_cursor)
                            })
                            .after(RaycastSystem::UpdateIntersections::<T>),
                    ),
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
    UpdateDebugCursor,
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
            Self::UpdateDebugCursor => Self::UpdateDebugCursor,
            Self::_Phantom(_) => Self::_Phantom(PhantomData),
        }
    }
}

/// Global plugin state used to enable or disable all ray casting for a given type T.
#[derive(Component)]
pub struct DefaultPluginState<T> {
    pub build_rays: bool,
    pub update_raycast: bool,
    pub update_debug_cursor: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for DefaultPluginState<T> {
    fn default() -> Self {
        DefaultPluginState {
            build_rays: true,
            update_raycast: true,
            update_debug_cursor: false,
            _marker: PhantomData,
        }
    }
}

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
pub struct RayCastMesh<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for RayCastMesh<T> {
    fn default() -> Self {
        RayCastMesh {
            _marker: PhantomData,
        }
    }
}

/// The `RayCastSource` component is used to generate rays with the specified `cast_method`. A `ray`
/// is generated when the RayCastSource is initialized, either by waiting for update_raycast system
/// to process the ray, or by using a `with_ray` function.
#[derive(Component)]
pub struct RayCastSource<T> {
    pub cast_method: RayCastMethod,
    ray: Option<Ray3d>,
    intersections: Vec<(Entity, IntersectionData)>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for RayCastSource<T> {
    fn default() -> Self {
        RayCastSource {
            cast_method: RayCastMethod::Screenspace(Vec2::ZERO),
            ray: None,
            intersections: Vec::new(),
            _marker: PhantomData::default(),
        }
    }
}

impl<T> RayCastSource<T> {
    /// Instantiates a [RayCastSource]. It will not be initialized until the update_raycast system
    /// runs, or one of the `with_ray` functions is run.
    pub fn new() -> RayCastSource<T> {
        RayCastSource::default()
    }
    /// Initializes a [RayCastSource] with a valid screenspace ray.
    pub fn with_ray_screenspace(
        self,
        cursor_pos_screen: Vec2,
        windows: &Res<Windows>,
        images: &Res<Assets<Image>>,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        RayCastSource {
            cast_method: RayCastMethod::Screenspace(cursor_pos_screen),
            ray: Ray3d::from_screenspace(
                cursor_pos_screen,
                windows,
                images,
                camera,
                camera_transform,
            ),
            intersections: self.intersections,
            _marker: self._marker,
        }
    }
    /// Initializes a [RayCastSource] with a valid ray derived from a transform.
    pub fn with_ray_transform(self, transform: Mat4) -> Self {
        RayCastSource {
            cast_method: RayCastMethod::Transform,
            ray: Some(Ray3d::from_transform(transform)),
            intersections: self.intersections,
            _marker: self._marker,
        }
    }
    /// Instantiates and initializes a [RayCastSource] with a valid screenspace ray.
    pub fn new_screenspace(
        cursor_pos_screen: Vec2,
        windows: &Res<Windows>,
        images: &Res<Assets<Image>>,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        RayCastSource::new().with_ray_screenspace(
            cursor_pos_screen,
            windows,
            images,
            camera,
            camera_transform,
        )
    }
    /// Initializes a [RayCastSource] with a valid ray derived from a transform.
    pub fn new_transform(transform: Mat4) -> Self {
        RayCastSource::new().with_ray_transform(transform)
    }
    /// Instantiates a [RayCastSource] with [RayCastMethod::Transform], and an empty ray. It will not
    /// be initialized until the [update_raycast] system is run and a [GlobalTransform] is present on
    /// this entity.
    /// # Warning
    /// Only use this if the entity this is associated with will have its [Transform] or
    /// [GlobalTransform] specified elsewhere. If the [GlobalTransform] is not set, this ray casting
    /// source will never be able to generate a raycast.
    pub fn new_transform_empty() -> Self {
        RayCastSource {
            cast_method: RayCastMethod::Transform,
            ..Default::default()
        }
    }
    pub fn intersect_list(&self) -> Option<&Vec<(Entity, IntersectionData)>> {
        if self.intersections.is_empty() {
            None
        } else {
            Some(&self.intersections)
        }
    }
    pub fn intersect_top(&self) -> Option<(Entity, &IntersectionData)> {
        if self.intersections.is_empty() {
            None
        } else {
            self.intersections.first().map(|(e, i)| (*e, i))
        }
    }
    pub fn intersect_primitive(&self, shape: Primitive3d) -> Option<IntersectionData> {
        let ray = self.ray?;
        match shape {
            Primitive3d::Plane {
                point: plane_origin,
                normal: plane_normal,
            } => {
                // assuming vectors are all normalized
                let denominator = ray.direction().dot(plane_normal);
                if denominator.abs() > f32::EPSILON {
                    let point_to_point = plane_origin - ray.origin();
                    let intersect_dist = plane_normal.dot(point_to_point) / denominator;
                    let intersect_position = ray.direction() * intersect_dist + ray.origin();
                    Some(IntersectionData::new(
                        intersect_position,
                        plane_normal,
                        intersect_dist,
                        None,
                    ))
                } else {
                    None
                }
            }
        }
    }
    /// Get a reference to the ray cast source's ray.
    pub fn ray(&self) -> Option<Ray3d> {
        self.ray
    }

    /// Get a mutable reference to the ray cast source's intersections.
    pub fn intersections_mut(&mut self) -> &mut Vec<(Entity, IntersectionData)> {
        &mut self.intersections
    }
}

/// Specifies the method used to generate rays.
pub enum RayCastMethod {
    /// Specify screen coordinates relative to the camera component associated with this entity.
    ///
    /// # Component Requirements
    ///
    /// This requires a [Windows] resource to convert the cursor coordinates to NDC, and a [Camera]
    /// component associated with this [RayCastSource]'s entity, to determine where the screenspace
    /// ray is firing from in the world.
    Screenspace(Vec2),
    /// Use a transform in world space to define a pick ray. This transform is applied to a vector
    /// at the origin pointing up to generate a ray.
    ///
    /// # Component Requirements
    ///
    /// Requires a [GlobalTransform] component associated with this [RayCastSource]'s entity.
    Transform,
}

#[allow(clippy::type_complexity)]
pub fn build_rays<T: 'static>(
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    mut pick_source_query: Query<(
        &mut RayCastSource<T>,
        Option<&GlobalTransform>,
        Option<&Camera>,
    )>,
) {
    for (mut pick_source, transform, camera) in &mut pick_source_query.iter_mut() {
        pick_source.ray = match &mut pick_source.cast_method {
            RayCastMethod::Screenspace(cursor_pos_screen) => {
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
                Ray3d::from_screenspace(
                    *cursor_pos_screen,
                    &windows,
                    &images,
                    camera,
                    camera_transform,
                )
            }
            // Use the specified transform as the origin and direction of the ray
            RayCastMethod::Transform => {
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

/// Iterates through all entities with the [RayCastMesh] component, checking for
/// intersections. If these entities have bounding volumes, these will be checked first, greatly
/// accelerating the process.
#[allow(clippy::type_complexity)]
pub fn update_raycast<T: 'static>(
    // Resources
    meshes: Res<Assets<Mesh>>,
    task_pool: Res<ComputeTaskPool>,
    // Queries
    mut pick_source_query: Query<&mut RayCastSource<T>>,
    culling_query: Query<
        (
            &Visibility,
            &ComputedVisibility,
            Option<&bevy::render::primitives::Aabb>,
            &GlobalTransform,
            Entity,
        ),
        With<RayCastMesh<T>>,
    >,
    mesh_query: Query<
        (
            &Handle<Mesh>,
            Option<&SimplifiedMesh>,
            &GlobalTransform,
            Entity,
        ),
        With<RayCastMesh<T>>,
    >,
) {
    for mut pick_source in pick_source_query.iter_mut() {
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
                            if let RayCastMethod::Screenspace(_) = pick_source.cast_method {
                                visibility.is_visible && comp_visibility.is_visible
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
            mesh_query.par_for_each(
                &task_pool,
                32,
                |(mesh_handle, simplified_mesh, transform, entity)| {
                    if culled_list.contains(&entity) {
                        let _raycast_guard = raycast.enter();
                        // Use the mesh handle to get a reference to a mesh asset
                        if let Some(mesh) =
                            meshes.get(simplified_mesh.map(|bm| &bm.mesh).unwrap_or(mesh_handle))
                        {
                            if let Some(intersection) =
                                ray_intersection_over_mesh(mesh, &transform.compute_matrix(), &ray)
                            {
                                picks.lock().unwrap().insert(
                                    FloatOrd(intersection.distance()),
                                    (entity, intersection),
                                );
                            }
                        }
                    }
                },
            );
            let picks = Arc::try_unwrap(picks).unwrap().into_inner().unwrap();
            pick_source.intersections = picks.into_values().map(|(e, i)| (e, i)).collect();
        }
    }
}
pub fn update_intersections<T: 'static>(
    mut commands: Commands,
    mut intersections: Query<&mut Intersection<T>>,
    sources: Query<&RayCastSource<T>>,
) {
    let mut intersect_iter = intersections.iter_mut();
    for (_, new_intersection) in sources.iter().filter_map(|source| source.intersect_top()) {
        match intersect_iter.next() {
            Some(mut intersection) => {
                // If there is an existing intersection, reuse it.
                intersection.data = Some(new_intersection.to_owned());
            }
            None => {
                // If there are no intersections left in the world, spawn one.
                commands
                    .spawn()
                    .insert(Intersection::<T>::new(new_intersection.to_owned()));
            }
        }
    }
    // Reset and despawn any remaining intersection. We need to be able to reset, because commands
    // take a full stage to update.
    for mut unused_intersect in intersect_iter {
        unused_intersect.data = None;
    }
}

/// Cast a ray on a mesh, and returns the intersection
pub fn ray_intersection_over_mesh(
    mesh: &Mesh,
    mesh_to_world: &Mat4,
    ray: &Ray3d,
) -> Option<IntersectionData> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        error!("bevy_mod_picking only supports TriangleList mesh topology");
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
            ),
            Indices::U32(vertex_indices) => ray_mesh_intersection(
                mesh_to_world,
                vertex_positions,
                vertex_normals,
                ray,
                Some(vertex_indices),
            ),
        }
    } else {
        ray_mesh_intersection(
            mesh_to_world,
            vertex_positions,
            vertex_normals,
            ray,
            None::<&Vec<u32>>,
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
) -> Option<IntersectionData> {
    if tri_vertices
        .iter()
        .any(|&vertex| (vertex - ray.origin).length_squared() < max_distance.powi(2))
    {
        // Run the raycast on the ray and triangle
        if let Some(ray_hit) = ray_triangle_intersection(&ray, &tri_vertices, Backfaces::default())
        {
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
