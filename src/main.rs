/// Blocca la rotazione del collider del pelvis sull'asse Y
fn lock_pelvis_rotation_system(mut query: Query<(&Name, &mut Transform), With<Collider>>) {
    for (name, mut transform) in query.iter_mut() {
        if name.as_str().to_lowercase().contains("pelvis") {
            // Mantieni la rotazione solo sull'asse Y (upright)
            let (_, y, _) = transform.rotation.to_euler(EulerRot::YXZ);
            transform.rotation = Quat::from_rotation_y(y);
        }
    }
}
fn set_right_hand_target_on_click(
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<Cursor>,
    mut ik_chains: Query<&mut IKChain>,
    names: Query<&Name>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        for mut chain in ik_chains.iter_mut() {
            // Find the right arm chain by bone names
            if chain.bones.len() == 3 {
                let hand_entity = chain.bones[2];
                if let Ok(name) = names.get(hand_entity) {
                    if name.as_str().to_lowercase().contains("hand_right") {
                        chain.target = cursor.cursor_position;
                    }
                }
            }
        }
    }
}
/// Set the right hand IK target to the cursor position
fn set_right_hand_target_to_cursor(
    cursor: Res<Cursor>,
    mut ik_chains: Query<&mut IKChain>,
    names: Query<&Name>,
) {
    for mut chain in ik_chains.iter_mut() {
        // Find the right arm chain by bone names
        if chain.bones.len() == 3 {
            let hand_entity = chain.bones[2];
            if let Ok(name) = names.get(hand_entity) {
                if name.as_str().to_lowercase().contains("hand_right") {
                    chain.target = cursor.cursor_position;
                }
            }
        }
    }
}
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use bevy::{image, scene};
use bevy_mod_imgui::prelude::*;
use bevy_rapier3d::prelude::*;
use std::fs::{self, DirEntry};
use std::path::Path;
mod camera;
mod ik;
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
        .add_systems(Startup, camera::spawn_camera)
        .add_systems(Startup, setup_game_assets)
        .add_systems(Startup, setup_physics)
        .add_systems(Startup, spawn_character)
        //.add_systems(Startup, generate_thumbnails.after(setup_game_assets))
        .add_systems(Update, imgui_ui)
        .add_systems(Update, calc_cursor_pos)
        .add_systems(Update, draw_cursor.after(calc_cursor_pos))
        .add_systems(Update, spawn_asset.after(calc_cursor_pos))
        .add_systems(Update, alive_entities_ui)
        .add_systems(Update, ik_system)
        .add_systems(Update, lock_pelvis_rotation_system)
        .add_systems(Update, set_right_hand_target_on_click)
        .add_systems(
            Update,
            camera::pan_orbit_camera.run_if(any_with_component::<camera::PanOrbitState>),
        )
        .add_plugins(RapierPickingPlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
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
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
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

    /* Create the bouncing ball. */
    commands
        .spawn((RigidBody::Dynamic,))
        .insert(Collider::ball(0.5))
        .insert(Restitution::coefficient(1.0))
        .insert(Mesh3d(meshes.add(Sphere::new(0.5))))
        .insert(MeshMaterial3d(materials.add(Color::WHITE)))
        .insert(Transform::from_xyz(0.0, 4.0, 0.0));
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

/// Rappresenta una catena IK (es. braccio, gamba)
#[derive(Component)]
struct IKChain {
    bones: Vec<Entity>,        // bones della catena dal root al terminale
    target: Vec3,              // posizione target finale
    pole_target: Option<Vec3>, // opzionale: aiuta a evitare torsioni innaturali
    iterations: usize,         // numero di iterazioni del solver FABRIK
}

/// Sistema IK che aggiorna i bones ogni frame
fn ik_system(mut chains: Query<&mut IKChain>, mut bones: Query<&mut Transform>) {
    for mut chain in chains.iter_mut() {
        if chain.bones.is_empty() {
            continue;
        }

        let mut positions: Vec<Vec3> = chain
            .bones
            .iter()
            .map(|&b| bones.get_mut(b).unwrap().translation)
            .collect();

        let root_position = positions[0];
        let target_position = chain.target;

        // Calcolo lunghezze osso
        let mut bone_lengths = Vec::with_capacity(positions.len() - 1);
        for i in 0..positions.len() - 1 {
            bone_lengths.push((positions[i + 1] - positions[i]).length());
        }

        let iterations = chain.iterations;
        let bone_count = chain.bones.len();
        for _ in 0..iterations {
            // Backward pass: posiziona l’ultimo osso sulla target
            positions[bone_count - 1] = target_position;
            for i in (0..bone_count - 1).rev() {
                let dir = (positions[i] - positions[i + 1]).normalize_or_zero();
                positions[i] = positions[i + 1] + dir * bone_lengths[i];
            }

            // Forward pass: mantiene root fisso
            positions[0] = root_position;
            for i in 0..positions.len() - 1 {
                let dir = (positions[i + 1] - positions[i]).normalize_or_zero();
                positions[i + 1] = positions[i] + dir * bone_lengths[i];
            }

            // Pole vector adjustment (gentle and stable)
            if let Some(pole) = chain.pole_target {
                // Only apply if chain has at least 3 bones (e.g., arm/leg)
                if positions.len() >= 3 {
                    let a = positions[0];
                    let b = positions[1];
                    let c = positions[2];
                    // Project pole onto plane defined by a-b-c
                    let ab = (b - a).normalize_or_zero();
                    let ac = (c - a).normalize_or_zero();
                    let plane_normal = ab.cross(ac).normalize_or_zero();
                    let ap = pole - a;
                    let proj_pole = ap - plane_normal * ap.dot(plane_normal);
                    let proj_b = b - a - plane_normal * (b - a).dot(plane_normal);
                    // Calculate direction from current middle joint to projected pole
                    let bend_dir = (proj_pole - proj_b).normalize_or_zero();
                    // Move middle joint gently toward projected pole
                    positions[1] += bend_dir * 0.05; // 0.05 is a gentler factor
                }
            }
        }

        // Aggiorna i bones con le nuove posizioni e rotazioni
        for i in 0..positions.len() - 1 {
            let bone_entity = chain.bones[i];
            let dir = (positions[i + 1] - positions[i]).normalize_or_zero();

            if let Ok(mut tf) = bones.get_mut(bone_entity) {
                tf.translation = positions[i];
                if dir.length_squared() > 0.0 {
                    tf.rotation = Quat::from_rotation_arc(Vec3::Y, dir);
                }
            }
        }

        // Ultimo bone
        if let Some(&last) = chain.bones.last() {
            if let Ok(mut tf) = bones.get_mut(last) {
                tf.translation = positions[positions.len() - 1];
            }
        }
    }
}
/// Estensione utile: normalizza un vettore, ritorna zero se nullo
trait NormalizeOrZero {
    fn normalize_or_zero(&self) -> Vec3;
}

impl NormalizeOrZero for Vec3 {
    fn normalize_or_zero(&self) -> Vec3 {
        let l = self.length();
        if l > 0.0 {
            *self / l
        } else {
            Vec3::ZERO
        }
    }
}

fn spawn_character(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("spawning character");

    // 1️⃣ Spawn scena GLTF
    let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset("man.glb"));
    commands
        .spawn((
            SceneRoot(scene_handle.clone()),
            Transform::from_xyz(0.0, 5.0, 0.0),
            Character,
            AssetName("Character Man".to_string()),
        ))
        // 2️⃣ Quando la scena è pronta, costruisci root e catene IK
        .observe(setup_character_skeleton_and_ik);
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
    for descendant in children.iter_descendants(root_entity) {
        if let Ok(name) = names.get(descendant) {
            println!("Descendant bone: {}", name.as_str());
        }
    }
    let root_bone = children.iter_descendants(root_entity).find(|&e| {
        names.get(e).map_or(false, |name| {
            let n = name.as_str().to_lowercase();
            n.contains("pelvis") || n.contains("root")
        })
    });

    let root_bone = match root_bone {
        Some(b) => b,
        None => {
            warn!("Root bone non trovato, abort setup");
            return;
        }
    };

    // 1️⃣ Assegna RigidBody e Collider al root
    commands
        .entity(root_bone)
        .insert(RigidBody::Dynamic)
        .insert(Collider::capsule_y(0.2, 0.12))
        .insert(Restitution::coefficient(0.0))
        .insert(Friction::coefficient(1.0))
        .insert(Alive);

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

    // Helper per aggiungere collider come child
    let add_collider_child = |parent: Entity, shape: Collider| {
        commands.entity(parent).with_children(|p| {
            p.spawn((shape, Transform::default()));
        });
    };

    //    if let (Some(upper_arm), Some(forearm), Some(hand)) = (
    //        find_bone("upper_arm_left"),
    //        find_bone("forearm_left"),
    //        find_bone("hand_left"),
    //    ) {
    //        println!("Creating left arm");
    //        // Bone colliders temporarily disabled
    //        // commands.entity(upper_arm).with_children(|p| {
    //        //     p.spawn((Collider::capsule_y(0.05, 0.1), Transform::default()));
    //        // });
    //        // commands.entity(forearm).with_children(|p| {
    //        //     p.spawn((Collider::capsule_y(0.05, 0.1), Transform::default()));
    //        // });
    //        // commands.entity(hand).with_children(|p| {
    //        //     p.spawn((Collider::ball(0.05), Transform::default()));
    //        // });
    //        let forearm_pos = transforms.get(forearm).unwrap().translation;
    //        let pole = forearm_pos + Vec3::new(0.0, 0.0, 0.3);
    //        let upper_arm_pos = transforms.get(upper_arm).unwrap().translation;
    //        // T-pose: hand target directly to the left
    //        let tpose_distance = 0.5; // adjust as needed for your model
    //        let hand_target = upper_arm_pos + Vec3::new(-tpose_distance, 0.0, 0.0);
    //        // Add joints: upper_arm -> forearm, forearm -> hand
    //        commands
    //            .entity(forearm)
    //            .insert(ImpulseJoint::new(upper_arm, SphericalJointBuilder::new()));
    //        commands
    //            .entity(hand)
    //            .insert(ImpulseJoint::new(forearm, SphericalJointBuilder::new()));
    //        commands.spawn(IKChain {
    //            bones: vec![upper_arm, forearm, hand],
    //            target: hand_target,
    //            pole_target: Some(pole),
    //            iterations: 10,
    //        });
    //    }
    //
    //    // Braccio destro
    //    if let (Some(upper_arm), Some(forearm), Some(hand)) = (
    //        find_bone("upper_arm_right"),
    //        find_bone("forearm_right"),
    //        find_bone("hand_right"),
    //    ) {
    //        println!("Creating right arm");
    //        // Bone colliders temporarily disabled
    //        // commands.entity(upper_arm).with_children(|p| {
    //        //     p.spawn((Collider::capsule_y(0.05, 0.1), Transform::default()));
    //        // });
    //        // commands.entity(forearm).with_children(|p| {
    //        //     p.spawn((Collider::capsule_y(0.05, 0.1), Transform::default()));
    //        // });
    //        // commands.entity(hand).with_children(|p| {
    //        //     p.spawn((Collider::ball(0.05), Transform::default()));
    //        // });
    //        let forearm_pos = transforms.get(forearm).unwrap().translation;
    //        let pole = forearm_pos + Vec3::new(0.0, 0.0, 0.3);
    //        let upper_arm_pos = transforms.get(upper_arm).unwrap().translation;
    //        // T-pose: hand target directly to the right
    //        let tpose_distance = 0.5; // adjust as needed for your model
    //        let hand_target = upper_arm_pos + Vec3::new(tpose_distance, 0.0, 0.0);
    //        commands
    //            .entity(forearm)
    //            .insert(ImpulseJoint::new(upper_arm, SphericalJointBuilder::new()));
    //        commands
    //            .entity(hand)
    //            .insert(ImpulseJoint::new(forearm, SphericalJointBuilder::new()));
    //        commands.spawn(IKChain {
    //            bones: vec![upper_arm, forearm, hand],
    //            target: hand_target,
    //            pole_target: Some(pole),
    //            iterations: 10,
    //        });
    //    }
    //
    // Gamba sinistra
    if let (Some(thigh), Some(shin), Some(foot)) = (
        find_bone("thigh_left"),
        find_bone("shin_left"),
        find_bone("foot_left"),
    ) {
        println!("Creating left leg");
        commands.entity(thigh).with_children(|p| {
            p.spawn((Collider::capsule_y(0.06, 0.12), Transform::default()));
        });
        commands.entity(shin).with_children(|p| {
            p.spawn((Collider::capsule_y(0.05, 0.1), Transform::default()));
        });
        commands.entity(foot).with_children(|p| {
            p.spawn((Collider::ball(0.06), Transform::default()));
        });
        let root_pos = transforms.get(root_bone).unwrap().translation;
        let thigh_pos = transforms.get(thigh).unwrap().translation;
        // Project root position onto ground (Y=0)
        let foot_target = Vec3::new(root_pos.x - 0.15, 0.0, root_pos.z); // left foot offset from root
        let pole = thigh_pos + Vec3::new(0.0, 0.0, 0.3);
        // Add joints: thigh -> shin, shin -> foot
        commands
            .entity(shin)
            .insert(ImpulseJoint::new(thigh, SphericalJointBuilder::new()));
        commands
            .entity(foot)
            .insert(ImpulseJoint::new(shin, SphericalJointBuilder::new()));
        commands.spawn(IKChain {
            bones: vec![thigh, shin, foot],
            target: foot_target,
            pole_target: Some(pole),
            iterations: 10,
        });
    }

    // Gamba destra
    if let (Some(thigh), Some(shin), Some(foot)) = (
        find_bone("thigh_right"),
        find_bone("shin_right"),
        find_bone("foot_right"),
    ) {
        println!("Creating right leg");
        commands.entity(thigh).with_children(|p| {
            p.spawn((Collider::capsule_y(0.06, 0.12), Transform::default()));
        });
        commands.entity(shin).with_children(|p| {
            p.spawn((Collider::capsule_y(0.05, 0.1), Transform::default()));
        });
        commands.entity(foot).with_children(|p| {
            p.spawn((Collider::ball(0.06), Transform::default()));
        });
        let root_pos = transforms.get(root_bone).unwrap().translation;
        let thigh_pos = transforms.get(thigh).unwrap().translation;
        // Project root position onto ground (Y=0)
        let foot_target = Vec3::new(root_pos.x + 0.15, 0.0, root_pos.z); // right foot offset from root
        let pole = thigh_pos + Vec3::new(0.0, 0.0, 0.3);
        commands
            .entity(shin)
            .insert(ImpulseJoint::new(thigh, SphericalJointBuilder::new()));
        commands
            .entity(foot)
            .insert(ImpulseJoint::new(shin, SphericalJointBuilder::new()));
        commands.spawn(IKChain {
            bones: vec![thigh, shin, foot],
            target: foot_target,
            pole_target: Some(pole),
            iterations: 10,
        });
    }

    // Collo/Head
    // if let (Some(neck), Some(head)) = (find_bone("neck"), find_bone("head")) {
    //     println!("Creating neck and head");
    //     // Find spine or use root as parent for neck joint
    //     let spine_or_root = find_bone("spine").or(Some(root_bone));
    //     if let Some(parent_bone) = spine_or_root {
    //         commands
    //             .entity(neck)
    //             .insert(ImpulseJoint::new(parent_bone, SphericalJointBuilder::new()));
    //     }
    //     // Optionally add joint from neck to head
    //     commands
    //         .entity(head)
    //         .insert(ImpulseJoint::new(neck, SphericalJointBuilder::new()));
    //     // IK chain for neck and head
    //     commands.spawn(IKChain {
    //         bones: vec![neck, head],
    //         target: transforms.get(head).unwrap().translation + Vec3::new(0.0, 0.2, 0.0),
    //         pole_target: None,
    //         iterations: 10,
    //     });
    // }
}
