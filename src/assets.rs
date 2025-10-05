use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_mod_imgui::prelude::*;
use bevy_mod_outline::*;
use bevy_rapier3d::prelude::*;
use std::collections::HashMap;
use std::fs::{self};
use std::path::Path;

#[derive(Component)]
pub struct Alive;

#[derive(Component)]
pub struct GameAsset {
    pub model_path: String,
    pub selected: bool,
    pub folder_path: String,
}

#[derive(Component)]
pub struct AssetName(String);

#[derive(Resource)]
pub struct AssetTree {
    folders: HashMap<String, Vec<Entity>>,
    folder_states: HashMap<String, bool>, // Per tenere traccia di quali folder sono aperti
}

impl Default for AssetTree {
    fn default() -> Self {
        Self {
            folders: HashMap::new(),
            folder_states: HashMap::new(),
        }
    }
}

pub fn setup_game_assets(mut commands: Commands, mut asset_tree: ResMut<AssetTree>) {
    let assets_dir = Path::new("assets");

    if let Ok(entries) = fs::read_dir(assets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "glb" || extension == "gltf" {
                        if let Some(relative_path) = path.strip_prefix("assets").ok() {
                            let entity = commands
                                .spawn((
                                    GameAsset {
                                        model_path: relative_path.to_string_lossy().to_string(),
                                        selected: false,
                                        folder_path: "root".to_string(),
                                    },
                                    AssetName(
                                        path.file_stem()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string(),
                                    ),
                                ))
                                .id();

                            asset_tree
                                .folders
                                .entry("root".to_string())
                                .or_insert_with(Vec::new)
                                .push(entity);
                        }
                    }
                }
            } else if path.is_dir() {
                walk_subdirs(&mut commands, &path, &mut asset_tree);
            }
        }
    }

    // Apri la root di default
    asset_tree.folder_states.insert("root".to_string(), true);
}

fn walk_subdirs(commands: &mut Commands, dir: &Path, asset_tree: &mut AssetTree) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "glb" || extension == "gltf" {
                        if let Some(relative_path) = path.strip_prefix("assets").ok() {
                            let folder_path = relative_path
                                .parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| "root".to_string());

                            let entity = commands
                                .spawn((
                                    GameAsset {
                                        model_path: relative_path.to_string_lossy().to_string(),
                                        selected: false,
                                        folder_path: folder_path.clone(),
                                    },
                                    AssetName(
                                        path.file_stem()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string(),
                                    ),
                                ))
                                .id();

                            asset_tree
                                .folders
                                .entry(folder_path)
                                .or_insert_with(Vec::new)
                                .push(entity);
                        }
                    }
                }
            } else if path.is_dir() {
                walk_subdirs(commands, &path, asset_tree);
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
    mut asset_tree: ResMut<AssetTree>,
    mut query: Query<(Entity, &AssetName, &mut GameAsset)>,
) {
    let ui = context.ui();
    let sidebar_window = ui.window("Asset Browser");
    sidebar_window
        .size([400.0, 900.0], imgui::Condition::FirstUseEver)
        .position([0.0, 0.0], imgui::Condition::FirstUseEver)
        .collapsible(true)
        .resizable(true)
        .movable(false)
        .build(|| {
            ui.text("Choose assets");
            ui.separator();

            let mut clicked_entity = None;

            // Ottieni tutte le cartelle e ordinale
            let mut folders: Vec<String> = asset_tree.folders.keys().cloned().collect();
            folders.sort();

            // Root sempre per prima
            if let Some(pos) = folders.iter().position(|f| f == "root") {
                folders.remove(pos);
                folders.insert(0, "root".to_string());
            }

            // Mostra l'albero delle cartelle
            for folder in folders {
                let is_open = asset_tree
                    .folder_states
                    .get(&folder)
                    .copied()
                    .unwrap_or(false);

                let folder_display = if folder == "root" {
                    "📁 Root".to_string()
                } else {
                    format!("📁 {}", folder)
                };

                if ui.collapsing_header(&folder_display, imgui::TreeNodeFlags::empty()) {
                    if !is_open {
                        asset_tree.folder_states.insert(folder.clone(), true);
                    }

                    // Mostra gli asset in questa cartella
                    if let Some(entities) = asset_tree.folders.get(&folder) {
                        for &entity in entities {
                            if let Ok((e, name, game_asset)) = query.get(entity) {
                                ui.indent();

                                let label = if game_asset.selected {
                                    format!("▶ {}", name.0)
                                } else {
                                    format!("  {}", name.0)
                                };

                                if ui.selectable(&label) {
                                    clicked_entity = Some(e);
                                }

                                ui.unindent();
                            }
                        }
                    }
                } else {
                    asset_tree.folder_states.insert(folder.clone(), false);
                }
            }

            // Handle selection
            if let Some(selected_entity) = clicked_entity {
                for (e, _name, mut game_asset) in query.iter_mut() {
                    game_asset.selected = e == selected_entity;
                }
            }

            ui.separator();
            ui.text("Selected asset:");

            // Show details for selected asset
            for (_e, name, game_asset) in query.iter() {
                if game_asset.selected {
                    ui.text(format!("Name: {}", name.0));
                    ui.text(format!("Path: {}", game_asset.model_path));
                    ui.text(format!("Folder: {}", game_asset.folder_path));
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
                    AsyncSceneInheritOutline::default(),
                    OutlineVolume {
                        visible: true,
                        width: 2.0,
                        colour: BLACK.into(),
                    },
                    OutlineMode::ExtrudeFlatDoubleSided,
                    Alive,
                    //crate::transform::Selected
                ))
                .id();
            println!("From: spawn_asset {}", asset);
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
        app.insert_resource(AssetTree::default())
            .insert_resource(ImguiState {
                demo_window_open: false,
            })
            .add_systems(Startup, setup_game_assets)
            .add_systems(Update, imgui_ui)
            .add_systems(Update, spawn_asset.after(crate::cursor::calc_cursor_pos));
    }
}
