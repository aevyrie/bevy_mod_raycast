//! # Deferred Raycasting API
//!
//! See the `minimal_deferred` example for reference.
//!
//! This API requires you add a [`RaycastSource`] to the entity that will be generating rays, and a
//! [`RaycastMesh`] to all meshes that you want to raycast against. The [`RaycastSource`] has some
//! built in modes for common use cases. You can set this entity to cast based on where it is
//! pointing, using [`RaycastMethod::Transform`], or you can use [`RaycastMethod::Screenspace`]
//! along with a screenspace coordinate if the entity is a camera and you want to shoot out of a
//! reticle, or you can use [`RaycastMethod::Cursor`] if you want to automatically use the cursor to
//! build rays.
//!
//! These components are both generic, and raycasts will only happen between entities with the same
//! generic parameter. For example, [`RaycastSource<Foo>`] can cast rays against meshes with
//! [`RaycastMesh<Foo>`], but not against meshes that instead only have a [`RaycastMesh<Bar>`]
//! component.

use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Ray3d, Vec2};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::camera::Camera;
use bevy_transform::components::GlobalTransform;
use bevy_utils::{default, tracing::*};
use bevy_window::{PrimaryWindow, Window};

use crate::{immediate::*, primitives::*};

pub struct DeferredRaycastingPlugin<T>(pub PhantomData<fn() -> T>);
impl<T: TypePath + Send + Sync> Plugin for DeferredRaycastingPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<RaycastPluginState<T>>().add_systems(
            First,
            (
                build_rays::<T>
                    .in_set(RaycastSystem::BuildRays::<T>)
                    .run_if(|state: Res<RaycastPluginState<T>>| state.build_rays),
                update_raycast::<T>
                    .in_set(RaycastSystem::UpdateRaycast::<T>)
                    .run_if(|state: Res<RaycastPluginState<T>>| state.update_raycast),
                update_target_intersections::<T>
                    .in_set(RaycastSystem::UpdateIntersections::<T>)
                    .run_if(|state: Res<RaycastPluginState<T>>| state.update_raycast),
            )
                .chain(),
        );

        app.register_type::<RaycastMesh<T>>()
            .register_type::<RaycastSource<T>>();

        #[cfg(feature = "debug")]
        app.add_systems(
            First,
            debug::update_debug_cursor::<T>
                .in_set(RaycastSystem::UpdateDebugCursor::<T>)
                .run_if(|state: Res<RaycastPluginState<T>>| state.update_debug_cursor)
                .after(RaycastSystem::UpdateIntersections::<T>),
        );
    }
}
impl<T> Default for DeferredRaycastingPlugin<T> {
    fn default() -> Self {
        DeferredRaycastingPlugin(PhantomData)
    }
}

#[derive(SystemSet)]
pub enum RaycastSystem<T> {
    BuildRays,
    UpdateRaycast,
    UpdateIntersections,
    #[cfg(feature = "debug")]
    UpdateDebugCursor,
    _Phantom(PhantomData<fn() -> T>),
}
impl<T> PartialEq for RaycastSystem<T> {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}
impl<T> Eq for RaycastSystem<T> {}
impl<T> Debug for RaycastSystem<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let set = std::any::type_name::<T>();
        match self {
            Self::BuildRays => write!(f, "BuildRays ({})", set),
            Self::UpdateRaycast => write!(f, "UpdateRaycast ({})", set),
            Self::UpdateIntersections => write!(f, "UpdateIntersections ({})", set),
            #[cfg(feature = "debug")]
            Self::UpdateDebugCursor => write!(f, "UpdateDebugCursor ({})", set),
            Self::_Phantom(_) => write!(f, "PhantomData<{}>", set),
        }
    }
}
impl<T> Hash for RaycastSystem<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let set = std::any::type_name::<T>();
        (core::mem::discriminant(self), set).hash(state);
    }
}
impl<T> Clone for RaycastSystem<T> {
    fn clone(&self) -> Self {
        match self {
            Self::BuildRays => Self::BuildRays,
            Self::UpdateRaycast => Self::UpdateRaycast,
            Self::UpdateIntersections => Self::UpdateIntersections,
            #[cfg(feature = "debug")]
            Self::UpdateDebugCursor => Self::UpdateDebugCursor,
            Self::_Phantom(_) => Self::_Phantom(PhantomData),
        }
    }
}

