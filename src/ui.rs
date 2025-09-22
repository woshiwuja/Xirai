use bevy::{gizmos::light, picking::window, prelude::*, ui};
use bevy_mod_imgui::prelude::*;

use crate::pp::PostProcessSettings;
fn shader_editor(
    mut context: NonSendMut<ImguiContext>,
    mut query: Query<(&Camera, &mut crate::pp::PostProcessSettings)>,
) {
    let ui = context.ui();
    let window = ui.window("Shedit");
    window
        .position([300., 1000.0], imgui::Condition::FirstUseEver)
        .size([300.0, 300.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text("Shader Edit: {}");
            for (camera, mut settings) in query.iter_mut() {
                ui.text("Pixel Art Settings");
                ui.slider(
                    "Pixel Width",
                    20.0,
                    1920.0,
                    &mut settings.pixel_resolution.x,
                );
                ui.slider(
                    "Pixel Height",
                    20.0,
                    1080.0,
                    &mut settings.pixel_resolution.y,
                );
                ui.slider("Edge Intensity", 0.0, 1.0, &mut settings.edge_intensity);
                ui.slider("Color Levels", 4.0, 16.0, &mut settings.color_levels);
                ui.slider("Cel Shading Levels", 0.0, 100.0, &mut settings.cel_levels);
                ui.slider("Contrast", 0.5, 10.0, &mut settings.contrast);
                ui.slider("Saturation", 0.0, 10.0, &mut settings.saturation);
                ui.slider(
                    "Scanline Intensity",
                    0.0,
                    10.0,
                    &mut settings.scanline_intensity,
                );
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
        });
}

fn light_color_picker(
    mut context: NonSendMut<ImguiContext>,
    mut light: Single<&mut DirectionalLight>,
) {
    let ui = context.ui();
    let window = ui.window("Light Color");
    let mut this_color = [
        light.color.to_linear().red,
        light.color.to_linear().green,
        light.color.to_linear().blue,
    ];
    window
        .position([900.0, 1000.0], imgui::Condition::FirstUseEver)
        .size([300.0, 150.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text("Light Color Picker");
            let edited = ui.color_edit3("Light Color", &mut this_color);
            if edited {
                light.color = bevy::color::Color::srgb(this_color[0], this_color[1], this_color[2]);
            }
        });
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, shader_editor);
        app.add_systems(Update, light_color_picker);
    }
}
