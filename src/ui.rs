use bevy::prelude::*;
use bevy::render::camera::{OrthographicProjection, PerspectiveProjection, Projection};
use bevy_mod_imgui::prelude::*;

use crate::pp::PostProcessSettings;
// =======================================
// Shader Editor
// =======================================
fn shader_editor(
    mut context: NonSendMut<ImguiContext>,
    mut query: Query<&mut PostProcessSettings>,
    mut retro_target: ResMut<crate::retrocamera::RetroRenderTarget>,
) {
    let ui = context.ui();
    let mut width = retro_target.width as i32;
    let mut height = retro_target.height as i32;
    let window = ui.window("Shader Editor");
    window
        .position([300., 1000.0], imgui::Condition::FirstUseEver)
        .size([300.0, 600.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text("Shader Settings");
            if ui.input_int("Width Resolution", &mut width).build() {
                if width > 0 {
                    retro_target.width = width as u32;
                }
            }
            if ui.input_int("height Resolution", &mut height).build() {
                if height > 0 {
                    retro_target.height = height as u32;
                }
            }
            for mut settings in query.iter_mut() {
                if ui.collapsing_header("Basic Settings", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                    ui.slider("Cel Shading Levels", 0.0, 20.0, &mut settings.cel_levels);
                    ui.slider("Contrast", 1.0, 10.0, &mut settings.contrast);
                    ui.slider("Saturation", 1.0, 10.0, &mut settings.saturation);
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

                // Toggle palette
                let mut palette_enabled = settings.color_count_and_pad.y != 0;
                if ui.checkbox("Toggle Palette", &mut palette_enabled) {
                    settings.color_count_and_pad.y = if palette_enabled { 1 } else { 0 };
                }

                // Palette Editor
                if palette_enabled
                    && ui.collapsing_header("Color Palette", imgui::TreeNodeFlags::empty())
                {
                    let mut color_count = settings.color_count_and_pad.x as i32;
                    if ui.slider("Colors", 2, 32, &mut color_count) {
                        settings.color_count_and_pad.x = color_count as u32;
                    }

                    ui.separator();

                    for i in 0..settings.color_count_and_pad.x.min(32) as usize {
                        let id = format!("color_{}", i);
                        let _curr_id = ui.push_id(&id);

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
                    ui.separator();
                    if ui.button("Grayscale 32 Preset") {
                        apply_grayscale32_preset(&mut settings);
                    }
                    ui.same_line();
                    if ui.button("Vibrant 32 Preset") {
                        apply_vibrant32_preset(&mut settings);
                    }
                    ui.same_line();
                    if ui.button("Pastel 32 Preset") {
                        apply_pastel32_preset(&mut settings);
                    }
                    if ui.button("Randomize Palette") {
                        randomize_palette(&mut settings);
                    }
                }
            }
        });
}

// =======================================
// Light Color Picker
// =======================================
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

            let linear = light.color.to_linear();
            let mut this_color = [linear.red, linear.green, linear.blue];

            if ui.color_edit3("Light Color", &mut this_color) {
                light.color = bevy::color::Color::LinearRgba(bevy::color::LinearRgba::new(
                    this_color[0],
                    this_color[1],
                    this_color[2],
                    1.0,
                ));
            }

            ui.separator();

            let mut illuminance = light.illuminance;
            if ui.slider("Intensity", 0.0, 100000.0, &mut illuminance) {
                light.illuminance = illuminance;
            }

            ui.separator();

            let mut shadow_depth_bias = light.shadow_depth_bias;
            if ui.slider("Shadow Depth Bias", 0.0, 0.1, &mut shadow_depth_bias) {
                light.shadow_depth_bias = shadow_depth_bias;
            }

            let mut shadow_normal_bias = light.shadow_normal_bias;
            if ui.slider("Shadow normal Bias", 0.0, 10.0, &mut shadow_normal_bias) {
                light.shadow_normal_bias = shadow_normal_bias;
            }

            let mut shadows_enabled = light.shadows_enabled;
            if ui.checkbox("Shadows Enabled", &mut shadows_enabled) {
                light.shadows_enabled = shadows_enabled;
            }

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

// =======================================
// Camera Controls
// =======================================
fn camera_controls_ui(
    mut context: NonSendMut<ImguiContext>,
    mut q_projection: Query<&mut Projection, With<Camera3d>>,
    mut q_retro_sprite: Query<Entity, With<crate::retrocamera::RetroScreen>>,
    mut commands: Commands,
    target: Option<Res<crate::retrocamera::RetroRenderTarget>>,
    windows: Query<&Window>,
) {
    let ui = context.ui();
    let window = ui.window("Camera Controls");

    window
        .position([620.0, 1000.0], imgui::Condition::FirstUseEver)
        .size([320.0, 120.0], imgui::Condition::FirstUseEver)
        .build(|| {
            if ui.button("Toggle RetroCamera") {
                if let Ok(entity) = q_retro_sprite.single() {
                    commands.entity(entity).despawn();
                } else if let (Some(t), Ok(window)) = (target.as_ref(), windows.single()) {
                    if let Some(handle) = t.handle.clone() {
                        let wsize = Vec2::new(window.width() as f32, window.height() as f32);
                        let tsize = Vec2::new(t.width as f32, t.height as f32);
                        let scale = (wsize.x / tsize.x).max(wsize.y / tsize.y);
                        commands.spawn((
                            Sprite {
                                image: handle,
                                ..Default::default()
                            },
                            Transform::from_scale(Vec3::new(scale, scale, 1.0)),
                            crate::retrocamera::RetroScreen,
                        ));
                    }
                }
            }

            ui.same_line();

            if ui.button("Toggle Ortho/Perspective") {
                if let Ok(mut proj) = q_projection.single_mut() {
                    *proj = match &*proj {
                        Projection::Perspective(_) => {
                            Projection::Orthographic(OrthographicProjection::default_3d())
                        }
                        Projection::Orthographic(_) => {
                            Projection::Perspective(PerspectiveProjection::default())
                        }
                        _ => return,
                    };
                }
            }
        });
}

// =======================================
// Palette Helpers
// =======================================

fn apply_grayscale_preset(settings: &mut PostProcessSettings) {
    settings.color_count_and_pad.x = 8;
    for i in 0..32 {
        let t = i as f32 / 7.0;
        settings.palette[i] = if i < 8 {
            Vec4::new(t, t, t, 1.0)
        } else {
            Vec4::ZERO
        };
    }
}

fn apply_retro_preset(settings: &mut PostProcessSettings) {
    settings.color_count_and_pad.x = 8;
    let preset = [
        Vec4::new(0.09, 0.05, 0.11, 1.0),
        Vec4::new(0.8, 0.2, 0.2, 1.0),
        Vec4::new(1.0, 0.6, 0.3, 1.0),
        Vec4::new(1.0, 0.9, 0.3, 1.0),
        Vec4::new(0.3, 0.7, 0.2, 1.0),
        Vec4::new(0.3, 0.5, 0.9, 1.0),
        Vec4::new(0.9, 0.3, 0.6, 1.0),
        Vec4::new(1.0, 1.0, 1.0, 1.0),
    ];
    for i in 0..32 {
        settings.palette[i] = if i < preset.len() {
            preset[i]
        } else {
            Vec4::ZERO
        };
    }
}

fn apply_gameboy_preset(settings: &mut PostProcessSettings) {
    settings.color_count_and_pad.x = 4;
    let preset = [
        Vec4::new(0.06, 0.22, 0.06, 1.0),
        Vec4::new(0.19, 0.38, 0.19, 1.0),
        Vec4::new(0.55, 0.68, 0.06, 1.0),
        Vec4::new(0.61, 0.74, 0.06, 1.0),
    ];
    for i in 0..32 {
        settings.palette[i] = if i < preset.len() {
            preset[i]
        } else {
            Vec4::ZERO
        };
    }
}

fn apply_nes_preset(settings: &mut PostProcessSettings) {
    settings.color_count_and_pad.x = 8;
    let preset = [
        Vec4::new(0.0, 0.0, 0.0, 1.0),
        Vec4::new(0.9, 0.1, 0.1, 1.0),
        Vec4::new(1.0, 0.5, 0.0, 1.0),
        Vec4::new(1.0, 1.0, 0.0, 1.0),
        Vec4::new(0.0, 0.8, 0.0, 1.0),
        Vec4::new(0.0, 0.5, 1.0, 1.0),
        Vec4::new(0.6, 0.2, 0.8, 1.0),
        Vec4::new(1.0, 1.0, 1.0, 1.0),
    ];
    for i in 0..32 {
        settings.palette[i] = if i < preset.len() {
            preset[i]
        } else {
            Vec4::ZERO
        };
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let h6 = (h * 6.0).fract();
    let x = c * (1.0 - ((h6 * 2.0 - 1.0).abs()));
    let (r1, g1, b1) = if h6 < 1.0 {
        (c, x, 0.0)
    } else if h6 < 2.0 {
        (x, c, 0.0)
    } else if h6 < 3.0 {
        (0.0, c, x)
    } else if h6 < 4.0 {
        (0.0, x, c)
    } else if h6 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    Vec3::new(r1 + m, g1 + m, b1 + m)
}

fn apply_grayscale32_preset(settings: &mut PostProcessSettings) {
    settings.color_count_and_pad.x = 32;
    for i in 0..32 {
        let t = i as f32 / 31.0;
        settings.palette[i] = Vec4::new(t, t, t, 1.0);
    }
}

fn apply_vibrant32_preset(settings: &mut PostProcessSettings) {
    settings.color_count_and_pad.x = 32;
    for h_idx in 0..8 {
        let h = h_idx as f32 / 8.0;
        for v_idx in 0..4 {
            let idx = h_idx * 4 + v_idx;
            let v = match v_idx {
                0 => 1.0,
                1 => 0.8,
                2 => 0.6,
                _ => 0.4,
            };
            let s = 0.95;
            let rgb = hsv_to_rgb(h, s, v);
            settings.palette[idx] = Vec4::new(rgb.x, rgb.y, rgb.z, 1.0);
        }
    }
}

fn apply_pastel32_preset(settings: &mut PostProcessSettings) {
    settings.color_count_and_pad.x = 32;
    for h_idx in 0..8 {
        let h = h_idx as f32 / 8.0;
        for v_idx in 0..4 {
            let idx = h_idx * 4 + v_idx;
            let s = 0.35 + (v_idx as f32) * 0.05;
            let v = 0.85 + (v_idx as f32) * 0.05;
            let rgb = hsv_to_rgb(h, s.min(0.6), v.min(1.0));
            settings.palette[idx] = Vec4::new(rgb.x, rgb.y, rgb.z, 1.0);
        }
    }
}

fn randomize_palette(settings: &mut PostProcessSettings) {
    use rand::Rng;
    let mut rng = rand::rng();
    for i in 0..settings.color_count_and_pad.x.min(32) as usize {
        settings.palette[i] = Vec4::new(
            rng.random_range(0.0..1.0),
            rng.random_range(0.0..1.0),
            rng.random_range(0.0..1.0),
            1.0,
        );
    }
}

// =======================================
// Plugin
// =======================================
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (shader_editor, light_color_picker, camera_controls_ui),
        );
    }
}
