use bevy::{math::Vec3A, render::primitives::Aabb};

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
/// octree cell:  7  6  5  4  3  2  1  0
/// node data     00 00 00 00 00 00 00 00  (u16)
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
#[derive(Clone, Copy, Debug, Default)]
pub struct Node {
    pub(super) children: u16,
}

impl Node {
    /// An empty node that will become a dead end in the tree.
    pub const EMPTY: u16 = 0b00;

    /// A node that contains more child nodes.
    pub const NODE: u16 = 0b01;

    /// A leaf node that contains only triangles.
    pub const LEAF: u16 = 0b10;

    /// The number of nodes held inside a parent node.
    pub const N_CHILD_NODES: u8 = 8;

    pub fn children(&self) -> u16 {
        self.children
    }
}

#[derive(Clone, Debug, Default)]
pub struct Leaf {
    pub(super) triangles: Vec<TriangleIndex>,
}

impl Leaf {
    pub fn triangles(&self) -> &[u32] {
        self.triangles.as_ref()
    }
}

pub type TriangleIndex = u32;

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
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct NodeAddr {
    pub(super) address: u32,
}

impl NodeAddr {
    pub const MAX_NODE_DEPTH: usize = 10;

    pub fn new_root() -> Self {
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

    /// Compute the mesh space AABB of this address given the mesh's AABB
    pub fn compute_aabb(&self, mesh_aabb: &Aabb) -> Aabb {
        let mut aabb = *mesh_aabb;
        let mut addr = self.address;
        addr >>= 1; // push the leaf bit off
        for _ in 0..Self::MAX_NODE_DEPTH {
            if addr == 1 {
                break; // When all bits are popped off the front, the address will just be 1
            }
            let (x, y, z) = (addr & 0b100, addr & 0b010, addr & 0b001);
            // Create an offset multiplier from -1 to 1
            let offset = Vec3A::new(x as f32, y as f32, z as f32) * 2.0 - 1.0;
            aabb.half_extents /= 2.0;
            aabb.center += aabb.half_extents * offset;
            addr >>= 3; // Push the last XYZ triplet off
        }
        aabb
    }

    /// The number of octree levels deep this address points to.
    pub fn depth(&self) -> usize {
        let lead = self.address.leading_zeros() as usize;
        let address_bits = 32 - 2 - lead;
        address_bits / 3
    }
}
