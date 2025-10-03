use bevy::{prelude::*, transform::systems};
use bevy::color::palettes::css::*;
use bevy_mod_outline::*;
use bevy_rapier3d::prelude::*;
use std::path::Path;
use bevy_mod_imgui::prelude::*;
use std::fs::{self};
#[derive(Component)]
pub struct Alive;
#[derive(Component)]
pub struct GameAsset {
    pub model_path: String,
    pub selected: bool,
    pub thumbnail_handle: Option<Handle<Image>>,
    pub texture_id: Option<imgui::TextureId>,
}
#[derive(Component)]
pub struct AssetName(String);
pub fn setup_game_assets(mut commands: Commands) {
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
#[derive(Resource)]
struct ImguiState {
    demo_window_open: bool,
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
        .collapsible(true)
        .resizable(true)
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

pub fn spawn_asset(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    query: Query<(Entity, &GameAsset)>,
    cursor: Res<crate::cursor::Cursor>,
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
                    AsyncSceneInheritOutline::default(),
                    OutlineVolume {
                        visible: true,
                        width: 2.0,
                        colour: BLACK.into(),
                    },
                    Alive,
                    crate::transform::Pickable,
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
pub struct AssetsPlugin;
impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app
        //.add_systems(Update, alive_entities_ui)
        .add_systems(Update,imgui_ui)
        .insert_resource(ImguiState {
            demo_window_open: true,
        })
        .add_systems(Update, spawn_asset)
        ;
    }
}