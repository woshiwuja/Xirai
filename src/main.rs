mod pp;
mod cursor;
mod assets;
//mod jump_flood;
mod transform;
mod chess;
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
        .add_plugins(retrocamera::RetroRenderPlugin {
            width: 320,
            height: 180,
        })
        .add_plugins(outline::OutlinePlugin)
        .add_systems(Startup, setup)
        .add_systems(Startup, assets::setup_game_assets)
        .add_systems(Startup, setup_physics)
        .add_systems(Startup, spawn_character)
        .add_plugins(ui::UiPlugin)
        .add_systems(
            Update,
            camera::pan_orbit_camera.run_if(any_with_component::<camera::PanOrbitState>),
        )
        .add_plugins(RapierPickingPlugin)
        .add_systems(Startup, camera::spawn_camera)
        //.add_plugins(jump_flood::JumpFloodOutlinePlugin)
        .add_plugins(pp::PostProcessPlugin)
        .add_plugins(RemotePlugin::default())
        .add_plugins(
            assets::AssetsPlugin
        )
        .add_plugins(RemoteHttpPlugin::default())
        .add_plugins(chess::ChessPlugin)
        .add_plugins((OutlinePlugin, AutoGenerateOutlineNormalsPlugin::default()))
        .add_plugins(transform::TransformGizmoPlugin)
        .add_plugins(cursor::CursorPluginRetro)
    .add_plugins(MeshPickingPlugin)

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
    // Abilitiamo l'uso come render target:
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    // Filtro nearest-neighbor per mantenere il pixelato:
    image.sampler = ImageSampler::nearest(); // filtro mag/min = Nearest:contentReference[oaicite:3]{index=3}
                                             //let texture_handle = asset_server.load("textures/grass_ground.png");
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
            illuminance: 2000.0,
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
            transform::Pickable,
        ))
        .insert(outline::Outlined);
}


/// Componenti per identificare il personaggio
#[derive(Component)]
struct Character;

fn spawn_character(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("spawning character");
    const ANIMATION_PATH: &str = "female.glb";
    let clip_handle: Handle<AnimationClip> =
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(ANIMATION_PATH));
    let (graph, index) = AnimationGraph::from_clip(clip_handle);
    const PATH: &str = "female.glb";
    // 1️⃣ Spawn scena GLTF
    let scene_handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(PATH));

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
            Transform::from_xyz(0.0, 3.0, 5.0),
            Character,
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED_X,
            Friction::coefficient(1.0),
            Restitution::coefficient(0.0),
            AsyncSceneInheritOutline::default(),
            OutlineVolume {
                visible: true,
                width: 2.0,
                colour: BLACK.into(),
            },
        ))
        .with_children(|children| {
            children.spawn((
                Collider::capsule_y(0.6, 0.2),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ));
        })
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
        assets::Alive,
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
