// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use models::server::api::PlayerResponse;

#[tauri::command]
async fn get_players() -> Result<PlayerResponse, ()> {
    let response = server::api_client::get_players().await.unwrap();

    Ok(response)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_players])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
