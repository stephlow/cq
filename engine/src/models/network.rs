use bevy::math::Vec3;
use bevy_quinnet::shared::ClientId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::movement::MoveModifier;

#[derive(Debug, Deserialize, Serialize)]
pub enum ClientMessage {
    Join { user_id: Uuid },
    Disconnect,
    ChatMessage { message: String },
    UpdatePosition { position: Vec3 },
    SendModifier(MoveModifier),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ServerMessage {
    ClientConnected {
        client_id: ClientId,
        user_id: Uuid,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    ChatMessage {
        client_id: ClientId,
        message: String,
    },
    UpdatePosition {
        client_id: ClientId,
        position: Vec3,
    },
    SendModifier {
        client_id: ClientId,
        key_code: MoveModifier,
    },
}
