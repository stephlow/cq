use anyhow::Result;
use models::server::api::PlayerResponse;
use once_cell::sync::Lazy;
use reqwest::{Client, Method};
use std::time::Duration;
use uuid::Uuid;

// TODO: Make configurable
static api_base_url: &'static str = "http://localhost:3001";

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .expect("failed to initialize client")
});

pub async fn get_players() -> Result<PlayerResponse> {
    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/players"))
        .send()
        .await?;

    let auth_response = response.json::<PlayerResponse>().await?;

    Ok(auth_response)
}

pub async fn kick_player(id: Uuid) -> Result<()> {
    CLIENT
        .request(Method::DELETE, format!("{api_base_url}/players/{id}"))
        .send()
        .await?;

    Ok(())
}
