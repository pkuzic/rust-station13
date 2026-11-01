// Putting the world on the screen (`docs/reference/lighting.md`, Air you can see): the
// world image, sampled with each pixel moved a little where the air is hot, so the heat
// over fires and the supermatter bends whatever is behind it.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var world_texture: texture_2d<f32>;
@group(1) @binding(1) var world_sampler: sampler;
// One texel per tile of our deck, top row first: how hot it is, 0 to 1.
@group(1) @binding(2) var heat_texture: texture_2d<f32>;
@group(1) @binding(3) var heat_sampler: sampler;

struct View {
    // Camera centre (world px), seconds, world px across the square.
    camera: vec2<f32>,
    time: f32,
    span: f32,
}
@group(1) @binding(4) var<uniform> view: View;

struct Map {
    // Width and height in tiles, sprite px per tile, unused.
    size: vec2<f32>,
    tile: f32,
    _pad: f32,
}
@group(1) @binding(5) var<uniform> map: Map;

// How far heat moves the picture at most, world px.
const SWAY: f32 = 1.6;
const LIFT: f32 = 0.9;

fn heat_at(w: vec2<f32>) -> f32 {
    // Tile centres sit at whole multiples of a tile.
    let t = w / map.tile + 0.5;
    let uv = vec2<f32>(t.x / map.size.x, 1.0 - t.y / map.size.y);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return 0.0;
    }
    return textureSampleLevel(heat_texture, heat_sampler, uv, 0.0).r;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let w = view.camera + vec2<f32>(uv.x - 0.5, 0.5 - uv.y) * view.span;
    // Hot air rises: a tile's heat also reaches up to a tile above it, fading.
    let half = map.tile * 0.5;
    let heat = max(
        heat_at(w),
        max(heat_at(w - vec2<f32>(0.0, half)) * 0.8, heat_at(w - vec2<f32>(0.0, map.tile)) * 0.5),
    );
    var at = uv;
    if heat > 0.001 {
        let t = view.time;
        let sway = sin(w.y * 0.30 - t * 5.0 + sin(w.x * 0.17 + t * 1.3) * 1.5)
            + 0.5 * sin(w.y * 0.71 - t * 8.3 + w.x * 0.05);
        let lift = sin(w.x * 0.23 + t * 4.1 + w.y * 0.11);
        at += vec2<f32>(sway * SWAY, -lift * LIFT) * heat / view.span;
    }
    return vec4<f32>(textureSample(world_texture, world_sampler, at).rgb, 1.0);
}
