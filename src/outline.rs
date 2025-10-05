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
        app.add_systems(Update, generate_outlines_for_assets);
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
use bevy_mod_outline::{OutlineVolume, AsyncSceneInheritOutline};

pub fn generate_outlines_for_assets(
    mut commands: Commands,
    query: Query<(Entity,&Mesh3d, &crate::assets::GameAsset),Added<Outline>>
) {
    for (e,_,_) in query.iter(){
    commands.entity(e).insert((
        OutlineVolume {
            visible: true,
            width: 4.0,
            colour: Color::BLACK,
        },
        AsyncSceneInheritOutline::default(),
    ));
    }
}