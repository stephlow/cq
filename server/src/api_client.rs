use anyhow::Result;
use models::server::api::{PlayerResponse, ServerInfoResponse};
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

pub async fn get_server_info(api_base_url: &str) -> Result<ServerInfoResponse> {
    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/"))
        .send()
        .await?;

    let auth_response = response.json::<ServerInfoResponse>().await?;

    Ok(auth_response)
}

pub async fn get_players(api_base_url: &str) -> Result<PlayerResponse> {
    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/players"))
        .send()
        .await?;

    let auth_response = response.json::<PlayerResponse>().await?;

    Ok(auth_response)
}

pub async fn kick_player(api_base_url: &str, id: Uuid) -> Result<()> {
    CLIENT
        .request(Method::DELETE, format!("{api_base_url}/players/{id}"))
        .send()
        .await?;

    Ok(())
}
