use sqlx::prelude::FromRow;
use std::net::IpAddr;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(FromRow)]
pub struct Server {
    pub id: Uuid,
    pub name: String,
    pub addr: IpAddr,
    pub port: i32,
    pub last_ping: OffsetDateTime,
}
