#![allow(clippy::type_complexity)]

#[cfg(feature = "debug")]
pub mod debug;
mod primitives;
mod raycast;
pub mod system_param;

use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use bevy::{
    math::Vec3A,
    prelude::*,
    reflect::TypePath,
    render::{
        camera::Camera,
        mesh::{Indices, Mesh, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    window::PrimaryWindow,
};
use system_param::{RaycastSettings, RaycastVisibility};

pub use crate::{primitives::*, raycast::*};
#[cfg(feature = "debug")]
pub use debug::*;

pub mod prelude {
    pub use crate::{
        low_latency_window_plugin,
        system_param::{Raycast, RaycastSettings, RaycastVisibility},
        DefaultRaycastingPlugin, Ray3d, RaycastMesh, RaycastMethod, RaycastPluginState,
        RaycastSource, RaycastSystem, SimplifiedMesh,
    };
}

pub struct DefaultRaycastingPlugin<T>(pub PhantomData<fn() -> T>);
impl<T: TypePath + Send + Sync> Plugin for DefaultRaycastingPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<RaycastPluginState<T>>().add_systems(
            First,
            (
                build_rays::<T>
                    .in_set(RaycastSystem::BuildRays::<T>)
                    .run_if(|state: Res<RaycastPluginState<T>>| state.build_rays),
                update_raycast::<T>
                    .in_set(RaycastSystem::UpdateRaycast::<T>)
                    .run_if(|state: Res<RaycastPluginState<T>>| state.update_raycast),
                update_target_intersections::<T>
                    .in_set(RaycastSystem::UpdateIntersections::<T>)
                    .run_if(|state: Res<RaycastPluginState<T>>| state.update_raycast),
            )
                .chain(),
        );

        app.register_type::<RaycastMesh<T>>()
            .register_type::<RaycastSource<T>>();

        #[cfg(feature = "debug")]
        app.add_systems(
            First,
            update_debug_cursor::<T>
                .in_set(RaycastSystem::UpdateDebugCursor::<T>)
                .run_if(|state: Res<RaycastPluginState<T>>| state.update_debug_cursor)
                .after(RaycastSystem::UpdateIntersections::<T>),
        );
    }
}
impl<T> Default for DefaultRaycastingPlugin<T> {
    fn default() -> Self {
        DefaultRaycastingPlugin(PhantomData)
    }
}

