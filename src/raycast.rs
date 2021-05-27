use std::f32::EPSILON;

use crate::primitives::*;
use bevy::prelude::*;

#[allow(dead_code)]
#[non_exhaustive]
pub enum RaycastAlgorithm {
    MollerTrumbore(Backfaces),
}

impl Default for RaycastAlgorithm {
    fn default() -> Self {
        RaycastAlgorithm::MollerTrumbore(Backfaces::Cull)
    }
}

#[allow(dead_code)]
pub enum Backfaces {
    Cull,
    Include,
}

/// Takes a ray and triangle and computes the intersection and normal
#[inline]
pub fn ray_triangle_intersection(
    ray: &Ray3d,
    triangle: &Triangle,
    algorithm: RaycastAlgorithm,
) -> Option<RayHit> {
    match algorithm {
        RaycastAlgorithm::MollerTrumbore(backface_culling) => {
            raycast_moller_trumbore(ray, triangle, backface_culling)
        }
    }
}

#[derive(Default, Debug)]
pub struct RayHit {
    distance: f32,
    uv_coords: (f32, f32),
}

impl RayHit {
    /// Get a reference to the intersection's uv coords.
    pub fn uv_coords(&self) -> &(f32, f32) {
        &self.uv_coords
    }

    /// Get a reference to the intersection's distance.
    pub fn distance(&self) -> &f32 {
        &self.distance
    }
}

/// Implementation of the Möller-Trumbore ray-triangle intersection test
#[inline]
pub fn raycast_moller_trumbore(
    ray: &Ray3d,
    triangle: &Triangle,
    backface_culling: Backfaces,
) -> Option<RayHit> {
    // Source: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection
    let vector_v0_to_v1: Vec3 = triangle.v1 - triangle.v0;
    let vector_v0_to_v2: Vec3 = triangle.v2 - triangle.v0;
    let p_vec: Vec3 = ray.direction().cross(vector_v0_to_v2);
    let determinant: f32 = vector_v0_to_v1.dot(p_vec);

    match backface_culling {
        Backfaces::Cull => {
            // if the determinant is negative the triangle is back facing
            // if the determinant is close to 0, the ray misses the triangle
            // This test checks both cases
            if determinant < EPSILON {
                return None;
            }
        }
        Backfaces::Include => {
            // ray and triangle are parallel if det is close to 0
            if determinant.abs() < EPSILON {
                return None;
            }
        }
    }

    let determinant_inverse = 1.0 / determinant;

    let t_vec: Vec3 = ray.origin() - triangle.v0;
    let u = t_vec.dot(p_vec) * determinant_inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q_vec = t_vec.cross(vector_v0_to_v1);
    let v = ray.direction().dot(q_vec) * determinant_inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    // The distance between ray origin and intersection is t.
    let t: f32 = vector_v0_to_v2.dot(q_vec) * determinant_inverse;

    Some(RayHit {
        distance: t,
        uv_coords: (u, v),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Triangle vertices to be used in a left-hand coordinate system
    const V0: [f32; 3] = [1.0, -1.0, 2.0];
    const V1: [f32; 3] = [1.0, 2.0, -1.0];
    const V2: [f32; 3] = [1.0, -1.0, -1.0];

    #[test]
    fn raycast_triangle_mt() {
        let triangle = Triangle::from([V0.into(), V1.into(), V2.into()]);
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let algorithm = RaycastAlgorithm::MollerTrumbore(Backfaces::Include);
        let result = ray_triangle_intersection(&ray, &triangle, algorithm);
        assert_eq!(result.unwrap().distance, 1.0);
    }

    #[test]
    fn raycast_triangle_mt_culling() {
        let triangle = Triangle::from([V2.into(), V1.into(), V0.into()]);
        let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
        let algorithm = RaycastAlgorithm::MollerTrumbore(Backfaces::Cull);
        let result = ray_triangle_intersection(&ray, &triangle, algorithm);
        assert!(result.is_none());
    }
}
