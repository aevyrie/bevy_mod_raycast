//! A small `bevy` plugin for raycasting against [`Mesh`](bevy_render::mesh::Mesh)es.
//!
//! ```
//! # use bevy::prelude::*;
//! use bevy_mod_raycast::prelude::*;
//!
//! fn raycast_system(mut raycast: Raycast) {
//!     let ray = Ray3d::new(Vec3::ZERO, Vec3::X);
//!     let hits = raycast.cast_ray(ray, &RaycastSettings::default());
//! }
//! ```
//!
//! *An example of the immediate mode raycasting API.*
//!
//! # Getting Started
//!
//! The plugin provides two ways of raycasting:
//! - An [immediate-mode API](immediate), which allows you to raycast into the scene on-demand in
//!   any system. Intersections are returned immediately as a sorted `Vec`.
//! - A [deferred API](deferred), where raycasts are performed once every frame based on entities
//!    tagged with specific components. Intersections can be queried from the ECS.
//!
//! The plugin also provides the [`CursorRayPlugin`] for automatically generating a world space 3D
//! ray corresponding to the mouse cursor. This is useful for mouse picking.
//!
//! ## Choosing an API
//!
//! While the deferred API requires adding components on entities, in return it's generally more
//! "hands-off". Once you add the components to entities, the plugin will run raycasts for you every
//! frame, and you can query your [`RaycastSource`]s to see what they have intersected that frame.
//!
//! You can also think of this as being the "declarative" API. Instead of defining how the raycast
//! happens, you instead describe what you want. For example, "this entity should cast rays in the
//! direction it faces", and you can then query that entity to find out what it hit.
//!
//! By comparison, the immediate mode API is more imperative. You must define the raycast directly,
//! but in return you are immediately given the results of the raycast without needing to wait for
//! the scheduled raycasting system to run and query the results.
//!
//! # Use Cases
//!
//! This plugin is well suited for use cases where you don't want to use a full physics engine, you
//! are putting together a simple prototype, or you just want the simplest-possible API. Using the
//! [`Raycast`] system param requires no added components or plugins. You can just start raycasting
//! in your systems.
//!
//! ## Limitations
//!
//! This plugin runs entirely on the CPU, with minimal acceleration structures, and without support
//! for skinned meshes. However, there is a good chance that this simply won't be an issue for your
//! application. The provided `stress_test` example is a worst-case scenario that can help you judge
//! if the plugin will meet your performance needs. Using a laptop with an i7-11800H, I am able to
//! reach 110-530 fps in the stress test, raycasting against 1,000 monkey meshes.

#![allow(clippy::type_complexity)]

pub mod cursor;
pub mod deferred;
pub mod immediate;
pub mod markers;
pub mod primitives;
pub mod raycast;

use bevy_utils::default;

#[allow(unused_imports)] // Needed for docs
use prelude::*;

pub mod prelude {
    pub use crate::{cursor::*, deferred::*, immediate::*, markers::*, primitives::*, raycast::*};

    #[cfg(feature = "debug")]
    pub use crate::debug::*;
}

/// Used for examples to reduce picking latency. Not relevant code for the examples.
#[doc(hidden)]
#[allow(dead_code)]
pub fn low_latency_window_plugin() -> bevy_window::WindowPlugin {
    bevy_window::WindowPlugin {
        primary_window: Some(bevy_window::Window {
            present_mode: bevy_window::PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }
}
