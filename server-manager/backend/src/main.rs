// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use models::server::api::PlayerResponse;
use uuid::Uuid;

// TODO: Make configurable
struct ApiBaseUrl(String);

#[tauri::command]
async fn get_players(state: tauri::State<'_, ApiBaseUrl>) -> Result<PlayerResponse, ()> {
    let response = server::api_client::get_players(&state.0).await.unwrap();
    Ok(response)
}

#[tauri::command]
async fn kick_player(id: Uuid, state: tauri::State<'_, ApiBaseUrl>) -> Result<(), ()> {
    server::api_client::kick_player(&state.0, id).await.unwrap();
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(ApiBaseUrl("http://localhost:3001".into()))
        .invoke_handler(tauri::generate_handler![get_players, kick_player])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
