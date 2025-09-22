use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

#[derive(Component)]
pub struct CharacterController {
    pub speed: f32,
    pub sensitivity: f32,
    pub jump_strength: f32,
    pub is_jumping: bool,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Component)]
pub struct ThirdPersonCamera {
    pub distance: f32,
    pub height: f32,
    pub sensitivity: Vec2,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            distance: 5.0,
            height: 2.0,
            sensitivity: Vec2::new(0.003, 0.002),
        }
    }
}

fn setup_third_person_controller(
    mut commands: Commands,
    characters: Query<Entity, With<CharacterController>>,
) {
    // Find the character entity
    if let Ok(character_entity) = characters.single() {
        // Spawn the third person camera as a child of the character
        commands.entity(character_entity).with_children(|parent| {
            parent.spawn((
                ThirdPersonCamera::default(),
                Camera3d::default(),
                Transform::from_xyz(0.0, 2.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
                crate::pp::PostProcessSettings {
                    pixel_resolution: Vec2::new(240.0, 160.0), // Risoluzione pixel art
                    edge_intensity: 0.5,                       // Intensità bordi
                    color_levels: 4.0,                         // Livelli colore (4-16)
                    cel_levels: 100.0,                         // Livelli cel shading (2-8)
                    scanline_intensity: 2.5,                   // Scanline (0.0-1.0)
                    contrast: 1.0,                             // Contrasto
                    saturation: 0.7,                           // Saturazione
                    window_size: Vec2::new(1920.0, 1080.0),    // Dimensioni finestra
                    dithering_strength: 0.75,                  // Intensità dithering
                    edge_threshold: 0.05,                      // Soglia bordi
                    color_snap_strength: 0.5,                  // Intensità snapping colore
                    edge_denoise: 0.5,                     // Denoise bordi
                    ..Default::default()
                },
            ));
        });
    }
}

fn update_third_person_camera(
    mut camera_query: Query<(&mut Transform, &ThirdPersonCamera)>,
    character_query: Query<&Transform, (With<CharacterController>, Without<ThirdPersonCamera>)>,
    mut controller_query: Query<&mut CharacterController>,
    mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    time: Res<Time>,
) {
    if let (
        Ok((mut camera_transform, camera_settings)),
        Ok(character_transform),
        Ok(mut controller),
    ) = (
        camera_query.single_mut(),
        character_query.single(),
        controller_query.single_mut(),
    ) {
        let delta = mouse_motion.delta;

        if delta != Vec2::ZERO {
            // Update yaw and pitch in the controller
            controller.yaw -= delta.x * camera_settings.sensitivity.x;

            const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.1;
            controller.pitch = (controller.pitch - delta.y * camera_settings.sensitivity.y)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT);
        }

        // Calculate camera position based on character position and controller rotation
        let yaw_rotation = Quat::from_rotation_y(controller.yaw);
        let pitch_rotation = Quat::from_rotation_x(controller.pitch);

        // Camera offset from character (behind and above)
        let camera_offset = yaw_rotation
            * pitch_rotation
            * Vec3::new(0.0, camera_settings.height, camera_settings.distance);

        // Position camera
        camera_transform.translation = character_transform.translation + camera_offset;

        // Make camera look at character
        camera_transform.look_at(
            character_transform.translation + Vec3::new(0.0, 1.0, 0.0),
            Vec3::Y,
        );
    }
}

pub struct ControllerPlugin;
impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_third_person_controller);
        app.add_systems(Update, update_third_person_camera);
    }
}
