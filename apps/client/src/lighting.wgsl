// Tile lighting (`docs/reference/lighting.md`). The texture holds one texel per tile, and
// each tile shows its own light flat, right up to its border, as BYOND's tile lighting
// does: light and shadow read tile by tile, with hard edges.
// The pipeline multiplies the result over the map.
// Darkness out of sight is drawn by the black squares of `shadows.rs`.

@group(2) @binding(0) var light_texture: texture_2d<f32>;

struct FragmentInput {
    @location(2) uv: vec2<f32>,
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(light_texture));
    let top = size - vec2<i32>(1, 1);
    let tile = clamp(vec2<i32>(floor(in.uv * vec2<f32>(size))), vec2<i32>(0, 0), top);
    return vec4<f32>(textureLoad(light_texture, tile, 0).rgb, 1.0);
}
