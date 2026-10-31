// Soft tile lighting (`docs/reference/lighting.md`). The texture holds one texel per
// tile; between tile centres the light is blended with smoothstep weights, so light
// fades smoothly with no creases. The pipeline multiplies the result over the map.
// Alpha says whether a tile is in sight: only those are blended, so light from a tile the
// player can't see (the room behind a wall) never bleeds into what they can. Darkness
// out of sight is drawn by the shadows.

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
    // Each corner's smoothstep weight, kept only if that tile is in sight.
    let wa = a.a * (1.0 - f.x) * (1.0 - f.y);
    let wb = b.a * f.x * (1.0 - f.y);
    let wc = c.a * (1.0 - f.x) * f.y;
    let wd = d.a * f.x * f.y;
    let total = wa + wb + wc + wd;
    let tile = clamp(vec2<i32>(floor(in.uv * vec2<f32>(size))), vec2<i32>(0, 0), top);
    var light = textureLoad(light_texture, tile, 0).rgb;
    if total > 0.0001 {
        light = (a.rgb * wa + b.rgb * wb + c.rgb * wc + d.rgb * wd) / total;
    }
    return vec4<f32>(light, 1.0);
}
