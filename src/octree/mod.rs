use std::{collections::HashMap, hash::BuildHasherDefault};

use bevy::{
    math::Vec3A,
    prelude::{info, GlobalTransform, Mesh},
    reflect::Reflect,
    render::primitives::Aabb,
    utils::Instant,
};
use nohash_hasher::NoHashHasher;

use crate::{ray_triangle_intersection, IntersectionData, Ray3d};

pub mod mesh_accessor;
pub mod node;

use mesh_accessor::*;
use node::*;

#[derive(Debug, Clone, Reflect)]
pub struct MeshOctree {
    aabb: Aabb,
    nodes: HashMap<NodeAddr, NodeMask, BuildHasherDefault<NoHashHasher<u32>>>,
    leaves: HashMap<NodeAddr, Leaf, BuildHasherDefault<NoHashHasher<u32>>>,
}

impl MeshOctree {
    /// A node containing `<= LEAF_TRI_CUTOFF` triangles will become a leaf node.
    pub const LEAF_TRI_CUTOFF: usize = 8;

    /// Build an octree from this mesh. This can take a significant amount time depending on mesh
    /// complexity, and should not be run on the main thread.
    pub fn build(mesh: &Mesh) -> Result<Self, OctreeError> {
        let mesh = MeshAccessor::from_mesh(mesh);
        Self::from_mesh_accessor(&mesh)
    }

    pub fn from_mesh_accessor(mesh: &MeshAccessor) -> Result<Self, OctreeError> {
        let start = Instant::now();
        let mut octree_builder = OctreeBuildData::from_mesh(mesh);
        let aabb = octree_builder.aabb;

        while let Some(stack_entry) = octree_builder.pop_stack() {
            let mut this_node = NodeMask::default();
            (0..NodeMask::SLOTS)
                .rev() // Needed because we build up the mask by pushing onto the right side
                .map(|i| stack_entry.build_child_from_intersecting_tris(i, &mesh, &aabb))
                .map(|child_entry: NodeStackEntry| octree_builder.consume_child_data(child_entry))
                .for_each(|child| this_node.push_child(child));

            octree_builder.insert_node(stack_entry.address, this_node);
        }

        let elapsed = start.elapsed().as_secs_f32();
        info!("{elapsed:#?}");

        Ok(octree_builder.into_octree())
    }

    /// Cast a ray into the [`MeshOctree`] acceleration structure, returning [`IntersectionData`] if
    /// the ray intersects with a triangle in the mesh.
    pub fn cast_ray(
        &self,
        ray: Ray3d,
        mesh: &Mesh,
        mesh_transform: &GlobalTransform,
    ) -> Option<IntersectionData> {
        let world_to_mesh = mesh_transform.compute_matrix().inverse();

        // Convert ray into mesh space
        let ray = Ray3d::new(
            world_to_mesh.transform_point3(ray.origin.into()),
            world_to_mesh.transform_vector3(ray.direction.into()),
        );

        let mesh = MeshAccessor::from_mesh(mesh);
        self.cast_ray_local(ray, mesh)
    }

    fn cast_ray_local(&self, ray: Ray3d, mesh: MeshAccessor) -> Option<IntersectionData> {
        let root_address = NodeAddr::new_root();
        let node_order = Self::node_intersect_order(ray);
        let mut op_stack: Vec<NodeAddr> = Vec::with_capacity(8);
        op_stack.push(root_address);

        while let Some(node_addr) = op_stack.pop() {
            if node_addr.is_leaf() {
                if let Some(value) = self.leaf_raycast(node_addr, &mesh, ray) {
                    return Some(value);
                }
            } else {
                for address in self.expand_child_nodes(node_addr, &node_order, ray) {
                    op_stack.push(address);
                }
            }
        }

        None
    }