/// Global plugin state used to enable or disable all ray casting for a given type T.
#[derive(Component, Resource)]
pub struct RaycastPluginState<T> {
    pub build_rays: bool,
    pub update_raycast: bool,
    #[cfg(feature = "debug")]
    pub update_debug_cursor: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for RaycastPluginState<T> {
    fn default() -> Self {
        RaycastPluginState {
            build_rays: true,
            update_raycast: true,
            #[cfg(feature = "debug")]
            update_debug_cursor: false,
            _marker: PhantomData,
        }
    }
}

#[cfg(feature = "debug")]
impl<T> RaycastPluginState<T> {
    pub fn with_debug_cursor(self) -> Self {
        RaycastPluginState {
            update_debug_cursor: true,
            ..self
        }
    }
}

/// Marks an entity as pickable, with type T.
///
/// # Requirements
///
/// The marked entity must also have a [Mesh](bevy_render::mesh::Mesh) component.
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct RaycastMesh<T: TypePath> {
    #[reflect(ignore)]
    pub intersections: Vec<(Entity, IntersectionData)>,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}

impl<T: TypePath> RaycastMesh<T> {
    /// Get a reference to the ray cast source's intersections.
    ///
    /// Here the [`Entity`] is the entity of the [`RaycastSource`] component.
    /// Returns the list of intersections with all the sources with matching generic parameter
    /// that intersected this mesh during the last raycast system run.
    /// Returns an empty list if there are no intersections.
    pub fn intersections(&self) -> &[(Entity, IntersectionData)] {
        &self.intersections
    }
}

impl<T: TypePath> Default for RaycastMesh<T> {
    fn default() -> Self {
        RaycastMesh {
            intersections: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T: TypePath> Clone for RaycastMesh<T> {
    fn clone(&self) -> Self {
        RaycastMesh {
            intersections: self.intersections.clone(),
            _marker: PhantomData,
        }
    }
}

/// The `RaycastSource` component is used to generate rays with the specified `cast_method`. A `ray`
/// is generated when the RaycastSource is initialized, either by waiting for update_raycast system
/// to process the ray, or by using a `with_ray` function.`
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct RaycastSource<T: TypePath> {
    /// The method used to generate rays for this raycast.
    pub cast_method: RaycastMethod,
    /// When `true`, raycasting will only hit the nearest entity, skipping any entities that are
    /// further away. This can significantly improve performance in cases where a ray intersects
    /// many AABBs.
    pub should_early_exit: bool,
    /// Determines how raycasting should consider entity visibility.
    pub visibility: RaycastVisibility,
    #[reflect(ignore)]
    pub ray: Option<Ray3d>,
    #[reflect(ignore)]
    intersections: Vec<(Entity, IntersectionData)>,
    #[reflect(ignore)]
    _marker: PhantomData<fn() -> T>,
}

impl<T: TypePath> Default for RaycastSource<T> {
    fn default() -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Screenspace(Vec2::ZERO),
            should_early_exit: true,
            visibility: RaycastVisibility::MustBeVisibleAndInView,
            ray: None,
            intersections: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T: TypePath> Clone for RaycastSource<T> {
    fn clone(&self) -> Self {
        Self {
            cast_method: self.cast_method.clone(),
            should_early_exit: self.should_early_exit,
            visibility: self.visibility,
            ray: self.ray,
            intersections: self.intersections.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: TypePath> RaycastSource<T> {
    /// Instantiates a [RaycastSource]. It will not be initialized until the update_raycast system
    /// runs, or one of the `with_ray` functions is run.
    pub fn new() -> RaycastSource<T> {
        RaycastSource::default()
    }
    /// Initializes a [RaycastSource] with a valid screenspace ray.
    pub fn with_ray_screenspace(
        self,
        cursor_pos_screen: Vec2,
        camera: &Camera,
        camera_transform: &GlobalTransform,
        window: &Window,
    ) -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Screenspace(cursor_pos_screen),
            ray: ray_from_screenspace(cursor_pos_screen, camera, camera_transform, window),
            ..self
        }
    }
    /// Initializes a [RaycastSource] with a valid ray derived from a transform.
    pub fn with_ray_transform(self, transform: Mat4) -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Transform,
            ray: Some(ray_from_transform(transform)),
            ..self
        }
    }

    /// Set the `should_early_exit` field of this raycast source.
    pub fn with_early_exit(self, should_early_exit: bool) -> Self {
        Self {
            should_early_exit,
            ..self
        }
    }

    /// Set the `visibility` field of this raycast source.
    pub fn with_visibility(self, visibility: RaycastVisibility) -> Self {
        Self { visibility, ..self }
    }

    /// Instantiates and initializes a [RaycastSource] with a valid screenspace ray.
    pub fn new_screenspace(
        cursor_pos_screen: Vec2,
        camera: &Camera,
        camera_transform: &GlobalTransform,
        window: &Window,
    ) -> Self {
        RaycastSource::new().with_ray_screenspace(
            cursor_pos_screen,
            camera,
            camera_transform,
            window,
        )
    }

    /// Initializes a [RaycastSource] for cursor raycasting.
    pub fn new_cursor() -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Cursor,
            ..default()
        }
    }

    /// Initializes a [RaycastSource] with a valid ray derived from a transform.
    pub fn new_transform(transform: Mat4) -> Self {
        RaycastSource::new().with_ray_transform(transform)
    }

    /// Instantiates a [RaycastSource] with [RaycastMethod::Transform], and an empty ray. It will
    /// not be initialized until the [update_raycast] system is run and a [GlobalTransform] is
    /// present on this entity.
    ///
    /// # Warning
    /// Only use this if the entity this is associated with will have its [Transform](bevy_transform::components::Transform) or
    /// [GlobalTransform] specified elsewhere. If the [GlobalTransform] is not set, this ray casting
    /// source will never be able to generate a raycast.
    pub fn new_transform_empty() -> Self {
        RaycastSource {
            cast_method: RaycastMethod::Transform,
            ..default()
        }
    }

    /// Get a reference to the ray cast source's intersections, if one exists.
    pub fn get_intersections(&self) -> Option<&[(Entity, IntersectionData)]> {
        if self.intersections.is_empty() {
            None
        } else {
            Some(&self.intersections)
        }
    }

    /// Get a reference to the ray cast source's intersections. Returns an empty list if there are
    /// no intersections.
    pub fn intersections(&self) -> &[(Entity, IntersectionData)] {
        &self.intersections
    }

    /// Get a reference to the nearest intersection point, if there is one.
    pub fn get_nearest_intersection(&self) -> Option<(Entity, &IntersectionData)> {
        if self.intersections.is_empty() {
            None
        } else {
            self.intersections.first().map(|(e, i)| (*e, i))
        }
    }

    /// Get a copy of the ray cast source's ray.
    pub fn get_ray(&self) -> Option<Ray3d> {
        self.ray
    }

    /// Get a mutable reference to the ray cast source's intersections.
    pub fn intersections_mut(&mut self) -> &mut Vec<(Entity, IntersectionData)> {
        &mut self.intersections
    }

    /// Returns `true` if this is using [`RaycastMethod::Screenspace`].
    pub fn is_screenspace(&self) -> bool {
        matches!(self.cast_method, RaycastMethod::Screenspace(_))
    }
}

/// Specifies the method used to generate rays.
#[derive(Clone, Debug, Reflect)]
pub enum RaycastMethod {
    /// Use the mouse cursor to build a ray.
    Cursor,
    /// Specify screen coordinates relative to the camera component associated with this entity.
    ///
    /// # Component Requirements
    ///
    /// This requires a [Camera] component on this [RaycastSource]'s entity, to determine where the
    /// screenspace ray is firing from in the world.
    Screenspace(Vec2),
    /// Use a transform in world space to define a pick ray. This transform is applied to a vector
    /// at the origin pointing up to generate a ray.
    ///
    /// # Component Requirements
    ///
    /// Requires a [GlobalTransform] component associated with this [RaycastSource]'s entity.
    Transform,
}

pub fn build_rays<T: TypePath>(
    mut pick_source_query: Query<(
        &mut RaycastSource<T>,
        Option<&GlobalTransform>,
        Option<&Camera>,
    )>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    for (mut pick_source, transform, camera) in &mut pick_source_query {
        pick_source.ray = match &mut pick_source.cast_method {
            RaycastMethod::Cursor => {
                query_window(&window, camera, transform).and_then(|(window, camera, transform)| {
                    window.cursor_position().and_then(|cursor_pos| {
                        ray_from_screenspace(cursor_pos, camera, transform, window)
                    })
                })
            }
            RaycastMethod::Screenspace(cursor_pos_screen) => {
                query_window(&window, camera, transform).and_then(|(window, camera, transform)| {
                    ray_from_screenspace(*cursor_pos_screen, camera, transform, window)
                })
            }
            RaycastMethod::Transform => transform
                .map(|t| t.compute_matrix())
                .map(ray_from_transform),
        };
    }
}

fn query_window<'q, 'a: 'q, 'b>(
    window: &'q Query<'_, '_, &'a Window, With<PrimaryWindow>>,
    camera: Option<&'b Camera>,
    transform: Option<&'b GlobalTransform>,
) -> Option<(&'q Window, &'b Camera, &'b GlobalTransform)> {
    let window = match window.get_single() {
        Ok(window) => window,
        Err(_) => {
            error!("No primary window found, cannot cast ray");
            return None;
        }
    };
    let camera = match camera {
        Some(camera) => camera,
        None => {
            error!(
                "The PickingSource is a CameraScreenSpace but has no associated Camera component"
            );
            return None;
        }
    };
    let camera_transform = match transform {
        Some(transform) => transform,
        None => {
            error!(
        "The PickingSource is a CameraScreenSpace but has no associated GlobalTransform component"
    );
            return None;
        }
    };
    Some((window, camera, camera_transform))
}

/// Iterates through all entities with the [RaycastMesh] component, checking for
/// intersections. If these entities have bounding volumes, these will be checked first, greatly
/// accelerating the process.
pub fn update_raycast<T: TypePath + Send + Sync + 'static>(
    mut raycast: crate::immediate::Raycast,
    mut pick_source_query: Query<&mut RaycastSource<T>>,
    targets: Query<&RaycastMesh<T>>,
) {
    for mut pick_source in &mut pick_source_query {
        if let Some(ray) = pick_source.ray {
            pick_source.intersections.clear();

            let filter = |entity| targets.contains(entity);
            let test = |_| pick_source.should_early_exit;
            let settings = RaycastSettings::default()
                .with_filter(&filter)
                .with_early_exit_test(&test)
                .with_visibility(pick_source.visibility);
            pick_source.intersections = raycast.cast_ray(ray, &settings).to_vec();
        }
    }
}

pub fn update_target_intersections<T: TypePath + Send + Sync>(
    sources: Query<(Entity, &RaycastSource<T>)>,
    mut meshes: Query<&mut RaycastMesh<T>>,
    mut previously_updated_raycast_meshes: Local<Vec<Entity>>,
) {
    // Clear any entities with intersections last frame
    for entity in previously_updated_raycast_meshes.drain(..) {
        if let Ok(mesh) = meshes.get_mut(entity).as_mut() {
            mesh.intersections.clear();
        }
    }

    for (source_entity, source) in sources.iter() {
        for (mesh_entity, intersection) in source.intersections().iter() {
            if let Ok(mut mesh) = meshes.get_mut(*mesh_entity) {
                mesh.intersections
                    .push((source_entity, intersection.to_owned()));
                previously_updated_raycast_meshes.push(*mesh_entity);
            }
        }
    }
}

#[cfg(feature = "debug")]
pub mod debug {
    #![allow(unused)]

