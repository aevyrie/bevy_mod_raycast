use std::marker::PhantomData;

use bevy::{math::Vec3A, prelude::*, render::primitives::Aabb};

pub use rays::*;

#[non_exhaustive]
pub enum Primitive3d {
    ///Sphere{ radius: f32, position: Vec3 },
    Plane { point: Vec3, normal: Vec3 },
}

#[derive(Debug, Clone)]
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
    pub(crate) data: Option<IntersectionData>,
    _phantom: PhantomData<fn(T) -> T>,
}
impl<T> std::fmt::Debug for Intersection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            Some(data) => f
                .debug_struct("Intersection")
                .field("position", &data.position)
                .field("normal", &data.normal)
                .field("distance", &data.distance)
                .field("triangle", &data.triangle)
                .finish(),
            None => write!(f, "None"),
        }
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
            data: Some(data),
            _phantom: PhantomData,
        }
    }
    /// Position vector describing the intersection position.
    pub fn position(&self) -> Option<&Vec3> {
        if let Some(data) = &self.data {
            Some(&data.position)
        } else {
            None
        }
    }
    /// Unit vector describing the normal of the intersected triangle.
    pub fn normal(&self) -> Option<Vec3> {
        self.data().map(|data| data.normal)
    }
    pub fn normal_ray(&self) -> Option<Ray3d> {
        self.data()
            .map(|data| Ray3d::new(data.position, data.normal))
    }
    /// Distance from the picking source to the entity.
    pub fn distance(&self) -> Option<f32> {
        self.data().map(|data| data.distance)
    }
    /// Triangle that was intersected with in World coordinates
    pub fn world_triangle(&self) -> Option<Triangle> {
        self.data().and_then(|data| data.triangle)
    }
    fn data(&self) -> Option<&IntersectionData> {
        self.data.as_ref()
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
            let direction = world_to_model.transform_vector3(self.direction());
            let origin = world_to_model.transform_point3(self.origin());
            Ray3d::new(origin, direction).intersects_local_aabb(aabb)
        }

        /// Checks if the ray intersects an AABB in the same coordinate space
        pub fn intersects_local_aabb(&self, aabb: &Aabb) -> Option<[f32; 2]> {
            let t_0: Vec3A = (aabb.min() - self.origin) / self.direction;
            let t_1: Vec3A = (aabb.max() - self.origin) / self.direction;
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
impl Triangle {
    pub fn intersects_aabb(&self, aabb: Aabb) -> bool {
        let tri = self;
        let h = aabb.half_extents;

        let v = [
            tri.v0 - aabb.center,
            tri.v1 - aabb.center,
            tri.v2 - aabb.center,
        ];

        // Category 1 (3 tests): triangle AABB vs AABB
        if v[0].x.max(v[1].x).max(v[2].x) < -h.x || v[0].x.min(v[1].x).min(v[2].x) > h.x {
            return false;
        }
        if v[0].y.max(v[1].y).max(v[2].y) < -h.y || v[0].y.min(v[1].y).min(v[2].y) > h.y {
            return false;
        }
        if v[0].z.max(v[1].z).max(v[2].z) < -h.z || v[0].z.min(v[1].z).min(v[2].z) > h.z {
            return false;
        }

        // Triangle edges
        let f = [tri.v1 - tri.v0, tri.v2 - tri.v1, tri.v0 - tri.v2];

        // Category 2 (1 test): triangle plane vs AABB
        let plane_norm = f[0].cross(f[1]);
        let plane_dist = plane_norm.dot(v[0]).abs();
        let r = h.x * plane_norm.x.abs() + h.y * plane_norm.y.abs() + h.z * plane_norm.z.abs();
        if plane_dist > r {
            return false;
        }

        // AABB normals
        let e = [Vec3A::X, Vec3A::Y, Vec3A::Z];

        // Category 3 (9 tests): projected triangle radius vs projected AABB radius
        fn axis_test_failed(h: Vec3A, v: [Vec3A; 3], e: Vec3A, f: Vec3A) -> bool {
            let a = e.cross(f);
            let p0 = v[0].dot(a);
            let p1 = v[1].dot(a);
            let p2 = v[2].dot(a);
            let r = h.x * a.x.abs() + h.y * a.y.abs() + h.z * a.z.abs();
            p0.max(p1).max(p2) < -r || p0.min(p1).min(p2) > r
        }
        // Run every combination of the axis test:
        for i in 0..3 {
            for j in 0..3 {
                if axis_test_failed(h, v, e[i], f[j]) {
                    return false;
                }
            }
        }

        true
    }
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
