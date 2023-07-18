#![allow(unused)]

use std::marker::PhantomData;

use bevy::{prelude::*, reflect::TypePath};

use crate::{RaycastMesh, RaycastSource};

/// Updates the 3d cursor to be in the pointed world coordinates
#[allow(clippy::too_many_arguments)]
pub fn update_debug_cursor<T: TypePath + Send + Sync>(
    mut commands: Commands,
    mut meshes: Query<&RaycastSource<T>>,
    mut gizmos: Gizmos,
) {
    for (_, intersection) in meshes.iter().flat_map(|m| m.intersections()) {
        gizmos.ray(intersection.position(), intersection.normal(), Color::GREEN);
        gizmos.circle(
            intersection.position(),
            intersection.normal(),
            0.1,
            Color::GREEN,
        );
    }
}

pub fn print_intersections<T: TypePath + Send + Sync>(query: Query<&RaycastMesh<T>>) {
    for (_, intersection) in query.iter().flat_map(|mesh| mesh.intersections.iter()) {
        info!(
            "Distance {:?}, Position {:?}",
            intersection.distance(),
            intersection.position()
        );
    }
}
