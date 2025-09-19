// Pixelate edges post-processing WGSL shader
// Inspired by Bevy custom post-processing example

struct VertexOutput {
    @builtin(position) position : vec4<f32>,
    @location(0) uv : vec2<f32>,
};

@group(0) @binding(0)
var post_process_texture: texture_2d<f32>;
@group(0) @binding(1)
var post_process_sampler: sampler;

// Parameters for pixelation
const PIXEL_SIZE: f32 = 4.0;
const EDGE_THRESHOLD: f32 = 0.2;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate pixelated UV
    let pixel_uv = floor(in.uv * PIXEL_SIZE) / PIXEL_SIZE;
    let color = textureSample(post_process_texture, post_process_sampler, pixel_uv);

    // Edge detection (simple Sobel)
    let dx = vec2(1.0 / PIXEL_SIZE, 0.0);
    let dy = vec2(0.0, 1.0 / PIXEL_SIZE);
    let c = color.rgb;
    let cx = textureSample(post_process_texture, post_process_sampler, pixel_uv + dx).rgb;
    let cy = textureSample(post_process_texture, post_process_sampler, pixel_uv + dy).rgb;
    let edge = length(cx - c) + length(cy - c);

    // If edge detected, darken pixel
    let edge_factor = select(1.0, 0.5, edge > EDGE_THRESHOLD);
    return vec4<f32>(color.rgb * edge_factor, color.a);
}
