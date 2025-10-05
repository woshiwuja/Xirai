// Vertex shader standard per post-processing
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // Genera un fullscreen quad
    let x = f32(i32(vertex_index) - 1);
    let y = f32(i32(vertex_index & 1u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// Fragment shader
@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: PastelloSettings;

struct PastelloSettings {
    // Parametri principali
    intensity: f32,
    color_levels: f32,
    outline_strength: f32,
    outline_threshold: f32,
    
    // Controlli aggiuntivi
    saturation_boost: f32,
    brightness_boost: f32,
    contrast_reduction: f32,
    edge_softness: f32,
    
    // Dimensioni texture
    texture_size: vec2<f32>,
    _pad0: vec2<f32>,
    
    // Palette personalizzata
    use_custom_palette: u32,
    palette_size: u32,
    _pad1: vec2<f32>,
    
    // Array di colori palette (max 8)
    palette_colors: array<vec4<f32>, 8>,
}

// Quantizzazione del colore
fn quantize_color(color: vec3<f32>, levels: f32) -> vec3<f32> {
    // Quantizzazione più morbida con smoothstep
    let quantized = floor(color * levels) / levels;
    let next_level = (floor(color * levels) + 1.0) / levels;
    let t = smoothstep(0.4, 0.6, fract(color * levels));
    return mix(quantized, next_level, t);
}
// Quantizzazione con palette personalizzata
fn quantize_to_palette(color: vec3<f32>) -> vec3<f32> {
    if (settings.use_custom_palette == 0u || settings.palette_size == 0u) {
        return quantize_color(color, settings.color_levels);
    }
    
    var closest_color = settings.palette_colors[0].rgb;
    var min_distance = length(color - closest_color);
    
    for (var i = 1u; i < settings.palette_size; i++) {
        let palette_color = settings.palette_colors[i].rgb;
        let distance = length(color - palette_color);
        
        if (distance < min_distance) {
            min_distance = distance;
            closest_color = palette_color;
        }
    }
    
    return closest_color;
}

// Conversione RGB a luminanza
fn rgb_to_luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

// Edge detection con Sobel
fn sobel_edge_detection(uv: vec2<f32>) -> f32 {
    let texel_size = 1.0 / settings.texture_size;
    
    // Kernel di Sobel 3x3
    let tl = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-texel_size.x, -texel_size.y)).rgb);
    let tm = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0, -texel_size.y)).rgb);
    let tr = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(texel_size.x, -texel_size.y)).rgb);
    
    let ml = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-texel_size.x, 0.0)).rgb);
    let mm = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv).rgb);
    let mr = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(texel_size.x, 0.0)).rgb);
    
    let bl = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-texel_size.x, texel_size.y)).rgb);
    let bm = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0, texel_size.y)).rgb);
    let br = rgb_to_luminance(textureSample(screen_texture, texture_sampler, uv + vec2<f32>(texel_size.x, texel_size.y)).rgb);
    
    // Sobel X: [-1, 0, 1; -2, 0, 2; -1, 0, 1]
    let sobel_x = (-1.0 * tl) + (1.0 * tr) +
                  (-2.0 * ml) + (2.0 * mr) +
                  (-1.0 * bl) + (1.0 * br);
    
    // Sobel Y: [-1, -2, -1; 0, 0, 0; 1, 2, 1]
    let sobel_y = (-1.0 * tl) + (-2.0 * tm) + (-1.0 * tr) +
                  (1.0 * bl) + (2.0 * bm) + (1.0 * br);
    
    return sqrt(sobel_x * sobel_x + sobel_y * sobel_y);
}

// Conversione RGB a HSV
fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let c_max = max(max(rgb.r, rgb.g), rgb.b);
    let c_min = min(min(rgb.r, rgb.g), rgb.b);
    let delta = c_max - c_min;
    
    var hue = 0.0;
    if (delta > 0.0001) {
        if (c_max == rgb.r) {
            hue = 60.0 * (((rgb.g - rgb.b) / delta) % 6.0);
        } else if (c_max == rgb.g) {
            hue = 60.0 * (((rgb.b - rgb.r) / delta) + 2.0);
        } else {
            hue = 60.0 * (((rgb.r - rgb.g) / delta) + 4.0);
        }
    }
    
    if (hue < 0.0) {
        hue += 360.0;
    }
    
    let saturation = select(0.0, delta / c_max, c_max > 0.0001);
    let value = c_max;
    
    return vec3<f32>(hue / 360.0, saturation, value);
}

// Conversione HSV a RGB
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let hue = hsv.x * 360.0;
    let saturation = hsv.y;
    let value = hsv.z;
    
    let c = value * saturation;
    let x = c * (1.0 - abs(((hue / 60.0) % 2.0) - 1.0));
    let m = value - c;
    
    var rgb = vec3<f32>(0.0);
    
    if (hue < 60.0) {
        rgb = vec3<f32>(c, x, 0.0);
    } else if (hue < 120.0) {
        rgb = vec3<f32>(x, c, 0.0);
    } else if (hue < 180.0) {
        rgb = vec3<f32>(0.0, c, x);
    } else if (hue < 240.0) {
        rgb = vec3<f32>(0.0, x, c);
    } else if (hue < 300.0) {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }
    
    return rgb + vec3<f32>(m);
}

// Effetto pastello sui colori
fn pastello_color_adjustment(color: vec3<f32>) -> vec3<f32> {
    // Converti in HSV per manipolazioni più precise
    let hsv = rgb_to_hsv(color);
    
    // Regola per effetto pastello
    // Nota: valori tipici per pastello sono saturation_boost < 1.0 e brightness_boost > 1.0
    let adjusted_hsv = vec3<f32>(
        hsv.x, // Mantieni l'hue
        hsv.y * settings.saturation_boost, // Modifica saturazione
        min(hsv.z * settings.brightness_boost, 1.0) // Aumenta luminosità con clamp
    );
    
    let pastello_color = hsv_to_rgb(adjusted_hsv);
    
    // Riduci contrasto per morbidezza
    let gray = vec3<f32>(0.5);
    return mix(gray, pastello_color, settings.contrast_reduction);
}

// Funzione per ammorbidire gli edge
fn soft_edge(edge_strength: f32, threshold: f32, softness: f32) -> f32 {
    if (edge_strength < threshold) {
        return 0.0;
    }
    
    let normalized_edge = (edge_strength - threshold) / (1.0 - threshold);
    return smoothstep(0.0, 1.0, normalized_edge * max(softness, 0.01));
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Colore originale
    let original_color = textureSample(screen_texture, texture_sampler, uv).rgb;
    
    // Applica effetto pastello
    let pastello_color = pastello_color_adjustment(original_color);
    
    // Quantizzazione del colore
    let quantized_color = quantize_to_palette(pastello_color);
    
    // Edge detection
    let edge_strength = sobel_edge_detection(uv);
    let soft_edge_strength = soft_edge(edge_strength, settings.outline_threshold, settings.edge_softness);
    
    // Applica outline
    var final_color = quantized_color;
    if (soft_edge_strength > 0.001) {
        // Colore outline scuro ma morbido
        let outline_color = vec3<f32>(0.15, 0.1, 0.2);
        let edge_intensity = min(soft_edge_strength * settings.outline_strength, 1.0);
        final_color = mix(quantized_color, outline_color, edge_intensity);
    }
    
    // Mix finale con intensità dell'effetto
    let result = mix(original_color, final_color, settings.intensity);
    
    return vec4<f32>(result, 1.0);
}