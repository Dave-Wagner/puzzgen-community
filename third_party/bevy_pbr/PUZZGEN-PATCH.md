# API 26 GLES compatibility patch

Pinned upstream: bevy_pbr 0.17.3. Original MIT/Apache licenses retained.

On Android x86 only, point-shadow bindings and views use a single cube instead of a cube array, matching the existing iOS simulator fallback. The paired bevy_render patch defines NO_CUBE_ARRAY_TEXTURES_SUPPORT on this target. This emulator target is for basic compatibility checks; multiple shadow-casting point lights are not supported on this fallback. The garden uses no shadow-casting point lights. Other targets retain upstream behavior.

Reason: the API 26 x86 emulator exposes GLES 3.0 and rejects cube-array shader declarations. Runtime garden/weather verification remains pending until recorded in the development checkpoint.
