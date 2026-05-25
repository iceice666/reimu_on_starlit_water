fn rain_impacts(uv: vec2<f32>, aspect: f32) -> vec3<f32> {
    // Keep the impact grid in screen/world units. Multiplying by aspect twice
    // makes the hit flashes sub-pixel on widescreen displays.
    let sample = vec2<f32>(uv.x * aspect, uv.y);

    var pins = 0.0;
    var rings = 0.0;
    var mist = 0.0;

    for (var layer: i32 = 0; layer < 2; layer = layer + 1) {
        let layer_f = f32(layer);
        let grid_scale = vec2<f32>(34.0 + layer_f * 20.0, 22.0 + layer_f * 14.0);
        let grid = sample * grid_scale;
        let base_cell = floor(grid);

        for (var y: i32 = -1; y <= 1; y = y + 1) {
            for (var x: i32 = -1; x <= 1; x = x + 1) {
                let cell = base_cell + vec2<f32>(f32(x), f32(y));
                let base_random = hash22(cell + vec2<f32>(17.0 + layer_f * 13.0, 41.0 - layer_f * 9.0));
                let cycle = uniforms.time * (2.2 + base_random.y * 5.8 + layer_f * 0.72) + base_random.x;
                let event_id = floor(cycle);
                let phase = fract(cycle);
                let event_cell = cell + vec2<f32>(event_id * 37.13, event_id * -19.71);
                let random = hash22(event_cell + vec2<f32>(17.0 + layer_f * 13.0, 41.0 - layer_f * 9.0));
                let random_b = hash22(event_cell + vec2<f32>(83.0 - layer_f * 5.0, 23.0 + layer_f * 17.0));
                let density_jitter = (random_b.x - 0.5) * 0.018;
                let present = step(0.96125 + layer_f * 0.00675 + density_jitter, hash21(event_cell + vec2<f32>(9.2, 4.7) * (layer_f + 1.0)));
                if (present < 0.5) {
                    continue;
                }
                let center = vec2<f32>(0.04, 0.04) + random * 0.92;
                let local = (grid - cell - center) * vec2<f32>(grid_scale.y / grid_scale.x, 1.0);
                let distance = length(local);
                let pulse = 0.72 + random_b.x * 0.56;
                let approach = (1.0 - smoothstep(0.025 + random_b.y * 0.030, 0.22 + random.x * 0.16, phase)) * pulse;
                let ring_fade = smoothstep(0.020, 0.12 + random_b.x * 0.07, phase) * (1.0 - smoothstep(0.30 + random.y * 0.16, 0.72 + random_b.y * 0.18, phase));
                let pin_radius = 0.024 + random.x * 0.024 + layer_f * 0.006;
                let bead_radius = 0.080 + random_b.y * 0.070 + layer_f * 0.026;
                let spray_radius = 0.170 + random.x * 0.140;
                let pin = 1.0 - smoothstep(0.000, pin_radius, distance);
                let bead = 1.0 - smoothstep(pin_radius * 0.92, bead_radius, distance);
                let spray = 1.0 - smoothstep(bead_radius * 0.70, spray_radius, distance);
                let ring_radius = 0.022 + random_b.x * 0.030 + phase * (0.24 + random.x * 0.19);
                let tiny_ring = local_ring_mask(local, ring_radius, 0.014 + random_b.y * 0.020 + phase * 0.030);
                let crown = local_ring_mask(local, 0.045 + random.x * 0.055 + phase * (0.035 + random_b.y * 0.065), 0.020 + random_b.x * 0.020) * (1.0 - smoothstep(0.10, 0.34 + random.y * 0.18, phase));
                let brightness = 0.62 + random_b.y * 0.70 + layer_f * 0.14;

                pins = max(pins, present * brightness * (pin * approach + bead * approach * 0.38));
                rings = max(rings, present * brightness * (tiny_ring * ring_fade + crown * approach * 0.42));
                mist = max(mist, present * brightness * spray * (approach * 0.22 + ring_fade * 0.18));
            }
        }
    }

    return vec3<f32>(saturate(pins), saturate(rings), saturate(mist));
}

