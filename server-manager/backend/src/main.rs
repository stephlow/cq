// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use uuid::Uuid;

#[tauri::command]
async fn get_players() -> Result<Vec<Uuid>, ()> {
    let body = reqwest::get("http://localhost:3001/players")
        .await
        .unwrap()
        .json::<Vec<Uuid>>()
        .await
        .unwrap();

    Ok(body)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_players])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
