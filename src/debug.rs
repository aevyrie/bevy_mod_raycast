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
    for (is_first, intersection) in meshes.iter().flat_map(|m| {
        m.intersections()
            .iter()
            .map(|i| i.1.clone())
            .enumerate()
            .map(|(i, hit)| (i == 0, hit))
    }) {
        let color = match is_first {
            true => Color::GREEN,
            false => Color::PINK,
        };
        gizmos.ray(intersection.position(), intersection.normal(), color);
        gizmos.circle(intersection.position(), intersection.normal(), 0.1, color);
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