fn top_down_raindrops(uv: vec2<f32>, aspect: f32) -> vec4<f32> {
    // Budget version: keep the current 2 * 3 * 3 candidate loop,
    // but make the 4th channel a cheap perspective rain tail.
    let point = vec2<f32>(uv.x * aspect, uv.y);

    var cores = 0.0;
    var halos = 0.0;
    var glints = 0.0;
    var trails = 0.0;

    for (var layer: i32 = 0; layer < 2; layer = layer + 1) {
        let layer_f = f32(layer);
        let scale = 32.0 + layer_f * 13.0;
        let layer_offset = vec2<f32>(layer_f * 19.17, layer_f * 37.31);
        let p = point * scale + layer_offset;
        let base_cell = floor(p);

        for (var y: i32 = -1; y <= 1; y = y + 1) {
            for (var x: i32 = -1; x <= 1; x = x + 1) {
                let cell = base_cell + vec2<f32>(f32(x), f32(y));

                // Keep the pre-gate work cheap. Only compute shape/detail
                // randomness after the cell is known to be active.
                let base_random = hash22(cell + vec2<f32>(71.0 + layer_f * 13.0, 53.0 - layer_f * 7.0));
                let cycle = uniforms.time * (1.55 + base_random.y * 3.85 + layer_f * 0.38) + base_random.x;
                let event_id = floor(cycle);
                let phase = fract(cycle);
                let event_cell = cell + vec2<f32>(event_id * 41.97, event_id * -23.39);
                let active_drop = step(
                    0.9535 + layer_f * 0.0100,
                    hash21(event_cell + vec2<f32>(11.0, 29.0) * (layer_f + 1.0))
                );

                if (active_drop < 0.5) {
                    continue;
                }

                let event_random = hash22(event_cell + vec2<f32>(17.0, 29.0));
                let event_shape = hash22(event_cell + vec2<f32>(113.0, 61.0));

                let center = vec2<f32>(0.05, 0.05) + event_random * 0.90;
                let local = p - cell - center;

                // Bead / wet highlight.
                let q = vec2<f32>(local.x, local.y * 1.10);
                let distance = length(q);

                let live = smoothstep(0.010, 0.070, phase) * (1.0 - smoothstep(0.32, 0.72, phase));
                let tail_live = 1.0 - smoothstep(0.42, 0.90, phase);
                let radius = 0.052 + event_shape.x * 0.034 + layer_f * 0.005;
                let brightness = 0.82 + event_shape.y * 0.46 + layer_f * 0.06;

                let core = (1.0 - smoothstep(radius * 0.30, radius, distance)) * live;
                let halo = (1.0 - smoothstep(radius * 0.82, radius * 1.95, distance)) * live;

                // Perspective alignment: drops fall toward the scene/water
                // vanishing point; the visible streak trails behind the bead.
                let drop_point = (cell + center - layer_offset) / scale;
                let center_ground_vanish = vec2<f32>(aspect * 0.50, 0.52);
                let fall_dir = normalize(center_ground_vanish - drop_point + vec2<f32>(0.0001, 0.0001));
                let tail_dir = -fall_dir;
                let side_dir = vec2<f32>(-tail_dir.y, tail_dir.x);

                // Single analytic streak: no 5x5 sampling, no helper call,
                // no second tail pass.
                let side_offset = (event_random.x - 0.5) * 0.055;
                let tail_local = local - side_dir * side_offset;
                let along = dot(tail_local, tail_dir);
                let across = abs(dot(tail_local, side_dir));

                let tail_length = 1.18 + event_shape.y * 0.48 + layer_f * 0.28;
                let tail_width = 0.026 + event_shape.x * 0.020 + layer_f * 0.004;
                let tail_t = saturate(along / max(tail_length, 0.001));

                let axial = smoothstep(-tail_width * 0.45, tail_width * 0.25, along) *
                    (1.0 - smoothstep(tail_length * 0.72, tail_length, along));
                let tapered_width = tail_width * (1.25 - tail_t * 0.95);
                let lateral = 1.0 - smoothstep(tapered_width * 0.42, tapered_width, across);
                let trail = axial * lateral * (1.0 - tail_t * 0.62) * tail_live;

                // Cheap directional glint on the bead.
                let glint_side = saturate(0.55 - (q.x * 0.58 + q.y * 0.86) / max(radius * 2.0, 0.001));
                let glint = core * glint_side * (0.72 + event_shape.y * 0.36);

                cores = max(cores, active_drop * brightness * core);
                halos = max(halos, active_drop * brightness * halo * 0.38);
                glints = max(glints, active_drop * brightness * glint);
                trails = max(trails, active_drop * brightness * trail * 0.92);
            }
        }
    }

    return vec4<f32>(saturate(cores), saturate(halos), saturate(glints), saturate(trails));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = clamp(input.uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    let aspect = max(uniforms.resolution.x / max(uniforms.resolution.y, 1.0), 0.25);
    let intensity = saturate(uniforms.intensity);
    if (intensity <= 0.001) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let vignette = 1.0 - smoothstep(0.50, 1.02, distance(uv, vec2<f32>(0.5, 0.50)));
    let center_soften = 1.0 - smoothstep(0.00, 0.34, length(water_space(uv, aspect) / vec2<f32>(1.20, 0.78))) * 0.18;
    let water_mask = water_visibility(uv, aspect);
    let effect_mask = vignette * center_soften * water_mask;
    if (effect_mask < 0.02) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let detail_mix = smoothstep(0.30, 0.90, intensity);
    let impacts = impact_ripples(uv, aspect);
    let broad = broad_surface_ripples(uv, aspect) * (0.55 + detail_mix * 0.45);
    let rain = rain_impacts(uv, aspect) * (0.60 + detail_mix * 0.40);
    let drops = top_down_raindrops(uv, aspect) * (0.72 + detail_mix * 0.28);
    let shimmer = surface_shimmer(uv, aspect);

    let right_water_boost = 1.0 + smoothstep(0.56, 0.98, uv.x) * 0.30;
    let ring_color = vec3<f32>(0.62, 0.82, 1.0) * impacts.x * 0.70 * right_water_boost;
    let splash_color = vec3<f32>(0.92, 0.98, 1.0) * impacts.y * 0.80 * right_water_boost;
    let halo_color = vec3<f32>(0.28, 0.48, 0.72) * impacts.z * 1.06 * right_water_boost;
    let glint_color = vec3<f32>(1.0, 1.0, 1.0) * impacts.w * 1.00 * right_water_boost;
    let broad_color = vec3<f32>(0.40, 0.62, 0.88) * broad * 0.40 * right_water_boost;
    let rain_color = vec3<f32>(0.90, 0.97, 1.0) * rain.x * 0.64 + vec3<f32>(0.74, 0.88, 1.0) * rain.y * 0.52 + vec3<f32>(0.50, 0.66, 0.88) * rain.z * 0.24;
    let drop_color =
        vec3<f32>(1.0, 1.0, 1.0) * drops.x * 0.82 +
        vec3<f32>(0.86, 0.94, 1.0) * drops.y * 0.30 +
        vec3<f32>(1.0, 1.0, 1.0) * drops.z * 0.72 +
        vec3<f32>(0.78, 0.90, 1.0) * drops.w * 0.92;
    let shimmer_color = vec3<f32>(0.36, 0.60, 0.92) * shimmer.x * 0.18 + vec3<f32>(0.004, 0.014, 0.030) * shimmer.y * 0.10;
    let water_tint = vec3<f32>(0.010, 0.022, 0.048) * 0.12;

    let color = clamp(
        (ring_color + splash_color + halo_color + glint_color + broad_color + rain_color + drop_color + shimmer_color + water_tint) * effect_mask,
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(1.0, 1.0, 1.0)
    );
    let alpha = clamp(
        (impacts.x * 0.28 + impacts.y * 0.22 + impacts.z * 0.12 + impacts.w * 0.18 + broad * 0.14 + rain.x * 0.22 + rain.y * 0.18 + rain.z * 0.10 + drops.x * 0.25 + drops.y * 0.08 + drops.z * 0.16 + drops.w * 0.30 + shimmer.x * 0.045 + shimmer.y * 0.030 + 0.032) * intensity * effect_mask,
        0.0,
        0.68
    );

    return vec4<f32>(color, alpha);
}
