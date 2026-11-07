// Putting the world on the screen (`docs/reference/lighting.md`, Air you can see): the
// world image, sampled with each pixel moved a little where the air is hot, so the heat
// over fires and the supermatter bends whatever is behind it. Then how the player is
// (`docs/reference/hud.md`, Screen effects): dark or red edges, a red flash, blur, grey,
// a cold or warm tint, and nothing at all for the blind.

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
    // Width and height in tiles, sprite px per tile, blind (0 or 1).
    size: vec2<f32>,
    tile: f32,
    blind: f32,
}
@group(1) @binding(5) var<uniform> map: Map;

struct Effects {
    // Each 0 to 1.
    breathless: f32,
    hurt: f32,
    flash: f32,
    unconscious: f32,
    dead: f32,
    cold: f32,
    hot: f32,
    dazzled: f32,
}
@group(1) @binding(6) var<uniform> fx_a: vec4<f32>;
@group(1) @binding(7) var<uniform> fx_b: vec4<f32>;

// How far unconscious blur reaches, world px.
const BLUR: f32 = 2.5;

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
    var colour = textureSample(world_texture, world_sampler, at).rgb;

    let fx = Effects(fx_a.x, fx_a.y, fx_a.z, fx_a.w, fx_b.x, fx_b.y, fx_b.z, 0.0);
    // Unconscious: blurred and dark.
    if fx.unconscious > 0.001 {
        let r = BLUR * fx.unconscious / view.span;
        var blur = vec3<f32>(0.0);
        for (var i = 0; i < 8; i++) {
            let a = f32(i) * 0.785398;
            blur += textureSampleLevel(world_texture, world_sampler, at + vec2<f32>(cos(a), sin(a)) * r, 0.0).rgb;
        }
        colour = mix(colour, blur / 8.0, fx.unconscious) * (1.0 - 0.55 * fx.unconscious);
    }
    // Dead: grey and dark.
    let grey = dot(colour, vec3<f32>(0.3, 0.59, 0.11));
    colour = mix(colour, vec3<f32>(grey * 0.55), fx.dead);
    // Cold: a little blue. Overheating: a little warm.
    colour = mix(colour, colour * vec3<f32>(0.78, 0.9, 1.18), fx.cold * 0.6);
    colour = mix(colour, colour * vec3<f32>(1.18, 0.92, 0.75), fx.hot * 0.6);
    // The edges: dark short of air, dark red when hurt, pulsing a little at their worst.
    let edge = smoothstep(0.35, 1.0, length(uv - 0.5) * 1.414);
    let pulse = 1.0 + 0.08 * sin(view.time * 3.0);
    colour *= 1.0 - edge * fx.breathless * pulse;
    colour = mix(colour, colour * vec3<f32>(0.55, 0.05, 0.05), edge * fx.hurt * pulse);
    // Hit: a red flash, strongest at the edges.
    colour = mix(colour, vec3<f32>(0.7, 0.02, 0.02), fx.flash * (0.12 + 0.5 * edge));
    // Dazzled by a flash: white, fading.
    colour = mix(colour, vec3<f32>(1.0), clamp(fx.dazzled, 0.0, 1.0));
    // Blind: Luna's black screen over the map.
    colour = mix(colour, vec3<f32>(0.0), map.blind);
    return vec4<f32>(colour, 1.0);
}
