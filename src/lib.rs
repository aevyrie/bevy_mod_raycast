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
    tasks::{ComputeTaskPool, ParallelIterator},
};
use std::marker::PhantomData;

/// Marks a Mesh entity as pickable
#[derive(Debug, Default)]
pub struct RayCastMesh<T>(PhantomData<T>);

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

/// Specifies the method used to generate rays
pub enum RayCastMethod {
    /// Specify screen coordinates relative to the camera component associated with this entity.
    Screenspace(Vec2),
    /// Use a tranform in world space to define pick ray.
    Transform,
}

// TODO
// instead of making user specify when to update the picks, have it be event driven in the bevy ecs system
// basically, the user is responsible for triggering events. Need a way of having a default every frame method

#[derive(Debug, Clone, Copy)]
pub enum UpdateOn {
    EveryFrame(Vec2),
    OnMouseEvent,
}

pub struct RayCastSource<T> {
    pub cast_method: RayCastMethod,
    ray: Option<Ray3d>,
    intersections: Vec<(Entity, Intersection)>,
    _marker: PhantomData<T>,
}

impl<T> RayCastSource<T> {
    pub fn new(pick_method: RayCastMethod) -> Self {
        RayCastSource {
            cast_method: pick_method,
            ray: None,
            intersections: Vec::new(),
            _marker: PhantomData::default(),
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
                if denominator > f32::EPSILON {
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
}

impl<T> Default for RayCastSource<T> {
    fn default() -> Self {
        RayCastSource {
            cast_method: RayCastMethod::Screenspace(Vec2::zero()),
            ..Default::default()
        }
    }
}

pub fn update_raycast<T: 'static + Send + Sync>(
    // Resources
    state: Res<PluginState<T>>,
    pool: Res<ComputeTaskPool>,
    meshes: Res<Assets<Mesh>>,
    windows: Res<Windows>,
    // Queries
    mut pick_source_query: Query<(
        &mut RayCastSource<T>,
        Option<&GlobalTransform>,
        Option<&Camera>,
    )>,
    culling_query: Query<
        (&Visible, Option<&BoundVol>, &GlobalTransform, Entity),
        With<RayCastMesh<T>>,
    >,
    mesh_query: Query<(&Handle<Mesh>, &GlobalTransform, Entity), With<RayCastMesh<T>>>,
) {
    if !state.enabled {
        return;
    }
    // Generate a ray for the picking source based on the pick method
    for (mut pick_source, transform, camera) in &mut pick_source_query.iter_mut() {
        pick_source.ray = match &mut pick_source.cast_method {
            RayCastMethod::Screenspace(cursor_pos_screen) => {
                // Get all the info we need from the camera and window
                let camera = match camera {
                    Some(camera) => camera,
                    None => panic!(
                        "The PickingSource is a CameraScreenSpace but has no associated Camera component"
                    ),
                };
                let window = windows
                    .get(camera.window)
                    .unwrap_or_else(|| panic!("WindowId {} does not exist", camera.window));
                let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
                let projection_matrix = camera.projection_matrix;
                let camera_position = match transform {
                    Some(transform) => transform,
                    None => panic!(
                        "The PickingSource is a CameraScreenSpace but has no associated GlobalTransform component"
                    ),
                }
                .compute_matrix();

                // Normalized device coordinate cursor position from (-1, -1, -1) to (1, 1, 1)
                let cursor_ndc = (*cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
                let cursor_pos_ndc_near: Vec3 = cursor_ndc.extend(-1.0);
                let cursor_pos_ndc_far: Vec3 = cursor_ndc.extend(1.0);

                // Use near and far ndc points to generate a ray in world space
                // This method is more robust than using the location of the camera as the start of
                // the ray, because ortho cameras have a focal point at infinity!
                let ndc_to_world: Mat4 = camera_position * projection_matrix.inverse();
                let cursor_pos_near: Vec3 = ndc_to_world.transform_point3(cursor_pos_ndc_near);
                let cursor_pos_far: Vec3 = ndc_to_world.transform_point3(cursor_pos_ndc_far);
                let ray_direction = cursor_pos_far - cursor_pos_near;
                Some(Ray3d::new(cursor_pos_near, ray_direction))
            }
            // Use the specified transform as the origin and direction of the ray
            RayCastMethod::Transform => {
                let pick_position_ndc = Vec3::from([0.0, 0.0, 1.0]);
                let source_transform = match transform {
                    Some(matrix) => matrix,
                    None => panic!(
                        "The PickingSource is a Transform but has no associated GlobalTransform component"
                    ),
                }
                .compute_matrix();
                let pick_position = source_transform.transform_point3(pick_position_ndc);

                let (_, _, source_origin) = source_transform.to_scale_rotation_translation();
                let ray_direction = pick_position - source_origin;

                Some(Ray3d::new(source_origin, ray_direction))
            }
        };

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
                    .par_iter(32)
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
                    .filter_map(|value| value)
                    .collect(&pool)
            };

            let mut picks = mesh_query
                .par_iter(8)
                .filter(|(_mesh_handle, _transform, entity)| culled_list.contains(&entity))
                .filter_map(|(mesh_handle, transform, entity)| {
                    let _raycast_guard = raycast.enter();
                    // Use the mesh handle to get a reference to a mesh asset
                    if let Some(mesh) = meshes.get(mesh_handle) {
                        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
                            panic!("bevy_mod_picking only supports TriangleList topology");
                        }
                        // Get the vertex positions from the mesh reference resolved from the mesh handle
                        let vertex_positions: &Vec<[f32; 3]> =
                            match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                                None => panic!("Mesh does not contain vertex positions"),
                                Some(vertex_values) => match &vertex_values {
                                    VertexAttributeValues::Float3(positions) => positions,
                                    _ => panic!("Unexpected vertex types in ATTRIBUTE_POSITION"),
                                },
                            };
                        if let Some(indices) = &mesh.indices() {
                            // Iterate over the list of pick rays that belong to the same group as this mesh
                            let mesh_to_world = transform.compute_matrix();
                            let new_intersection = match indices {
                                Indices::U16(vector) => ray_mesh_intersection(
                                    &mesh_to_world,
                                    vertex_positions,
                                    &ray,
                                    &vector.iter().map(|x| *x as u32).collect(),
                                ),
                                Indices::U32(vector) => ray_mesh_intersection(
                                    &mesh_to_world,
                                    vertex_positions,
                                    &ray,
                                    vector,
                                ),
                            };
                            //pickable.intersection = new_intersection;
                            if let Some(new_intersection) = new_intersection {
                                Some((entity, new_intersection))
                            } else {
                                None
                            }
                        } else {
                            // If we get here the mesh doesn't have an index list!
                            panic!(
                                "No index matrix found in mesh {:?}\n{:?}",
                                mesh_handle, mesh
                            );
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<(Entity, Intersection)>>(&pool);
            picks.sort_by(|a, b| {
                a.1.distance()
                    .partial_cmp(&b.1.distance())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            pick_source.intersections = picks;
        }
    }
}

fn ray_mesh_intersection(
    mesh_to_world: &Mat4,
    vertex_positions: &[[f32; 3]],
    pick_ray: &Ray3d,
    indices: &Vec<u32>,
) -> Option<Intersection> {
    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut min_pick_distance_squared = f32::MAX;
    let mut pick_intersection = None;

    // Make sure this chunk has 3 vertices to avoid a panic.
    if indices.len() % 3 == 0 {
        // Now that we're in the vector of vertex indices, we want to look at the vertex
        // positions for each triangle, so we'll take indices in chunks of three, where each
        // chunk of three indices are references to the three vertices of a triangle.
        for index in indices.chunks(3) {
            // Construct a triangle in world space using the mesh data
            let mut world_vertices: [Vec3; 3] = [Vec3::zero(), Vec3::zero(), Vec3::zero()];
            for i in 0..3 {
                let vertex_index = index[i] as usize;
                world_vertices[i] =
                    mesh_to_world.transform_point3(Vec3::from(vertex_positions[vertex_index]));
            }
            // If all vertices in the triangle are further away than the nearest hit, skip
            if world_vertices
                .iter()
                .map(|vert| (*vert - pick_ray.origin()).length_squared().abs())
                .fold(f32::INFINITY, |a, b| a.min(b))
                > min_pick_distance_squared
            {
                continue;
            }
            let world_triangle = Triangle::from(world_vertices);
            // Run the raycast on the ray and triangle
            if let Some(intersection) =
                ray_triangle_intersection(pick_ray, &world_triangle, RaycastAlgorithm::default())
            {
                let distance: f32 = (intersection.origin() - pick_ray.origin())
                    .length_squared()
                    .abs();
                if distance < min_pick_distance_squared {
                    min_pick_distance_squared = distance;
                    pick_intersection = Some(Intersection::new(
                        intersection,
                        distance,
                        Some(world_triangle),
                    ));
                }
            }
        }
    }
    pick_intersection
}
