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

fn tapered_tail(local: vec2<f32>, tail_dir: vec2<f32>, tail_length: f32, width: f32) -> f32 {
    let along = dot(local, tail_dir);
    let travel = saturate(along / max(tail_length, 0.001));
    let closest = local - tail_dir * clamp(along, 0.0, tail_length);
    let across = length(closest);
    let tapered_width = mix(width * 1.10, width * 0.20, travel);
    let axial = smoothstep(-width * 0.80, width * 0.40, along) * (1.0 - smoothstep(tail_length * 0.70, tail_length, along));
    let lateral = 1.0 - smoothstep(tapered_width * 0.45, tapered_width, across);
    let head_weight = 1.0 - travel * 0.58;

    return saturate(axial * lateral * head_weight);
}

fn top_down_raindrops(uv: vec2<f32>, aspect: f32) -> vec4<f32> {
    // Render the drops as bright beads with a deliberately long motion trace.
    // Neighbor-cell sampling lets the tail extend past its source cell instead
    // of being clipped into a short dot.
    let point = vec2<f32>(uv.x * aspect, uv.y);

    var beads = 0.0;
    var blooms = 0.0;
    var glints = 0.0;
    var trails = 0.0;

    for (var layer: i32 = 0; layer < 3; layer = layer + 1) {
        let layer_f = f32(layer);
        let scale = 26.0 + layer_f * 16.0;
        let layer_offset = vec2<f32>(layer_f * 19.17, layer_f * 37.31);
        let p = point * scale + layer_offset;
        let base_cell = floor(p);

        for (var y: i32 = -2; y <= 2; y = y + 1) {
            for (var x: i32 = -2; x <= 2; x = x + 1) {
                let cell = base_cell + vec2<f32>(f32(x), f32(y));
                let base_random = hash22(cell + vec2<f32>(71.0 + layer_f * 13.0, 53.0 - layer_f * 7.0));
                let base_timing = hash22(cell + vec2<f32>(149.0 - layer_f * 3.0, 67.0 + layer_f * 23.0));
                let cycle = uniforms.time * (1.75 + base_random.y * 5.25 + base_timing.x * 1.85 + layer_f * 0.45) + base_random.x + base_timing.y * 0.53;
                let event_id = floor(cycle);
                let phase = fract(cycle);
                let event_cell = cell + vec2<f32>(event_id * 41.97, event_id * -23.39);
                let random = hash22(event_cell + vec2<f32>(71.0 + layer_f * 13.0, 53.0 - layer_f * 7.0));
                let shape = hash22(event_cell + vec2<f32>(5.0 + layer_f * 31.0, 101.0 - layer_f * 11.0));
                let timing = hash22(event_cell + vec2<f32>(149.0 - layer_f * 3.0, 67.0 + layer_f * 23.0));
                let density_jitter = (shape.x - 0.5) * 0.020;
                let active_drop = step(0.97150 + layer_f * 0.00450 + density_jitter, hash21(event_cell + vec2<f32>(11.0, 29.0) * (layer_f + 1.0)));
                if (active_drop < 0.5) {
                    continue;
                }
                let center = vec2<f32>(0.03, 0.03) + random * 0.94;
                let wobble = vec2<f32>(
                    sin(uniforms.time * (0.37 + timing.x * 0.31) + timing.y * TAU),
                    cos(uniforms.time * (0.29 + timing.y * 0.28) + timing.x * TAU)
                ) * (0.010 + shape.y * 0.018);
                let local = p - cell - center - wobble;
                let distance = length(local);
                let focus = smoothstep(0.012, 0.18 + shape.x * 0.16, phase);
                let approach = 1.0 - smoothstep(0.07 + shape.y * 0.08, 0.31 + timing.x * 0.20, phase);
                let impact = smoothstep(0.035, 0.10 + shape.x * 0.08, phase) * (1.0 - smoothstep(0.18 + timing.y * 0.10, 0.45 + random.y * 0.20, phase));
                let drop_radius = 0.086 + shape.x * 0.060 - focus * (0.036 + shape.y * 0.040) + layer_f * 0.004;
                let core = 1.0 - smoothstep(drop_radius * 0.26, drop_radius, distance);
                let soft_bloom = 1.0 - smoothstep(drop_radius * (0.58 + shape.y * 0.28), drop_radius * (1.65 + timing.x * 0.95), distance);
                let highlight_offset = vec2<f32>(-0.018 - shape.x * 0.024, -0.020 - shape.y * 0.027) * (0.65 + random.y * 0.60);
                let highlight = 1.0 - smoothstep(0.0, drop_radius * 0.24, length(local - highlight_offset));
                let impact_ring = 1.0 - smoothstep(0.0, 0.022 + shape.y * 0.024 + layer_f * 0.004, abs(distance - (0.040 + shape.x * 0.036 + phase * (0.210 + timing.y * 0.160))));
                // The falling direction converges at the center of the
                // water/ground. The visible tail trails behind the falling
                // bead, so it extends away from that vanishing point.
                let drop_point = (cell + center - layer_offset) / scale;
                let center_ground_vanish = vec2<f32>(aspect * 0.50, 0.52);
                let fall_dir = normalize(center_ground_vanish - drop_point + vec2<f32>(0.0001, 0.0001));
                let tail_skew = (shape.x - 0.5) * 0.34;
                let tail_dir = normalize(-fall_dir + vec2<f32>(-fall_dir.y, fall_dir.x) * tail_skew);
                let side_dir = vec2<f32>(-tail_dir.y, tail_dir.x);
                let side_offset = (random.x - 0.5) * (0.040 + shape.y * 0.090);
                let tail_length = 1.10 + layer_f * 0.34 + random.y * 0.95 + timing.x * 0.70;
                let tail_width = 0.034 + shape.x * 0.046 + layer_f * 0.007;
                let tail_phase = (1.0 - smoothstep(0.42 + timing.y * 0.14, 0.82 + shape.y * 0.16, phase)) * (0.58 + focus * 0.34 + shape.x * 0.38);
                let tail_wide = tapered_tail(local - side_dir * side_offset, tail_dir, tail_length, tail_width);
                let tail_core = tapered_tail(local - side_dir * side_offset, tail_dir, tail_length * (0.66 + timing.y * 0.26), tail_width * (0.26 + shape.y * 0.22));
                let brightness = 0.58 + shape.y * 0.58 + timing.x * 0.28 + layer_f * 0.08;

                beads = max(beads, active_drop * brightness * core * (approach * 1.12 + impact * 0.42));
                blooms = max(blooms, active_drop * brightness * (soft_bloom * approach * 0.45 + impact_ring * impact * 0.46));
                glints = max(glints, active_drop * brightness * (highlight * approach * 1.00 + impact_ring * impact * 0.32));
                trails = max(trails, active_drop * brightness * (tail_wide * 0.76 + tail_core * 1.08) * tail_phase);
            }
        }
    }

    return vec4<f32>(saturate(beads), saturate(blooms), saturate(glints), saturate(trails));
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
    let drops = top_down_raindrops(uv, aspect) * (0.58 + detail_mix * 0.42);
    let shimmer = surface_shimmer(uv, aspect);

    let right_water_boost = 1.0 + smoothstep(0.56, 0.98, uv.x) * 0.30;
    let ring_color = vec3<f32>(0.62, 0.82, 1.0) * impacts.x * 0.70 * right_water_boost;
    let splash_color = vec3<f32>(0.92, 0.98, 1.0) * impacts.y * 0.80 * right_water_boost;
    let halo_color = vec3<f32>(0.28, 0.48, 0.72) * impacts.z * 1.06 * right_water_boost;
    let glint_color = vec3<f32>(1.0, 1.0, 1.0) * impacts.w * 1.00 * right_water_boost;
    let broad_color = vec3<f32>(0.40, 0.62, 0.88) * broad * 0.40 * right_water_boost;
    let rain_color = vec3<f32>(0.90, 0.97, 1.0) * rain.x * 0.64 + vec3<f32>(0.74, 0.88, 1.0) * rain.y * 0.52 + vec3<f32>(0.50, 0.66, 0.88) * rain.z * 0.24;
    let drop_color = vec3<f32>(0.98, 1.0, 1.0) * drops.x * 0.72 + vec3<f32>(0.92, 0.98, 1.0) * drops.y * 0.42 + vec3<f32>(1.0, 1.0, 1.0) * drops.z * 0.78 + vec3<f32>(0.94, 0.99, 1.0) * drops.w * 1.08;
    let shimmer_color = vec3<f32>(0.36, 0.60, 0.92) * shimmer.x * 0.18 + vec3<f32>(0.004, 0.014, 0.030) * shimmer.y * 0.10;
    let water_tint = vec3<f32>(0.010, 0.022, 0.048) * 0.12;

    let color = clamp(
        (ring_color + splash_color + halo_color + glint_color + broad_color + rain_color + drop_color + shimmer_color + water_tint) * effect_mask,
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(1.0, 1.0, 1.0)
    );
    let alpha = clamp(
        (impacts.x * 0.28 + impacts.y * 0.22 + impacts.z * 0.12 + impacts.w * 0.18 + broad * 0.14 + rain.x * 0.22 + rain.y * 0.18 + rain.z * 0.10 + drops.x * 0.26 + drops.y * 0.17 + drops.z * 0.24 + drops.w * 0.48 + shimmer.x * 0.045 + shimmer.y * 0.030 + 0.032) * intensity * effect_mask,
        0.0,
        0.68
    );

    return vec4<f32>(color, alpha);
}
