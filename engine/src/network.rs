use bevy_quinnet::shared::ClientId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub enum ClientMessage {
    Join { user_id: Uuid },
    Disconnect,
    ChatMessage { message: String },
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
}
