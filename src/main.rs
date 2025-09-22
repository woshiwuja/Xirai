mod pp;
mod retrocamera;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use bevy::{image, scene};
use bevy::image::Image;
use bevy::image::*;
use bevy_mod_imgui::prelude::*;
use bevy_rapier3d::prelude::*;
use std::fs::{self, DirEntry};
use std::path::Path;
mod camera;
mod ik;
mod thumbnail;
use bevy_rapier3d::prelude::*;
use imgui;
use std::f32::consts::{PI, TAU};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
mod ui;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
mod character_controller;
#[derive(Resource)]
struct ImguiState {
    demo_window_open: bool,
}
#[derive(Component)]
struct Ground;
#[derive(Component)]
struct Alive;
#[derive(Component)]
struct AssetName(String);
#[derive(Component)]
struct GameAsset {
    pub model_path: String,
    pub selected: bool,
    pub thumbnail_handle: Option<Handle<Image>>,
    pub texture_id: Option<imgui::TextureId>,
}
#[derive(Resource)]
struct Cursor {
    cursor_position: Vec3,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgba(0.2, 0.2, 0.2, 1.0)))
        .insert_resource(ImguiState {
            demo_window_open: true,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(bevy_mod_imgui::ImguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_game_assets)
        .add_systems(Startup, setup_physics)
        .add_systems(Startup, spawn_character)
        .add_systems(Update, imgui_ui)
        .add_systems(Update, calc_cursor_pos)
        .add_systems(Update, draw_cursor.after(calc_cursor_pos))
        .add_systems(Update, spawn_asset.after(calc_cursor_pos))
        .add_systems(Update, alive_entities_ui)
        .add_plugins(ui::UiPlugin)
        .add_systems(
            Update,
            camera::pan_orbit_camera.run_if(any_with_component::<camera::PanOrbitState>),
        )
        .add_plugins(RapierPickingPlugin)
        .add_plugins(pp::PostProcessPlugin)
        .add_systems(Startup, camera::spawn_camera)
        .add_plugins(retrocamera::RetroRenderPlugin{ width: 240, height: 160 })
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
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
    // Abilitiamo l'uso come render target:
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    // Filtro nearest-neighbor per mantenere il pixelato:
    image.sampler = ImageSampler::nearest(); // filtro mag/min = Nearest:contentReference[oaicite:3]{index=3}
    //let texture_handle = asset_server.load("textures/grass_ground.png");
    let material_handle = materials.add(StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

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
        base_color: Color::WHITE, // Green color
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
        Ground,
    ));
    commands.spawn((
        DirectionalLight{
            shadows_enabled: true,
            color: YELLOW.into(),
            illuminance: 2000.0,
            shadow_depth_bias: 0.0005,
            shadow_normal_bias: 0.05,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 40.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 2.0),
            ..default()
        },
        // The default cascade config is designed to handle large scenes.
        // As this example has a much smaller world, we can tighten the shadow
        // bounds for better visual quality.
        CascadeShadowConfigBuilder {
            ..default()
        }
        .build(),
    ));
    commands.insert_resource(Cursor {
        cursor_position: Vec3::ZERO,
    });
}

