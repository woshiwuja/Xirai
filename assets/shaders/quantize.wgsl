#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

// Palette in stile retro/GameBoy Color con più colori
const PALETTE : array<vec3<f32>, 16> = array<vec3<f32>, 16>(
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(0.067, 0.067, 0.067),
    vec3<f32>(0.133, 0.133, 0.133),
    vec3<f32>(0.2, 0.2, 0.2),
    vec3<f32>(0.267, 0.267, 0.267),
    vec3<f32>(0.333, 0.333, 0.333),
    vec3<f32>(0.4, 0.4, 0.4),
    vec3<f32>(0.467, 0.467, 0.467),
    vec3<f32>(0.533, 0.533, 0.533),
    vec3<f32>(0.6, 0.6, 0.6),
    vec3<f32>(0.667, 0.667, 0.667),
    vec3<f32>(0.733, 0.733, 0.733),
    vec3<f32>(0.8, 0.8, 0.8),
    vec3<f32>(0.867, 0.867, 0.867),
    vec3<f32>(0.933, 0.933, 0.933),
    vec3<f32>(1.0, 1.0, 1.0)
);
fn nearest_color(c: vec3<f32>) -> vec3<f32> {
    var min_dist: f32 = 99999.0;
    var best: vec3<f32> = c;
    for (var i: i32 = 0; i < 16; i = i + 1) {
        let p = PALETTE[i];
        let d = dot(c - p, c - p);
        if (d < min_dist) {
            min_dist = d;
            best = p;
        }
    }
    return best;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let color = textureSample(screen_texture, texture_sampler, uv).rgb;
    
    // Rimuovi o riduci questo clamp - stava rendendo troppo nero
    let lum = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let color_clamped = select(color, vec3<f32>(0.09, 0.05, 0.11), lum < 0.05); // soglia più bassa
    
    let quantized = nearest_color(color_clamped);
    return vec4<f32>(quantized, 1.0);
}