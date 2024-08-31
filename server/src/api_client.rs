use crate::data::api::PlayerResponse;
use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::{Client, Method};
use std::time::Duration;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .expect("failed to initialize client")
});

pub async fn get_players() -> Result<PlayerResponse> {
    let api_base_url = "http://localhost:3001";

    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/auth"))
        .send()
        .await?;

    let auth_response = response.json::<PlayerResponse>().await?;

    Ok(auth_response)
}