    use bevy_color::palettes::css;
    use bevy_ecs::system::{Commands, Query};
    use bevy_gizmos::gizmos::Gizmos;
    use bevy_math::{Dir3, Quat, Vec3};
    use bevy_reflect::TypePath;
    use bevy_utils::tracing::info;
    use std::marker::PhantomData;

    use crate::prelude::*;

    /// Updates the 3d cursor to be in the pointed world coordinates
    #[allow(clippy::too_many_arguments)]
    pub fn update_debug_cursor<T: TypePath + Send + Sync>(
        mut commands: Commands,
        mut sources: Query<&RaycastSource<T>>,
        mut gizmos: Gizmos,
    ) {
        for ray in sources.iter().filter_map(|s| s.ray) {
            let orientation = Quat::from_rotation_arc(Vec3::NEG_Z, *ray.direction);
            gizmos.ray(ray.origin, *ray.direction, css::BLUE);
            gizmos.sphere(ray.origin, orientation, 0.1, css::BLUE);
        }

        for (is_first, intersection) in sources.iter().flat_map(|m| {
            m.intersections()
                .iter()
                .map(|i| i.1.clone())
                .enumerate()
                .map(|(i, hit)| (i == 0, hit))
        }) {
            let color = match is_first {
                true => css::GREEN,
                false => css::PINK,
            };
            gizmos.ray(intersection.position(), intersection.normal(), color);
            gizmos.circle(
                intersection.position(),
                Dir3::new_unchecked(intersection.normal().normalize()),
                0.1,
                color,
            );
            gizmos.circle_2d(intersection.position().truncate(), 10.0, color);
        }
    }

    /// Used to debug [`RaycastMesh`] intersections.
    pub fn print_intersections<T: TypePath + Send + Sync>(query: Query<&RaycastMesh<T>>) {
        for (_, intersection) in query.iter().flat_map(|mesh| mesh.intersections.iter()) {
            info!(
                "Distance {:?}, Position {:?}",
                intersection.distance(),
                intersection.position()
            );
        }
    }
}
