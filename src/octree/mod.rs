use std::{collections::HashMap, hash::BuildHasherDefault};

use bevy::{
    math::Vec3A,
    prelude::{GlobalTransform, Mesh, Vec3},
    render::{
        mesh::{Indices, VertexAttributeValues},
        primitives::Aabb,
    },
};
use nohash_hasher::NoHashHasher;

use crate::{ray_triangle_intersection, IntersectionData, Ray3d, RayHit, Triangle};

use node::*;
pub mod node;

pub struct MeshOctree {
    nodes: HashMap<NodeAddr, Node, BuildHasherDefault<NoHashHasher<u32>>>,
    leaves: HashMap<NodeAddr, Leaf, BuildHasherDefault<NoHashHasher<u32>>>,
}

impl MeshOctree {
    pub const MIN_TRIS_PER_LEAF: usize = 8;

    pub fn from_mesh(mesh: &Mesh, aabb: Aabb) -> Self {
        let mut nodes = HashMap::with_hasher(BuildHasherDefault::default());
        let mut leaves = HashMap::with_hasher(BuildHasherDefault::default());
        let mesh = MeshAccessor::from_mesh(mesh);
        let mut op_stack = Vec::new();

        let root_tris = (
            NodeAddr::new_root(),
            mesh.iter_triangles().collect::<Vec<_>>(),
        );
        op_stack.push(root_tris);

        while let Some((parent_addr, parent_tris)) = op_stack.pop() {
            let mut parent_node = Node::default();
            (0..Node::N_CHILD_NODES)
                .map(|i| triangle_node_intersections(i, parent_addr, &parent_tris, &mesh, &aabb))
                .for_each(|(child_addr, child_tris)| {
                    evaluate_child_node(
                        child_addr,
                        child_tris,
                        &mut parent_node,
                        &mut leaves,
                        &mut op_stack,
                    );
                });

            nodes.insert(parent_addr, parent_node);
        }

        Self { nodes, leaves }
    }

    pub fn cast_ray(
        &self,
        ray: Ray3d,
        mesh: &Mesh,
        mesh_aabb: Aabb,
        mesh_transform: &GlobalTransform,
    ) -> Option<IntersectionData> {
        let world_to_mesh = mesh_transform.compute_matrix().inverse();

        let mesh_space_ray = Ray3d::new(
            world_to_mesh.transform_point3(ray.origin.into()),
            world_to_mesh.transform_vector3(ray.direction.into()),
        );

        let mesh = MeshAccessor::from_mesh(mesh);
        let root_address = NodeAddr::new_root();
        let node_order = Self::node_intersect_order(mesh_space_ray);
        let mut op_stack: Vec<NodeAddr> = Vec::with_capacity(16);
        op_stack.push(root_address);

        while let Some(node_addr) = op_stack.pop() {
            if node_addr.is_leaf() {
                if let Some(value) = self.leaf_raycast(node_addr, &mesh, ray) {
                    return Some(value);
                }
            } else {
                for address in self.expand_child_nodes(node_addr, &node_order, mesh_aabb, ray) {
                    op_stack.push(address);
                }
            }
        }

        None
    }

    #[inline]
    fn leaf_raycast(
        &self,
        current_node_addr: NodeAddr,
        mesh: &MeshAccessor,
        ray: Ray3d,
    ) -> Option<IntersectionData> {
        let current_leaf = self
            .leaves
            .get(&current_node_addr)
            .expect("Malformed mesh octree, leaf address does not exist.");
        let mut hits = Vec::new();
        for &triangle_index in current_leaf.triangles() {
            let triangle = mesh
                .get_triangle(triangle_index)
                .expect("Malformed mesh indices, triangle address does not exist.");
            if let Some(hit) = ray_triangle_intersection(&ray, &triangle, crate::Backfaces::Cull) {
                if hit.distance() <= 0.0 {
                    hits.push(IntersectionData::new(
                        ray.position(hit.distance()),
                        mesh.intersection_normal(triangle_index, hit),
                        hit.distance(),
                        Some(triangle),
                    ));
                }
            }
        }
        hits.sort_by(|a, b| {
            a.distance()
                .partial_cmp(&b.distance())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(hit) = hits.drain(..).next() {
            return Some(hit);
        };
        None
    }

    /// Expands the children of this node, returning an iterator over non-empty child addresses from
    /// farthest to nearest to the ray. Children are returned in this order so they can be pushed
    /// onto the stack sequentially with the nearest child being the last item on the stack.
    #[inline]
    fn expand_child_nodes<'a>(
        &'a self,
        node_addr: NodeAddr,
        node_order: &'a [u8; 8],
        mesh_aabb: Aabb,
        ray: Ray3d,
    ) -> impl Iterator<Item = NodeAddr> + 'a {
        let current_node = self
            .nodes
            .get(&node_addr)
            .expect("Malformed mesh octree, node address does not exist.");

        node_order
            .iter()
            .filter_map(move |i| {
                let shifted = current_node.children() >> i * 2; // Shift child bits to rightmost spot
                let child_state = shifted & 0b11; // Mask all but these two child bits
                match child_state {
                    Node::EMPTY => None,
                    Node::NODE => Some(node_addr.push_bits(*i, false)),
                    Node::LEAF => Some(node_addr.push_bits(*i, true)),
                    _ => unreachable!("Malformed octree node"),
                }
            })
            .filter_map(move |child_addr| {
                let child_aabb = child_addr.compute_aabb(&mesh_aabb);
                ray.intersects_local_aabb(&child_aabb).map(|_| child_addr)
            })
            .rev() // Reverse the order - see method docs.
    }

