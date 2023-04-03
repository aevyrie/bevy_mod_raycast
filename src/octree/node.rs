use std::fmt::{Debug, Display};

use bevy::{
    math::Vec3A,
    reflect::{FromReflect, Reflect},
    render::primitives::Aabb,
};

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
#[derive(Clone, Copy, Default, Reflect, FromReflect)]
pub struct NodeMask {
    pub(super) children: u16,
}

impl NodeMask {
    /// The number of nodes held inside a parent node.
    pub const SLOTS: u8 = 8;

    pub fn children(&self) -> u16 {
        self.children
    }

    /// Pushes a child's node data into this mask.
    pub fn push_child(&mut self, child: NodeKind) {
        self.children <<= 2; // Make room for new child node entry
        self.children |= child as u16;
    }
}

impl Debug for NodeMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeMask {{ {self} }}")
    }
}

impl Display for NodeMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mask = format!("{:016b}", self.children);
        let mask_with_spaces = mask
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if i != 0 && i % 2 == 0 {
                    Some(' ')
                } else {
                    None
                }
                .into_iter()
                .chain(std::iter::once(c))
            })
            .collect::<String>();

        write!(f, "{mask_with_spaces}")
    }
}

pub enum NodeKind {
    /// An empty node that will become a dead end in the tree.
    Empty = 0,
    /// A node that contains more child nodes.
    Node = 1,
    /// A leaf node that contains only triangles.
    Leaf = 2,
}

#[derive(Clone, Debug, Default, Reflect, FromReflect)]
pub struct Leaf {
    pub(super) triangles: Vec<TriangleIndex>,
}

impl Leaf {
    pub fn new(triangles: Vec<TriangleIndex>) -> Self {
        Self { triangles }
    }

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
#[derive(Clone, Copy, Hash, PartialEq, Eq, Reflect, FromReflect)]
pub struct NodeAddr {
    pub(super) address: u32,
}

impl NodeAddr {
    pub fn new(address: u32) -> Self {
        Self { address }
    }

    pub const MAX_NODE_DEPTH: usize = 10;

    pub fn new_root() -> Self {
        Self { address: 0b10 }
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

    /// Converts this address to point to a leaf.
    pub fn to_leaf(self) -> Self {
        Self {
            address: self.address | 0b1,
        }
    }

    /// Converts this address to point to a node.
    pub fn to_node(self) -> Self {
        Self {
            address: self.address & 0b1_111_111_111_111_111_111_111_111_111_111_0,
        }
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
        let lead = self.address.leading_zeros();
        let address_bits = (32 - 2u32).saturating_sub(lead) as usize;
        address_bits / 3
    }
}

impl Debug for NodeAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeAddr {{ {self} }}")
    }
}

impl Display for NodeAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let addr = format!("{:032b}", self.address);
        let mut start_bit_found = None;
        let addr_with_spaces = addr
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if start_bit_found.is_none() && c == '1' {
                    start_bit_found = Some(i + 1);
                    if i == 0 {
                        None
                    } else {
                        Some(' ')
                    }
                } else if let Some(k) = start_bit_found {
                    let n = i - k;
                    if n % 3 == 0 {
                        Some(' ')
                    } else {
                        None
                    }
                } else if i != 0 && i % 3 == 0 {
                    Some(' ')
                } else {
                    None
                }
                .into_iter()
                .chain(std::iter::once(c))
            })
            .collect::<String>();

        write!(f, "{addr_with_spaces}")
    }
}

#[cfg(test)]
mod tests {
    use super::NodeAddr;

    #[test]
    fn depth() {
        let d3 = NodeAddr::new(0b_000_000_000_000_000_000_000_1_000_000_000_0).depth();
        assert_eq!(d3, 3);
        let d10 = NodeAddr::new(0b_1_000_000_000_000_000_000_000_000_000_000_1).depth();
        assert_eq!(d10, 10);
    }
}
