# Bevy renderer compatibility patch

Pinned upstream: bevy_render 0.17.3 (original MIT/Apache licenses retained).

Changed only window surface view-format selection and output attachment format. A surface which lacks sRGB formats and SURFACE_VIEW_FORMATS now keeps its native format rather than configuring an unsupported sRGB reinterpretation. Devices offering sRGB surfaces retain the existing path. Selected view format is carried consistently through texture view creation and window output attachment.

Triggered by API 26 Android x86 / native GLES startup: wgpu Surface::configure rejected SURFACE_VIEW_FORMATS. Actual emulator rendering verification is pending. This is a local compatibility patch, not an upstream release or a claim of universal GLES support.

Android x86 also defines NO_CUBE_ARRAY_TEXTURES_SUPPORT, paired with the local bevy_pbr cube-shadow binding/view fallback. Home surface rendering is verified on API 26; garden weather acceptance is pending.
