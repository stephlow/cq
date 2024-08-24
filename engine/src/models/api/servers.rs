use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Server {
    pub id: Uuid,
    pub addr: IpAddr,
    pub port: u16,
    pub name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub last_ping: OffsetDateTime,
}

impl Server {
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

impl From<models::data::servers::Server> for Server {
    fn from(value: models::data::servers::Server) -> Self {
        // let addr = IpAddr::from_str(&value.addr).expect("Invalid ip");
        let port: u16 = value.port.try_into().expect("Invalid port");

        Self {
            id: value.id,
            addr: value.addr,
            port,
            name: value.name,
            last_ping: value.last_ping,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterServer {
    pub addr: IpAddr,
    pub port: u16,
    pub name: String,
}