fn imgui_ui(
    mut context: NonSendMut<ImguiContext>,
    mut state: ResMut<ImguiState>,
    mut query: Query<(Entity, &AssetName, &mut GameAsset)>,
    images: Res<Assets<Image>>,
) {
    let ui = context.ui();
    let sidebar_window = ui.window("Asset Browser");
    sidebar_window
        .size([600.0, 900.0], imgui::Condition::FirstUseEver)
        .position([0.0, 0.0], imgui::Condition::FirstUseEver)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .build(|| {
            ui.text("Choose assets");
            ui.separator();

            let mut clicked_entity = None;
            let grid_columns = 2;
            let thumbnail_size = [200.0, 200.0];
            let mut column_count = 0;

            // Grid layout for thumbnails
            for (e, name, game_asset) in query.iter() {
                if column_count % grid_columns == 0 && column_count > 0 {
                    // Start new row
                }

                // Create a group for each asset item
                ui.group(|| {
                    // Thumbnail placeholder or actual thumbnail
                    if let Some(thumbnail_handle) = &game_asset.thumbnail_handle {
                        if let Some(_image) = images.get(thumbnail_handle) {
                            // For now, just show a colored button as placeholder
                            // ImGui-rs doesn't have direct image support without additional setup
                            let button_color = if game_asset.selected {
                                [0.2, 0.8, 0.2, 1.0] // Green if selected
                            } else {
                                [0.4, 0.4, 0.4, 1.0] // Gray if not selected
                            };

                            let clicked = ui.button_with_size(
                                &format!("##thumb_{}", e.index()),
                                thumbnail_size,
                            );

                            if clicked {
                                clicked_entity = Some(e);
                            }
                        } else {
                            // Loading placeholder
                            ui.button_with_size(
                                &format!("Loading...##thumb_{}", e.index()),
                                thumbnail_size,
                            );
                        }
                    } else {
                        // No thumbnail - show placeholder
                        let clicked = ui.button_with_size(
                            &format!("No Image##thumb_{}", e.index()),
                            thumbnail_size,
                        );
                        if clicked {
                            clicked_entity = Some(e);
                        }
                    }

                    // Asset name below thumbnail
                    ui.text_wrapped(&name.0);
                });

                // Same line for grid layout
                if column_count % grid_columns < grid_columns - 1 {
                    ui.same_line();
                }
                column_count += 1;
            }

            // Handle selection
            if let Some(selected_entity) = clicked_entity {
                for (e, _name, mut game_asset) in query.iter_mut() {
                    game_asset.selected = e == selected_entity;
                }
            }

            ui.separator();
            ui.text("Selected asset details:");

            // Show details for selected asset
            for (_e, name, game_asset) in query.iter() {
                if game_asset.selected {
                    ui.text(format!("Name: {}", name.0));
                    ui.text(format!("Path: {}", game_asset.model_path.to_string()));
                    break;
                }
            }
        });

    if state.demo_window_open {
        ui.show_demo_window(&mut state.demo_window_open);
    }
}
fn alive_entities_ui(mut context: NonSendMut<ImguiContext>, query: Query<Entity, With<Alive>>) {
    let ui = context.ui();
    let window = ui.window("Alive entities");
    window
        .position([1000., 1000.0], imgui::Condition::FirstUseEver)
        .size([300.0, 300.0], imgui::Condition::FirstUseEver)
        .build(|| {
            for e in query.iter() {
                ui.text(format!("Alive Entities: {}", e));
            }
        });
}
fn draw_cursor(
    cursor: Res<Cursor>,
    ground: Single<&GlobalTransform, With<Ground>>,
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
        metallic: 0.0,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    /* Create the bouncing ball. */
    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(0.5),
        Restitution::coefficient(2.0),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(ball_material),
        Transform::from_xyz(0.0, 4.0, 0.0),
    ));
}

fn spawn_asset(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    query: Query<(Entity, &GameAsset)>,
    cursor: Res<Cursor>,
    asset_server: Res<AssetServer>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }

    for (_, game_asset) in query.iter() {
        if game_asset.selected {
            let scene_handle = asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(game_asset.model_path.clone()));
            let asset = commands
                .spawn((
                    SceneRoot(scene_handle),
                    Transform::from_xyz(
                        cursor.cursor_position.x,
                        cursor.cursor_position.y,
                        cursor.cursor_position.z,
                    ),
                    RigidBody::Fixed,
                    Alive,
                ))
                .id();
            // Spawn collider as child, offset by half_height on Y
            commands.entity(asset).with_children(|parent| {
                parent.spawn((
                    Collider::cuboid(0.25, 0.8, 0.25),
                    Transform::from_xyz(0.0, 0.8, 0.0),
                ));
            });
            break;
        };
    }
}

