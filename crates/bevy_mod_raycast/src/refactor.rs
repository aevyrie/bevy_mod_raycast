use bevy::{prelude::*, render::mesh::Indices};

use crate::{ray_triangle_intersection, Backfaces, IntersectionData};

fn get_vert(mesh: &Mesh, index: usize) -> Option<&[f32; 3]> {
    let positions = &mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
    if let Some(indices) = mesh.indices() {
        let index = match indices {
            Indices::U16(i) => *i.get(index)? as usize,
            Indices::U32(i) => *i.get(index)? as usize,
        };
        positions.get(index)
    } else {
        positions.get(index)
    }
}

pub fn get_tri(mesh: &Mesh, index: usize) -> Option<[&[f32; 3]; 3]> {
    let pos = &mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
    Some([
        get_vert(mesh, index * 3)?,
        get_vert(mesh, index * 3 + 1)?,
        get_vert(mesh, index * 3 + 2)?,
    ])
}

// fn triangle_intersection(
//     tri_vertices: [Vec3A; 3],
//     tri_normals: Option<[Vec3A; 3]>,
//     max_distance: f32,
//     ray: Ray3d,
//     backface_culling: Backfaces,
// ) -> Option<IntersectionData> {
//     if tri_vertices
//         .iter()
//         .any(|&vertex| (vertex - ray.origin).length_squared() < max_distance.powi(2))
//     {
//         // Run the raycast on the ray and triangle
//         if let Some(ray_hit) = ray_triangle_intersection(&ray, &tri_vertices, backface_culling) {
//             let distance = *ray_hit.distance();
//             if distance > 0.0 && distance < max_distance {
//                 let position = ray.position(distance);
//                 let normal = if let Some(normals) = tri_normals {
//                     let u = ray_hit.uv_coords().0;
//                     let v = ray_hit.uv_coords().1;
//                     let w = 1.0 - u - v;
//                     normals[1] * u + normals[2] * v + normals[0] * w
//                 } else {
//                     (tri_vertices.v1() - tri_vertices.v0())
//                         .cross(tri_vertices.v2() - tri_vertices.v0())
//                         .normalize()
//                 };
//                 let intersection = IntersectionData::new(
//                     position,
//                     normal.into(),
//                     distance,
//                     Some(tri_vertices.to_triangle()),
//                 );
//                 return Some(intersection);
//             }
//         }
//     }
//     None
// }
