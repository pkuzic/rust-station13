// Space behind the station (`docs/reference/lighting.md`, Space): three layers of stars,
// each sliding past at its own fraction of the camera's movement, so the far ones seem far
// and the near ones near, while the ship drifts slowly through them. Stars sit on whole
// sprite pixels, so they look like the rest of the art.

struct Params {
    // Camera position (world px), seconds.
    camera: vec2<f32>,
    time: f32,
    _pad: f32,
}

@group(2) @binding(0) var<uniform> params: Params;

struct FragmentInput {
    @location(0) world_position: vec4<f32>,
}

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(127.1, 311.7));
    let r = q + dot(q, q.yx + 19.19);
    return fract((r.x + r.y) * r.x);
}

// One layer: a star in some cells of a grid `cell` px wide, `density` of them lit.
fn layer(at: vec2<f32>, cell: f32, density: f32, salt: f32) -> vec3<f32> {
    let c = floor(at / cell);
    let h = hash(c + salt);
    if h > density {
        return vec3<f32>(0.0);
    }
    // Where in its cell the star is, on a whole pixel.
    let spot = floor(vec2<f32>(hash(c + salt + 3.1), hash(c + salt + 7.7)) * (cell - 2.0)) + 1.0;
    let d = abs(floor(at - c * cell) - spot);
    let big = hash(c + salt + 11.3) < 0.08;
    var lit = 0.0;
    if d.x < 0.5 && d.y < 0.5 {
        lit = 1.0;
    } else if big && d.x + d.y < 1.5 {
        lit = 0.45;
    }
    if lit == 0.0 {
        return vec3<f32>(0.0);
    }
    let twinkle = 0.75 + 0.25 * sin(params.time * (1.0 + 3.0 * hash(c + salt + 5.5)) + h * 40.0);
    // White, a few blue and a few warm.
    let tint = hash(c + salt + 9.9);
    var colour = vec3<f32>(1.0, 1.0, 1.0);
    if tint < 0.2 {
        colour = vec3<f32>(0.6, 0.75, 1.0);
    } else if tint > 0.9 {
        colour = vec3<f32>(1.0, 0.85, 0.6);
    }
    var bright = 0.9;
    if big {
        bright = 1.3;
    }
    return colour * lit * twinkle * bright;
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let p = floor(in.world_position.xy);
    let drift = vec2<f32>(params.time * 4.0, params.time * 1.0);
    var light = vec3<f32>(0.0);
    // Far, middle, near: each moves by its share of the camera's movement.
    light += layer(floor(p - params.camera * 0.9 + drift * 0.3), 9.0, 0.4, 0.0) * 0.5;
    light += layer(floor(p - params.camera * 0.75 + drift * 0.6), 16.0, 0.4, 17.0) * 0.75;
    light += layer(floor(p - params.camera * 0.5 + drift), 28.0, 0.4, 41.0);
    return vec4<f32>(light, 1.0);
}
