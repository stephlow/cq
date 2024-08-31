// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use models::server::api::PlayerResponse;
use once_cell::sync::Lazy;
use server::server_api_client::ServerApiClient;
use uuid::Uuid;

// TODO: Make configurable
static CLIENT: Lazy<ServerApiClient> = Lazy::new(|| ServerApiClient::new("http://localhost:3001"));

#[tauri::command]
async fn get_players() -> Result<PlayerResponse, ()> {
    let response = CLIENT.get_players().await.unwrap();
    Ok(response)
}

#[tauri::command]
async fn kick_player(id: Uuid) -> Result<(), ()> {
    CLIENT.kick_player(id).await.unwrap();
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_players, kick_player])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
