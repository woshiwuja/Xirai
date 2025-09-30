use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::render::mesh::skinning::SkinnedMesh;
use bevy::render::render_resource::Face;
use bevy::scene::SceneInstanceReady;
#[derive(Component)]
pub struct Outlined;

pub struct OutlinePlugin;
impl Plugin for OutlinePlugin{
    fn build(&self, app: &mut App) {
        app.add_systems(Update, generate_outlines);
    }
}

fn generate_outlines(
    mut commands: Commands,
    query: Query<(Entity, &Mesh3d, &Transform), Added<Outlined>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mesh_handle, transform) in &query {
        // Materiale nero unlit per l'outline
        let outline_mat = materials.add(StandardMaterial {
            base_color: BLACK.into(),
            unlit: true,
            alpha_mode: AlphaMode::Opaque,
            metallic: 0.0,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            cull_mode: Some(Face::Front),
            ..Default::default()
        });

        // Mesh duplicato come outline, leggermente più grande
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(mesh_handle.0.clone()),            // wrapper per il mesh
                MeshMaterial3d(outline_mat),              // wrapper per il materiale
                Transform::from_scale(Vec3::splat(1.05)), // scalatura per l'outline
                Visibility::default(),
            ));
        });
    }
}
//pub fn generate_outlines_for_gltf(
//    trigger: Trigger<SceneInstanceReady>,
//    mut commands: Commands,
//    mesh_query: Query<&Mesh3d>,
//    transform_query: Query<&GlobalTransform>,
//    skinned_query: Query<&SkinnedMesh>,
//    children: Query<&Children>,
//    mut materials: ResMut<Assets<StandardMaterial>>,
//) {
//    let outline_mat = materials.add(StandardMaterial {
//        base_color: Color::BLACK,
//        unlit: true,
//        alpha_mode: AlphaMode::Opaque,
//        cull_mode: Some(Face::Front),
//        ..default()
//    });
//
//    let root_entity = trigger.target();
//
//    for entity in children.iter_descendants(root_entity) {
//        if let Ok(mesh) = mesh_query.get(entity) {
//            if let Ok(global_transform) = transform_query.get(entity) {
//                let mut outline_entity = commands.spawn((
//                    Mesh3d(mesh.0.clone()),
//                    MeshMaterial3d(outline_mat.clone()),
//                    Transform {
//                        translation: global_transform.translation(),
//                        rotation: global_transform.rotation(),
//                        scale: global_transform.scale() * 1.05,
//                    },
//                    Visibility::default(),
//                ));
//
//                // Se la mesh originale è skinned, copia anche quello
//                if let Ok(skinned) = skinned_query.get(entity) {
//                    outline_entity.insert(skinned.clone());
//                }
//            }
//        }
//    }
//}

use bevy_mod_outline::{OutlineVolume, AsyncSceneInheritOutline};

pub fn generate_outlines_for_gltf(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
) {
    // L'entity principale della scena GLTF
    let root_entity = trigger.target();
    
    // Aggiungi l'outline alla root entity della scena
    // AsyncSceneInheritOutline propaga automaticamente l'outline 
    // a tutte le mesh figlie, incluse quelle skinned
    commands.entity(root_entity).insert((
        OutlineVolume {
            visible: true,
            width: 3.0,
            colour: Color::BLACK,
        },
        AsyncSceneInheritOutline::default(),
    ));
}