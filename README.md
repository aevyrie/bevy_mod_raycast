<div align="center">

# Simple Bevy Raycasting

A small [Bevy](https://github.com/bevyengine/bevy) plugin for mesh raycasting.
  
[![CI](https://github.com/aevyrie/bevy_mod_raycast/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/aevyrie/bevy_mod_raycast/actions?query=workflow%3A%22rust.yml%22+branch%3Amain)
[![docs.rs](https://docs.rs/bevy_mod_raycast/badge.svg)](https://docs.rs/bevy_mod_raycast)
[![crates.io](https://img.shields.io/crates/v/bevy_mod_raycast)](https://crates.io/crates/bevy_mod_raycast)

![reflecting_lasers example](https://github.com/aevyrie/bevy_mod_raycast/assets/2632925/4a1019d3-cbfa-4b20-b5c9-19a71ca09e04)  
</div>

## Getting Started

Using the [`Raycast`](https://docs.rs/bevy_mod_raycast/latest/bevy_mod_raycast/system_param/struct.Raycast.html) system param, you don't even need to add a plugin to your app. You can simply start raycasting:

```rs
use bevy_mod_raycast::prelude::*;

fn my_raycast_system(mut raycast: Raycast) {
    let hits = raycast.cast_ray(Ray3d::default(), &RaycastSettings::default());
}
```

- [Read the docs!](https://docs.rs/bevy_mod_raycast)
- Play with the [examples](./examples).

## Bevy Version Support

I intend to track the `main` branch of Bevy. PRs supporting this are welcome! 

| bevy | bevy_mod_raycast |
| ---- | ---------------- |
| 0.11 | 0.9 - 0.14       |
| 0.10 | 0.8              |
| 0.9  | 0.7              |
| 0.8  | 0.6              |
| 0.7  | 0.4 - 0.5        |
| 0.6  | 0.3              |
| 0.5  | 0.2              |
| 0.4  | 0.1              |