
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

@group(0) @binding(0) var screen_texture: texture_2d<f32>;      // source
@group(0) @binding(1) var texture_sampler: sampler;            
@group(0) @binding(2) var<uniform> settings: OutlineSettings;   
@group(0) @binding(3) var distance_texture: texture_2d<f32>;    // distance_map

struct OutlineSettings {
    outline_thickness: f32,
    outline_color: vec4<f32>,
    edge_threshold: f32,
    texture_size: vec2<f32>,
    _pad: vec2<f32>,
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Colore originale della scena
    let original_color = textureSample(screen_texture, texture_sampler, uv);
    
    // Seed più vicino dal distance field
    let nearest_seed = textureSample(distance_texture, texture_sampler, uv).rg;

    // Se non c'è un seed valido, restituisci il colore originale
    if (nearest_seed.x < 0.0 || nearest_seed.y < 0.0) {
        return original_color;
    }

    // Calcola la distanza dal bordo più vicino
    let distance_to_edge = length((uv - nearest_seed) * settings.texture_size);

    // Se siamo nella zona dell'outline
    if (distance_to_edge < settings.outline_thickness) {
        // Opzione 1: Outline solido
        return settings.outline_color;
        
        // Opzione 2: Blend con il colore originale (uncommenta per usare)
        // let outline_alpha = settings.outline_color.a;
        // return vec4<f32>(
        //     mix(original_color.rgb, settings.outline_color.rgb, outline_alpha),
        //     max(original_color.a, outline_alpha)
        // );
        
        // Opzione 3: Outline con fade (uncommenta per usare)
        // let fade_factor = 1.0 - (distance_to_edge / settings.outline_thickness);
        // let outline_alpha = settings.outline_color.a * fade_factor;
        // return vec4<f32>(
        //     mix(original_color.rgb, settings.outline_color.rgb, outline_alpha),
        //     max(original_color.a, outline_alpha)
        // );
    }

    // IMPORTANTE: Restituisci sempre il colore originale, MAI bianco fisso!
    return original_color;
}