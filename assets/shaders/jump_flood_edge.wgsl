// assets/shaders/jump_flood_edge.wgsl

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
@group(0) @binding(2) var depth_texture: texture_depth_multisampled_2d;
@group(0) @binding(3) var<uniform> settings: OutlineSettings;

struct OutlineSettings {
    outline_thickness: f32,
    outline_color: vec4<f32>,
    edge_threshold: f32,
    depth_threshold: f32,  // Soglia separata per la profondità
    texture_size: vec2<f32>,
    _pad: vec2<f32>,
}

// Conversione RGB a luminanza
fn rgb_to_luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

// Edge detection con Sobel sulla luminanza
fn sobel_edge_detection(uv: vec2<f32>) -> f32 {
    let texture_coords = vec2<i32>(uv * settings.texture_size);
    
    // textureLoad per screen_texture
    let tl = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(-1, -1), 0).rgb);
    let tm = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(0, -1), 0).rgb);
    let tr = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(1, -1), 0).rgb);
    
    let ml = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(-1, 0), 0).rgb);
    let mr = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(1, 0), 0).rgb);
    
    let bl = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(-1, 1), 0).rgb);
    let bm = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(0, 1), 0).rgb);
    let br = rgb_to_luminance(textureLoad(screen_texture, texture_coords + vec2<i32>(1, 1), 0).rgb);
    
    // Sobel X kernel: [-1, 0, 1; -2, 0, 2; -1, 0, 1]
    let sobel_x = (-1.0 * tl) + (1.0 * tr) +
                  (-2.0 * ml) + (2.0 * mr) +
                  (-1.0 * bl) + (1.0 * br);
    
    // Sobel Y kernel: [-1, -2, -1; 0, 0, 0; 1, 2, 1]
    let sobel_y = (-1.0 * tl) + (-2.0 * tm) + (-1.0 * tr) +
                  (1.0 * bl) + (2.0 * bm) + (1.0 * br);
    
    return sqrt(sobel_x * sobel_x + sobel_y * sobel_y);
}

// Edge detection con Sobel sulla profondità
fn depth_edge_detection(uv: vec2<f32>) -> f32 {
    let texture_coords = vec2<i32>(uv * settings.texture_size);
    
    // textureLoad per multisampled texture, sample index 0
    let tl = textureLoad(depth_texture, texture_coords + vec2<i32>(-1, -1), 0);
    let tm = textureLoad(depth_texture, texture_coords + vec2<i32>(0, -1), 0);
    let tr = textureLoad(depth_texture, texture_coords + vec2<i32>(1, -1), 0);
    
    let ml = textureLoad(depth_texture, texture_coords + vec2<i32>(-1, 0), 0);
    let mr = textureLoad(depth_texture, texture_coords + vec2<i32>(1, 0), 0);
    
    let bl = textureLoad(depth_texture, texture_coords + vec2<i32>(-1, 1), 0);
    let bm = textureLoad(depth_texture, texture_coords + vec2<i32>(0, 1), 0);
    let br = textureLoad(depth_texture, texture_coords + vec2<i32>(1, 1), 0);
    
    // Sobel X sulla profondità
    let sobel_x = (-1.0 * tl) + (1.0 * tr) +
                  (-2.0 * ml) + (2.0 * mr) +
                  (-1.0 * bl) + (1.0 * br);
    
    // Sobel Y sulla profondità
    let sobel_y = (-1.0 * tl) + (-2.0 * tm) + (-1.0 * tr) +
                  (1.0 * bl) + (2.0 * bm) + (1.0 * br);
    
    return sqrt(sobel_x * sobel_x + sobel_y * sobel_y);
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec2<f32> {
    // 1. Edge detection basata sul colore (per i cambi di colore/luminanza)
    let color_edge_strength = sobel_edge_detection(uv);

    // 2. Edge detection basata sulla profondità (per i bordi oggetto/sfondo)
    let depth_edge_strength = depth_edge_detection(uv);

    // 3. Combina le due forze con soglie separate
    // Questo permette di rilevare sia bordi di colore che bordi di profondità
    let is_color_edge = color_edge_strength > settings.edge_threshold;
    let is_depth_edge = depth_edge_strength > settings.depth_threshold;
    
    // Se uno dei due rileva un bordo, restituisci le UV, altrimenti -1
    if (is_color_edge || is_depth_edge) {
        return uv;
    } else {
        return vec2<f32>(-1.0, -1.0);
    }
}