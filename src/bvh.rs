use std::collections::HashMap;

use bevy::{
    math::Vec3A,
    prelude::*,
    render::{mesh::Indices, primitives::Aabb},
};

use crate::{ray_triangle_intersection, IntersectionData, Triangle};

fn generate_mesh_bvh(mesh: &Mesh) {
    // starting at the top level
    // split into 8 leaf nodes
    // triangles that are at least partially inside a leaf are counted in that leaf
    // only keep leaves that contain triangles
    // continue this process down to some N subdivision levels
    //
    //
    // represent nodes with a bitmask? u8 per bit, where each bit is a level
    //
    //
}

/// Makes it easier to get triangle data out of a mesh
pub struct MeshAccessor<'a> {
    verts: &'a [[f32; 3]],
    indices: Option<&'a Indices>,
}

impl<'a> MeshAccessor<'a> {
    pub fn from_mesh(mesh: &'a Mesh) -> Self {
        // Get the vertex positions from the mesh reference resolved from the mesh handle
        let verts: &'a [[f32; 3]] = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            None => panic!("Mesh does not contain vertex positions"),
            Some(vertex_values) => match &vertex_values {
                bevy::render::mesh::VertexAttributeValues::Float32x3(positions) => positions,
                _ => panic!("Unexpected types in {:?}", Mesh::ATTRIBUTE_POSITION),
            },
        };

        Self {
            verts,
            indices: mesh.indices(),
        }
    }

    // Get the triangle vertices at the given `index`.
    pub fn triangle(&self, index: TriangleIndex) -> Triangle {
        let index = index as usize;
        let data = match self.indices {
            Some(indices) => match indices {
                Indices::U16(indices) => [
                    self.verts[indices[index * 3] as usize],
                    self.verts[indices[index * 3 + 1] as usize],
                    self.verts[indices[index * 3 + 2] as usize],
                ],
                Indices::U32(indices) => [
                    self.verts[indices[index * 3] as usize],
                    self.verts[indices[index * 3 + 1] as usize],
                    self.verts[indices[index * 3 + 2] as usize],
                ],
            },
            None => [
                self.verts[index * 3],
                self.verts[index * 3 + 1],
                self.verts[index * 3 + 2],
            ],
        };
        Triangle {
            v0: data[0].into(),
            v1: data[1].into(),
            v2: data[2].into(),
        }
    }
}

pub struct MeshOctree {
    nodes: HashMap<NodeAddr, Node>,
    leaves: HashMap<NodeAddr, Leaf>,
}

impl MeshOctree {
    pub fn cast_ray(
        &self,
        ray: Ray,
        mesh: &Mesh,
        mesh_aabb: Aabb,
        mesh_transform: &GlobalTransform,
    ) -> Option<IntersectionData> {
        let world_to_mesh = mesh_transform.compute_matrix().inverse();

        let mesh_space_ray = Ray {
            origin: world_to_mesh.transform_point3(ray.origin),
            direction: world_to_mesh.transform_vector3(ray.direction).normalize(),
        };

        let mesh = MeshAccessor::from_mesh(mesh);

        let root_address = NodeAddr::new();
        let ordered_nodes = Self::node_intersect_order(mesh_space_ray);
        let mut op_stack: Vec<NodeAddr> = vec![root_address];

        while let Some(current_node_addr) = op_stack.pop() {
            if current_node_addr.is_leaf() {
                let current_leaf = self
                    .leaves
                    .get(&current_node_addr)
                    .expect("Malformed mesh octree, leaf address does not exist.");

                let mut hits = Vec::new();
                for triangle_index in current_leaf.triangles {
                    let triangle = mesh.triangle(triangle_index);
                    if let Some(hit) =
                        ray_triangle_intersection(&ray, &triangle, crate::Backfaces::Cull)
                    {
                        hits.push(hit);
                    }
                }
            } else {
                self.expand_child_nodes(current_node_addr, ordered_nodes, &mut op_stack);
            }
        }

        None
    }

