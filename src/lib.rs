//! A small [`bevy`] plugin for raycasting against [`Mesh`]es.
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
//! - A [deferred API](deferred), where raycasts are performed once every frame based on
//!    entities tagged with specific components. Intersections can be queried from the ECS.
//!
//! ## Choosing an API
//!
//! While the deferred API requires adding components on entities, in return it's generally
//! more "hands-off". Once you add the components to entities, the plugin will run raycasts for you
//! every frame, and you can query your [`RaycastSource`]s to see what they have intersected that
//! frame.
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

pub mod deferred;
pub mod immediate;
pub mod markers;
pub mod primitives;
pub mod raycast;

use bevy_app::prelude::*;
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_render::camera::Camera;
use bevy_transform::components::GlobalTransform;
use bevy_utils::default;
use bevy_window::Window;

#[allow(unused_imports)] // Needed for docs
use prelude::*;

pub mod prelude {
    pub use crate::{
        deferred::*, immediate::*, markers::*, primitives::*, raycast::*, CursorRay,
        DefaultRaycastingPlugin,
    };

    #[cfg(feature = "debug")]
    pub use crate::debug::*;
}

#[derive(Default)]
pub struct DefaultRaycastingPlugin;
impl Plugin for DefaultRaycastingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, update_cursor_ray)
            .add_systems(
                PostUpdate,
                update_cursor_ray.after(bevy_transform::TransformSystem::TransformPropagate),
            )
            .init_resource::<CursorRay>();
    }
}

/// Holds the latest cursor position as a 3d ray.
///
/// Requires the [`DefaultRaycastingPlugin`] is added to your app. This is updated in both [`First`]
/// and [`PostUpdate`]. The ray built in `First` will have the latest cursor position, but will not
/// account for any updates to camera position done in [`Update`]. The ray built in `PostUpdate`
/// will account for the camera position being updated and any camera transform propagation.
#[derive(Resource, Default, Deref)]
pub struct CursorRay(pub Option<Ray3d>);

/// Updates the [`CursorRay`] every frame.
pub fn update_cursor_ray(
    primary_window: Query<Entity, With<bevy_window::PrimaryWindow>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut cursor_ray: ResMut<CursorRay>,
) {
    cursor_ray.0 = cameras
        .iter()
        .filter_map(|(camera, transform)| {
            if let bevy_render::camera::RenderTarget::Window(window_ref) = camera.target {
                Some(((camera, transform), window_ref))
            } else {
                None
            }
        })
        .filter_map(|(cam, window_ref)| {
            window_ref
                .normalize(primary_window.get_single().ok())
                .map(|window_ref| (cam, window_ref.entity()))
        })
        .filter_map(|(cam, window_entity)| windows.get(window_entity).ok().map(|w| (cam, w)))
        .filter_map(|(cam, window)| window.cursor_position().map(|pos| (cam, window, pos)))
        .filter_map(|((camera, transform), window, cursor)| {
            Ray3d::from_screenspace(cursor, camera, transform, window)
        })
        .next();
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
