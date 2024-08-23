use bevy_quinnet::shared::ClientId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum ClientMessage {
    Join { username: String },
    Disconnect,
    ChatMessage { message: String },
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ServerMessage {
    ClientConnected {
        client_id: ClientId,
        username: String,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    ChatMessage {
        client_id: ClientId,
        message: String,
    },
}
