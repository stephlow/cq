use super::api::ApiResource;
use crate::ConnectionState;
use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use engine::{
    components::{
        movement::{MoveModifier, Movement},
        player::Player,
    },
    models::network::ClientMessage,
};

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
    mut players: Query<(&Player, &Transform, &mut Movement)>,
    api: Res<ApiResource>,
    keys: Res<ButtonInput<KeyCode>>,
    client: ResMut<QuinnetClient>,
) {
    if let Some(user) = &api.profile.data {
        if let Some((_, _, mut movement)) = players
            .iter_mut()
            .find(|(player, _, _)| player.user_id == user.id)
        {
            if keys.just_pressed(KeyCode::KeyW) {
                let modifier = MoveModifier::StartForward;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            } else if keys.just_released(KeyCode::KeyW) {
                let modifier = MoveModifier::StopForward;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            }

            if keys.just_pressed(KeyCode::KeyS) {
                let modifier = MoveModifier::StartBackward;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            } else if keys.just_released(KeyCode::KeyS) {
                let modifier = MoveModifier::StopBackward;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            }

            if keys.just_pressed(KeyCode::KeyD) {
                let modifier = MoveModifier::StartRight;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            } else if keys.just_released(KeyCode::KeyD) {
                let modifier = MoveModifier::StopRight;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            }

            if keys.just_pressed(KeyCode::KeyA) {
                let modifier = MoveModifier::StartLeft;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            } else if keys.just_released(KeyCode::KeyA) {
                let modifier = MoveModifier::StopLeft;
                movement.modify(modifier.clone());
                client
                    .connection()
                    .send_message(ClientMessage::SendModifier(modifier))
                    .unwrap();
            }
        }
    }
}
