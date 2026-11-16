# GLES shader-language compatibility patch

Pinned upstream: wgpu-hal 26.0.6; original licenses retained.

The Samsung SM-T380 / Adreno 308 reports a GLES 3.0 context but GLSL ES 3.10. Its compiler rejects generated #version 310 es even for the UI pipeline. Capping the shader-language version at the actual GLES context version avoids that inconsistent driver report. Desktop GL and non-GL backends are unchanged. Supported GLES versions are retained and shader language versions are never upgraded.

The production cap helper has two standalone regression tests. Real tablet runtime verification remains required. The debug Java wrapper can enable wgpu GLES tracing before NativeActivity initialization; this adds no visible game controls and remains debug-only.
