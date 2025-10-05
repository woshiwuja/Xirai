struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index) - 1);
    let y = f32(i32(vertex_index & 1u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: OutlineSettings;
@group(0) @binding(3) var<uniform> step_uniform: StepSize;   // 👈 nuovo binding

struct OutlineSettings {
    outline_thickness: f32,
    outline_color: vec4<f32>,
    edge_threshold: f32,
    texture_size: vec2<f32>,
    _pad: vec2<f32>,
};

struct StepSize {
    value: f32,
    _pad: vec3<f32>, // allineamento a 16 byte richiesto da WGSL
};

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec2<f32> {
    let texel_size = 1.0 / settings.texture_size;
    let step_offset = texel_size * step_uniform.value;  // 👈 usa uniform invece del const
    
    var closest_seed = vec2<f32>(-1.0, -1.0);
    var min_distance = 999999.0;

    let current_seed = textureSample(input_texture, texture_sampler, uv).rg;
    if (current_seed.x >= 0.0 && current_seed.y >= 0.0) {
        let distance = length(uv - current_seed);
        if (distance < min_distance) {
            min_distance = distance;
            closest_seed = current_seed;
        }
    }

    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            if (dx == 0 && dy == 0) { continue; }
            let neighbor_uv = uv + vec2<f32>(f32(dx), f32(dy)) * step_offset;
            if (neighbor_uv.x < 0.0 || neighbor_uv.x > 1.0 ||
                neighbor_uv.y < 0.0 || neighbor_uv.y > 1.0) {
                continue;
            }
            let neighbor_seed = textureSample(input_texture, texture_sampler, neighbor_uv).rg;
            if (neighbor_seed.x >= 0.0 && neighbor_seed.y >= 0.0) {
                let distance = length(uv - neighbor_seed);
                if (distance < min_distance) {
                    min_distance = distance;
                    closest_seed = neighbor_seed;
                }
            }
        }
    }

    return closest_seed;
}