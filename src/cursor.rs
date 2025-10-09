use bevy::{pbr::{NotShadowCaster, NotShadowReceiver}, prelude::*};
use bevy_mod_outline::{OutlineMode, OutlineVolume};
#[derive(Resource)]
pub struct Cursor {
   pub cursor_position: Vec3,
}
pub fn calc_cursor_pos(
    retro_camera_query: Query<(&Camera, &GlobalTransform), With<crate::retrocamera::RetroCamera>>,
    sprite_query: Query<&Transform, With<Sprite>>,
    main_camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: Single<&GlobalTransform, With<crate::ground::Ground>>,
    windows: Query<&Window>,
    mut cursor: ResMut<Cursor>,
    target_opt: Option<Res<crate::retrocamera::RetroRenderTarget>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Try retrocamera path first (if resource and entities exist)
    if let Some(target) = target_opt {
        if let (Ok((retro_camera, retro_transform)), Ok(sprite_transform)) =
            (retro_camera_query.single(), sprite_query.single())
        {
            let window_size = Vec2::new(window.width(), window.height());
            let texture_size = Vec2::new(target.width as f32, target.height as f32);

            // Get the actual scale from the sprite transform
            let scale = sprite_transform.scale.x; // Assuming uniform scaling
            let sprite_size = texture_size * scale;

            // Calculate sprite position on screen (centered)
            let sprite_offset = (window_size - sprite_size) * 0.5;

            // Convert screen cursor to sprite-local coordinates
            let sprite_local = cursor_position - sprite_offset;

            // Check if cursor is within sprite bounds
            if sprite_local.x < 0.0
                || sprite_local.y < 0.0
                || sprite_local.x > sprite_size.x
                || sprite_local.y > sprite_size.y
            {
                return; // Cursor is outside the sprite
            }

            // Convert to texture coordinates (0 to texture_size)
            let texture_coords = sprite_local / scale;

            // Use the retro camera to cast the ray
            let Ok(ray) = retro_camera.viewport_to_world(retro_transform, texture_coords) else {
                return;
            };

            let Some(distance) =
                ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up()))
            else {
                return;
            };

            cursor.cursor_position = ray.get_point(distance);
            return;
        }
    }

    // Fallback: use main 3D camera if retrocamera is not active
    if let Ok((main_camera, main_transform)) = main_camera_query.single() {
        let Ok(ray) = main_camera.viewport_to_world(main_transform, cursor_position) else {
            return;
        };

        let Some(distance) =
            ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up()))
        else {
            return;
        };

        cursor.cursor_position = ray.get_point(distance);
    }
}

fn draw_cursor(
    cursor: Res<Cursor>,
    ground: Single<&GlobalTransform, With<crate::ground::Ground>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_entities: Query<(Entity, &mut Transform), With<CursorToroid>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        // Remove existing cursor toroids
        for (entity, _) in cursor_entities.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new toroid at cursor position
        let toroid_material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(Torus::new(0.15, 0.05))),
            MeshMaterial3d(toroid_material),
            Transform::from_translation(cursor.cursor_position + ground.up() * 0.01),
            CursorToroid,
            OutlineVolume {
                visible: true,
                width: 4.0,
                colour: Color::BLACK.into(),
            },
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }

    if buttons.pressed(MouseButton::Left) {
        // Update toroid position to follow cursor
        for (_, mut transform) in cursor_entities.iter_mut() {
            transform.translation = cursor.cursor_position + ground.up() * 0.01;
        }
    }

    if buttons.just_released(MouseButton::Left) {
        // Remove cursor toroids when mouse is released
        for (entity, _) in cursor_entities.iter() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
struct CursorToroid;
pub struct CursorPluginRetro;
impl Plugin for CursorPluginRetro {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(Cursor {
            cursor_position: Vec3::ZERO,
        })
        .add_systems(Update, calc_cursor_pos)
        .add_systems(Update, draw_cursor.after(calc_cursor_pos))
        ;
    }
}
