use crate::ConnectionState;

use super::api::ApiResource;
use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use engine::{components::player::Player, models::network::ClientMessage};

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            keyboard_input.run_if(in_state(ConnectionState::Connected)),
        );
    }
}

fn keyboard_input(
    players: Query<(&Player, &Transform)>,
    api: Res<ApiResource>,
    keys: Res<ButtonInput<KeyCode>>,
    client: ResMut<QuinnetClient>,
) {
    if let Some(user) = &api.profile.data {
        if let Some((_, transform)) = players.iter().find(|(player, _)| player.user_id == user.id) {
            if keys.pressed(KeyCode::KeyW) {
                let mut position = transform.translation;
                position.x += 0.1;

                client
                    .connection()
                    .send_message(ClientMessage::UpdatePosition { position })
                    .unwrap();
            } else if keys.pressed(KeyCode::KeyS) {
                let mut position = transform.translation;
                position.x -= 0.1;

                client
                    .connection()
                    .send_message(ClientMessage::UpdatePosition { position })
                    .unwrap();
            } else if keys.pressed(KeyCode::KeyA) {
                let mut position = transform.translation;
                position.z += 0.1;

                client
                    .connection()
                    .send_message(ClientMessage::UpdatePosition { position })
                    .unwrap();
            } else if keys.pressed(KeyCode::KeyD) {
                let mut position = transform.translation;
                position.z -= 0.1;

                client
                    .connection()
                    .send_message(ClientMessage::UpdatePosition { position })
                    .unwrap();
            }
        }
    }
}
