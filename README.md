# Ray Casting for Bevy


[![CI](https://github.com/aevyrie/bevy_mod_raycast/workflows/CI/badge.svg?branch=master)](https://github.com/aevyrie/bevy_mod_picking/actions?query=workflow%3A%22CI%22+branch%3Amaster)
[![crates.io](https://img.shields.io/crates/v/bevy_mod_raycast)](https://crates.io/crates/bevy_mod_raycast)
[![docs.rs](https://docs.rs/bevy_mod_raycast/badge.svg)](https://docs.rs/bevy_mod_raycast)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)

A [Bevy](https://github.com/bevyengine/bevy) plugin for ray casting. Contributions welcome!

This plugin makes it simple to create ray casting sources, such as a camera (first person shooter), transform (third person shooter), or screenspace coordinates (mouse picking). Rays are shot from these sources every frame using a bevy system, and the intersections are stored in the ray casting source's component. Only meshes that you mark with a component will be checked for intersections.

## Bevy Version Support

I intend to track the `main` branch of Bevy. PRs supporting this are welcome! 

|bevy|bevy_mod_raycst|
|---|---|
|0.5|0.2|
|0.4|0.1|

## Demo

Run a rudimentary mouse picking example with:

```shell
cargo run --example mouse_picking --features="ex"
```
