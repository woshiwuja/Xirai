use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Bundle, Default)]
pub struct PanOrbitCameraBundle {
    pub camera_3d: Camera3d,
    pub camera: Camera,
    pub projection: Projection,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub state: PanOrbitState,
    pub settings: PanOrbitSettings,
}

#[derive(Component)]
pub struct PanOrbitState {
    pub center: Vec3,
    pub radius: f32,
    pub upside_down: bool,
    pub pitch: f32,
    pub yaw: f32,
}

#[derive(Component)]
pub struct PanOrbitSettings {
    pub pan_sensitivity: f32,
    pub orbit_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub pan_key: Option<MouseButton>,
    pub orbit_key: Option<MouseButton>,
    pub zoom_key: Option<KeyCode>,
    pub scroll_action: Option<PanOrbitAction>,
    pub scroll_line_sensitivity: f32,
    pub scroll_pixel_sensitivity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanOrbitAction {
    Pan,
    Orbit,
    Zoom,
}

impl Default for PanOrbitState {
    fn default() -> Self {
        PanOrbitState {
            center: Vec3::ZERO,
            radius: 1.0,
            upside_down: false,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

impl Default for PanOrbitSettings {
    fn default() -> Self {
        PanOrbitSettings {
            pan_sensitivity: 0.001,
            orbit_sensitivity: 0.1f32.to_radians(),
            zoom_sensitivity: 0.01,
            pan_key: Some(MouseButton::Left),
            orbit_key: Some(MouseButton::Middle),
            zoom_key: Some(KeyCode::ShiftLeft),
            scroll_action: Some(PanOrbitAction::Zoom),
            scroll_line_sensitivity: 16.0,
            scroll_pixel_sensitivity: 1.0,
        }
    }
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        PanOrbitCameraBundle {
            transform: Transform::from_xyz(1.0, 2.0, 3.0),
            state: PanOrbitState {
                center: Vec3::new(1.0, 2.0, 3.0),
                radius: 1.0,
                pitch: 15.0f32.to_radians(),
                yaw: 30.0f32.to_radians(),
                ..default()
            },
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        crate::retrocamera::RetroCamera,
        crate::pp::PostProcessSettings {
            edge_denoise: 0.0,
            edge_intensity: 0.5,
            color_levels: 8.0,
            cel_levels: 0.0,
            contrast: 1.2,
            saturation: 1.0,
            scanline_intensity: 0.3,
            _pad1: 0.0,
            window_size: Vec2::new(1920.0, 1080.0),
            _pad2: Vec2::ZERO,
            dithering_strength: 0.1,
            edge_threshold: 0.1,
            color_snap_strength: 0.5,
            _pad3: 0.0,
            palette: [Vec4::ZERO; 32], 
            color_count_and_pad: UVec4::new(8, 1, 0, 0),
        },
        MeshPickingCamera,
        Pickable::default(),
    ));
}

pub fn pan_orbit_camera(
    kbd: Res<ButtonInput<KeyCode>>,
    btn: Res<ButtonInput<MouseButton>>,
    mut evr_motion: EventReader<MouseMotion>,
    mut evr_scroll: EventReader<MouseWheel>,
    mut q_camera: Query<(
        &PanOrbitSettings,
        &mut PanOrbitState,
        &mut Transform,
        &mut Projection,
    )>,
) {
    let mut total_motion: Vec2 = evr_motion.read().map(|ev| ev.delta).sum();
    total_motion.y = -total_motion.y;
    
    let mut total_scroll_lines = Vec2::ZERO;
    let mut total_scroll_pixels = Vec2::ZERO;
    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                total_scroll_lines.x += ev.x;
                total_scroll_lines.y -= ev.y;
            }
            MouseScrollUnit::Pixel => {
                total_scroll_pixels.x += ev.x;
                total_scroll_pixels.y -= ev.y;
            }
        }
    }
    
