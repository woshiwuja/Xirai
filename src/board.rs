use bevy::color::palettes::css::*;
use bevy::prelude::*;
use crate::ground;
#[derive(Component)]
pub struct Board;

#[derive(Component)]
pub struct Tile;
#[derive(Component)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

fn setup_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let board_entity = commands
        .spawn((
            Board,
            Transform::from_xyz(10.0, 0.0, 10.0),
            GlobalTransform::default(),
        ))
        .id();

    let tile_size = 1.5; // Dimensione di ogni tile
    let board_width = 8; // Numero di tile in larghezza
    let board_height = 8; // Numero di tile in altezza
    let mut square_material = StandardMaterial {
        base_color: Color::WHITE, // Green color
        alpha_mode: AlphaMode::Opaque,
        unlit: true, // Flat pixel art look
        metallic: 0.0,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    };
    let mesh = Mesh::from(Plane3d::default().mesh().size(tile_size, tile_size));
    for x in 0..board_width {
        for y in 0..board_height {
            println!("Creating square at ({}, {})", x, y);
            let current_square = commands
                .spawn((
                    Mesh3d(meshes.add(mesh.clone())),
                    Transform::from_xyz(x as f32 * tile_size, 0.1, y as f32 * tile_size),
                    Size { w: 1, h: 1 },
                ))
                .id();
            if (x + y) % 2 == 0 {
                square_material.base_color = BLACK.into();
            } else {
                square_material.base_color = WHITE.into();
            }
            commands
                .entity(current_square)
                .insert(MeshMaterial3d(materials.add(square_material.clone())));
        }
    }
}


#[derive(Component)]
struct Piece;

fn setup_pieces(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>, mut meshes: ResMut<Assets<Mesh>>) {
    let white_material = StandardMaterial {
        base_color: Color::WHITE, // Green color
        alpha_mode: AlphaMode::Opaque,
        unlit: true, // Flat pixel art look
        metallic: 0.0,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    };
    let black_material = StandardMaterial {
        base_color: Color::BLACK, // Green color
        alpha_mode: AlphaMode::Opaque,
        unlit: true, // Flat pixel art look
        metallic: 0.0,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    };

    let pawn_mesh = Mesh::from(Sphere::new(0.1,));
    let rook_mesh = Mesh::from(Cuboid::new(0.2,0.2,0.2));
    let bishop_mesh = Mesh::from(Cylinder::new(0.1,0.3,));
    let knight_mesh = Mesh::from(Cylinder::new(0.2,0.2,));

    // Spawn white pieces
    for i in 0..8 {
        commands.spawn((
            Mesh3d(meshes.add(pawn_mesh.clone())),
            MeshMaterial3d(materials.add(white_material.clone())),
            Transform::from_xyz(i as f32 * 1.0, 0.5, 1.0),
            Piece,
        ));
    }
    commands.spawn((
        Mesh3d(meshes.add(rook_mesh.clone())),
        MeshMaterial3d(materials.add(white_material.clone())),
        Transform::from_xyz(1.0, 0.5, 0.0),
        Piece,
    ));
    commands.spawn((
        Mesh3d(meshes.add(knight_mesh.clone())),
        MeshMaterial3d(materials.add(white_material.clone())),
        Transform::from_xyz(2.0, 0.5, 0.0),
        Piece,
    ));
    commands.spawn((
        Mesh3d(meshes.add(bishop_mesh.clone())),
        MeshMaterial3d(materials.add(white_material.clone())),
        Transform::from_xyz(3.0, 0.5, 0.0),
        Piece,
    ));
}

pub struct BoardPlugin;
impl Plugin for BoardPlugin{
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_board)
        .add_systems(Startup, setup_pieces);
    }
}
