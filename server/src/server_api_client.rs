use anyhow::Result;
use models::server::api::{PlayerResponse, ServerInfoResponse};
use reqwest::{Client, Method};
use std::time::Duration;
use uuid::Uuid;

pub struct ServerApiClient<'a> {
    base_url: &'a str,
    client: Client,
}

impl<'a> ServerApiClient<'a> {
    pub fn new(base_url: &'a str) -> Self {
        Self {
            base_url,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(30))
                .build()
                .expect("failed to initialize server api client"),
        }
    }

    pub async fn get_server_info(&self) -> Result<ServerInfoResponse> {
        let response = self
            .client
            .request(Method::GET, format!("{}/", self.base_url))
            .send()
            .await?;

        let auth_response = response.json::<ServerInfoResponse>().await?;

        Ok(auth_response)
    }

    pub async fn get_players(&self) -> Result<PlayerResponse> {
        let response = self
            .client
            .request(Method::GET, format!("{}/players", self.base_url))
            .send()
            .await?;

        let auth_response = response.json::<PlayerResponse>().await?;

        Ok(auth_response)
    }

    pub async fn kick_player(&self, id: Uuid) -> Result<()> {
        self.client
            .request(Method::DELETE, format!("{}/players/{}", self.base_url, id))
            .send()
            .await?;

        Ok(())
    }
}
