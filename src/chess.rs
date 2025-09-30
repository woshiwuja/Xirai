use crate::board;
use bevy::prelude::*;

pub struct ChessPlugin;
impl Plugin for ChessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(board::BoardPlugin);
    }
} 