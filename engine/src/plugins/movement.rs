use bevy::prelude::*;

use crate::components::{
    movement::Movement,
    player::{Player, PlayerPosition},
};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_movement);
    }
}

static SPEED: f32 = 0.1;

fn handle_movement(mut players: Query<(&mut PlayerPosition, &Movement), With<Player>>) {
    for (mut player_position, movement) in players.iter_mut() {
        if movement.forward {
            player_position.0.x += SPEED;
        }

        if movement.backward {
            player_position.0.x -= SPEED;
        }

        if movement.left {
            player_position.0.z -= SPEED;
        }

        if movement.right {
            player_position.0.z += SPEED;
        }
    }
}