fn calc_cursor_pos(
    retro_camera_query: Query<(&Camera, &GlobalTransform), With<retrocamera::RetroCamera>>,
    sprite_query: Query<&Transform, With<Sprite>>,
    ground: Single<&GlobalTransform, With<Ground>>,
    windows: Query<&Window>,
    mut cursor: ResMut<Cursor>,
    target: Res<retrocamera::RetroRenderTarget>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok((retro_camera, retro_transform)) = retro_camera_query.get_single() else {
        return;
    };

    let Ok(sprite_transform) = sprite_query.get_single() else {
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
    if sprite_local.x < 0.0 || sprite_local.y < 0.0 || 
       sprite_local.x > sprite_size.x || sprite_local.y > sprite_size.y {
        return; // Cursor is outside the sprite
    }
    
    // Convert to texture coordinates (0 to texture_size)
    let texture_coords = sprite_local / scale;
    
    // Use the retro camera to cast the ray
    let Ok(ray) = retro_camera.viewport_to_world(retro_transform, texture_coords) else {
        return;
    };

    let Some(distance) = ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up())) else {
        return;
    };
    
    cursor.cursor_position = ray.get_point(distance);
}

fn setup_game_assets(mut commands: Commands) {
    let assets_dir = Path::new("assets");

    if let Ok(entries) = fs::read_dir(assets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "glb" || extension == "gltf" {
                        if let Some(relative_path) = path.strip_prefix("assets").ok() {
                            commands.spawn((
                                GameAsset {
                                    model_path: relative_path.to_string_lossy().to_string(),
                                    selected: false,
                                    thumbnail_handle: None,
                                    texture_id: None,
                                },
                                AssetName(
                                    path.file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }
            } else if path.is_dir() {
                walk_subdirs(&mut commands, &path);
            }
        }
    }
}

fn walk_subdirs(commands: &mut Commands, dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "glb" || extension == "gltf" {
                        if let Some(relative_path) = path.strip_prefix("assets").ok() {
                            commands.spawn((
                                GameAsset {
                                    model_path: relative_path.to_string_lossy().to_string(),
                                    selected: false,
                                    thumbnail_handle: None,
                                    texture_id: None,
                                },
                                AssetName(
                                    path.file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }
            } else if path.is_dir() {
                walk_subdirs(commands, &path);
            }
        }
    }
}

/// Componenti per identificare il personaggio
#[derive(Component)]
struct Character;

fn spawn_character(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    println!("spawning character");
    const ANIMATION_PATH: &str = "humanoid.glb";
    const PATH: &str = "humanoid.glb";
    // 1️⃣ Spawn scena GLTF
    let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(PATH));
    let (graph, index) =
        AnimationGraph::from_clip(asset_server.load(GltfAssetLabel::Animation(0).from_asset(ANIMATION_PATH)));

    // Store the animation graph as an asset.
    let graph_handle = graphs.add(graph);

    // Create a component that stores a reference to our animation.
    let animation_to_play = AnimationToPlay {
        graph_handle,
        index,
    };
    commands
        .spawn((
            animation_to_play,
            SceneRoot(scene_handle.clone()),
            Transform::from_xyz(0.0, 2.0, 5.0),
            Character,
            AssetName("Character Man".to_string()),
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED_X,
            Collider::capsule_y(0.9, 0.3),
            Friction::coefficient(1.0),
            Restitution::coefficient(0.0),

        ))
        // 2️⃣ Quando la scena è pronta, costruisci root e catene IK
        //.observe(setup_character_skeleton_and_ik);
    .observe(play_animation_when_ready);
}
#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

fn play_animation_when_ready(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    // The entity we spawned in `setup_mesh_and_animation` is the trigger's target.
    // Start by finding the AnimationToPlay component we added to that entity.
    if let Ok(animation_to_play) = animations_to_play.get(trigger.target()) {
        // The SceneRoot component will have spawned the scene as a hierarchy
        // of entities parented to our entity. Since the asset contained a skinned
        // mesh and animations, it will also have spawned an animation player
        // component. Search our entity's descendants to find the animation player.
        for child in children.iter_descendants(trigger.target()) {
            if let Ok(mut player) = players.get_mut(child) {
                // Tell the animation player to start the animation and keep
                // repeating it.
                //
                // If you want to try stopping and switching animations, see the
                // `animated_mesh_control.rs` example.
                player.play(animation_to_play.index).repeat();

                // Add the animation graph. This only needs to be done once to
                // connect the animation player to the mesh.
                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
        }
    }
}

/// Funzione chiamata quando il GLTF è pronto
fn setup_character_skeleton_and_ik(
    trigger: Trigger<scene::SceneInstanceReady>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&Transform>,
    mut commands: Commands,
) {
    let root_entity = trigger.target();

    // Helper per trovare bones per nome parziale
    let find_bone = |partial: &str| -> Option<Entity> {
        children.iter_descendants(root_entity).find(|&e| {
            names.get(e).map_or(false, |name| {
                name.as_str()
                    .to_lowercase()
                    .contains(&partial.to_lowercase())
            })
        })
    };

    // Debug: print all bone names
    for descendant in children.iter_descendants(root_entity) {
        if let Ok(name) = names.get(descendant) {
            println!("Descendant bone: {}", name.as_str());
        }
    }

    // Find root bone
    let root_bone = children.iter_descendants(root_entity).find(|&e| {
        names.get(e).map_or(false, |name| {
            let n = name.as_str().to_lowercase();
            n.contains("pelvis") || n.contains("root") || n.contains("hips")
        })
    });

    let root_bone = match root_bone {
        Some(b) => b,
        None => {
            warn!("Root bone not found, aborting setup");
            return;
        }
    };

    // Setup root bone with physics
    commands.entity(root_bone).insert((
        RigidBody::Dynamic,
        LockedAxes::ROTATION_LOCKED,
        Restitution::coefficient(0.0),
        Friction::coefficient(1.0),
        Alive,
        RapierPickable,
    ));

    // Setup torso collider
    setup_torso_collider(&mut commands, &find_bone, &transforms);

    // Setup limb colliders and joints
    setup_arm_colliders(&mut commands, &find_bone, "left");
    setup_arm_colliders(&mut commands, &find_bone, "right");
    setup_leg_colliders_and_joints(&mut commands, &find_bone, &transforms, root_bone, "left");
    setup_leg_colliders_and_joints(&mut commands, &find_bone, &transforms, root_bone, "right");

    // Uncomment if you want head/neck setup
    // setup_head_and_neck(&mut commands, &find_bone, &transforms, root_bone);
}

fn setup_torso_collider(
    commands: &mut Commands,
    find_bone: &impl Fn(&str) -> Option<Entity>,
    transforms: &Query<&Transform>,
) {
    if let (Some(pelvis), Some(neck)) = (find_bone("pelvis"), find_bone("neck")) {
        // Calculate torso dimensions
        let pelvis_pos = transforms
            .get(pelvis)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);
        let neck_pos = transforms
            .get(neck)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        let torso_height = (neck_pos.y - pelvis_pos.y).abs().max(0.1);
        let torso_radius = 0.18;

        commands.entity(pelvis).with_children(|parent| {
            parent.spawn((
                Collider::capsule_y(torso_height / 2.0, torso_radius),
                Transform::from_xyz(0.0, torso_height / 2.0, 0.0),
            ));
        });
    }
}

fn setup_arm_colliders(
    commands: &mut Commands,
    find_bone: &impl Fn(&str) -> Option<Entity>,
    side: &str,
) {
    if let (Some(upper_arm), Some(forearm), Some(hand)) = (
        find_bone(&format!("upper_arm_{}", side)),
        find_bone(&format!("forearm_{}", side)),
        find_bone(&format!("hand_{}", side)),
    ) {
        println!("Creating {} arm colliders", side);

        // Upper arm collider
        commands.entity(upper_arm).with_children(|parent| {
            parent.spawn((
                Collider::capsule_y(0.12, 0.04),      // length, radius
                Transform::from_xyz(0.0, -0.06, 0.0), // offset to center
            ));
        });

        // Forearm collider
        commands.entity(forearm).with_children(|parent| {
            parent.spawn((
                Collider::capsule_y(0.10, 0.035),
                Transform::from_xyz(0.0, -0.05, 0.0),
            ));
        });

        // Hand collider
        commands.entity(hand).with_children(|parent| {
            parent.spawn((Collider::ball(0.04), Transform::from_xyz(0.0, -0.03, 0.0)));
        });

        // Add joints between arm segments
        commands
            .entity(forearm)
            .insert(ImpulseJoint::new(upper_arm, SphericalJointBuilder::new()));
        commands
            .entity(hand)
            .insert(ImpulseJoint::new(forearm, SphericalJointBuilder::new()));
    }
}

fn setup_leg_colliders_and_joints(
    commands: &mut Commands,
    find_bone: &impl Fn(&str) -> Option<Entity>,
    transforms: &Query<&Transform>,
    root_bone: Entity,
    side: &str,
) {
    if let (Some(thigh), Some(shin), Some(foot)) = (
        find_bone(&format!("thigh_{}", side)),
        find_bone(&format!("shin_{}", side)),
        find_bone(&format!("foot_{}", side)),
    ) {
        println!("Creating {} leg colliders and joints", side);

        // Thigh collider
        commands.entity(thigh).with_children(|parent| {
            parent.spawn((
                Collider::capsule_y(0.20, 0.06), // length, radius
                Transform::from_xyz(0.0, -0.10, 0.0),
            ));
        });

        // Shin collider
        commands.entity(shin).with_children(|parent| {
            parent.spawn((
                Collider::capsule_y(0.18, 0.04),
                Transform::from_xyz(0.0, -0.09, 0.0),
            ));
        });

        // Foot collider
        commands.entity(foot).with_children(|parent| {
            parent.spawn((
                Collider::cuboid(0.08, 0.03, 0.12), // foot-shaped
                Transform::from_xyz(0.0, -0.03, 0.06),
            ));
        });

        // Add joints between leg segments
        commands
            .entity(thigh)
            .insert(ImpulseJoint::new(root_bone, SphericalJointBuilder::new()));
        commands
            .entity(shin)
            .insert(ImpulseJoint::new(thigh, SphericalJointBuilder::new()));
        commands
            .entity(foot)
            .insert(ImpulseJoint::new(shin, SphericalJointBuilder::new()));
    }
}

fn setup_head_and_neck(
    commands: &mut Commands,
    find_bone: &impl Fn(&str) -> Option<Entity>,
    transforms: &Query<&Transform>,
    root_bone: Entity,
) {
    if let (Some(neck), Some(head)) = (find_bone("neck"), find_bone("head")) {
        println!("Creating neck and head");

        // Add colliders
        commands.entity(neck).with_children(|parent| {
            parent.spawn((
                Collider::capsule_y(0.06, 0.03),
                Transform::from_xyz(0.0, 0.03, 0.0),
            ));
        });

        commands.entity(head).with_children(|parent| {
            parent.spawn((Collider::ball(0.08), Transform::default()));
        });

        // Find spine or use root as parent for neck joint
        let spine_parent = find_bone("spine").unwrap_or(root_bone);

        commands.entity(neck).insert(ImpulseJoint::new(
            spine_parent,
            SphericalJointBuilder::new(),
        ));
        commands
            .entity(head)
            .insert(ImpulseJoint::new(neck, SphericalJointBuilder::new()));
    }
}