    fn expand_child_nodes(
        &self,
        node_addr: NodeAddr,
        node_order: [u8; 8],
        op_stack: &mut Vec<NodeAddr>,
    ) {
        let current_node = self
            .nodes
            .get(&node_addr)
            .expect("Malformed mesh octree, node address does not exist.");

        // TODO: this can probably be SIMD'd. Raycast against all node AABBs to generate a u8
        // bitmask where 1's are nodes that were hit. This can be AND'ed with the bitmask of nodes
        // that contain children (node or leaf). The results of this are added in the correct order
        // to the stack. Nodes are popped off the stack and expanded like this, into multiple nodes
        // and leaves, added in order that the node would be hit by the ray. When a leaf node is
        // popped off the stack, a raycast is run on all tris in the leaf. If it hits, we are done.
        // If not, we continue evaluating the stack.
        //
        // Note that the stack is just a list of node addresses. We can tell if they are a leaf or a
        // node based on the encoding.

        // Iterate in reverse order because we are pushing to the stack, and want the first of the
        // ordered nodes to be the next one that is popped off the stack. Consequently, it must be
        // pushed last.
        node_order.iter().rev().for_each(|i| {
            // Bit shift to move the current node's bits to the rightmost spot
            let shifted_node_data = current_node.children >> i * 2;
            // Mask the bits to get just this node's bits
            let child_node_data = shifted_node_data & 0b11;

            match child_node_data {
                Node::EMPTY => {}
                Node::NODE => op_stack.push(node_addr.push_bits(*i, false)),
                Node::LEAF => op_stack.push(node_addr.push_bits(*i, true)),
                _ => unreachable!(), // We control the encoding.
            }
        });
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
    fn node_intersect_order(ray: Ray) -> [u8; 8] {
        let ray_dir = ray.direction.into();
        // dot product to project points onto ray
        // The lower the number, the earlier the node will be hit
        let mut distances: Vec<_> = (0..8u8)
            .map(|i| {
                let (x, y, z) = (i & 0b100, i & 0b010, i & 0b001);
                let node_vec = Vec3A::new(x as f32, y as f32, z as f32);
                let dist = node_vec.dot(ray_dir);
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

/// An address that uniquely describes a node in an octree as a list of triplets and some metadata.
///
/// Each triplet represents the XYZ position of the node at that level in the octree. A value of `0`
/// means the node is located toward the origin, while a value of 1 means the node is located away
/// from the origin.
///
/// ```text
///    2----6       010--110      Y
/// 3----7  |    011--111 |       |
/// |  0-|--4     | 000|-100      o---X   
/// 1----5       001--101       Z
///
///  Decimal        Binary       Coords
/// ```
///
/// Note (x0, y1, z1) = 011 (binary) = 3 (decimal).
///
/// ### Encoding
///
/// ```text
/// XYZ XYZ XYZ etc...                      leftover bits
/// 000 000 000 000 000 000 000 000 000 000 xx
/// ```
///
/// This 32-bit integer allows for 10 levels of subdivision (`3 * 10 = 30`), with 2 bits left over.
///
/// How do you differentiate between nodes of different depth? We start each address with a 1. E.g.
/// all of the zeros following the initial 1 are significant:
///
/// ```text
/// 1 000 000 000 000 000 000 000 000 000 000 x -> depth-10
/// 000 000 000 000 000 000 000 1 000 000 000 x -> depth-3
/// ```
///
/// How do we know if the address is for a node or a leaf? We use the very last bit.
///
/// - Node = 0
/// - Leaf = 1
///
/// ```text
/// 1 000 000 000 000 000 000 000 000 000 000 1 -> depth-10 leaf
/// 000 000 000 000 000 000 000 1 000 000 000 0 -> depth-3 node
/// ```
///
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct NodeAddr {
    address: u32,
}

impl NodeAddr {
    pub fn new() -> Self {
        Self { address: 1u32 }
    }

    /// Push the 3 bits representing a child of this address onto this address, to produce the full
    /// address of the child node. The last bit, which signifies whether the address points to a
    /// leaf or node, is added to the end.
    pub fn push_bits(&self, bits: u8, leaf: bool) -> Self {
        let mut bits = (bits & 0b111) as u32; // mask off all but the last 3 bits of the input
        bits <<= 1; // shift one to the left to make room for the leaf bit
        bits |= leaf as u32; // OR the leaf bit onto the end, `bits` now has 4 bits of data
        let mut new = *self;
        new.address >>= 1; // push the node/leaf bit off
        new.address <<= 4; // shift address left by four bits, the right four bits are now zeros
        new.address |= bits; // add the bits to the end of the address by using the OR operator
        new
    }

    pub fn is_leaf(&self) -> bool {
        self.address & 1 == 1
    }
}

/// The node address of each child cam be computed contextually to significantly reduce the size of
/// the type.
///
/// ```text
///   2━━━━━━6
///  ╱│     ╱│
/// 3━━━━━━7 │
/// │ 0━━━━│━4
/// │╱     │╱
/// 1━━━━━━5
///
/// octree cell:  7   6  5  4  3  2  1  0
/// node data     00  00 00 00 00 00 00 00  (u16)
///
/// 00 -> empty
/// 01 -> contains a node
/// 10 -> contains a leaf
/// 11 -> unused (could use if contains node and leaf?)
/// ```
///
/// To compute the next address, take the octree cell, and push it onto the `NodeAddr`.
///
/// Note that this takes 3 bits. `000 == 0`, `111 == 7`.
struct Node {
    children: u16,
}

impl Node {
    pub const EMPTY: u16 = 0b00;
    pub const NODE: u16 = 0b01;
    pub const LEAF: u16 = 0b10;

    fn new(children: u16) -> Self {
        Self { children }
    }
}

struct Leaf {
    triangles: Vec<TriangleIndex>,
}

type TriangleIndex = u32;

// bool intersection(box b, ray r) {
//     double tx1 = (b.min.x - r.x0.x)*r.n_inv.x;
//     double tx2 = (b.max.x - r.x0.x)*r.n_inv.x;

//     double tmin = min(tx1, tx2);
//     double tmax = max(tx1, tx2);

//     double ty1 = (b.min.y - r.x0.y)*r.n_inv.y;
//     double ty2 = (b.max.y - r.x0.y)*r.n_inv.y;

//     tmin = max(tmin, min(ty1, ty2));
//     tmax = min(tmax, max(ty1, ty2));

//     return tmax >= tmin;
// }
