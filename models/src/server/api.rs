use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::servers::Server;

#[derive(PartialEq, Deserialize, Serialize)]
pub struct ServerInfoResponse {
    pub server: Option<Server>,
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct PlayerResponse {
    pub players: Vec<Uuid>,
}
