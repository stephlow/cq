use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use time::OffsetDateTime;
use uuid::Uuid;

pub mod auth;
pub mod users;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameServer {
    pub id: Uuid,
    pub addr: IpAddr,
    pub port: u16,
    pub name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub last_ping: OffsetDateTime,
}

impl GameServer {
    pub fn new(addr: IpAddr, port: u16, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            addr,
            port,
            last_ping: OffsetDateTime::now_utc(),
        }
    }

    pub fn ping(&mut self) {
        self.last_ping = OffsetDateTime::now_utc();
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterGameServer {
    pub addr: IpAddr,
    pub port: u16,
    pub name: String,
}
