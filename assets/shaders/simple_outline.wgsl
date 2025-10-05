// assets/shaders/simple_outline.wgsl
// Simple single-pass outline shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index) - 1);
    let y = f32(i32(vertex_index & 1u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: SimpleOutlineSettings;

struct SimpleOutlineSettings {
    outline_thickness: f32,
    outline_color: vec4<f32>,
    edge_threshold: f32,
    texture_size: vec2<f32>,
    intensity: f32,
    _pad: vec3<f32>,
}

fn rgb_to_luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

fn sample_outline(uv: vec2<f32>, thickness: f32) -> f32 {
    let texel_size = 1.0 / settings.texture_size;
    let step = texel_size * thickness;
    
    let center = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv).rgb);
    var max_diff = 0.0;
    
    // Sample in multiple directions
    let directions = array<vec2<f32>, 8>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 0.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  0.0),
        vec2<f32>( 1.0,  0.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 0.0,  1.0),
        vec2<f32>( 1.0,  1.0),
    );
    
    for (var i = 0; i < 8; i++) {
        let sample_uv = uv + directions[i] * step;
        let sample_lum = rgb_to_luminance(textureSample(screen_texture, texture_sampler, sample_uv).rgb);
        max_diff = max(max_diff, abs(center - sample_lum));
    }
    
    return max_diff;
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let original_color = textureSample(screen_texture, texture_sampler, uv);
    
    // Calculate outline strength
    let outline_strength = sample_outline(uv, settings.outline_thickness);
    
    // Apply outline if above threshold
    if (outline_strength > settings.edge_threshold) {
        let outline_alpha = min(outline_strength * 5.0, 1.0); // Enhance contrast
        return mix(original_color, settings.outline_color, outline_alpha * settings.intensity);
    }
    
    return original_color;
}