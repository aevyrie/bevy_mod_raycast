use bevy_app::prelude::*;
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_math::Ray3d;
use bevy_render::camera::Camera;
use bevy_transform::components::GlobalTransform;
use bevy_window::Window;

use crate::ray_from_screenspace;

/// Automatically generates a ray in world space corresponding to the mouse cursor, and stores it in
/// [`CursorRay`].
#[derive(Default)]
pub struct CursorRayPlugin;
impl Plugin for CursorRayPlugin {
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
/// Requires the [`CursorRayPlugin`] is added to your app. This is updated in both [`First`] and
/// [`PostUpdate`]. The ray built in `First` will have the latest cursor position, but will not
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
            ray_from_screenspace(cursor, camera, transform, window)
        })
        .next();
}
