use bevy::prelude::*;
pub use rays::*;

pub enum Primitive3d {
    ///Sphere{ radius: f32, position: Vec3 },
    Plane { point: Vec3, normal: Vec3 },
}

/// Holds computed intersection information
#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub struct Intersection {
    normal: Ray3d,
    pick_distance: f32,
    triangle: Option<Triangle>,
}
impl Intersection {
    pub fn new(normal: Ray3d, pick_distance: f32, triangle: Option<Triangle>) -> Self {
        Intersection {
            normal,
            pick_distance,
            triangle,
        }
    }
    /// Position vector describing the intersection position.
    pub fn position(&self) -> Vec3 {
        self.normal.origin()
    }
    /// Unit vector describing the normal of the intersected triangle.
    pub fn unit_normal(&self) -> Vec3 {
        self.normal.direction()
    }
    pub fn normal_ray(&self) -> &Ray3d {
        &self.normal
    }
    /// Distance from the picking source to the entity.
    pub fn distance(&self) -> f32 {
        self.pick_distance
    }
    /// Triangle that was intersected with in World coordinates
    pub fn world_triangle(&self) -> Option<Triangle> {
        self.triangle
    }
}

/// Encapsulates Ray3D, preventing use of struct literal syntax. This allows us to guarantee that
/// the `Ray3d` direction is normalized, because it can only be instantiated with the constructor.
pub mod rays {
    use bevy::{prelude::*, render::camera::Camera};

    /// A 3D ray, with an origin and direction. The direction is guaranteed to be normalized.
    #[derive(Debug, PartialOrd, PartialEq, Copy, Clone, Default)]
    pub struct Ray3d {
        origin: Vec3,
        direction: Vec3,
    }

    impl Ray3d {
        /// Constructs a `Ray3d`, normalizing the direction vector.
        pub fn new(origin: Vec3, direction: Vec3) -> Self {
            Ray3d {
                origin,
                direction: direction.normalize(),
            }
        }
        /// Position vector describing the ray origin
        pub fn origin(&self) -> Vec3 {
            self.origin
        }
        /// Unit vector describing the ray direction
        pub fn direction(&self) -> Vec3 {
            self.direction
        }
        pub fn to_transform(&self) -> Mat4 {
            let position = self.origin;
            let normal = self.direction;
            let up = Vec3::from([0.0, 1.0, 0.0]);
            let axis = up.cross(normal).normalize();
            let angle = up.dot(normal).acos();
            let epsilon = 0.0001;
            let new_rotation = if angle.abs() > epsilon {
                Quat::from_axis_angle(axis, angle)
            } else {
                Quat::default()
            };
            Mat4::from_rotation_translation(new_rotation, position)
        }
        pub fn from_screenspace(
            cursor_pos_screen: Vec2,
            windows: &Res<Windows>,
            camera: &Camera,
            camera_transform: &GlobalTransform,
        ) -> Self {
            let camera_position = camera_transform.compute_matrix();
            let window = windows
                .get(camera.window)
                .unwrap_or_else(|| panic!("WindowId {} does not exist", camera.window));
            let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
            let projection_matrix = camera.projection_matrix;

            // Normalized device coordinate cursor position from (-1, -1, -1) to (1, 1, 1)
            let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
            let cursor_pos_ndc_near: Vec3 = cursor_ndc.extend(-1.0);
            let cursor_pos_ndc_far: Vec3 = cursor_ndc.extend(1.0);

            // Use near and far ndc points to generate a ray in world space
            // This method is more robust than using the location of the camera as the start of
            // the ray, because ortho cameras have a focal point at infinity!
            let ndc_to_world: Mat4 = camera_position * projection_matrix.inverse();
            let cursor_pos_near: Vec3 = ndc_to_world.transform_point3(cursor_pos_ndc_near);
            let cursor_pos_far: Vec3 = ndc_to_world.transform_point3(cursor_pos_ndc_far);
            let ray_direction = cursor_pos_far - cursor_pos_near;
            Ray3d::new(cursor_pos_near, ray_direction)
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub struct Triangle {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
}
impl From<(Vec3, Vec3, Vec3)> for Triangle {
    fn from(vertices: (Vec3, Vec3, Vec3)) -> Self {
        Triangle {
            v0: vertices.0,
            v1: vertices.1,
            v2: vertices.2,
        }
    }
}
impl From<Vec<Vec3>> for Triangle {
    fn from(vertices: Vec<Vec3>) -> Self {
        Triangle {
            v0: *vertices.get(0).unwrap(),
            v1: *vertices.get(1).unwrap(),
            v2: *vertices.get(2).unwrap(),
        }
    }
}
impl From<[Vec3; 3]> for Triangle {
    fn from(vertices: [Vec3; 3]) -> Self {
        Triangle {
            v0: vertices[0],
            v1: vertices[1],
            v2: vertices[2],
        }
    }
}