    /// Given a ray, returns the order that the nodes of the octree will be intersected with,
    /// regardless of the octree's subdivision level. This is simply a matter of viewing the cell
    /// centroids from the point of view of the ray (a projection).
    ///
    /// No matter how deep you are in the octree's hierarchy, the orientation of the octree relative
    /// to the ray is constant. In other words, when traversing through the eight cells of an
    /// octree, a ray will always intersect a near cell before a far cell. No matter where a
    /// triangle is located inside a cell `A`, if it is in front of another cell `B`, **any**
    /// intersections inside `A` will happen before any intersections in cell `B`.
    fn node_intersect_order(ray: Ray3d) -> [u8; 8] {
        // dot product to project points onto ray
        // The lower the number, the earlier the node will be hit
        let mut distances: Vec<_> = (0..8u8)
            .map(|i| {
                let (x, y, z) = (i & 0b100, i & 0b010, i & 0b001);
                let node_vec = Vec3A::new(x as f32, y as f32, z as f32);
                let dist = node_vec.dot(ray.direction);
                match dist.is_finite() {
                    true => (dist, i),
                    false => (f32::MAX, i),
                }
            })
            .collect();
        // Can unwrap because we've ensured `dist` is finite
        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let order: Vec<u8> = distances.iter().map(|(_, i)| *i).collect();
        order.try_into().unwrap()
    }
}

/// Evaluate this child node to determine if it is empty, a leaf, or another node.
///
/// A leaf will push its triangles into the leaf hashmap.
/// A node will push itself onto the stack to be further evaluated.
#[inline]
fn evaluate_child_node(
    child_addr: NodeAddr,
    triangles: Vec<TriangleIndex>,
    parent_node: &mut Node,
    leaves: &mut HashMap<NodeAddr, Leaf, BuildHasherDefault<NoHashHasher<u32>>>,
    op_stack: &mut Vec<(NodeAddr, Vec<u32>)>,
) {
    let parent_bits = &mut parent_node.children;
    *parent_bits <<= 2; // Make room for new child node entry

    if triangles.len() == 0 {
        *parent_bits |= Node::EMPTY;
    } else if triangles.len() <= MeshOctree::MIN_TRIS_PER_LEAF
        || child_addr.depth() >= NodeAddr::MAX_NODE_DEPTH
    {
        leaves.insert(child_addr, Leaf { triangles });
        *parent_bits |= Node::LEAF;
    } else {
        op_stack.push((child_addr, triangles));
        *parent_bits |= Node::NODE;
    }
}

/// Get a list of the parent triangles that intersect with this node
#[inline]
fn triangle_node_intersections(
    octree_node: u8,
    parent_addr: NodeAddr,
    parent_tris: &[TriangleIndex],
    mesh: &MeshAccessor,
    mesh_aabb: &Aabb,
) -> (NodeAddr, Vec<TriangleIndex>) {
    let child_addr = parent_addr.push_bits(octree_node, false);
    let child_tris = parent_tris
        .iter()
        .copied()
        .filter(|tri| {
            mesh.get_triangle(*tri).iter().any(|triangle| {
                let aabb = child_addr.compute_aabb(mesh_aabb);
                triangle.intersects_aabb(aabb)
            })
        })
        .collect();
    (child_addr, child_tris)
}

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
                        self.verts[indices[index * 3] as usize],
                        self.verts[indices[index * 3 + 1] as usize],
                        self.verts[indices[index * 3 + 2] as usize],
                    ]
                }
                Indices::U32(indices) => {
                    if indices.len() <= index * 3 + 2 {
                        return None;
                    }
                    [
                        self.verts[indices[index * 3] as usize],
                        self.verts[indices[index * 3 + 1] as usize],
                        self.verts[indices[index * 3 + 2] as usize],
                    ]
                }
            },
            None => [
                self.verts[index * 3],
                self.verts[index * 3 + 1],
                self.verts[index * 3 + 2],
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
}
