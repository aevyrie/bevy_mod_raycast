use std::marker::PhantomData;

use bevy::{math::Vec3A, prelude::*};

pub use rays::*;

#[non_exhaustive]
pub enum Primitive3d {
    ///Sphere{ radius: f32, position: Vec3 },
    Plane { point: Vec3, normal: Vec3 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntersectionData {
    position: Vec3,
    normal: Vec3,
    distance: f32,
    triangle: Option<Triangle>,
}

impl From<rays::PrimitiveIntersection> for IntersectionData {
    fn from(data: rays::PrimitiveIntersection) -> Self {
        Self {
            position: data.position(),
            normal: data.normal(),
            distance: data.distance(),
            triangle: None,
        }
    }
}

impl IntersectionData {
    pub fn new(position: Vec3, normal: Vec3, distance: f32, triangle: Option<Triangle>) -> Self {
        Self {
            position,
            normal,
            distance,
            triangle,
        }
    }

    /// Get the intersection data's position.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Get the intersection data's normal.
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        self.normal
    }

    /// Get the intersection data's distance.
    #[must_use]
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Get the intersection data's triangle.
    #[must_use]
    pub fn triangle(&self) -> Option<Triangle> {
        self.triangle
    }
}

/// Holds the topmost intersection for the raycasting set `T`.
///
/// ### Example
///
/// Lets say you've created a raycasting set `T`. If you have a [`crate::RaycastSource<T>`], a
/// [`crate::RaycastMesh<T>`], and an intersection occurs, the `RaycastMesh` will have an
/// `Intersection` component added to it, with the intersection data.
#[derive(Component)]
pub struct Intersection<T> {
    pub(crate) data: IntersectionData,
    _phantom: PhantomData<fn(T) -> T>,
}
impl<T> std::fmt::Debug for Intersection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Intersection")
            .field("position", &self.data.position)
            .field("normal", &self.data.normal)
            .field("distance", &self.data.distance)
            .field("triangle", &self.data.triangle)
            .finish()
    }
}
impl<T> Clone for Intersection<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            _phantom: PhantomData,
        }
    }
}
impl<T> Intersection<T> {
    pub fn new(data: IntersectionData) -> Self {
        Intersection {
            data,
            _phantom: PhantomData,
        }
    }
    /// Position vector describing the intersection position.
    pub fn position(&self) -> Vec3 {
        self.data.position
    }
    /// Unit vector describing the normal of the intersected triangle.
    pub fn normal(&self) -> Vec3 {
        self.data.normal
    }
    pub fn normal_ray(&self) -> Ray3d {
        Ray3d::new(self.data.position, self.data.normal)
    }
    /// Distance from the picking source to the entity.
    pub fn distance(&self) -> f32 {
        self.data.distance
    }
    /// Triangle that was intersected with in World coordinates
    pub fn world_triangle(&self) -> Option<Triangle> {
        self.data.triangle
    }
}

/// Encapsulates Ray3D, preventing use of struct literal syntax. This allows us to guarantee that
/// the `Ray3d` direction is normalized, because it can only be instantiated with the constructor.
pub mod rays {
    use super::Primitive3d;
    use bevy::{
        math::Vec3A,
        prelude::*,
        render::{camera::Camera, primitives::Aabb},
    };

    pub struct PrimitiveIntersection {
        position: Vec3,
        normal: Vec3,
        distance: f32,
    }

    impl PrimitiveIntersection {
        pub fn new(position: Vec3, normal: Vec3, distance: f32) -> Self {
            Self {
                position,
                normal,
                distance,
            }
        }

        /// Get the intersection's position
        #[must_use]
        pub fn position(&self) -> Vec3 {
            self.position
        }

        /// Get the normal vector of the primitive at the point of intersection
        #[must_use]
        pub fn normal(&self) -> Vec3 {
            self.normal
        }

        /// Get the distance between the ray origin and the intersection position
        #[must_use]
        pub fn distance(&self) -> f32 {
            self.distance
        }
    }

    /// A 3D ray, with an origin and direction. The direction is guaranteed to be normalized.
    #[derive(Debug, PartialEq, Copy, Clone, Default)]
    pub struct Ray3d {
        pub(crate) origin: Vec3A,
        pub(crate) direction: Vec3A,
    }

    impl Ray3d {
        /// Constructs a `Ray3d`, normalizing the direction vector.
        pub fn new(origin: Vec3, direction: Vec3) -> Self {
            Ray3d {
                origin: origin.into(),
                direction: direction.normalize().into(),
            }
        }

        /// Position vector describing the ray origin
        pub fn origin(&self) -> Vec3 {
            self.origin.into()
        }

        /// Unit vector describing the ray direction
        pub fn direction(&self) -> Vec3 {
            self.direction.into()
        }

        pub fn position(&self, distance: f32) -> Vec3 {
            (self.origin + self.direction * distance).into()
        }

        pub fn to_transform(self) -> Mat4 {
            self.to_aligned_transform([0., 1., 0.].into())
        }

