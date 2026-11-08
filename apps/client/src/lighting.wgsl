// Tile lighting (`docs/reference/lighting.md`). The texture holds one texel per tile, and
// each tile shows its own light flat, right up to its border, as BYOND's tile lighting
// does: light and shadow read tile by tile, with hard edges.
// The pipeline multiplies the result over the map.
// Darkness out of sight is drawn by the black squares of `shadows.rs`.
//
// Normal maps (`normals.rs`): each pixel's normal, drawn into an image the size of the
// world image, turns the light up on faces that turn towards where the light comes from on
// its tile and down on those that turn away. A flat face, or any face in light that falls
// evenly, keeps the tile's light as it is.

@group(2) @binding(0) var light_texture: texture_2d<f32>;
@group(2) @binding(1) var normal_texture: texture_2d<f32>;
@group(2) @binding(2) var direction_texture: texture_2d<f32>;
// Strength (0 off), tilt, least and most shade.
@group(2) @binding(3) var<uniform> normal_params: vec4<f32>;

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

// A direction stored as bytes from 1 to 255, 128 for none.
fn decode(stored: vec3<f32>) -> vec3<f32> {
    return (stored * 255.0 - 128.0) / 127.0;
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(light_texture));
    let top = size - vec2<i32>(1, 1);
    let tile = clamp(vec2<i32>(floor(in.uv * vec2<f32>(size))), vec2<i32>(0, 0), top);
    let light = textureLoad(light_texture, tile, 0).rgb;
    let strength = normal_params.x;
    if strength <= 0.0 {
        return vec4<f32>(light, 1.0);
    }
    let normal_size = vec2<i32>(textureDimensions(normal_texture));
    let pixel = clamp(vec2<i32>(in.position.xy), vec2<i32>(0, 0), normal_size - vec2<i32>(1, 1));
    let n = normalize(decode(textureLoad(normal_texture, pixel, 0).xyz));
    let falls = textureLoad(direction_texture, tile, 0);
    // Only the light that leans one way shades: in even light every face keeps the
    // tile's light, whichever way it turns.
    let towards = decode(vec3<f32>(falls.xy, 0.5)).xy * falls.z * normal_params.y;
    let shade = clamp(1.0 + dot(n.xy, towards), normal_params.z, normal_params.w);
    return vec4<f32>(light * mix(1.0, shade, strength), 1.0);
}
