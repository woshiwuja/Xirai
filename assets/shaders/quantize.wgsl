struct Palette {
    colors: array<vec4<f32>, 16>;
};

@group(0) @binding(0) var uSampler: sampler;
@group(0) @binding(1) var uScene: texture_2d<f32>;
@group(0) @binding(0) var<uniform> uPalette: Palette;
// Definizione della palette
const PALETTE_SIZE: u32 = 4u;  // puoi arrivare tranquillamente a 16
const palette: array<vec3<f32>, PALETTE_SIZE> = array<vec3<f32>, PALETTE_SIZE>(
    vec3<f32>(1.0, 0.0, 0.0), // Rosso
    vec3<f32>(0.0, 1.0, 0.0), // Verde
    vec3<f32>(0.0, 0.0, 1.0), // Blu
    vec3<f32>(1.0, 1.0, 0.0)  // Giallo
);

fn closest_palette_color(c: vec3<f32>) -> vec3<f32> {
    var best = uPalette.colors[0].rgb;
    var bestDist = 999999.0;

    for (var i: u32 = 0u; i < 16u; i = i + 1u) {
        let p = uPalette.colors[i].rgb;
        let d = distance(c, p);
        if (d < bestDist) {
            bestDist = d;
            best = p;
        }
    }
    return best;
}

@fragment
fn main_fs(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Colore originale della scena
    let sceneColor = textureSample(uScene, uSampler, uv).rgb;

    // Trova il colore della palette più vicino
    let mapped = closest_palette_color(sceneColor);

    return vec4<f32>(mapped, 1.0);
}