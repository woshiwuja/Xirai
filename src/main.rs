mod assets;
mod cursor;
mod character;
mod pp;
//mod jump_flood;
mod chess;
mod transform;
use bevy::color::palettes::css::*;
mod camera;
mod character_controller;
mod ground;
mod ik;
mod pastel;
mod retrocamera;
mod simple_outline;
mod ui;
use bevy::image::Image;
use bevy::image::*;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::remote::RemotePlugin;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::scene;
use bevy::scene::SceneInstanceReady;
use bevy_mod_outline::{
    AsyncSceneInheritOutline, AutoGenerateOutlineNormalsPlugin, OutlinePlugin, OutlineVolume,
};
use bevy_rapier3d::prelude::*;
use std::f32::consts::PI;
mod board;
mod outline;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(BLUE.into()))
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(bevy_mod_imgui::ImguiPlugin::default())
        //.add_plugins(retrocamera::RetroRenderPlugin {
        //    width: 320,
        //    height: 180,
        //})
        .add_plugins(cursor::CursorPluginRetro)
        .add_plugins(outline::OutlinePlugin)
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_physics)
        .add_plugins(ui::UiPlugin)
        .add_systems(
            Update,
            camera::pan_orbit_camera.run_if(any_with_component::<camera::PanOrbitState>),
        )
        .add_plugins(RapierPickingPlugin)
        .add_systems(Startup, camera::spawn_camera)
        .add_plugins(pp::PostProcessPlugin)
        .add_plugins(RemotePlugin::default())
        .add_plugins(assets::AssetsPlugin)
        .add_plugins(RemoteHttpPlugin::default())
        .add_plugins(chess::ChessPlugin)
        .add_plugins((OutlinePlugin, AutoGenerateOutlineNormalsPlugin::default()))
        .add_plugins(transform::TransformGizmoPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(character::CharacterPlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = Extent3d {
        width: 280,
        height: 144,
        ..default()
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::linear(); // filtro mag/min = Nearest:contentReference[oaicite:3]{index=3}
    let mut mesh = Mesh::from(Plane3d::default().mesh().size(1000.0, 1000.0));
    let repeat_factor = 50.0;
    if let Some(uvs) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        if let bevy::render::mesh::VertexAttributeValues::Float32x2(uv_coords) = uvs {
            for uv in uv_coords.iter_mut() {
                uv[0] *= repeat_factor;
                uv[1] *= repeat_factor;
            }
        }
    }
    let ground_material = materials.add(StandardMaterial {
        base_color: GREEN.into(), // Green color
        alpha_mode: AlphaMode::Opaque,
        unlit: false, // Flat pixel art look
        metallic: 0.0,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(ground_material),
        ground::Ground,
    ));
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            color: WHITE.into(),
            illuminance: 20000.0,
            shadow_depth_bias: 0.0005,
            shadow_normal_bias: 0.05,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 40.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.0) * Quat::from_rotation_y(PI / 8.0),
            ..Default::default()
        },
        CascadeShadowConfigBuilder { ..default() }.build(),
        Pickable::default(),
    ));
}

fn setup_physics(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(1000.0, 0.1, 1000.0))
        .insert(Friction::coefficient(1.0))
        .insert(Transform::from_xyz(0.0, 0.0, 0.0));

    let ball_material = materials.add(StandardMaterial {
        base_color: GREEN.into(), // Green color
        alpha_mode: AlphaMode::Opaque,
        metallic: 10.0,
        perceptual_roughness: 1.0,
        reflectance: 1.0,
        ..default()
    });
    /* Create the bouncing ball. */
    commands
        .spawn((
            RigidBody::Dynamic,
            Collider::ball(0.5),
            Restitution::coefficient(2.0),
            Mesh3d(meshes.add(Sphere::new(0.5))),
            MeshMaterial3d(ball_material),
            Transform::from_xyz(0.0, 4.0, 0.0),
            Pickable::default(),
            RapierPickable,
        ))
        .insert(outline::Outlined);
}
