#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct PostProcessSettings {
    // gruppo 0 (16 bytes)
    edge_denoise: f32,
    edge_intensity: f32,
    color_levels: f32,
    cel_levels: f32,

    // gruppo 1 (16 bytes)
    contrast: f32,
    saturation: f32,
    scanline_intensity: f32,
    _pad1: f32,

    // gruppo 2 (16 bytes)
    window_size: vec2<f32>,
    _pad2: vec2<f32>,

    // gruppo 3 (16 bytes)
    dithering_strength: f32,
    edge_threshold: f32,
    color_snap_strength: f32,
    _pad3: f32,

    // palette
    palette: array<vec4<f32>, 32>,

    // ultimo gruppo
    color_count_and_pad: vec4<u32>,
}

@group(0) @binding(2) var<uniform> settings: PostProcessSettings;


// Trova il colore più vicino nella palette
fn nearest_color(gray_value: f32) -> vec3<f32> {
    var min_dist: f32 = 99999.0;
    var best: vec3<f32> = vec3<f32>(gray_value);

    let count = i32(settings.color_count_and_pad.x);

    for (var i: i32 = 0; i < count && i < 32; i = i + 1) {
        let palette_color = settings.palette[i].rgb;
        let p_lum = dot(palette_color, vec3<f32>(0.2126, 0.7152, 0.0722));
        let d = abs(gray_value - p_lum);
        if (d < min_dist) {
            min_dist = d;
            best = palette_color;
        }
    }
    return best;
}


fn nearest_color_rgb(color: vec3<f32>) -> vec3<f32> {
    var min_dist: f32 = 1.0e8;
    var best: vec3<f32> = color;

    let count = i32(settings.color_count_and_pad.x);

    for (var i: i32 = 0; i < count && i < 32; i = i + 1) {
        let palette_color = settings.palette[i].rgb;
        let diff = color - palette_color;
        let d = dot(diff, diff);
        if (d < min_dist) {
            min_dist = d;
            best = palette_color;
        }
    }
    return best;
}


// Dithering
fn ordered_dither(uv: vec2<f32>, value: f32) -> f32 {
    let bayer = array<f32, 16>(
        0.0/16.0,  8.0/16.0,  2.0/16.0, 10.0/16.0,
        12.0/16.0, 4.0/16.0, 14.0/16.0,  6.0/16.0,
        3.0/16.0, 11.0/16.0,  1.0/16.0,  9.0/16.0,
        15.0/16.0, 7.0/16.0, 13.0/16.0,  5.0/16.0
    );

    let pixel_pos = uv * settings.window_size;
    let x = i32(pixel_pos.x) % 4;
    let y = i32(pixel_pos.y) % 4;
    let index = y * 4 + x;

    let dither_value = (bayer[index] - 0.5) * settings.dithering_strength;
    return clamp(value + dither_value, 0.0, 1.0);
}


// Edge detection
fn detect_edges(uv: vec2<f32>) -> f32 {
    let texel_size = 1.0 / settings.window_size;

    let center = textureSample(screen_texture, texture_sampler, uv);
    let left = textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-texel_size.x, 0.0));
    let right = textureSample(screen_texture, texture_sampler, uv + vec2<f32>(texel_size.x, 0.0));
    let up = textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0, -texel_size.y));
    let down = textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0, texel_size.y));

    let center_lum = dot(center.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let left_lum = dot(left.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let right_lum = dot(right.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let up_lum = dot(up.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let down_lum = dot(down.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));

    let edge_x = abs(right_lum - left_lum);
    let edge_y = abs(down_lum - up_lum);
    let edge = sqrt(edge_x * edge_x + edge_y * edge_y);

    return edge;
}


// Scanlines CRT
fn apply_scanlines(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let scanline = sin(uv.y * settings.window_size.y * 3.14159) * 0.5 + 0.5;
    let scanline_effect = mix(1.0, scanline, settings.scanline_intensity);
    return color * scanline_effect;
}


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let color = textureSample(screen_texture, texture_sampler, uv).rgb;

    // 1. Contrasto
    var contrasted_color = (color - vec3<f32>(0.5)) * settings.contrast + vec3<f32>(0.5);
    contrasted_color = clamp(contrasted_color, vec3<f32>(0.0), vec3<f32>(1.0));

    // 2. Saturazione
    let gray = dot(contrasted_color, vec3<f32>(0.2126, 0.7152, 0.0722));
    var saturated_color = mix(vec3<f32>(gray), contrasted_color, settings.saturation);

    // 3. Luminanza dal colore corretto
    var lum = dot(saturated_color, vec3<f32>(0.2126, 0.7152, 0.0722));
    lum = clamp(lum, 0.0, 1.0);

    // 4. Edge detection
    let edge = detect_edges(uv);
    if (edge > settings.edge_threshold) {
        lum = mix(lum, lum * 0.5, settings.edge_intensity);
    }

    // 5. Dithering
    lum = ordered_dither(uv, lum);

    // 6. Quantizzazione / palette
    var final_color = nearest_color(lum);

// 7. Cel shading (quantizzazione della luce)
if (settings.cel_levels >= 1.0) {
    let cel_steps = max(settings.cel_levels, 1.0);
    let cel_lum = floor(lum * cel_steps) / cel_steps;

    // Applica la quantizzazione alla luminanza ma mantieni il colore originale
    final_color = saturated_color * (cel_lum / max(lum, 0.0001));
}

    // 8. Color snap
    if (settings.color_snap_strength > 0.0) {
        if (lum > 0.5) {
            final_color = mix(final_color, vec3<f32>(1.0), settings.color_snap_strength * 0.3);
        } else {
            final_color = mix(final_color, vec3<f32>(0.0), settings.color_snap_strength * 0.3);
        }
    }

    // 9. Scanlines
    final_color = apply_scanlines(final_color, uv);

    return vec4<f32>(final_color, 1.0);
}
