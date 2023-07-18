# 0.9.0

- Removed the `Intersection` component. Intersection data can be found using `RaycastMesh::intersections()` and `RaycastSource::intersections()`.
- Changed: `Ray3d::from_screenspace` start from the near plane
- Fixed: Raycasts do not hit bottoms of un-rotated `RayCastMesh`es

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