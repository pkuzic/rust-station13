// Soft tile lighting (`docs/reference/lighting.md`). The texture holds one texel per
// tile; between tile centres the light is blended with smoothstep weights, so light
// fades smoothly with no creases. The pipeline multiplies the result over the map.

@group(2) @binding(0) var light_texture: texture_2d<f32>;

struct FragmentInput {
    @location(2) uv: vec2<f32>,
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(light_texture));
    let p = in.uv * vec2<f32>(size) - 0.5;
    let base = floor(p);
    var f = p - base;
    f = f * f * (3.0 - 2.0 * f);
    let top = size - vec2<i32>(1, 1);
    let i0 = clamp(vec2<i32>(base), vec2<i32>(0, 0), top);
    let i1 = clamp(vec2<i32>(base) + vec2<i32>(1, 1), vec2<i32>(0, 0), top);
    let a = textureLoad(light_texture, vec2<i32>(i0.x, i0.y), 0);
    let b = textureLoad(light_texture, vec2<i32>(i1.x, i0.y), 0);
    let c = textureLoad(light_texture, vec2<i32>(i0.x, i1.y), 0);
    let d = textureLoad(light_texture, vec2<i32>(i1.x, i1.y), 0);
    let light = mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
    return vec4<f32>(light.rgb, 1.0);
}
