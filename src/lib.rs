mod bounding;
mod debug;
mod primitives;
mod raycast;

pub use crate::bounding::{update_bound_sphere, BoundVol, BoundingSphere};
pub use crate::debug::*;
pub use crate::primitives::*;

use crate::raycast::*;
use bevy::{
    prelude::*,
    render::{
        camera::Camera,
        mesh::{Indices, Mesh, VertexAttributeValues},
        pipeline::PrimitiveTopology,
    },
};
use std::marker::PhantomData;

pub struct DefaultRaycastingPlugin<T: 'static + Send + Sync>(pub PhantomData<T>);
impl<T: 'static + Send + Sync> Plugin for DefaultRaycastingPlugin<T> {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<PluginState<T>>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                build_rays::<T>.system().label(RaycastSystem::BuildRays),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                update_raycast::<T>
                    .system()
                    .label(RaycastSystem::UpdateRaycast)
                    .after(RaycastSystem::BuildRays),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                update_debug_cursor::<T>
                    .system()
                    .label(RaycastSystem::UpdateDebugCursor)
                    .after(RaycastSystem::UpdateRaycast),
            );
    }
}
impl<T: 'static + Send + Sync> Default for DefaultRaycastingPlugin<T> {
    fn default() -> Self {
        DefaultRaycastingPlugin(PhantomData::<T>)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum RaycastSystem {
    BuildRays,
    UpdateRaycast,
    UpdateDebugCursor,
}

/// Global plugin state used to enable or disable all ray casting for a given type T.
pub struct PluginState<T> {
    pub enabled: bool,
    _marker: PhantomData<T>,
}
impl<T> Default for PluginState<T> {
    fn default() -> Self {
        PluginState {
            enabled: true,
            _marker: PhantomData::<T>::default(),
        }
    }
}

/// Marks an entity as pickable, with type T.
///
/// # Requirements
///
/// The marked entity must also have a [Mesh] component.
#[derive(Debug)]
pub struct RayCastMesh<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for RayCastMesh<T> {
    fn default() -> Self {
        RayCastMesh {
            _marker: PhantomData::default(),
        }
    }
}

/// The `RayCastSource` component is used to generate rays with the specified `cast_method`. A `ray`
/// is generated when the RayCastSource is initialized, either by waiting for update_raycast system
/// to process the ray, or by using a `with_ray` function.
pub struct RayCastSource<T> {
    pub cast_method: RayCastMethod,
    ray: Option<Ray3d>,
    intersections: Vec<(Entity, Intersection)>,
    _marker: PhantomData<T>,
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
        &self,
        cursor_pos_screen: Vec2,
        windows: &Res<Windows>,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        RayCastSource {
            cast_method: RayCastMethod::Screenspace(cursor_pos_screen),
            ray: Ray3d::from_screenspace(cursor_pos_screen, windows, camera, camera_transform),
            intersections: self.intersections.clone(),
            _marker: self._marker,
        }
    }
    /// Initializes a [RayCastSource] with a valid ray derived from a transform.
    pub fn with_ray_transform(&self, transform: Mat4) -> Self {
        RayCastSource {
            cast_method: RayCastMethod::Transform,
            ray: Some(Ray3d::from_transform(transform)),
            intersections: self.intersections.clone(),
            _marker: self._marker,
        }
    }
    /// Instantiates and initializes a [RayCastSource] with a valid screenspace ray.
    pub fn new_screenspace(
        cursor_pos_screen: Vec2,
        windows: &Res<Windows>,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        RayCastSource::new().with_ray_screenspace(
            cursor_pos_screen,
            windows,
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
    pub fn intersect_list(&self) -> Option<&Vec<(Entity, Intersection)>> {
        if self.intersections.is_empty() {
            None
        } else {
            Some(&self.intersections)
        }
    }
    pub fn intersect_top(&self) -> Option<(Entity, Intersection)> {
        if self.intersections.is_empty() {
            None
        } else {
            self.intersections.first().copied()
        }
    }
    pub fn intersect_primitive(&self, shape: Primitive3d) -> Option<Intersection> {
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
                    Some(Intersection::new(
                        Ray3d::new(intersect_position, plane_normal),
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
    pub fn intersections_mut(&mut self) -> &mut Vec<(Entity, Intersection)> {
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
pub fn build_rays<T: 'static + Send + Sync>(
    windows: Res<Windows>,
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
                Ray3d::from_screenspace(*cursor_pos_screen, &windows, camera, camera_transform)
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
pub fn update_raycast<T: 'static + Send + Sync>(
    // Resources
    state: Res<PluginState<T>>,
    meshes: Res<Assets<Mesh>>,
    // Queries
    mut pick_source_query: Query<&mut RayCastSource<T>>,
    culling_query: Query<
        (&Visible, Option<&BoundVol>, &GlobalTransform, Entity),
        With<RayCastMesh<T>>,
    >,
    mesh_query: Query<(&Handle<Mesh>, &GlobalTransform, Entity), With<RayCastMesh<T>>>,
) {
    if !state.enabled {
        return;
    }
    for mut pick_source in pick_source_query.iter_mut() {
        if let Some(ray) = pick_source.ray {
            pick_source.intersections.clear();
            // Create spans for tracing
            let ray_cull = info_span!("ray culling");
            let raycast = info_span!("raycast");

            // Check all entities to see if the source ray intersects the bounding sphere, use this
            // to build a short list of entities that are in the path of the ray.
            let culled_list: Vec<Entity> = {
                let _ray_cull_guard = ray_cull.enter();
                culling_query
                    .iter()
                    .map(|(visibility, bound_vol, transform, entity)| {
                        let visible = visibility.is_visible;
                        let bound_hit = if let Some(bound_vol) = bound_vol {
                            if let Some(sphere) = &bound_vol.sphere {
                                let scaled_radius: f32 =
                                    1.01 * sphere.radius() * transform.scale.max_element();
                                let translated_origin =
                                    sphere.origin() * transform.scale + transform.translation;
                                let det = (ray.direction().dot(ray.origin() - translated_origin))
                                    .powi(2)
                                    - (Vec3::length_squared(ray.origin() - translated_origin)
                                        - scaled_radius.powi(2));
                                det >= 0.0 // Ray intersects the bounding sphere if det>=0
                            } else {
                                true // This bounding volume's sphere is not yet defined
                            }
                        } else {
                            true // This entity has no bounding volume
                        };
                        if visible && bound_hit {
                            Some(entity)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect()
            };

            let mut picks = mesh_query
                .iter()
                .filter(|(_mesh_handle, _transform, entity)| culled_list.contains(&entity))
                .filter_map(|(mesh_handle, transform, entity)| {
                    let _raycast_guard = raycast.enter();
                    // Use the mesh handle to get a reference to a mesh asset
                    if let Some(mesh) = meshes.get(mesh_handle) {
                        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
                            error!("bevy_mod_picking only supports TriangleList mesh topology");
                        }
                        // Get the vertex positions from the mesh reference resolved from the mesh handle
                        let vertex_positions: &Vec<[f32; 3]> =
                            match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                                None => panic!("Mesh does not contain vertex positions"),
                                Some(vertex_values) => match &vertex_values {
                                    VertexAttributeValues::Float32x3(positions) => positions,
                                    _ => panic!("Unexpected vertex types in ATTRIBUTE_POSITION"),
                                },
                            };
                        let mesh_to_world = transform.compute_matrix();
                        if let Some(indices) = &mesh.indices() {
                            // Iterate over the list of pick rays that belong to the same group as this mesh
                            let new_intersection = match indices {
                                Indices::U16(vector) => ray_mesh_intersection(
                                    &mesh_to_world,
                                    vertex_positions,
                                    &ray,
                                    Some(&vector.iter().map(|x| *x as u32).collect()),
                                ),
                                Indices::U32(vector) => ray_mesh_intersection(
                                    &mesh_to_world,
                                    vertex_positions,
                                    &ray,
                                    Some(vector),
                                ),
                            };
                            new_intersection.map(|new_intersection| (entity, new_intersection))
                        } else {
                            let new_intersection =
                                ray_mesh_intersection(&mesh_to_world, vertex_positions, &ray, None);
                            new_intersection.map(|new_intersection| (entity, new_intersection))
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<(Entity, Intersection)>>();
            picks.sort_by(|a, b| {
                a.1.distance()
                    .partial_cmp(&b.1.distance())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            pick_source.intersections = picks;
        }
    }
}

/// Checks if a ray intersects a mesh, and returns the nearest intersection if one exists.
#[inline(always)]
fn ray_mesh_intersection(
    mesh_to_world: &Mat4,
    vertex_positions: &[[f32; 3]],
    pick_ray: &Ray3d,
    indices: Option<&Vec<u32>>,
) -> Option<Intersection> {
    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut min_pick_distance_squared = f32::MAX;
    let mut pick_intersection = None;

    let world_to_mesh = mesh_to_world.inverse();

    let pick_ray_mesh = Ray3d::new(
        world_to_mesh.transform_point3(pick_ray.origin()),
        world_to_mesh.transform_vector3(pick_ray.direction()),
    );

    if let Some(indices) = indices {
        // Make sure this chunk has 3 vertices to avoid a panic.
        if indices.len() % 3 != 0 {
            warn!("Index list not a multiple of 3");
            return pick_intersection;
        }
        // Now that we're in the vector of vertex indices, we want to look at the vertex
        // positions for each triangle, so we'll take indices in chunks of three, where each
        // chunk of three indices are references to the three vertices of a triangle.
        for index in indices.chunks(3) {
            let tri_vertex_positions = [
                Vec3::from(vertex_positions[index[0] as usize]),
                Vec3::from(vertex_positions[index[1] as usize]),
                Vec3::from(vertex_positions[index[2] as usize]),
            ];
            triangle_intersection(
                tri_vertex_positions,
                pick_ray_mesh,
                &mut min_pick_distance_squared,
                &mut pick_intersection,
                mesh_to_world,
            );
        }
    } else {
        for vertex in vertex_positions.chunks(3) {
            let tri_vertex_positions = [
                Vec3::from(vertex[0]),
                Vec3::from(vertex[1]),
                Vec3::from(vertex[2]),
            ];
            triangle_intersection(
                tri_vertex_positions,
                pick_ray_mesh,
                &mut min_pick_distance_squared,
                &mut pick_intersection,
                mesh_to_world,
            );
        }
    }
    pick_intersection
}

#[inline(always)]
fn triangle_intersection(
    tri_vertex_positions: [Vec3; 3],
    pick_ray_mesh: Ray3d,
    min_pick_distance_squared: &mut f32,
    pick_intersection: &mut Option<Intersection>,
    mesh_to_world: &Mat4,
) {
    if tri_vertex_positions
        .iter()
        .filter(|vert| {
            (**vert - pick_ray_mesh.origin()).length_squared() < *min_pick_distance_squared
        })
        .count()
        != 0
    {
        let mesh_triangle = Triangle::from(tri_vertex_positions);
        // Run the raycast on the ray and triangle
        if let Some(intersection) =
            ray_triangle_intersection(&pick_ray_mesh, &mesh_triangle, RaycastAlgorithm::default())
        {
            let distance = (intersection.origin() - pick_ray_mesh.origin()).length_squared();
            if distance < *min_pick_distance_squared {
                *min_pick_distance_squared = distance;
                *pick_intersection = Some(Intersection::new(
                    Ray3d::new(
                        mesh_to_world.transform_point3(intersection.origin()),
                        mesh_to_world.transform_vector3(intersection.direction()),
                    ),
                    (mesh_to_world.transform_point3(intersection.origin())
                        - mesh_to_world.transform_point3(pick_ray_mesh.origin()))
                    .length(),
                    Some(Triangle::from([
                        mesh_to_world.transform_point3(mesh_triangle.v0),
                        mesh_to_world.transform_point3(mesh_triangle.v1),
                        mesh_to_world.transform_point3(mesh_triangle.v2),
                    ])),
                ));
            }
        }
    }
}
