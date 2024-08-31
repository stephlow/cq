use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct PlayerResponse {
    pub players: Vec<Uuid>,
}
