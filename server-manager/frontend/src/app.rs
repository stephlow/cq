use models::server::api::PlayerResponse;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct GetPlayersArgs;

#[derive(Serialize, Deserialize)]
struct KickPlayerArgs {
    id: Uuid,
}

async fn invoke_get_players() -> PlayerResponse {
    let args = to_value(&GetPlayersArgs).unwrap();
    let response = invoke("get_players", args).await;
    serde_wasm_bindgen::from_value(response).unwrap()
}

async fn invoke_kick_player(id: Uuid) {
    let args = to_value(&KickPlayerArgs { id }).unwrap();
    invoke("kick_player", args).await;
}

#[function_component(App)]
pub fn app() -> Html {
    let players: UseStateHandle<Option<PlayerResponse>> = use_state(|| None);

    let load_players = use_callback(players.clone(), move |_, players| {
        let players = players.clone();
        spawn_local(async move {
            let value = invoke_get_players().await;
            players.set(Some(value));
        });
    });

    use_effect_with(load_players.clone(), move |load_players| {
        load_players.emit(());
    });

    let reload = use_callback(
        load_players.clone(),
        move |event: MouseEvent, load_players| {
            event.prevent_default();
            load_players.emit(());
        },
    );

    let kick = use_callback(load_players.clone(), move |id: Uuid, load_players| {
        let load_players = load_players.clone();
        spawn_local(async move {
            invoke_kick_player(id).await;
            load_players.emit(());
        });
    });

    let player_list = use_memo(players.clone(), |players| {
        let players = players.clone();
        match players.as_ref() {
            Some(response) => {
                let players = response.players.clone();

                players
                    .into_iter()
                    .enumerate()
                    .map(|(index, player)| {
                        let kick = kick.clone();
                        let onclick = Callback::from(move |_| {
                            kick.emit(player);
                        });

                        html! {
                            <li key={index}>
                                {player.to_string()}
                                <button onclick={onclick}>{"Kick"}</button>
                            </li>
                        }
                    })
                    .collect::<Html>()
            }
            None => html! { <p>{"No players"}</p> },
        }
    });

    html! {
        <main>
            <ul>{(*player_list).clone()}</ul>
            <button type="submit" onclick={reload}>{"Reload"}</button>
        </main>
    }
}
