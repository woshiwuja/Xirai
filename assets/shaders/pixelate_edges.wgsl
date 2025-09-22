#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct PostProcessSettings {
    pixel_resolution: vec2<f32>,
    edge_intensity: f32,
    color_levels: f32,
    cel_levels: f32,        // Livelli di cel shading
    contrast: f32,
    saturation: f32,
    scanline_intensity: f32,
    window_size: vec2<f32>,
}

@group(0) @binding(2) var<uniform> settings: PostProcessSettings;

// Funzione per pixelation
fn pixelate(uv: vec2<f32>, pixel_res: vec2<f32>) -> vec2<f32> {
    return floor(uv * pixel_res) / pixel_res;
}

// Funzione per campionare la texture con pixelation
fn sample_pixelated(uv: vec2<f32>) -> vec4<f32> {
    let pixelated_uv = pixelate(uv, settings.pixel_resolution);
    return textureSample(screen_texture, texture_sampler, pixelated_uv);
}

// Edge detection usando Sobel operator
fn detect_edges(uv: vec2<f32>) -> f32 {
    let texel_size = 1.0 / settings.pixel_resolution;
    
    var gx = 0.0;
    var gy = 0.0;
    
    // Applica i kernel Sobel manualmente
    let offsets = array<vec2<f32>, 9>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(0.0, -1.0), vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0,  0.0), vec2<f32>(0.0,  0.0), vec2<f32>(1.0,  0.0),
        vec2<f32>(-1.0,  1.0), vec2<f32>(0.0,  1.0), vec2<f32>(1.0,  1.0)
    );
    
    let sobel_x_weights = array<f32, 9>(-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0);
    let sobel_y_weights = array<f32, 9>(-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0);
    
    for (var i = 0; i < 9; i++) {
        let sample_uv = uv + offsets[i] * texel_size;
        let sample_color = sample_pixelated(sample_uv);
        let luminance = dot(sample_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        
        gx += luminance * sobel_x_weights[i];
        gy += luminance * sobel_y_weights[i];
    }
    
    return sqrt(gx * gx + gy * gy);
}

// Quantizzazione dei colori
fn quantize_color(color: vec3<f32>, levels: f32) -> vec3<f32> {
    return floor(color * levels) / levels;
}

// Cel shading function
fn cel_shade(color: vec3<f32>, levels: f32) -> vec3<f32> {
    // Calcola la luminanza
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    
    // Posterizza la luminanza
    let cel_luminance = floor(luminance * levels) / levels;
    
    // Calcola il fattore di shading
    let shade_factor = cel_luminance / max(luminance, 0.001);
    
    // Applica il cel shading mantenendo l'hue
    return color * shade_factor;
}

// Funzione per aumentare il contrasto delle silhouette
fn enhance_silhouette(color: vec3<f32>, edge: f32) -> vec3<f32> {
    // Se è un bordo forte, scurisci molto
    if (edge > 0.3) {
        return color * 0.1;
    }
    // Altrimenti mantieni il colore
    return color;
}

// Effetto scanline
fn apply_scanlines(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let scanline = 0.9 + 0.1 * sin(uv.y * settings.window_size.y * 3.14159265);
    return color * mix(1.0, scanline, settings.scanline_intensity);
}

// Fragment shader
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Campiona la texture con pixelation
    var color = sample_pixelated(in.uv);
    
    // Quantizza i colori di base
    let quantized_rgb = quantize_color(color.rgb, max(settings.color_levels, 2.0));
    color = vec4<f32>(quantized_rgb, color.a);
    
    // Applica cel shading per zone piatte
    let cel_rgb = cel_shade(color.rgb, settings.cel_levels);
    color = vec4<f32>(cel_rgb, color.a);
    
    // Edge detection per silhouette
    let edge = detect_edges(in.uv);
    
    // Enhance silhouette (bordi molto scuri)
    let silhouette_rgb = enhance_silhouette(color.rgb, edge * settings.edge_intensity);
    color = vec4<f32>(silhouette_rgb, color.a);
    
    // Contrasto alto per stile Another World
    let contrast_rgb = (color.rgb - 0.5) * settings.contrast + 0.5;
    color = vec4<f32>(clamp(contrast_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);
    
    // Saturazione per colori vividi
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let saturated_rgb = mix(vec3<f32>(luminance), color.rgb, settings.saturation);
    color = vec4<f32>(saturated_rgb, color.a);
    
    // Scanline opzionale
    if (settings.scanline_intensity > 0.0) {
        let scanline_rgb = apply_scanlines(color.rgb, in.uv);
        color = vec4<f32>(scanline_rgb, color.a);
    }
    return color;
}