use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::HashMap,
};
use bvh::{
    aabb::{Bounded, AABB},
    bounding_hierarchy::BHShape,
    bvh::{BVHNode, BVH},
};
use serde::Deserialize;

pub struct MeshBvhPlugin;
impl Plugin for MeshBvhPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_mesh_bvh);
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct BvhMap(HashMap<Handle<Mesh>, Handle<MeshBvh>>);

pub fn update_mesh_bvh(
    meshes: Res<Assets<Mesh>>,
    mut bvhs: ResMut<Assets<MeshBvh>>,
    mut bvh_map: ResMut<BvhMap>,
    mut mesh_events: EventReader<AssetEvent<Mesh>>,
) {
    let mut update_bvhs = |event: &AssetEvent<Mesh>| -> Option<()> {
        let mesh_handle = match event {
            AssetEvent::Created { handle } => handle,
            AssetEvent::Modified { handle } => handle,
            AssetEvent::Removed { handle } => handle,
        };
        let mesh = meshes.get(mesh_handle)?;
        let new_bvh = mesh.try_into().ok()?;
        match bvh_map.get(mesh_handle) {
            Some(bvh_handle) => {
                let mesh_bvh = bvhs.get_mut(bvh_handle)?;
                *mesh_bvh = new_bvh;
            }
            None => {
                let bhv_handle = bvhs.add(new_bvh);
                bvh_map.insert(mesh_handle.clone(), bhv_handle);
            }
        }
        None
    };
    for event in mesh_events.iter() {
        update_bvhs(event);
    }
}

#[derive(Debug, Deserialize, TypeUuid, TypePath)]
#[uuid = "b006d707-dc37-4fa8-a4f9-66cef3f864c0"]
pub struct MeshBvh {
    bvh: BVH,
}

impl MeshBvh {
    /// Returns the index of the triangle with the AABB that was intersected by this ray.
    pub fn raycast(&self, ray: &Ray) -> Vec<usize> {
        let Ray { origin, direction } = ray;
        let ray = bvh::ray::Ray::new(
            [origin.x, origin.y, origin.z].into(),
            [direction.x, direction.y, direction.z].into(),
        );
        let mut indices = Vec::new();
        BVHNode::traverse_recursive(&self.bvh.nodes, 0, &ray, &mut indices);
        indices
    }
}

impl TryFrom<&Mesh> for MeshBvh {
    type Error = ();
    fn try_from(mesh: &Mesh) -> Result<Self, Self::Error> {
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .ok_or(())?
            .as_float3()
            .ok_or(())?;
        let positions: Vec<[f32; 3]> = if let Some(indices) = mesh.indices() {
            indices.iter().map(|i| positions[i]).collect()
        } else {
            positions.to_vec()
        };
        let mut tri_shapes = positions
            .chunks_exact(3)
            .map(|verts| {
                let aabb = AABB::empty()
                    .grow(&bvh::Point3::from(verts[0]))
                    .grow(&bvh::Point3::from(verts[1]))
                    .grow(&bvh::Point3::from(verts[2]));
                TriShape::new(aabb)
            })
            .collect::<Vec<_>>();
        let bvh = BVH::build(&mut tri_shapes);
        Ok(MeshBvh { bvh })
    }
}

pub struct TriShape {
    node_index: usize,
    aabb: AABB,
}

impl TriShape {
    pub fn new(aabb: AABB) -> Self {
        Self {
            node_index: 0,
            aabb,
        }
    }
}

impl Bounded for TriShape {
    fn aabb(&self) -> AABB {
        self.aabb
    }
}

impl BHShape for TriShape {
    fn set_bh_node_index(&mut self, new_value: usize) {
        self.node_index = new_value;
    }

    fn bh_node_index(&self) -> usize {
        self.node_index
    }
}
