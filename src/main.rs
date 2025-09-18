use std::fs::{self,DirEntry};
use std::path::Path;
use bevy::{image, scene};
use bevy_rapier3d::prelude::*;
use bevy_mod_imgui::prelude::*;
use bevy::{
    prelude::*,
};
mod camera;
mod thumbnail;

use bevy_rapier3d::prelude::*;
use imgui;

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
struct GameAsset{
    pub model_path:String,
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
        .add_systems(Startup, camera::spawn_camera)
        .add_systems(Startup, setup_game_assets)
        .add_systems(Startup, setup_physics)
        .add_systems(Startup, spawn_knight)
    //.add_systems(Startup, generate_thumbnails.after(setup_game_assets))
        .add_systems(Update, imgui_ui)
        .add_systems(Update, calc_cursor_pos)
        .add_systems(Update, update_physics)
        .add_systems(Update, draw_cursor.after(calc_cursor_pos))
        .add_systems(Update, spawn_asset.after(calc_cursor_pos))
        .add_systems(Update, alive_entities_ui)
        .add_systems(Update, camera::pan_orbit_camera.run_if(any_with_component::<camera::PanOrbitState>))
    .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, asset_server: Res<AssetServer>) {
    let texture_handle = asset_server.load("textures/grass_ground.png");
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
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

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material_handle),
        Ground,
        Collider::cuboid(100.0, 0.1, 100.0)
    ));    commands.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(Cursor{cursor_position: Vec3::ZERO});
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

                            let clicked = ui.button_with_size(&format!("##thumb_{}", e.index()), thumbnail_size);

                            if clicked {
                                clicked_entity = Some(e);
                            }
                        } else {
                            // Loading placeholder
                            ui.button_with_size(&format!("Loading...##thumb_{}", e.index()), thumbnail_size);
                        }
                    } else {
                        // No thumbnail - show placeholder
                        let clicked = ui.button_with_size(&format!("No Image##thumb_{}", e.index()), thumbnail_size);
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
fn alive_entities_ui(
    mut context: NonSendMut<ImguiContext>,
    mut query: Query<(Entity, &GameAsset,), With<Alive>>,
){
    let ui = context.ui();
    let window = ui.window("Alive entities");
    window.position([1000., 1000.0], imgui::Condition::FirstUseEver).size([300.0, 300.0], imgui::Condition::FirstUseEver).build(||{
        for (e, game_asset,) in query.iter_mut() {
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
fn setup_physics(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(1000.0, 0.1, 1000.0))
        .insert(Transform::from_xyz(0.0, 0.0, 0.0));

    /* Create the bouncing ball. */
    commands
        .spawn((
            RigidBody::Dynamic,))
        .insert(Collider::ball(0.5))
        .insert(Restitution::coefficient(1.0))
        .insert(Velocity {
            linvel: Vec3::new(1.0, 1.0, 1.0),
            angvel: Vec3::new(20.2, 0.0, 0.0),
        })
        .insert(Mesh3d(meshes.add(Sphere::new(0.5))))
        .insert(MeshMaterial3d(materials.add(Color::WHITE)))
        .insert(Transform::from_xyz(0.0, 4.0, 0.0))
    .insert(KinematicCharacterController {
        ..KinematicCharacterController::default()
    });
}
fn update_physics(time: Res<Time>, mut controllers: Query<&mut KinematicCharacterController>) {
    for mut controller in controllers.iter_mut() {
        controller.translation = Some(Vec3::new(1.0, -5.0, -1.0) * time.delta_secs());
    }
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

    for (e, game_asset,) in query.iter() {
        if game_asset.selected {
            let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(game_asset.model_path.clone()));
            commands.spawn((
                SceneRoot(scene_handle),
                Transform::from_xyz(
                    cursor.cursor_position.x,
                    cursor.cursor_position.y,
                    cursor.cursor_position.z
                ),
            ));
            commands.entity(e).insert(Alive);

            commands.spawn((
                RigidBody::Fixed,
                Collider::cuboid(1.0, 1.0, 1.0), // Usa un collider semplice per ora
                Transform::from_xyz(
                    cursor.cursor_position.x,
                    cursor.cursor_position.y+1.,
                    cursor.cursor_position.z
                ),
                ));
            println!("spawning asset");
            println!("{:?} {:?}", e, game_asset.model_path);
            break;
        }
    }
}

fn calc_cursor_pos(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    ground: Single<&GlobalTransform, With<Ground>>,
    windows: Query<&Window>,
    mut cursor: ResMut<Cursor>,
) {
    let Ok(windows) = windows.single() else {
        return;
    };

    let (camera, camera_transform) = *camera_query;

    let Some(cursor_position) = windows.cursor_position() else {
        return;
    };

    // Calculate a ray pointing from the camera into the world based on the cursor's position.
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Calculate if and where the ray is hitting the ground plane.
    let Some(distance) =
        ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up()))
    else {
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
                                AssetName(path.file_stem().unwrap_or_default().to_string_lossy().to_string()),
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

fn walk_subdirs(commands: &mut Commands, dir: &Path, ) {
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
                                AssetName(path.file_stem().unwrap_or_default().to_string_lossy().to_string()),
                            ));
                        }
                    }
                }
            } else if path.is_dir() {
                walk_subdirs(commands, &path,);
            }
        }
    }
}


#[derive(Component)]
pub struct Character;

fn spawn_knight(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("spawning knight");
    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("knight.glb"))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Character,
        AssetName("Knight".to_string()),
    )).observe(add_rapier_joint).observe(setup_knight_skeleton);
}

fn add_rapier_joint(trigger: Trigger<scene::SceneInstanceReady>,query: Query<(Entity, &AssetName, Option<&Children>), With<Character>>) {
    for (_, _, maybe_children) in query.iter() {
        match maybe_children {
            Some(children) => {
                for child in children.iter() {
                    println!("  child: {:?}", child);
                }
            }
            None => println!("  no children yet"),
        }
    }
}
fn setup_knight_skeleton(
    trigger: Trigger<scene::SceneInstanceReady>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let root = trigger.target();
    commands.entity(root).insert(RigidBody::Fixed); // root is static

    for descendant in children.iter_descendants(root) {
        if let Ok(name) = names.get(descendant) {
            if name.as_str().contains("Bone") {
                // Add physics colliders to bones
                if let Ok(parent_rel) = parents.get(descendant) {
                    let parent = parent_rel.parent();

                    if let (Ok(parent_tf), Ok(bone_tf)) = (
                        transforms.get(parent),
                        transforms.get(descendant),
                    ) {
                        let parent_pos = parent_tf.translation();
                        let bone_pos = bone_tf.translation();
                        let dir = bone_pos - parent_pos;
                        let length = dir.length();

                        if length == 0.0 {
                            continue; // skip degenerate bones
                        }

                        let mid = parent_pos + dir * 0.5;
                        let rot = Quat::from_rotation_arc(Vec3::Y, dir.normalize());

                        // Insert collider + rigidbody
                        commands.entity(descendant).insert((
                            RigidBody::Dynamic,
                            Collider::capsule_y(length * 0.5, 0.03),
                            Transform {
                                translation: mid,
                                rotation: rot,
                                scale: Vec3::ONE,
                            },
                            GlobalTransform::default(),
                        ));

                        // Connect with joint to parent bone
                        commands.entity(descendant).insert(
                            ImpulseJoint::new(
                                parent,
                                SphericalJointBuilder::new()
                                    .local_anchor1(Vec3::ZERO)
                                    .local_anchor2(Vec3::ZERO),
                            ),
                        );
                    }
                }
            }
        }
    }
}