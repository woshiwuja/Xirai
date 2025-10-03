use bevy::prelude::*;
#[derive(Resource)]
pub struct Cursor {
   pub cursor_position: Vec3,
}
fn calc_cursor_pos(
    retro_camera_query: Query<(&Camera, &GlobalTransform), With<crate::retrocamera::RetroCamera>>,
    sprite_query: Query<&Transform, With<Sprite>>,
    ground: Single<&GlobalTransform, With<crate::ground::Ground>>,
    windows: Query<&Window>,
    mut cursor: ResMut<Cursor>,
    target: Res<crate::retrocamera::RetroRenderTarget>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok((retro_camera, retro_transform)) = retro_camera_query.single() else {
        return;
    };

    let Ok(sprite_transform) = sprite_query.single() else {
        return;
    };

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
}

fn draw_cursor(
    cursor: Res<Cursor>,
    ground: Single<&GlobalTransform, With<crate::ground::Ground>>,
    mut gizmos: Gizmos,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    if buttons.pressed(MouseButton::Left) {
        gizmos.circle(
            Isometry3d::new(
                cursor.cursor_position + ground.up() * 0.01,
                Quat::from_rotation_arc(Vec3::Z, ground.up().as_vec3()),
            ),
            0.2,
            Color::WHITE,
        );
    }
}
pub struct CursorPluginRetro;
impl Plugin for CursorPluginRetro {
    fn build(&self, app: &mut App) {
        app.insert_resource(Cursor {
            cursor_position: Vec3::ZERO,
        })
        .add_systems(Update, calc_cursor_pos)
        .add_systems(Update, draw_cursor.after(calc_cursor_pos))
        .add_systems(Update, crate::assets::spawn_asset.after(calc_cursor_pos));
    }
}
