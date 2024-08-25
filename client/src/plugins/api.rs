use crate::ClientArgs;
use bevy::prelude::*;
use engine::{api_client::list_servers, models::api::servers::Server};
use tokio::{runtime::Runtime, sync::mpsc};

pub struct ApiPlugin;

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ApiResource>()
            .add_event::<ApiEvent>()
            .add_systems(Update, api_event_handler_system)
            .add_systems(Update, api_message_handler_system);
    }
}

#[derive(Event)]
pub enum ApiEvent {
    LoadServers,
}

enum ApiMessage {
    LoadServers(Vec<Server>),
}

#[derive(Resource)]
pub struct ApiResource {
    runtime: Runtime,
    tx: mpsc::Sender<ApiMessage>,
    rx: mpsc::Receiver<ApiMessage>,
    pub servers: LoadableData<Vec<Server>>,
}

impl Default for ApiResource {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(100);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        Self {
            runtime,
            tx,
            rx,
            servers: LoadableData::default(),
        }
    }
}

#[derive(Clone)]
pub struct LoadableData<T> {
    pub loading: bool,
    pub data: Option<T>,
}

impl<T> Default for LoadableData<T> {
    fn default() -> Self {
        Self {
            loading: false,
            data: None,
        }
    }
}

impl<T> LoadableData<T> {
    fn start(&mut self) {
        self.loading = true;
    }

    fn finish(&mut self, data: T) {
        self.data = Some(data);
        self.loading = true;
    }
}

fn api_event_handler_system(
    mut api_resource: ResMut<ApiResource>,
    client_args: Res<ClientArgs>,
    mut events: EventReader<ApiEvent>,
) {
    for event in events.read() {
        match event {
            ApiEvent::LoadServers => {
                let api_base_url = client_args.api_base_url.clone();
                let tx = api_resource.tx.clone();

                if !api_resource.servers.loading {
                    api_resource.servers.start();

                    api_resource.runtime.spawn(async move {
                        match list_servers(&api_base_url).await {
                            Ok(servers) => {
                                tx.send(ApiMessage::LoadServers(servers)).await.unwrap();
                            }
                            Err(_) => {}
                        }
                    });
                }
            }
        }
    }
}

fn api_message_handler_system(mut api_resource: ResMut<ApiResource>) {
    match api_resource.rx.try_recv() {
        Ok(message) => match message {
            ApiMessage::LoadServers(servers) => {
                api_resource.servers.finish(servers);
            }
        },
        Err(_) => {}
    }
}
