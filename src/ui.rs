use bevy::prelude::*;
use bevy_mod_imgui::prelude::*;

use crate::pp::PostProcessSettings;

fn shader_editor(
    mut context: NonSendMut<ImguiContext>,
    mut query: Query<(&Camera, &mut PostProcessSettings)>,
) {
    let ui = context.ui();
    let window = ui.window("Shedit");
    window
        .position([300., 1000.0], imgui::Condition::FirstUseEver)
        .size([300.0, 600.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text("Shader Settings");

            for (camera, mut settings) in query.iter_mut() {
                if ui.collapsing_header("Basic Settings", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    ui.slider("Edge Intensity", 0.0, 1.0, &mut settings.edge_intensity);
                    ui.slider("Color Levels", 4.0, 16.0, &mut settings.color_levels);
                    ui.slider("Cel Shading Levels", 0.0, 1.0, &mut settings.cel_levels);
                    ui.slider("Contrast", 0.5, 10.0, &mut settings.contrast);
                    ui.slider("Saturation", 0.0, 10.0, &mut settings.saturation);
                    ui.slider(
                        "Scanline Intensity",
                        0.0,
                        10.0,
                        &mut settings.scanline_intensity,
                    );
                }

                if ui.collapsing_header("Advanced", imgui::TreeNodeFlags::empty()) {
                    ui.slider("Dithering", 0.0, 1.0, &mut settings.dithering_strength);
                    ui.slider("Edge Threshold", 0.0, 1.0, &mut settings.edge_threshold);
                    ui.slider("Edge Denoise", 0.0, 1.0, &mut settings.edge_denoise);
                    ui.slider(
                        "Color Snap Strength",
                        0.0,
                        1.0,
                        &mut settings.color_snap_strength,
                    );
                }

                // Palette Editor
                if ui.collapsing_header("Color Palette", imgui::TreeNodeFlags::empty()) {
                    ui.text("Number of colors:");
                    let mut color_count_i32 = settings.color_count as i32;
                    if ui.slider("Colors", 2, 8, &mut color_count_i32) {
                        settings.color_count = color_count_i32 as u32;
                    }

                    ui.separator();

                    for i in 0..settings.color_count.min(8) as usize {
                        let id = format!("color_{}", i);
                        let curr_id = ui.push_id(&id);
                        // Converti Vec4 a array [f32; 4] per imgui
                        let mut color = [
                            settings.palette[i].x,
                            settings.palette[i].y,
                            settings.palette[i].z,
                            settings.palette[i].w,
                        ];

                        let label = format!("Color {}", i + 1);
                        if ui.color_edit4(&label, &mut color) {
                            settings.palette[i] = Vec4::new(color[0], color[1], color[2], color[3]);
                        }
                        curr_id.end()
                    }

                    ui.separator();

                    // Preset buttons
                    if ui.button("Grayscale Preset") {
                        apply_grayscale_preset(&mut settings);
                    }
                    ui.same_line();
                    if ui.button("Retro Preset") {
                        apply_retro_preset(&mut settings);
                    }

                    if ui.button("GameBoy Preset") {
                        apply_gameboy_preset(&mut settings);
                    }
                    ui.same_line();
                    if ui.button("NES Preset") {
                        apply_nes_preset(&mut settings);
                    }
                    if ui.button("Randomize Palette") {
                        randomize_palette(&mut settings);
                    }
                }
            }
        });
}

fn light_color_picker(
    mut context: NonSendMut<ImguiContext>,
    mut light: Single<&mut DirectionalLight>,
) {
    let ui = context.ui();
    let window = ui.window("Light Color");

    window
        .position([900.0, 1000.0], imgui::Condition::FirstUseEver)
        .size([300.0, 150.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text("Light Color Picker");

            // Converti da Srgba a linear per la UI
            let linear = light.color.to_linear();
            let mut this_color = [linear.red, linear.green, linear.blue];

            // Color picker con preview
            if ui.color_edit3("Light Color", &mut this_color) {
                // Crea il colore in spazio lineare e poi converti a Srgba
                light.color = bevy::color::Color::LinearRgba(bevy::color::LinearRgba::new(
                    this_color[0],
                    this_color[1],
                    this_color[2],
                    1.0,
                ));
            }

            ui.separator();

            // Slider per intensità
            let mut illuminance = light.illuminance;
            if ui.slider("Intensity", 0.0, 100000.0, &mut illuminance) {
                light.illuminance = illuminance;
            }

            // Preset comuni
            ui.separator();
            ui.text("Presets:");

            if ui.button("Warm White") {
                light.color = bevy::color::Color::srgb(1.0, 0.95, 0.8);
            }
            ui.same_line();
            if ui.button("Cool White") {
                light.color = bevy::color::Color::srgb(0.8, 0.9, 1.0);
            }

            if ui.button("Sunset") {
                light.color = bevy::color::Color::srgb(1.0, 0.6, 0.3);
            }
            ui.same_line();
            if ui.button("Moonlight") {
                light.color = bevy::color::Color::srgb(0.6, 0.7, 1.0);
            }
        });
}

