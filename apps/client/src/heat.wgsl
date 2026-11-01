// Heat shimmer over a fire (`docs/reference/lighting.md`, Fire): faint warm bands that
// waver and rise, strongest just over the flames and fading upward and to the sides.
// Added to what is under it.

struct Params {
    // Seconds; a per-fire offset so neighbouring fires don't shimmer in step.
    time: f32,
    seed: f32,
    _pad: vec2<f32>,
}

@group(2) @binding(0) var<uniform> params: Params;

struct FragmentInput {
    @location(2) uv: vec2<f32>,
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // uv.y is 0 at the top of the quad, 1 at its foot.
    let up = 1.0 - in.uv.y;
    let x = in.uv.x - 0.5;
    let t = params.time + params.seed;
    // Bands rising, bent side to side.
    let bend = sin(up * 7.0 - t * 2.3) * 0.08 + sin(up * 13.0 + t * 3.1) * 0.03;
    let bands = 0.5 + 0.5 * sin((up * 18.0 - t * 5.0) + sin((x + bend) * 14.0 + t * 1.7) * 1.6);
    // Strongest over the flames, gone at the top and sides.
    let fade = (1.0 - up) * (1.0 - smoothstep(0.15, 0.5, abs(x + bend)));
    let a = bands * fade * 0.16;
    return vec4<f32>(vec3<f32>(1.0, 0.75, 0.45) * a, 1.0);
}
