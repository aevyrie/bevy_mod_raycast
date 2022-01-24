use bevy::{math::Vec3A, prelude::*};
pub use rays::*;

#[non_exhaustive]
pub enum Primitive3d {
    ///Sphere{ radius: f32, position: Vec3 },
    Plane { point: Vec3, normal: Vec3 },
}

/// Holds computed intersection information
#[derive(Debug, PartialEq, Copy, Clone, Component)]
pub struct Intersection {
    position: Vec3,
    normal: Vec3,
    distance: f32,
    triangle: Option<Triangle>,
}
impl Intersection {
    pub fn new(
        position: Vec3,
        normal: Vec3,
        pick_distance: f32,
        triangle: Option<Triangle>,
    ) -> Self {
        Intersection {
            position,
            normal,
            distance: pick_distance,
            triangle,
        }
    }
    /// Position vector describing the intersection position.
    pub fn position(&self) -> Vec3 {
        self.position
    }
    /// Unit vector describing the normal of the intersected triangle.
    pub fn normal(&self) -> Vec3 {
        self.normal
    }
    pub fn normal_ray(&self) -> Ray3d {
        Ray3d::new(self.position, self.normal)
    }
    /// Distance from the picking source to the entity.
    pub fn distance(&self) -> f32 {
        self.distance
    }
    /// Triangle that was intersected with in World coordinates
    pub fn world_triangle(&self) -> Option<Triangle> {
        self.triangle
    }
}

/// Encapsulates Ray3D, preventing use of struct literal syntax. This allows us to guarantee that
/// the `Ray3d` direction is normalized, because it can only be instantiated with the constructor.
pub mod rays {
    use bevy::{
        math::Vec3A,
        prelude::*,
        render::{camera::Camera, primitives::Aabb},
    };

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
            let position = self.origin();
            let normal = self.direction();
            let up = Vec3::from([0.0, 1.0, 0.0]);
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
            windows: &Res<Windows>,
            camera: &Camera,
            camera_transform: &GlobalTransform,
        ) -> Option<Self> {
            let view = camera_transform.compute_matrix();
            let window = match windows.get(camera.window) {
                Some(window) => window,
                None => {
                    error!("WindowId {} does not exist", camera.window);
                    return None;
                }
            };
            let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
            let projection = camera.projection_matrix;

            // 2D Normalized device coordinate cursor position from (-1, -1) to (1, 1)
            let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
            let ndc_to_world: Mat4 = view * projection.inverse();
            let world_to_ndc = projection * view;
            let is_orthographic = projection.w_axis[3] == 1.0;

            // Compute the cursor position at the near plane. The bevy camera looks at -Z.
            let ndc_near = world_to_ndc.transform_point3(-Vec3::Z * camera.near).z;
            let cursor_pos_near = ndc_to_world.transform_point3(cursor_ndc.extend(ndc_near));

            // Compute the ray's direction depending on the projection used.
            let ray_direction = match is_orthographic {
                true => view.transform_vector3(-Vec3::Z), // All screenspace rays are parallel in ortho
                false => cursor_pos_near - camera_transform.translation, // Direction from camera to cursor
            };

            Some(Ray3d::new(cursor_pos_near, ray_direction))
        }
        /// Checks if the ray intersects with an AABB of a mesh.
        pub fn intersects_aabb(&self, aabb: &Aabb, model_to_world: &Mat4) -> Option<[f32; 2]> {
            // Transform the ray to model space
            let world_to_model = model_to_world.inverse();
            let ray_dir: Vec3A = world_to_model.transform_vector3(self.direction()).into();
            let ray_origin: Vec3A = world_to_model.transform_point3(self.origin()).into();
            // Check if the ray intersects the mesh's AABB. It's useful to work in model space because
            // we can do an AABB intersection test, instead of an OBB intersection test.

            let t_0: Vec3A = (Vec3A::from(aabb.min()) - ray_origin) / ray_dir;
            let t_1: Vec3A = (Vec3A::from(aabb.max()) - ray_origin) / ray_dir;
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
