use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use server::data::api::PlayerResponse;
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

async fn get_players() -> PlayerResponse {
    let args = to_value(&GetPlayersArgs).unwrap();
    let response = invoke("get_players", args).await;
    serde_wasm_bindgen::from_value(response).unwrap()
}

#[function_component(App)]
pub fn app() -> Html {
    let players: UseStateHandle<Option<PlayerResponse>> = use_state(|| None);
    {
        let players = players.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let value = get_players().await;
                players.set(Some(value));
            });
        });
    }

    let reload = {
        let players = players.clone();
        Callback::from(move |e: MouseEvent| {
            let players = players.clone();
            e.prevent_default();
            spawn_local(async move {
                let value = get_players().await;
                players.set(Some(value));
            });
        })
    };

    // let greet_input_ref = use_node_ref();
    //
    // let name = use_state(|| String::new());
    //
    // let greet_msg = use_state(|| String::new());
    // {
    //     let greet_msg = greet_msg.clone();
    //     let name = name.clone();
    //     let name2 = name.clone();
    //     use_effect_with(name2, move |_| {
    //         spawn_local(async move {
    //             if name.is_empty() {
    //                 return;
    //             }
    //
    //             let args = to_value(&GreetArgs { name: &*name }).unwrap();
    //             // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    //             let new_msg = invoke("greet", args).await.as_string().unwrap();
    //             greet_msg.set(new_msg);
    //         });
    //
    //         || {}
    //     });
    // }
    //
    // let greet = {
    //     let name = name.clone();
    //     let greet_input_ref = greet_input_ref.clone();
    //     Callback::from(move |e: SubmitEvent| {
    //         e.prevent_default();
    //         name.set(
    //             greet_input_ref
    //                 .cast::<web_sys::HtmlInputElement>()
    //                 .unwrap()
    //                 .value(),
    //         );
    //     })
    // };

    let player_list = match &*players {
        Some(PlayerResponse { players }) => players
            .iter()
            .enumerate()
            .map(|(id, player)| {
                html! {
                    <li key={id}>{format!("{}", player)}</li>
                }
            })
            .collect::<Html>(),
        None => html! { <p>{"No players connected"}</p>},
    };

    html! {
        <main>
            <ul>{player_list}</ul>
            <button type="submit" onclick={reload}>{"Reload"}</button>
        </main>
    }

    // html! {
    //     <main class="container">
    //         {players.len()}
    //         <div style="background: red">{pls}</div>
    //         <div class="row">
    //             <a href="https://tauri.app" target="_blank">
    //                 <img src="public/tauri.svg" class="logo tauri" alt="Tauri logo"/>
    //             </a>
    //             <a href="https://yew.rs" target="_blank">
    //                 <img src="public/yew.png" class="logo yew" alt="Yew logo"/>
    //             </a>
    //         </div>

    //         <p>{"Click on the Tauri and Yew logos to learn more."}</p>
    //         <form class="row" onsubmit={greet}>
    //             <input id="greet-input" ref={greet_input_ref} placeholder="Enter a name..." />
    //             <button type="submit">{"Greet"}</button>
    //         </form>

    //         <p><b>{ &*greet_msg }</b></p>
    //     </main>
    // }
}
