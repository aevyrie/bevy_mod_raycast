<div align="center">

# `bevy_mod_raycast`

A small [Bevy](https://github.com/bevyengine/bevy) plugin for mesh raycasting.
  
[![CI](https://github.com/aevyrie/bevy_mod_raycast/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/aevyrie/bevy_mod_raycast/actions?query=workflow%3A%22rust.yml%22+branch%3Amain)
[![docs.rs](https://docs.rs/bevy_mod_raycast/badge.svg)](https://docs.rs/bevy_mod_raycast)
[![crates.io](https://img.shields.io/crates/v/bevy_mod_raycast)](https://crates.io/crates/bevy_mod_raycast)

![reflecting_lasers example](https://github.com/aevyrie/bevy_mod_raycast/assets/2632925/4a1019d3-cbfa-4b20-b5c9-19a71ca09e04)  
</div>

## Getting Started

Using the [`Raycast`](https://docs.rs/bevy_mod_raycast/latest/bevy_mod_raycast/immediate/struct.Raycast.html) system param, you don't even need to add a plugin, you can directly raycast into the ECS:

```rs
use bevy_mod_raycast::prelude::*;

fn my_raycast_system(mut raycast: Raycast) {
    let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
    let hits = raycast.cast_ray(ray, &RaycastSettings::default());
}
```

- 👉 [Read the docs!](https://docs.rs/bevy_mod_raycast)
- Play with the [examples](./examples).


<details>
<summary><h2>Bevy Version Support</h2></summary>
I intend to track the `main` branch of Bevy. PRs supporting this are welcome!

| bevy | bevy_mod_raycast |
| ---- | ---------------- |
| 0.14 | 0.18             |
| 0.13 | 0.17             |
| 0.12 | 0.16             |
| 0.11 | 0.9 - 0.15       |
| 0.10 | 0.8              |
| 0.9  | 0.7              |
| 0.8  | 0.6              |
| 0.7  | 0.4 - 0.5        |
| 0.6  | 0.3              |
| 0.5  | 0.2              |
| 0.4  | 0.1              |
</details>
