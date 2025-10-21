use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    window::WindowResized,
};

/// Marker for cameras that should render to a low-res texture
#[derive(Component)]
pub struct RetroCamera;

/// Plugin that redirects retro cameras into a low-res render target
pub struct RetroRenderPlugin {
    pub width: u32,
    pub height: u32,
}
impl Default for RetroRenderPlugin {
    fn default() -> Self {
        RetroRenderPlugin {
            width: 320,
            height: 180,
        }
    }
}

impl Plugin for RetroRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RetroRenderTarget {
            width: self.width,
            height: self.height,
            handle: None,
        })
        .add_systems(Startup, setup_retro_target)
        .add_systems(
            PostStartup,
            (attach_retro_cameras, setup_fullscreen_quad_sprite).chain(),
        )
        .add_systems(Update, update_fullscreen_quad_scale)
        .add_systems(Update, update_retro_resolution)
        ;
    }
}

#[derive(Resource)]
pub struct RetroRenderTarget {
    pub width: u32,
    pub height: u32,
    pub handle: Option<Handle<Image>>,
}

fn setup_retro_target(mut images: ResMut<Assets<Image>>, mut target: ResMut<RetroRenderTarget>) {
    let size = Extent3d {
        width: target.width,
        height: target.height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = bevy_image::ImageSampler::nearest();
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    target.handle = Some(images.add(image));
}

/// Redirects any camera with `RetroCamera` to render into the retro texture
fn attach_retro_cameras(
    mut query: Query<&mut Camera, With<RetroCamera>>,
    target: Res<RetroRenderTarget>,
) {
    if let Some(handle) = target.handle.clone() {
        for mut camera in &mut query {
            camera.target = RenderTarget::Image(handle.clone().into());
        }
    }
}

#[derive(Component)]
pub struct RetroScreen;

/// Sets up a fullscreen quad that displays the retro texture
fn setup_fullscreen_quad_sprite(
    mut commands: Commands,
    target: Res<RetroRenderTarget>,
    windows: Query<&Window>,
) {
    let Some(retro_texture_handle) = target.handle.clone() else {
        return;
    };

    let window = windows.single().expect("No primary window");
    let wsize = Vec2::new(window.width() as f32, window.height() as f32);
    let tsize = Vec2::new(target.width as f32, target.height as f32);
    let scale = (wsize.x / tsize.x).max(wsize.y / tsize.y);
    // Use SpriteBundle - much simpler
    commands.spawn((
        Sprite {
            image: retro_texture_handle,
            ..default()
        },
        Transform::from_scale(Vec3::new(scale, scale, 1.0)),
        RetroScreen,
    ));

    // Simple 2D camera without explicit transform
    commands.spawn((
        Camera2d,
        Camera {
            order: 1, // Render after retro cameras
            ..Default::default()
        },
    ));
}
// Aggiungi questo sistema per reagire ai cambi di dimensione della finestra
fn update_fullscreen_quad_scale(
    mut sprite_query: Query<&mut Transform, With<Sprite>>,
    target: Res<RetroRenderTarget>,
    windows: Query<&Window>,
    mut window_events: EventReader<WindowResized>,
) {
    // Controlla se c'è stato un evento di resize
    if window_events.read().next().is_some() {
        let Ok(window) = windows.single() else {
            return;
        };

        let window_size = Vec2::new(window.width(), window.height());
        let texture_size = Vec2::new(target.width as f32, target.height as f32);

        let scale_x = window_size.x / texture_size.x;
        let scale_y = window_size.y / texture_size.y;
        let scale = scale_x.max(scale_y);

        // Aggiorna la scala dello sprite
        for mut transform in sprite_query.iter_mut() {
            transform.scale = Vec3::new(scale, scale, 1.0);
        }
    }
}
fn update_retro_resolution(
    mut images: ResMut<Assets<Image>>,
    mut target: ResMut<RetroRenderTarget>,
    mut cameras: Query<&mut Camera, With<RetroCamera>>,
    mut sprite_query: Query<(&mut Sprite, &mut Transform), With<RetroScreen>>,
    windows: Query<&Window>,
) {
    if !target.is_changed() {
        return;
    }

    let Some(old_handle) = target.handle.clone() else {
        return;
    };

    // Ricrea la texture
    let size = Extent3d {
        width: target.width,
        height: target.height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = bevy_image::ImageSampler::nearest();
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING 
        | TextureUsages::COPY_DST 
        | TextureUsages::RENDER_ATTACHMENT;

    let new_handle = images.add(image);

    // Aggiorna le camere
    for mut camera in &mut cameras {
        camera.target = RenderTarget::Image(new_handle.clone().into());
    }

    // Aggiorna lo sprite con il nuovo handle E la scala
    if let Ok((mut sprite, mut transform)) = sprite_query.get_single_mut() {
        sprite.image = new_handle.clone();
        
        if let Ok(window) = windows.get_single() {
            let window_size = Vec2::new(window.width(), window.height());
            let texture_size = Vec2::new(target.width as f32, target.height as f32);
            let scale = (window_size.x / texture_size.x).max(window_size.y / texture_size.y);
            transform.scale = Vec3::new(scale, scale, 1.0);
        }
    }

    // Rimuovi la vecchia texture
    images.remove(&old_handle);

    target.handle = Some(new_handle);
}
#[derive(Resource)]
pub struct ExtractedRetroTargetHandle(pub Handle<Image>);
