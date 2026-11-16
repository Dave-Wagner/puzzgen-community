#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> weather: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var snow_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var snow_sampler: sampler;

fn garden_hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.11369));
    return fract((q.x + q.y) * (q.x * q.y + 19.19));
}
fn garden_grain(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let w = f * f * (vec2<f32>(3.0) - 2.0 * f);
    return mix(
        mix(garden_hash(cell), garden_hash(cell + vec2<f32>(1.0, 0.0)), w.x),
        mix(garden_hash(cell + vec2<f32>(0.0, 1.0)), garden_hash(cell + vec2<f32>(1.0)), w.x), w.y);
}
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var p = pbr_input_from_standard_material(in, front);
    p.material.base_color = alpha_discard(p.material, p.material.base_color);
    // World-space detail stays continuous across hedge cells. No extra textures or alpha layers.
    let world = in.world_position.xyz;
    let leaf = garden_grain(world.xz * 18.0 + vec2<f32>(world.y * 11.0));
    let canopy = garden_grain(world.xz * 1.7 + vec2<f32>(world.y * 0.6));
    let color = p.material.base_color.rgb;
    let foliage = smoothstep(0.025, 0.11, color.g - max(color.r, color.b));
    let detail = mix(0.96 + 0.08 * canopy, 0.78 + 0.27 * leaf + 0.12 * canopy, foliage);
    let grounding = mix(1.0, 0.76 + 0.24 * smoothstep(0.0, 1.1, world.y), foliage);
    p.material.base_color = vec4<f32>(color * detail * grounding, p.material.base_color.a);
    let top = smoothstep(0.12, 0.75, p.world_normal.y);
    let snow = clamp(weather.x * top * 1.5, 0.0, 1.0);
    let tex = textureSample(snow_map, snow_sampler, fract(in.world_position.xz * 0.25)).rgb;
    p.material.base_color = vec4<f32>(mix(p.material.base_color.rgb * (1.0-weather.y*0.3), mix(vec3<f32>(0.8,0.88,0.96), tex,0.25),snow),p.material.base_color.a);
    p.material.perceptual_roughness = mix(mix(p.material.perceptual_roughness,0.24,weather.y*top),0.95,snow);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(p);
    out.color = main_pass_post_lighting_processing(p,out.color);
    return out;
}
