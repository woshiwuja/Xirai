#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct PostProcessSettings {
    pixel_resolution: vec2<f32>,
    edge_intensity: f32,
    color_levels: f32,
    light_threshold: f32,      // Threshold for light/shadow separation
    shadow_darkness: f32,      // How dark shadows should be
    horizontal_dither: f32,    // Intensity of horizontal dithering
    contrast: f32,
    saturation: f32,
    scanline_intensity: f32,
    window_size: vec2<f32>,
    dithering_strength: f32,
    edge_threshold: f32,
    color_snap_strength: f32,
}

@group(0) @binding(2) var<uniform> settings: PostProcessSettings;

// Funzione per pixelation
fn pixelate(uv: vec2<f32>, pixel_res: vec2<f32>) -> vec2<f32> {
    return floor(uv * pixel_res + 0.5) / pixel_res;
}

fn sample_pixelated(uv: vec2<f32>) -> vec4<f32> {
    let pixelated_uv = pixelate(uv, settings.pixel_resolution);
    return textureSample(screen_texture, texture_sampler, pixelated_uv);
}

// Aggressive color reduction for crisp look
fn posterize_colors(color: vec3<f32>, levels: f32) -> vec3<f32> {
    return floor(color * levels + 0.5) / levels;
}

// Light/shadow separation based on luminance
fn apply_lighting_separation(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    
    // Horizontal dithering pattern for light areas
    let pixel_y = floor(uv.y * settings.pixel_resolution.y);
    let dither_pattern = fract(pixel_y * 0.5) * 2.0; // Creates 0 or 1 pattern
    let dither_offset = (dither_pattern - 0.5) * settings.horizontal_dither * 0.1;
    
    let adjusted_threshold = settings.light_threshold + dither_offset;
    
    if (luminance > adjusted_threshold) {
        // Light area - keep original color but posterized
        return posterize_colors(color, settings.color_levels);
    } else {
        // Shadow area - darken significantly and reduce to single color
        let darkened = color * settings.shadow_darkness;
        return posterize_colors(darkened, max(settings.color_levels * 0.5, 2.0));
    }
}

// Create very flat, cartoon-like shading
fn flat_shading(color: vec3<f32>) -> vec3<f32> {
    // Reduce to very few color levels for flat appearance
    let flat_levels = 3.0; // Very aggressive posterization
    return floor(color * flat_levels + 0.5) / flat_levels;
}

// Enhance character crispness by sharpening color boundaries
fn enhance_crispness(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let texel_size = 1.0 / settings.pixel_resolution;
    
    // Sample surrounding pixels
    let center = sample_pixelated(uv);
    let right = sample_pixelated(uv + vec2<f32>(texel_size.x, 0.0));
    let down = sample_pixelated(uv + vec2<f32>(0.0, texel_size.y));
    
    // If there's a significant color difference, snap to more extreme values
    let center_lum = dot(center.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let right_lum = dot(right.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let down_lum = dot(down.rgb, vec3<f32>(0.299, 0.587, 0.114));
    
    let max_diff = max(abs(center_lum - right_lum), abs(center_lum - down_lum));
    
    if (max_diff > 0.1) {
        // Near an edge - make color more extreme
        if (center_lum > 0.5) {
            return mix(color, vec3<f32>(1.0), 0.3); // Push towards white
        } else {
            return mix(color, vec3<f32>(0.0), 0.3); // Push towards black
        }
    }
    
    return color;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Sample with pixelation
    var color = sample_pixelated(in.uv);
    
    // Apply flat shading first
    let flat_color = flat_shading(color.rgb);
    color = vec4<f32>(flat_color, color.a);
    
    // Apply lighting separation with horizontal dithering
    let lit_color = apply_lighting_separation(color.rgb, in.uv);
    color = vec4<f32>(lit_color, color.a);
    
    // Enhance crispness for characters
    let crisp_color = enhance_crispness(color.rgb, in.uv);
    color = vec4<f32>(crisp_color, color.a);
    
    // Strong contrast for that sharp look
    let contrast_color = (color.rgb - 0.5) * max(settings.contrast, 1.5) + 0.5;
    color = vec4<f32>(clamp(contrast_color, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);
    
    // Controlled saturation
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let saturated_color = mix(vec3<f32>(luminance), color.rgb, settings.saturation);
    color = vec4<f32>(saturated_color, color.a);
    
    return color;
}