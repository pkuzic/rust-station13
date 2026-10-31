// Lighting (`docs/reference/lighting.md`). The texture holds the light worked out
// geometrically on the CPU (`light_map.rs`), sixteen texels to a tile, with walls already
// showing the light on the side the player sees; it is sampled smoothly and multiplied
// over the map by the pipeline. Darkness out of sight is drawn by the shadows.

@group(2) @binding(0) var light_texture: texture_2d<f32>;
@group(2) @binding(1) var light_sampler: sampler;

struct FragmentInput {
    @location(2) uv: vec2<f32>,
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(light_texture, light_sampler, in.uv).rgb, 1.0);
}
