use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameServer {
    pub id: Uuid,
    pub addr: SocketAddr,
    pub name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub last_ping: OffsetDateTime,
}

impl GameServer {
    pub fn new(addr: SocketAddr, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            addr,
            last_ping: OffsetDateTime::now_utc(),
        }
    }

    pub fn ping(&mut self) {
        self.last_ping = OffsetDateTime::now_utc();
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterGameServer {
    pub name: String,
}
