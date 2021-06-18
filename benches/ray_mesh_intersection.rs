use bevy::math::{Mat4, Vec3};
use bevy_mod_raycast::Ray3d;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn ptoxznorm(p: u32, size: u32) -> (f32, f32) {
    let ij = (p / (size), p % (size));
    (ij.0 as f32 / size as f32, ij.1 as f32 / size as f32)
}

struct SimpleMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn mesh_creation(vertices_per_side: u32) -> SimpleMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for p in 0..vertices_per_side.pow(2) {
        let xz = ptoxznorm(p, vertices_per_side);
        positions.push([xz.0 - 0.5, 0.0, xz.1 - 0.5]);
        normals.push([0.0, 1.0, 0.0]);
    }

    let mut indices = vec![];
    for p in 0..vertices_per_side.pow(2) {
        if p % (vertices_per_side) != vertices_per_side - 1
            && p / (vertices_per_side) != vertices_per_side - 1
        {
            indices.extend_from_slice(&[p, p + 1, p + vertices_per_side]);
            indices.extend_from_slice(&[p + vertices_per_side, p + 1, p + vertices_per_side + 1]);
        }
    }

    SimpleMesh {
        positions,
        normals,
        indices,
    }
}

fn ray_mesh_intersection(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_mesh_intersection");
    group.warm_up_time(std::time::Duration::from_millis(500));

    for vertices_per_side in [10_u32, 100, 1000] {
        group.bench_function(format!("{}_vertices", vertices_per_side.pow(2)), |b| {
            let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
            let mesh_to_world = Mat4::IDENTITY;
            let mesh = mesh_creation(vertices_per_side);

            b.iter(|| {
                black_box(bevy_mod_raycast::ray_mesh_intersection(
                    &mesh_to_world,
                    &mesh.positions,
                    Some(&mesh.normals),
                    &ray,
                    Some(&mesh.indices),
                ));
            });
        });
    }
}

fn ray_mesh_intersection_no_intersection(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_mesh_intersection_no_intersection");
    group.warm_up_time(std::time::Duration::from_millis(500));

    for vertices_per_side in [10_u32, 100, 1000] {
        group.bench_function(format!("{}_vertices", vertices_per_side.pow(2)), |b| {
            let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
            let mesh_to_world = Mat4::IDENTITY;
            let mesh = mesh_creation(vertices_per_side);

            b.iter(|| {
                black_box(bevy_mod_raycast::ray_mesh_intersection(
                    &mesh_to_world,
                    &mesh.positions,
                    Some(&mesh.normals),
                    &ray,
                    Some(&mesh.indices),
                ));
            });
        });
    }
}

criterion_group!(
    benches,
    ray_mesh_intersection,
    ray_mesh_intersection_no_intersection
);
criterion_main!(benches);
