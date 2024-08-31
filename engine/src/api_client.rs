use anyhow::Result;
use models::api::{
    auth::{AuthResponse, Credentials},
    servers::{RegisterServer, Server},
    users::{NewUser, User},
};
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

pub async fn authenticate(api_base_url: &str, credentials: &Credentials) -> Result<AuthResponse> {
    let response = CLIENT
        .request(Method::POST, format!("{api_base_url}/auth"))
        .json(&credentials)
        .send()
        .await?;

    let auth_response = response.json::<AuthResponse>().await?;

    Ok(auth_response)
}

pub async fn get_profile(api_base_url: &str, token: &str) -> Result<User> {
    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/auth"))
        .header("Authorization", format!("Bearer: {}", token))
        .send()
        .await?;

    let user = response.json::<User>().await.unwrap();

    Ok(user)
}

pub async fn get_user(api_base_url: &str, id: &Uuid) -> Result<User> {
    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/users/{id}"))
        .send()
        .await?;

    let user = response.json::<User>().await.unwrap();

    Ok(user)
}

pub async fn register_user(api_base_url: &str, new_user: NewUser) -> Result<AuthResponse> {
    let response = CLIENT
        .request(Method::POST, format!("{api_base_url}/users"))
        .json(&new_user)
        .send()
        .await?;

    let user = response.json::<AuthResponse>().await?;

    Ok(user)
}

pub async fn list_servers(api_base_url: &str) -> Result<Vec<Server>> {
    let response = CLIENT
        .request(Method::GET, format!("{api_base_url}/servers"))
        .send()
        .await?;

    let servers = response.json::<Vec<Server>>().await.unwrap();

    Ok(servers)
}

pub async fn register_server(api_base_url: &str, new_server: &RegisterServer) -> Result<Server> {
    let response = CLIENT
        .request(Method::POST, format!("{api_base_url}/servers"))
        .json(new_server)
        .send()
        .await?;

    let server = response.json::<Server>().await?;

    Ok(server)
}

pub async fn ping_server(api_base_url: &str, id: &Uuid) -> Result<Server> {
    let response = CLIENT
        .request(Method::POST, format!("{api_base_url}/servers/{id}/ping"))
        .send()
        .await?;

    let server = response.json::<Server>().await?;

    Ok(server)
}
