# Ray Casting for Bevy


[![CI](https://github.com/aevyrie/bevy_mod_raycast/workflows/CI/badge.svg?branch=master)](https://github.com/aevyrie/bevy_mod_raycast/actions?query=workflow%3A%22CI%22+branch%3Amaster)
[![crates.io](https://img.shields.io/crates/v/bevy_mod_raycast)](https://crates.io/crates/bevy_mod_raycast)
[![docs.rs](https://docs.rs/bevy_mod_raycast/badge.svg)](https://docs.rs/bevy_mod_raycast)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)

A [Bevy](https://github.com/bevyengine/bevy) plugin for 3D ray casting against meshes. Used to build [`bevy_mod_picking`](https://github.com/aevyrie/bevy_mod_picking). Contributions welcome!

![ezgif-2-69b8dc941be6](https://user-images.githubusercontent.com/2632925/121757699-420d2500-cad3-11eb-8d1d-ac697fc1ed21.gif)

## Uses

This plugin makes it simple to create ray casting sources, such as a transform (first person, third person shooter), or screenspace coordinates (mouse picking). Rays are shot from these sources every frame using a bevy system, and the intersections are stored in the ray casting source's component. 

- Only meshes that you mark with a component will be checked for intersections. 
- You can define which ray casting source(s) should interact with which mesh(es) by marking grouped sources and targets with the same type. 
- This plugin also provides some functionality to compute the intersection of rays with primitive shapes.
- Acceleration is provided using Bevy's AABBs and visibility culling.

## Bevy Version Support

I intend to track the `main` branch of Bevy. PRs supporting this are welcome! 

|bevy|bevy_mod_raycast|
|---|---|
|0.7|0.4|
|0.6|0.3|
|0.5|0.2|
|0.4|0.1|

## Examples

Mouse picking using a ray cast built using screen space coordinates:

```shell
cargo run --example mouse_picking
```

Ray casting from a camera using ray casts from the camera entity's GlobalTransform:

```shell
cargo run --example minimal
```

*Optimization* Mouse picking over many meshes using AABBs:

```shell
cargo run --example bounding_volume
```

*Optimization* Mouse picking over complicated mesh using simplified mesh for the raycasting:

```shell
cargo run --example simplified_mesh
```