#[derive(SystemSet)]
pub enum RaycastSystem<T> {
    BuildRays,
    UpdateRaycast,
    UpdateIntersections,
    #[cfg(feature = "debug")]
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
pub struct RaycastPluginState<T> {
    pub build_rays: bool,
    pub update_raycast: bool,
    #[cfg(feature = "debug")]
    pub update_debug_cursor: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for RaycastPluginState<T> {
    fn default() -> Self {
        RaycastPluginState {
            build_rays: true,
            update_raycast: true,
            #[cfg(feature = "debug")]
            update_debug_cursor: false,
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "debug")]
impl<T> RaycastPluginState<T> {
    pub fn with_debug_cursor(self) -> Self {
        RaycastPluginState {
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
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct RaycastMesh<T: TypePath> {
    #[reflect(ignore)]
    pub intersections: Vec<(Entity, IntersectionData)>,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

impl<T: TypePath> RaycastMesh<T> {
    /// Get a reference to the ray cast source's intersections. Returns an empty list if there are
    /// no intersections.
    pub fn intersections(&self) -> &[(Entity, IntersectionData)] {
        &self.intersections
    }
}

impl<T: TypePath> Default for RaycastMesh<T> {
    fn default() -> Self {
        RaycastMesh {
            intersections: Vec::new(),
            _marker: PhantomData,
        }
    }
}

/// The `RaycastSource` component is used to generate rays with the specified `cast_method`. A `ray`
/// is generated when the RaycastSource is initialized, either by waiting for update_raycast system
/// to process the ray, or by using a `with_ray` function.`
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct RaycastSource<T: TypePath> {
    /// The method used to generate rays for this raycast.
    pub cast_method: RaycastMethod,
    /// When `true`, raycasting will only hit the nearest entity, skipping any entities that are
    /// further away. This can significantly improve performance in cases where a ray intersects
    /// many AABBs.
    pub should_early_exit: bool,
    /// Determines how raycasting should consider entity visibility.
    pub visibility: RaycastVisibility,
    #[reflect(skip_serializing)]
    pub ray: Option<Ray3d>,
    #[reflect(ignore)]
    intersections: Vec<(Entity, IntersectionData)>,
    #[reflect(ignore)]
    _marker: PhantomData<fn() -> T>,
}

impl<T: TypePath> Default for RaycastSource<T> {
    fn default() -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Screenspace(Vec2::ZERO),
            should_early_exit: true,
            visibility: RaycastVisibility::MustBeVisibleAndInView,
            ray: None,
            intersections: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T: TypePath> RaycastSource<T> {
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
        window: &Window,
    ) -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Screenspace(cursor_pos_screen),
            ray: Ray3d::from_screenspace(cursor_pos_screen, camera, camera_transform, window),
            ..self
        }
    }
    /// Initializes a [RaycastSource] with a valid ray derived from a transform.
    pub fn with_ray_transform(self, transform: Mat4) -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Transform,
            ray: Some(Ray3d::from_transform(transform)),
            ..self
        }
    }

    /// Set the `should_early_exit` field of this raycast source.
    pub fn with_early_exit(self, should_early_exit: bool) -> Self {
        Self {
            should_early_exit,
            ..self
        }
    }

    /// Set the `visibility` field of this raycast source.
    pub fn with_visibility(self, visibility: RaycastVisibility) -> Self {
        Self { visibility, ..self }
    }

    /// Instantiates and initializes a [RaycastSource] with a valid screenspace ray.
    pub fn new_screenspace(
        cursor_pos_screen: Vec2,
        camera: &Camera,
        camera_transform: &GlobalTransform,
        window: &Window,
    ) -> Self {
        RaycastSource::new().with_ray_screenspace(
            cursor_pos_screen,
            camera,
            camera_transform,
            window,
        )
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

    /// Returns `true` if this is using [`RaycastMethod::Screenspace`].
    pub fn is_screenspace(&self) -> bool {
        matches!(self.cast_method, RaycastMethod::Screenspace(_))
    }
}

/// Specifies the method used to generate rays.
#[derive(Clone, Debug, Reflect)]
pub enum RaycastMethod {
    /// Specify screen coordinates relative to the camera component associated with this entity.
    ///
    /// # Component Requirements
    ///
    /// This requires a [Camera] component on this [RaycastSource]'s entity, to determine where the
    /// screenspace ray is firing from in the world.
    Screenspace(Vec2),
    /// Use a transform in world space to define a pick ray. This transform is applied to a vector
    /// at the origin pointing up to generate a ray.
    ///
    /// # Component Requirements
    ///
    /// Requires a [GlobalTransform] component associated with this [RaycastSource]'s entity.
    Transform,
}

pub fn build_rays<T: TypePath>(
    mut pick_source_query: Query<(
        &mut RaycastSource<T>,
        Option<&GlobalTransform>,
        Option<&Camera>,
    )>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    for (mut pick_source, transform, camera) in &mut pick_source_query {
        pick_source.ray = match &mut pick_source.cast_method {
            RaycastMethod::Screenspace(cursor_pos_screen) => {
                // Get all the info we need from the camera and window
                let window = match window.get_single() {
                    Ok(window) => window,
                    Err(_) => {
                        error!("No primary window found, cannot cast ray");
                        return;
                    }
                };
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
                Ray3d::from_screenspace(*cursor_pos_screen, camera, camera_transform, window)
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
pub fn update_raycast<T: TypePath + Send + Sync + 'static>(
    mut raycast: system_param::Raycast,
    mut pick_source_query: Query<&mut RaycastSource<T>>,
    targets: Query<&RaycastMesh<T>>,
) {
    for mut pick_source in &mut pick_source_query {
        if let Some(ray) = pick_source.ray {
            pick_source.intersections.clear();

            let filter = |entity| targets.contains(entity);
            let test = |_| pick_source.should_early_exit;
            let settings = RaycastSettings::default()
                .with_filter(&filter)
                .with_early_exit_test(&test);
            pick_source.intersections = raycast.cast_ray(ray, &settings).to_vec();
        }
    }
}

pub fn update_target_intersections<T: TypePath + Send + Sync>(
    sources: Query<(Entity, &RaycastSource<T>)>,
    mut meshes: Query<&mut RaycastMesh<T>>,
    mut previously_updated_raycast_meshes: Local<Vec<Entity>>,
) {
    // Clear any entities with intersections last frame
    for entity in previously_updated_raycast_meshes.drain(..) {
        if let Ok(mesh) = meshes.get_mut(entity).as_mut() {
            mesh.intersections.clear();
        }
    }

    for (source_entity, source) in sources.iter() {
        for (mesh_entity, intersection) in source.intersections().iter() {
            if let Ok(mut mesh) = meshes.get_mut(*mesh_entity) {
                mesh.intersections
                    .push((source_entity, intersection.to_owned()));
                previously_updated_raycast_meshes.push(*mesh_entity);
            }
        }
    }
}

/// Cast a ray on a mesh, and returns the intersection
pub fn ray_intersection_over_mesh(
    mesh: &Mesh,
    mesh_transform: &Mat4,
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
                mesh_transform,
                vertex_positions,
                vertex_normals,
                ray,
                Some(vertex_indices),
                backface_culling,
            ),
            Indices::U32(vertex_indices) => ray_mesh_intersection(
                mesh_transform,
                vertex_positions,
                vertex_normals,
                ray,
                Some(vertex_indices),
                backface_culling,
            ),
        }
    } else {
        ray_mesh_intersection(
            mesh_transform,
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
    mesh_transform: &Mat4,
    vertex_positions: &[[f32; 3]],
    vertex_normals: Option<&[[f32; 3]]>,
    ray: &Ray3d,
    indices: Option<&Vec<impl IntoUsize>>,
    backface_culling: Backfaces,
) -> Option<IntersectionData> {
    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut min_pick_distance = f32::MAX;
    let mut pick_intersection = None;

    let world_to_mesh = mesh_transform.inverse();

    let mesh_space_ray = Ray3d::new(
        world_to_mesh.transform_point3(ray.origin()),
        world_to_mesh.transform_vector3(ray.direction()),
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
                    mesh_transform.transform_point3(i.position()),
                    mesh_transform.transform_vector3(i.normal()),
                    mesh_transform
                        .transform_vector3(mesh_space_ray.direction() * i.distance())
                        .length(),
                    i.triangle().map(|tri| {
                        Triangle::from([
                            mesh_transform.transform_point3a(tri.v0),
                            mesh_transform.transform_point3a(tri.v1),
                            mesh_transform.transform_point3a(tri.v2),
                        ])
                    }),
                ));
                min_pick_distance = i.distance();
            }
        }
    } else {
        for i in (0..vertex_positions.len()).step_by(3) {
            let tri_vertex_positions = [
                Vec3A::from(vertex_positions[i]),
                Vec3A::from(vertex_positions[i + 1]),
                Vec3A::from(vertex_positions[i + 2]),
            ];
            let tri_normals = vertex_normals.map(|normals| {
                [
                    Vec3A::from(normals[i]),
                    Vec3A::from(normals[i + 1]),
                    Vec3A::from(normals[i + 2]),
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
                    mesh_transform.transform_point3(i.position()),
                    mesh_transform.transform_vector3(i.normal()),
                    mesh_transform
                        .transform_vector3(mesh_space_ray.direction() * i.distance())
                        .length(),
                    i.triangle().map(|tri| {
                        Triangle::from([
                            mesh_transform.transform_point3a(tri.v0),
                            mesh_transform.transform_point3a(tri.v1),
                            mesh_transform.transform_point3a(tri.v2),
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

/// Used for examples to reduce picking latency. Not relevant code for the examples.
#[doc(hidden)]
#[allow(dead_code)]
pub fn low_latency_window_plugin() -> bevy::window::WindowPlugin {
    bevy::window::WindowPlugin {
        primary_window: Some(bevy::window::Window {
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            ..Default::default()
        }),
        ..Default::default()
    }
}
