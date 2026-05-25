struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    intensity: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

const TAU: f32 = 6.2831853;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0)
    );

    let position = positions[vertex_index];

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);

    return output;
}

fn saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

fn ease_out_cubic(value: f32) -> f32 {
    let t = saturate(value);
    return 1.0 - pow(1.0 - t, 3.0);
}

fn hash21(value: vec2<f32>) -> f32 {
    return fract(sin(dot(value, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn hash22(value: vec2<f32>) -> vec2<f32> {
    return fract(sin(vec2<f32>(
        dot(value, vec2<f32>(269.5, 183.3)),
        dot(value, vec2<f32>(113.5, 271.9))
    )) * 43758.5453123);
}

fn water_space(uv: vec2<f32>, aspect: f32) -> vec2<f32> {
    // The bundled wallpaper is a top-down view of a figure floating on a
    // water-like surface. Keep rings circular in world space instead of using a
    // horizon/ground-plane projection.
    return vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);
}

fn water_visibility(uv: vec2<f32>, aspect: f32) -> f32 {
    // Keep the effect strongest on the open-water areas around the character
    // while still letting thin highlights pass over the reflected clothing.
    let body_soft_mask = 1.0 - smoothstep(
        0.38,
        0.86,
        length((uv - vec2<f32>(0.48, 0.52)) / vec2<f32>(0.42, 0.36))
    );
    let right_pool = smoothstep(0.50, 0.96, uv.x);
    let left_pool = 1.0 - smoothstep(0.16, 0.50, uv.x);
    let bottom_pool = smoothstep(0.56, 0.98, uv.y);
    let upper_pool = 1.0 - smoothstep(0.10, 0.40, uv.y);
    let aspect_fade = smoothstep(0.35, 1.20, length(water_space(uv, aspect) / vec2<f32>(1.20, 0.72)));

    return clamp(
        0.68 + right_pool * 0.20 + left_pool * 0.12 + bottom_pool * 0.08 + upper_pool * 0.07 + aspect_fade * 0.05 - body_soft_mask * 0.22,
        0.44,
        1.08
    );
}

fn ring_mask(point: vec2<f32>, center: vec2<f32>, radius: f32, width: f32) -> f32 {
    let delta = point - center;
    let distance = length(delta);

    return 1.0 - smoothstep(0.0, width, abs(distance - radius));
}

fn local_ring_mask(local: vec2<f32>, radius: f32, width: f32) -> f32 {
    return 1.0 - smoothstep(0.0, width, abs(length(local) - radius));
}

fn splash_mask(point: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
    return 1.0 - smoothstep(0.0, radius, length(point - center));
}

fn broken_arc(point: vec2<f32>, center: vec2<f32>, seed: f32, life: f32) -> f32 {
    let direction = normalize(point - center + vec2<f32>(0.0001, -0.0002));
    let wave_a = sin((direction.x * 13.0 + direction.y * 17.0) + seed * TAU + life * 4.0);
    let wave_b = sin((direction.x * -19.0 + direction.y * 11.0) + seed * 11.37 - life * 3.4);
    let wave = max(wave_a, wave_b * 0.78);

    return 0.44 + 0.56 * smoothstep(-0.55, 0.96, wave);
}

fn crescent_highlight(point: vec2<f32>, center: vec2<f32>, seed: f32, life: f32) -> f32 {
    let direction = normalize(point - center + vec2<f32>(0.0002, 0.0001));
    let light_dir = normalize(vec2<f32>(-0.58, -0.82));
    let facing = smoothstep(0.08, 0.94, dot(direction, light_dir));
    let twinkle = 0.72 + 0.28 * sin(seed * TAU + life * 8.0 + direction.x * 9.0 - direction.y * 6.0);

    return facing * twinkle;
}

fn surface_shimmer(uv: vec2<f32>, aspect: f32) -> vec2<f32> {
    let point = water_space(uv, aspect);
    let time = uniforms.time;
    let wave_a = sin(dot(point, vec2<f32>(8.5, 5.2)) + time * 0.46);
    let wave_b = sin(dot(point, vec2<f32>(-5.8, 11.2)) - time * 0.38 + wave_a * 0.36);
    let wave_c = sin(dot(point, vec2<f32>(22.0, -15.5)) + time * 1.02 + wave_b * 0.58);
    let fine = wave_a * 0.46 + wave_b * 0.36 + wave_c * 0.18;
    let thread = sin(dot(point, vec2<f32>(54.0, 37.0)) + time * 1.55 + fine * 1.2);
    let soft_glow = smoothstep(0.30, 0.94, fine) * smoothstep(-0.85, 0.40, wave_b);

    let highlights = smoothstep(0.62, 1.0, fine) * 0.58 + smoothstep(0.90, 1.0, thread) * 0.18 + soft_glow * 0.14;
    let troughs = smoothstep(0.58, 0.98, -fine) * 0.46;

    return vec2<f32>(saturate(highlights), saturate(troughs));
}

fn impact_ripples(uv: vec2<f32>, aspect: f32) -> vec4<f32> {
    let point = water_space(uv, aspect);
    let grid_scale = 3.85;
    let base_cell = floor(point * grid_scale);

    var rings = 0.0;
    var splashes = 0.0;
    var halos = 0.0;
    var glints = 0.0;

    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let cell = base_cell + vec2<f32>(f32(x), f32(y));
            let random = hash22(cell);
            let center = (cell + random) / grid_scale;
            let center_uv_x = center.x / aspect + 0.5;
            let right_bias = smoothstep(0.52, 0.96, center_uv_x);
            let left_bias = 1.0 - smoothstep(0.18, 0.54, center_uv_x);
            let activity_threshold = 0.70 - right_bias * 0.30 - left_bias * 0.12;
            let is_active = step(activity_threshold, hash21(cell + vec2<f32>(9.2, 4.7)));
            if (is_active < 0.5) {
                continue;
            }
            if (hash21(cell + vec2<f32>(13.7, 22.3)) > 0.5) {
                continue;
            }
            let rate = 0.16 + random.y * 0.24;
            let life = fract(uniforms.time * rate + random.x);
            let grow = ease_out_cubic(life);
            let fade_in = smoothstep(0.02, 0.13, life);
            let fade_out = 1.0 - smoothstep(0.58, 1.0, life);
            let fade = fade_in * fade_out;
            let radius = 0.014 + grow * (0.23 + random.y * 0.16);
            let width = 0.0038 + life * 0.016;
            let arc = broken_arc(point, center, random.x, life);
            let main_ring = ring_mask(point, center, radius, width) * arc;
            let inner_ring = ring_mask(point, center, max(radius * 0.52 - 0.010, 0.0), width * 0.66) * (1.0 - smoothstep(0.16, 0.54, life));
            let outer_ring = ring_mask(point, center, radius * 1.34 + 0.022, width * 1.20) * broken_arc(point, center, random.y + 0.37, life) * (1.0 - smoothstep(0.22, 0.88, life));
            let crown = ring_mask(point, center, 0.020 + life * 0.034, 0.007 + life * 0.005) * broken_arc(point, center, random.x + 0.61, life) * (1.0 - smoothstep(0.10, 0.32, life));
            let splash = splash_mask(point, center, 0.014 + random.y * 0.012);
            let halo = splash_mask(point, center, radius * 0.92 + 0.060);
            let glint_offset = vec2<f32>(-0.010, -0.015) * (0.65 + random.x);
            let impact_glint = splash_mask(point, center + glint_offset, 0.008 + random.y * 0.004);
            let ring_glint = (main_ring + outer_ring * 0.45) * crescent_highlight(point, center, random.y, life);

            rings = max(rings, (main_ring + inner_ring * 0.36 + outer_ring * 0.38) * fade);
            splashes = max(splashes, splash * pow(1.0 - life, 2.8) + crown * 0.72);
            halos = max(halos, halo * fade * 0.32);
            glints = max(glints, ring_glint * 0.88 * fade + impact_glint * pow(1.0 - life, 4.6));
        }
    }

    return vec4<f32>(saturate(rings), saturate(splashes), saturate(halos), saturate(glints));
}

fn broad_surface_ripples(uv: vec2<f32>, aspect: f32) -> f32 {
    let point = water_space(uv, aspect);
    let grid_scale = 1.35;
    let base_cell = floor(point * grid_scale);

    var ripples = 0.0;

    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let cell = base_cell + vec2<f32>(f32(x), f32(y));
            if (hash21(cell + vec2<f32>(4.1, 7.7)) < 0.58) {
                continue;
            }
            if (hash21(cell + vec2<f32>(18.3, 13.6)) > 0.5) {
                continue;
            }
            let random = hash22(cell + vec2<f32>(31.0, 11.0));
            let center = (cell + random) / grid_scale;
            let life = fract(uniforms.time * (0.040 + random.y * 0.040) + random.x);
            let grow = ease_out_cubic(life);
            let radius = 0.11 + grow * (0.54 + random.y * 0.28);
            let width = 0.0075 + life * 0.026;
            let fade = smoothstep(0.03, 0.24, life) * (1.0 - smoothstep(0.68, 1.0, life));
            let main = ring_mask(point, center, radius, width) * broken_arc(point, center, random.y, life);
            let echo = ring_mask(point, center, radius * 0.58 + 0.050, width * 0.70) * broken_arc(point, center, random.x + 0.53, life) * (1.0 - smoothstep(0.26, 0.82, life));
            let glancing = crescent_highlight(point, center, random.x, life) * 0.40 + 0.66;

            ripples = max(ripples, (main + echo * 0.46) * fade * glancing);
        }
    }

    return saturate(ripples);
}