// Funzioni helper per i preset della palette
fn apply_grayscale_preset(settings: &mut PostProcessSettings) {
    settings.color_count = 8;
    settings.palette = [
        Vec4::new(0.0, 0.0, 0.0, 1.0),    // nero
        Vec4::new(0.14, 0.14, 0.14, 1.0), // grigio molto scuro
        Vec4::new(0.29, 0.29, 0.29, 1.0), // grigio scuro
        Vec4::new(0.43, 0.43, 0.43, 1.0), // grigio medio-scuro
        Vec4::new(0.57, 0.57, 0.57, 1.0), // grigio medio
        Vec4::new(0.71, 0.71, 0.71, 1.0), // grigio chiaro
        Vec4::new(0.86, 0.86, 0.86, 1.0), // grigio molto chiaro
        Vec4::new(1.0, 1.0, 1.0, 1.0),    // bianco
    ];
}

fn apply_retro_preset(settings: &mut PostProcessSettings) {
    settings.color_count = 8;
    settings.palette = [
        Vec4::new(0.09, 0.05, 0.11, 1.0), // quasi nero viola
        Vec4::new(0.8, 0.2, 0.2, 1.0),    // rosso
        Vec4::new(1.0, 0.6, 0.3, 1.0),    // arancione
        Vec4::new(1.0, 0.9, 0.3, 1.0),    // giallo
        Vec4::new(0.3, 0.7, 0.2, 1.0),    // verde
        Vec4::new(0.3, 0.5, 0.9, 1.0),    // blu
        Vec4::new(0.9, 0.3, 0.6, 1.0),    // rosa
        Vec4::new(1.0, 1.0, 1.0, 1.0),    // bianco
    ];
}

fn apply_gameboy_preset(settings: &mut PostProcessSettings) {
    settings.color_count = 4;
    settings.palette = [
        Vec4::new(0.06, 0.22, 0.06, 1.0),
        Vec4::new(0.19, 0.38, 0.19, 1.0),
        Vec4::new(0.55, 0.68, 0.06, 1.0),
        Vec4::new(0.61, 0.74, 0.06, 1.0),
        Vec4::ZERO,
        Vec4::ZERO,
        Vec4::ZERO,
        Vec4::ZERO,
    ];
}

fn apply_nes_preset(settings: &mut PostProcessSettings) {
    settings.color_count = 8;
    settings.palette = [
        Vec4::new(0.0, 0.0, 0.0, 1.0), // nero
        Vec4::new(0.9, 0.1, 0.1, 1.0), // rosso
        Vec4::new(1.0, 0.5, 0.0, 1.0), // arancione
        Vec4::new(1.0, 1.0, 0.0, 1.0), // giallo
        Vec4::new(0.0, 0.8, 0.0, 1.0), // verde
        Vec4::new(0.0, 0.5, 1.0, 1.0), // blu
        Vec4::new(0.6, 0.2, 0.8, 1.0), // viola
        Vec4::new(1.0, 1.0, 1.0, 1.0), // bianco
    ];
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (shader_editor, light_color_picker))
        ;
    }
}
fn randomize_palette(settings: &mut PostProcessSettings) {
    use rand::Rng;
    let mut rng = rand::rng();

    for i in 0..settings.color_count.min(8) as usize {
        settings.palette[i] = Vec4::new(
            rng.random_range(0.0..1.0), // R
            rng.random_range(0.0..1.0), // G
            rng.random_range(0.0..1.0), // B
            1.0,                     // A
        );
    }
}