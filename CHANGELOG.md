# 0.14.1

- Changed: relaxed type bounds on the generic raycast set type parameter in `RaycastSource<T>` and
  `RaycastMesh<T>` to no longer require `Clone.`
- Added: substantially improved documentation.
- Fixed: plugin not building with `--no-default-features`.
- Fixed: `RaycastSource`'s visibility settings being ignored.

# 0.14.0

- Fixed: window scale factor not being considered for screenspace raycasts.

# 0.13.1

- Fixed: overly-strict query filter preventing raycasting 2d meshes.

# 0.13.0

- Changed: the immediate mode raycasting system param `Raycast` no longer requires a type parameter
  for a raycasting set. Instead, you can supply this constraint as a filter in `RaycastSettings`.
  This makes it possible to raycast any bevy mesh without any special components on cameras or
  meshes.
- Fixed: `SimplifiedMesh` and `RaycastSettings` added to the prelude.

# 0.12.0

- Changed: the `should_early_exit` boolean field  has been removed from `RaycastSettings` in favor
  of a more flexible closure `early_exit_test`. This early exit test is run on every entity to
  determine if it should block further entities from being hit. The previous behavior can be
  replicated by passing in a closure that ignores the input and returns a boolean, such as `&|_|
  true` instead of `true`.
- Added: raycasts can now apply a test to determine which entities to allow during a raycast by
  using the `filter` field on `RaycastSettings`.

# 0.11.0

- Changed: `Raycast::cast_ray` now accepts a `RaycastSettings` parameter.
- Added: entity visibility handling can now be configured using the `RaycastVisibility` field on
  `RaycastSettings` and `RaycastSource`:
  - `Ignore`: Completely ignore visibility checks. Hidden items can still be raycasted against.
  - `MustBeVisible`: Only raycast against entities that are visible in the hierarchy.
  - `MustBeVisibleAndInView`: Only raycast against entities that are visible in the hierarchy and
    visible to a camera or light. This is the same as setting the `should_frustum_cull` parameter of
    `cast_ray` to `true` in 0.10.

# 0.10.0

- Changed: `Raycast::cast_ray` is now a mutable query. The system param now stores allocated buffers
  in `Local`s to reuse allocated memory.
- Changed: parallel AABB culling now uses an unbounded channel to reduce time spent allocating a
  bounded channel when many entities are present.

# 0.9.0

- Added: `Raycast` system param allows immediate raycasting into the world using the `cast_ray`
  method.
- Removed the `Intersection` component. Intersection data can be found using
  `RaycastMesh::intersections()` and `RaycastSource::intersections()`.
- Changed: `Ray3d::from_screenspace` start from the near plane.
- Fixed: Raycasts do not hit bottoms of un-rotated `RayCastMesh`es.
- Changed: `DefaultPluginState` renamed to `RaycastPluginState`.

# 0.8.0

- Implement `Reflect` for `RaycastMesh`, `RaycastSource`, and `RaycastMethod`.
- Fix raycasting for non-indexed meshes.
- Update to bevy 0.10.

# 0.7.0

- All internal debug related code is now behind feature flags to enable running with
  `default_features = false`. 
- Renamed:
  - `RayCastSource` to `RaycastSource`
  - `RayCastMesh` to `RaycastMesh`
  - `RayCastMethod` to `RaycastMethod`
  - `RayCastSet` to `RaycastSet`
- Update method naming to be more consistent with the ecosystem:
  - `intersect_list()` -> `get_intersections()`
  - `intersect_top()` -> `get_nearest_intersection()`
  - `ray()` -> `get_ray()`
- Added `intersections()`, the counterpart to `get_intersections()`. This method returns an empty
  slice if there are no intersections, instead of an `Option`. The latter is useful if you want a
  guarantee that there is at least one intersection after unwrapping the `Option`.