    for (settings, mut state, mut transform, mut projection) in &mut q_camera {
        // Check if projection is orthographic
        let is_orthographic = matches!(*projection, Projection::Orthographic(_));
        
        // WASD movement for orbit center
        let mut wasd_move = Vec3::ZERO;
        if kbd.pressed(KeyCode::KeyW) {
            let fwd = transform.forward().as_vec3();
            wasd_move += Vec3::new(fwd.x, 0.0, fwd.z);
        }
        if kbd.pressed(KeyCode::KeyS) {
            let fwd = transform.forward().as_vec3();
            wasd_move -= Vec3::new(fwd.x, 0.0, fwd.z);
        }
        if kbd.pressed(KeyCode::KeyA) {
            wasd_move -= transform.right().as_vec3();
        }
        if kbd.pressed(KeyCode::KeyD) {
            wasd_move += transform.right().as_vec3();
        }
        
        let wasd_delta = if wasd_move != Vec3::ZERO {
            Some(wasd_move.normalize())
        } else {
            None
        };
        
        let radius = state.radius;
        
        let mut total_pan = Vec2::ZERO;
        if settings.scroll_action == Some(PanOrbitAction::Pan) {
            total_pan -=
                total_scroll_lines * settings.scroll_line_sensitivity * settings.pan_sensitivity;
            total_pan -=
                total_scroll_pixels * settings.scroll_pixel_sensitivity * settings.pan_sensitivity;
        }
        
        let mut total_orbit = Vec2::ZERO;
        let orbiting = settings
            .orbit_key
            .map(|mb| btn.pressed(mb))
            .unwrap_or(false);
        if orbiting {
            total_orbit -= total_motion * settings.orbit_sensitivity;
            if total_motion.length_squared() > 0.0 {
                state.center = transform.translation + transform.forward().as_vec3() * state.radius;
            }
        }
        if settings.scroll_action == Some(PanOrbitAction::Orbit) {
            total_orbit -=
                total_scroll_lines * settings.scroll_line_sensitivity * settings.orbit_sensitivity;
            total_orbit -= total_scroll_pixels
                * settings.scroll_pixel_sensitivity
                * settings.orbit_sensitivity;
        }
        
        let mut total_zoom = Vec2::ZERO;
        if settings
            .zoom_key
            .map(|key| kbd.pressed(key))
            .unwrap_or(false)
        {
            total_zoom -= total_motion * settings.zoom_sensitivity;
        }
        if settings.scroll_action == Some(PanOrbitAction::Zoom) {
            total_zoom -=
                total_scroll_lines * settings.scroll_line_sensitivity * settings.zoom_sensitivity;
            total_zoom -=
                total_scroll_pixels * settings.scroll_pixel_sensitivity * settings.zoom_sensitivity;
        }
        
        if settings
            .orbit_key
            .map(|mb| btn.just_pressed(mb))
            .unwrap_or(false)
        {
            state.upside_down = state.pitch < -FRAC_PI_2 || state.pitch > FRAC_PI_2;
        }
        
        if state.upside_down {
            total_orbit.x = -total_orbit.x;
        }
        
        let mut any = false;
        
        if let Some(delta) = wasd_delta {
            any = true;
            state.center += delta * radius * 0.05;
        }
        
        // Handle zoom differently based on projection type
        if total_zoom != Vec2::ZERO {
            any = true;
            
            if is_orthographic {
                // For orthographic projection, modify the scale
                if let Projection::Orthographic(ref mut ortho) = *projection {
                    let zoom_factor = (-total_zoom.y).exp();
                    ortho.scale *= zoom_factor;
                    // Clamp to reasonable values
                    ortho.scale = ortho.scale.clamp(0.01, 100.0);
                }
            } else {
                // For perspective projection, modify the radius
                state.radius *= (-total_zoom.y).exp();
            }
        }
        
        if total_orbit != Vec2::ZERO {
            any = true;
            state.yaw += total_orbit.x;
            state.pitch += total_orbit.y;
            
            if state.yaw < 0.0 {
                state.yaw += TAU;
            } else if state.yaw > TAU {
                state.yaw -= TAU;
            }
            
            state.pitch = state.pitch.clamp(-PI / 2.0, PI / 2.0);
        }
        
        if total_pan != Vec2::ZERO {
            any = true;
            let radius = state.radius;
            state.center += transform.right() * total_pan.x * radius;
            state.center += transform.up() * total_pan.y * radius;
        }
        
        if any || state.is_added() {
            transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
            transform.translation = state.center + transform.back() * state.radius;
        }
    }
}