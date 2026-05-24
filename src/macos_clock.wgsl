struct Uniforms {
    resolution: vec2<f32>,
    wallpaper_size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var wallpaper_texture: texture_2d<f32>;

@group(0) @binding(2)
var wallpaper_sampler: sampler;

@group(0) @binding(3)
var mask_texture: texture_2d<f32>;

@group(0) @binding(4)
var mask_sampler: sampler;

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

fn cover_wallpaper_uv(screen_uv: vec2<f32>) -> vec2<f32> {
    let screen_aspect = uniforms.resolution.x / max(uniforms.resolution.y, 1.0);
    let wallpaper_aspect = uniforms.wallpaper_size.x / max(uniforms.wallpaper_size.y, 1.0);

    if (screen_aspect > wallpaper_aspect) {
        let visible_height = wallpaper_aspect / screen_aspect;
        return vec2<f32>(
            screen_uv.x,
            (screen_uv.y - 0.5) * visible_height + 0.5
        );
    }

    let visible_width = screen_aspect / wallpaper_aspect;
    return vec2<f32>(
        (screen_uv.x - 0.5) * visible_width + 0.5,
        screen_uv.y
    );
}

fn mask_at(uv: vec2<f32>, offset_px: vec2<f32>) -> f32 {
    let offset_uv = offset_px / max(uniforms.resolution, vec2<f32>(1.0, 1.0));
    return textureSample(mask_texture, mask_sampler, clamp(uv + offset_uv, vec2<f32>(0.0), vec2<f32>(1.0))).r;
}

fn blurred_wallpaper(screen_uv: vec2<f32>) -> vec3<f32> {
    let texel = 1.0 / max(uniforms.resolution, vec2<f32>(1.0, 1.0));
    let radius = 13.0;
    var color = textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv)).rgb * 0.16;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>( radius,  0.0))).rgb * 0.105;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>(-radius,  0.0))).rgb * 0.105;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>( 0.0,  radius))).rgb * 0.105;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>( 0.0, -radius))).rgb * 0.105;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>( radius,  radius))).rgb * 0.08;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>(-radius,  radius))).rgb * 0.08;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>( radius, -radius))).rgb * 0.08;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>(-radius, -radius))).rgb * 0.08;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>( radius * 1.55,  radius * 0.55))).rgb * 0.045;
    color += textureSample(wallpaper_texture, wallpaper_sampler, cover_wallpaper_uv(screen_uv + texel * vec2<f32>(-radius * 1.55, -radius * 0.55))).rgb * 0.045;

    return color;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = clamp(vec2<f32>(input.uv.x, 1.0 - input.uv.y), vec2<f32>(0.0), vec2<f32>(1.0));
    let core = mask_at(uv, vec2<f32>(0.0));
    let near = max(
        max(mask_at(uv, vec2<f32>( 1.4, 0.0)), mask_at(uv, vec2<f32>(-1.4, 0.0))),
        max(mask_at(uv, vec2<f32>(0.0,  1.4)), mask_at(uv, vec2<f32>(0.0, -1.4)))
    );
    let glow = max(
        max(mask_at(uv, vec2<f32>( 7.0, 0.0)), mask_at(uv, vec2<f32>(-7.0, 0.0))),
        max(mask_at(uv, vec2<f32>(0.0,  7.0)), mask_at(uv, vec2<f32>(0.0, -7.0)))
    );
    let shadow = mask_at(uv, vec2<f32>(2.8, 3.6));
    let alpha = max(core, glow * 0.18);

    if (alpha <= 0.002) {
        return vec4<f32>(0.0);
    }

    let blurred = blurred_wallpaper(uv);
    let warm_white = vec3<f32>(1.0, 0.985, 0.90);
    let milky = mix(blurred * 1.10, warm_white, 0.42);
    let edge_lift = warm_white * near * 0.105;
    let shadow_tint = vec3<f32>(0.16, 0.20, 0.24) * shadow * 0.13;
    let glow_tint = warm_white * glow * 0.13;
    let color = clamp(milky * core + edge_lift + glow_tint - shadow_tint, vec3<f32>(0.0), vec3<f32>(1.0));
    let out_alpha = clamp(core * 0.48 + near * 0.11 + glow * 0.075 + shadow * 0.035, 0.0, 0.66);

    return vec4<f32>(color, out_alpha);
}
