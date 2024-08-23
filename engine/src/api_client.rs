use crate::models::api::{GameServer, RegisterGameServer};
use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::{Client, Method};
use std::time::Duration;
use uuid::Uuid;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .expect("failed to initialize client")
});

pub async fn list_servers(api_base_url: &str) -> Result<Vec<GameServer>> {
    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/servers"))
        .send()
        .await?;

    let servers = response.json::<Vec<GameServer>>().await.unwrap();

    Ok(servers)
}

pub async fn register_server(
    api_base_url: &str,
    new_server: &RegisterGameServer,
) -> Result<GameServer> {
    let response = CLIENT
        .request(Method::POST, format!("{api_base_url}/servers"))
        .json(new_server)
        .send()
        .await?;

    let server = response.json::<GameServer>().await?;

    Ok(server)
}

pub async fn ping_server(api_base_url: &str, id: &Uuid) -> Result<GameServer> {
    let response = CLIENT
        .request(Method::POST, format!("{api_base_url}/servers/{id}/ping"))
        .send()
        .await?;

    let server = response.json::<GameServer>().await?;

    Ok(server)
}