        /// Create a transform whose origin is at the origin of the ray and
        /// whose up-axis is aligned with the direction of the ray. Use `up` to
        /// specify which axis of the transform should align with the ray.
        pub fn to_aligned_transform(self, up: Vec3) -> Mat4 {
            let position = self.origin();
            let normal = self.direction();
            let axis = up.cross(normal).normalize();
            let angle = up.dot(normal).acos();
            let epsilon = f32::EPSILON;
            let new_rotation = if angle.abs() > epsilon {
                Quat::from_axis_angle(axis, angle)
            } else {
                Quat::default()
            };
            Mat4::from_rotation_translation(new_rotation, position)
        }

        pub fn from_transform(transform: Mat4) -> Self {
            let pick_position_ndc = Vec3::from([0.0, 0.0, -1.0]);
            let pick_position = transform.project_point3(pick_position_ndc);
            let (_, _, source_origin) = transform.to_scale_rotation_translation();
            let ray_direction = pick_position - source_origin;
            Ray3d::new(source_origin, ray_direction)
        }

        pub fn from_screenspace(
            cursor_pos_screen: Vec2,
            camera: &Camera,
            camera_transform: &GlobalTransform,
        ) -> Option<Self> {
            let view = camera_transform.compute_matrix();

            let (viewport_min, viewport_max) = camera.logical_viewport_rect()?;
            let screen_size = camera.logical_target_size()?;
            let viewport_size = viewport_max - viewport_min;
            let adj_cursor_pos =
                cursor_pos_screen - Vec2::new(viewport_min.x, screen_size.y - viewport_max.y);

            let projection = camera.projection_matrix();
            let far_ndc = projection.project_point3(Vec3::NEG_Z).z;
            let near_ndc = projection.project_point3(Vec3::Z).z;
            let cursor_ndc = (adj_cursor_pos / viewport_size) * 2.0 - Vec2::ONE;
            let ndc_to_world: Mat4 = view * projection.inverse();
            let near = ndc_to_world.project_point3(cursor_ndc.extend(near_ndc));
            let far = ndc_to_world.project_point3(cursor_ndc.extend(far_ndc));
            let ray_direction = far - near;
            Some(Ray3d::new(near, ray_direction))
        }

        /// Checks if the ray intersects with an AABB of a mesh.
        pub fn intersects_aabb(&self, aabb: &Aabb, model_to_world: &Mat4) -> Option<[f32; 2]> {
            // Transform the ray to model space
            let world_to_model = model_to_world.inverse();
            let ray_dir: Vec3A = world_to_model.transform_vector3(self.direction()).into();
            let ray_origin: Vec3A = world_to_model.transform_point3(self.origin()).into();
            // Check if the ray intersects the mesh's AABB. It's useful to work in model space because
            // we can do an AABB intersection test, instead of an OBB intersection test.

            let t_0: Vec3A = (aabb.min() - ray_origin) / ray_dir;
            let t_1: Vec3A = (aabb.max() - ray_origin) / ray_dir;
            let t_min: Vec3A = t_0.min(t_1);
            let t_max: Vec3A = t_0.max(t_1);

            let mut hit_near = t_min.x;
            let mut hit_far = t_max.x;

            if hit_near > t_max.y || t_min.y > hit_far {
                return None;
            }

            if t_min.y > hit_near {
                hit_near = t_min.y;
            }
            if t_max.y < hit_far {
                hit_far = t_max.y;
            }

            if (hit_near > t_max.z) || (t_min.z > hit_far) {
                return None;
            }

            if t_min.z > hit_near {
                hit_near = t_min.z;
            }
            if t_max.z < hit_far {
                hit_far = t_max.z;
            }
            Some([hit_near, hit_far])
        }

        /// Checks if the ray intersects with a primitive shape
        pub fn intersects_primitive(&self, shape: Primitive3d) -> Option<PrimitiveIntersection> {
            match shape {
                Primitive3d::Plane {
                    point: plane_origin,
                    normal: plane_normal,
                } => {
                    // assuming vectors are all normalized
                    let denominator = self.direction().dot(plane_normal);
                    if denominator.abs() > f32::EPSILON {
                        let point_to_point = plane_origin - self.origin();
                        let intersect_dist = plane_normal.dot(point_to_point) / denominator;
                        let intersect_position = self.direction() * intersect_dist + self.origin();
                        Some(PrimitiveIntersection::new(
                            intersect_position,
                            plane_normal,
                            intersect_dist,
                        ))
                    } else {
                        None
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Triangle {
    pub v0: Vec3A,
    pub v1: Vec3A,
    pub v2: Vec3A,
}
impl From<(Vec3A, Vec3A, Vec3A)> for Triangle {
    fn from(vertices: (Vec3A, Vec3A, Vec3A)) -> Self {
        Triangle {
            v0: vertices.0,
            v1: vertices.1,
            v2: vertices.2,
        }
    }
}
impl From<Vec<Vec3A>> for Triangle {
    fn from(vertices: Vec<Vec3A>) -> Self {
        Triangle {
            v0: *vertices.get(0).unwrap(),
            v1: *vertices.get(1).unwrap(),
            v2: *vertices.get(2).unwrap(),
        }
    }
}
impl From<[Vec3A; 3]> for Triangle {
    fn from(vertices: [Vec3A; 3]) -> Self {
        Triangle {
            v0: vertices[0],
            v1: vertices[1],
            v2: vertices[2],
        }
    }
}
