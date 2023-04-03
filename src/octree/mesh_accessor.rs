use super::node::TriangleIndex;
use crate::{RayHit, Triangle};
use bevy::{
    self,
    prelude::{Mesh, Vec3},
    render::{
        mesh::{Indices, VertexAttributeValues},
        primitives::Aabb,
    },
};

/// Makes it easier to get triangle data out of a mesh
pub struct MeshAccessor<'a> {
    pub(super) verts: &'a [[f32; 3]],
    pub(super) normals: Option<&'a [[f32; 3]]>,
    pub(super) indices: Option<&'a Indices>,
}

impl<'a> MeshAccessor<'a> {
    pub fn from_mesh(mesh: &'a Mesh) -> Self {
        let verts: &'a [[f32; 3]] = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            None => panic!("Mesh does not contain vertex positions"),
            Some(vertex_values) => match &vertex_values {
                bevy::render::mesh::VertexAttributeValues::Float32x3(positions) => positions,
                _ => panic!("Unexpected types in {:?}", Mesh::ATTRIBUTE_POSITION),
            },
        };

        let normals: Option<&[[f32; 3]]> =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
                .and_then(|normals| match &normals {
                    VertexAttributeValues::Float32x3(normals) => Some(normals.as_slice()),
                    _ => None,
                });

        Self {
            verts,
            normals,
            indices: mesh.indices(),
        }
    }

    pub fn iter_triangles(&self) -> impl Iterator<Item = TriangleIndex> + '_ {
        // If the triangle exists, we pass on the index.
        self.verts // num triangles will always be <= the number of verts
            .iter()
            .enumerate()
            .map(|(i, _v)| i as u32)
            .map_while(move |i| self.get_triangle(i).map(|_| i))
    }

    // Get the triangle vertices at the given `index`.
    pub fn get_triangle(&self, index: TriangleIndex) -> Option<Triangle> {
        let index = index as usize;
        let data = match self.indices {
            Some(indices) => match indices {
                Indices::U16(indices) => {
                    if indices.len() <= index * 3 + 2 {
                        return None;
                    }
                    [
                        self.verts[*indices.get(index * 3)? as usize],
                        self.verts[*indices.get(index * 3 + 1)? as usize],
                        self.verts[*indices.get(index * 3 + 2)? as usize],
                    ]
                }
                Indices::U32(indices) => {
                    if indices.len() <= index * 3 + 2 {
                        return None;
                    }
                    [
                        self.verts[*indices.get(index * 3)? as usize],
                        self.verts[*indices.get(index * 3 + 1)? as usize],
                        self.verts[*indices.get(index * 3 + 2)? as usize],
                    ]
                }
            },
            None => [
                *self.verts.get(index * 3)?,
                *self.verts.get(index * 3 + 1)?,
                *self.verts.get(index * 3 + 2)?,
            ],
        };
        Some(Triangle {
            v0: data[0].into(),
            v1: data[1].into(),
            v2: data[2].into(),
        })
    }

    // Get the triangle vertices at the given `index`.
    pub fn triangle_normals(&self, index: TriangleIndex) -> Option<[[f32; 3]; 3]> {
        let index = index as usize;
        let Some(normals) = self.normals else {
            return None
        };

        let triangle_normals = match self.indices {
            Some(indices) => match indices {
                Indices::U16(indices) => [
                    normals[indices[index * 3] as usize],
                    normals[indices[index * 3 + 1] as usize],
                    normals[indices[index * 3 + 2] as usize],
                ],
                Indices::U32(indices) => [
                    normals[indices[index * 3] as usize],
                    normals[indices[index * 3 + 1] as usize],
                    normals[indices[index * 3 + 2] as usize],
                ],
            },
            None => [
                normals[index * 3],
                normals[index * 3 + 1],
                normals[index * 3 + 2],
            ],
        };

        Some(triangle_normals)
    }

    pub fn intersection_normal(&self, index: TriangleIndex, hit: RayHit) -> Vec3 {
        if let Some(normals) = self.triangle_normals(index) {
            let u = hit.uv_coords().0;
            let v = hit.uv_coords().1;
            let w = 1.0 - u - v;
            Vec3::from(normals[1]) * u + Vec3::from(normals[2]) * v + Vec3::from(normals[0]) * w
        } else {
            let triangle = self.get_triangle(index).unwrap();
            (triangle.v1 - triangle.v0)
                .cross(triangle.v2 - triangle.v0)
                .normalize()
                .into()
        }
    }

    pub(crate) fn min(&self) -> Option<[f32; 3]> {
        self.verts
            .iter()
            .copied()
            .reduce(|acc, v| [acc[0].min(v[0]), acc[1].min(v[1]), acc[2].min(v[2])])
    }

    pub(crate) fn max(&self) -> Option<[f32; 3]> {
        self.verts
            .iter()
            .copied()
            .reduce(|acc, v| [acc[0].max(v[0]), acc[1].max(v[1]), acc[2].max(v[2])])
    }

    pub(crate) fn generate_aabb(&self) -> Aabb {
        let min: Vec3 = self.min().unwrap_or_default().into();
        let max: Vec3 = self.max().unwrap_or_default().into();
        Aabb::from_min_max(min, max)
    }
}

pub mod test_util {
    use super::MeshAccessor;

    /// A quad centered on the origin, laying on the X-Z plane.
    pub fn build_vert_only_xz_quad<'a>() -> MeshAccessor<'a> {
        let verts = &[
            [-1., 0., 0.],
            [0., 0., 1.],
            [1., 0., 0.],
            [1., 0., 0.],
            [0., 0., -1.],
            [-1., 0., 0.],
        ];
        MeshAccessor {
            verts,
            normals: None,
            indices: None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use bevy::math::Vec3A;

    use crate::octree::mesh_accessor::test_util;

    #[test]
    fn test_get_tri() {
        let mesh = test_util::build_vert_only_xz_quad();
        let tri = mesh.get_triangle(0).unwrap();
        assert_eq!([tri.v0, tri.v1, tri.v2], [-Vec3A::X, Vec3A::Z, Vec3A::X])
    }
}