    /// Raycast against the triangles in this leaf. This does **not** do a ray-box intersection test
    /// against the leaf's AABB.
    #[inline]
    fn leaf_raycast(
        &self,
        leaf_addr: NodeAddr,
        mesh: &MeshAccessor,
        ray: Ray3d,
    ) -> Option<IntersectionData> {
        let current_leaf = self.leaves.get(&leaf_addr).expect(&format!(
            "Malformed mesh octree, leaf address {leaf_addr} does not exist.\n{self:#?}"
        ));
        let mut hits = Vec::new();
        for &triangle_index in current_leaf.triangles() {
            let triangle = mesh.get_triangle(triangle_index).expect(&format!(
                "Malformed mesh indices, triangle index {triangle_index} does not exist."
            ));
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
        ray: Ray3d,
    ) -> impl Iterator<Item = NodeAddr> + 'a {
        let current_node = self
            .nodes
            .get(&node_addr)
            .expect("Malformed mesh octree, node address does not exist.");

        node_order
            .iter()
            .filter_map(move |i| {
                dbg!(i);
                let shifted = current_node.children() >> i * 2; // Shift children to rightmost spot
                let child_state = shifted & 0b11; // Mask all but these two child bits
                match child_state {
                    x if x == NodeKind::Empty as u16 => None,
                    x if x == NodeKind::Node as u16 => Some(node_addr.push_bits(*i, false)),
                    x if x == NodeKind::Leaf as u16 => Some(node_addr.push_bits(*i, true)),
                    _ => unreachable!("Malformed octree node"),
                }
            })
            .filter_map(move |child_addr| {
                let child_aabb = child_addr.compute_aabb(&self.aabb);
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

pub struct OctreeBuildData {
    aabb: Aabb,
    nodes: HashMap<NodeAddr, NodeMask, BuildHasherDefault<NoHashHasher<u32>>>,
    leaves: HashMap<NodeAddr, Leaf, BuildHasherDefault<NoHashHasher<u32>>>,
    node_stack: Vec<NodeStackEntry>,
}

impl OctreeBuildData {
    fn insert_leaf(&mut self, node: NodeStackEntry) {
        self.leaves
            .insert(node.address.to_leaf(), Leaf::new(node.triangles));
    }

    fn insert_node(&mut self, address: NodeAddr, node: NodeMask) {
        self.nodes.insert(address.to_node(), node);
    }

    fn push_stack(&mut self, node: NodeStackEntry) {
        self.node_stack.push(node);
    }

    fn pop_stack(&mut self) -> Option<NodeStackEntry> {
        self.node_stack.pop()
    }

    /// Evaluate this child to determine if it is empty, a leaf, or a node.
    ///
    /// A leaf will push its triangles into the leaf hashmap.
    /// A node will push itself onto the stack to be further evaluated.
    ///
    /// Returns the type of node of the `child_node`.
    #[inline]
    fn consume_child_data(&mut self, child: NodeStackEntry) -> NodeKind {
        let triangle_cutoff_reached = child.triangles.len() <= MeshOctree::LEAF_TRI_CUTOFF;
        let octree_depth_limit_reached = child.address.depth() >= NodeAddr::MAX_NODE_DEPTH;

        if child.triangles.len() == 0 {
            NodeKind::Empty
        } else if triangle_cutoff_reached || octree_depth_limit_reached {
            self.insert_leaf(child);
            NodeKind::Leaf
        } else {
            self.push_stack(child);
            NodeKind::Node
        }
    }

    pub fn from_mesh(mesh: &MeshAccessor) -> Self {
        let root_node = NodeAddr::new_root();
        let root_tris = mesh.iter_triangles().collect::<Vec<_>>();
        Self {
            aabb: mesh.generate_aabb(),
            nodes: HashMap::with_hasher(BuildHasherDefault::default()),
            leaves: HashMap::with_hasher(BuildHasherDefault::default()),
            node_stack: vec![NodeStackEntry::new(root_node, root_tris)],
        }
    }

    pub fn into_octree(self) -> MeshOctree {
        MeshOctree {
            aabb: self.aabb,
            nodes: self.nodes,
            leaves: self.leaves,
        }
    }
}

/// An entry in the [`OctreeBuildData`]'s stack. Data is popped on and off the stack in the process
/// of building the octree instead of using a recursive algorithm.
pub struct NodeStackEntry {
    pub(crate) address: NodeAddr,
    pub(crate) triangles: Vec<TriangleIndex>,
}

impl NodeStackEntry {
    pub fn new(address: NodeAddr, triangles: Vec<TriangleIndex>) -> Self {
        Self { address, triangles }
    }

    pub fn triangles(&self) -> impl Iterator<Item = TriangleIndex> + '_ {
        self.triangles.iter().copied()
    }

    /// Sets the address tp point to a leaf.
    pub fn to_leaf(self) -> Self {
        Self {
            address: self.address.to_leaf(),
            triangles: self.triangles,
        }
    }

    /// Sets the address tp point to a node.
    pub fn to_node(self) -> Self {
        Self {
            address: self.address.to_node(),
            triangles: self.triangles,
        }
    }

    /// Get a list of the triangles that intersect this node's AABB.
    #[inline]
    pub fn build_child_from_intersecting_tris(
        &self,
        octree_node: u8,
        mesh: &MeshAccessor,
        mesh_aabb: &Aabb,
    ) -> NodeStackEntry {
        let child_addr = self.address.push_bits(octree_node, false);
        let child_tris = self
            .triangles()
            .filter(|tri_index| {
                let Some(triangle) = mesh.get_triangle(*tri_index) else {
                    return false
                };
                let aabb = child_addr.compute_aabb(mesh_aabb);
                triangle.intersects_aabb(aabb)
            })
            .collect();
        NodeStackEntry::new(child_addr, child_tris)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OctreeError {
    InvalidAabb,
    MeshLargerThanAabb,
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec3;

    use crate::Ray3d;

    use super::{
        mesh_accessor::{self},
        MeshOctree,
    };

    #[test]
    fn intersection() {
        let mesh = mesh_accessor::test_util::build_vert_only_xz_quad();
        let octree = dbg!(MeshOctree::from_mesh_accessor(&mesh).unwrap());

        let ray = Ray3d::new(-Vec3::Y, Vec3::Y);
        let intersection = octree.cast_ray_local(ray, mesh).unwrap();
        assert_eq!(intersection.distance(), 1.0)
    }
}
