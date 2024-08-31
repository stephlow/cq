use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(PartialEq, Deserialize, Serialize)]
pub struct PlayerResponse {
    pub players: Vec<Uuid>,
}
