use crate::{AuthState, ClientArgs};
use bevy::prelude::*;
use engine::{
    api_client::{authenticate, get_profile, list_servers, register_user},
    models::api::{
        auth::{AuthResponse, Credentials},
        servers::Server,
        users::{NewUser, User},
    },
};
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
    Authenticate {
        username: String,
        password: String,
    },
    Register {
        username: String,
        email: String,
        password: String,
    },
    LoadProfile,
    LoadServers,
}

enum ApiMessage {
    AuthenticateFulfilled(String),
    RegisterFulfilled(String),
    LoadProfileFulfilled(User),
    LoadServersFulfilled(Vec<Server>),
}

#[derive(Resource)]
pub struct ApiResource {
    runtime: Runtime,
    tx: mpsc::Sender<ApiMessage>,
    rx: mpsc::Receiver<ApiMessage>,
    pub token: LoadableData<String>,
    pub profile: LoadableData<User>,
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
            token: LoadableData::default(),
            profile: LoadableData::default(),
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
        println!("START");
        self.loading = true;
    }

    fn finish(&mut self, data: T) {
        println!("FINISH");
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
            ApiEvent::Authenticate { username, password } => {
                if !api_resource.token.loading {
                    api_resource.token.start();

                    let tx = api_resource.tx.clone();
                    let api_base_url = client_args.api_base_url.clone();
                    let username = username.clone();
                    let password = password.clone();

                    api_resource.runtime.spawn(async move {
                        match authenticate(&api_base_url, &Credentials { username, password }).await
                        {
                            Ok(AuthResponse { token }) => tx
                                .send(ApiMessage::AuthenticateFulfilled(token))
                                .await
                                .unwrap(),
                            Err(_) => {}
                        }
                    });
                }
            }
            ApiEvent::Register {
                username,
                email,
                password,
            } => {
                if !api_resource.token.loading {
                    api_resource.token.start();

                    let tx = api_resource.tx.clone();
                    let api_base_url = client_args.api_base_url.clone();
                    let username = username.clone();
                    let email = email.clone();
                    let password = password.clone();

                    api_resource.runtime.spawn(async move {
                        match register_user(
                            &api_base_url,
                            NewUser {
                                username,
                                email,
                                password,
                            },
                        )
                        .await
                        {
                            Ok(AuthResponse { token }) => {
                                tx.send(ApiMessage::RegisterFulfilled(token)).await.unwrap()
                            }
                            Err(_) => {}
                        }
                    });
                }
            }
            ApiEvent::LoadProfile => {
                if !api_resource.profile.loading {
                    api_resource.profile.start();

                    let tx = api_resource.tx.clone();
                    let api_base_url = client_args.api_base_url.clone();
                    if let Some(token) = &api_resource.token.data {
                        let token = token.clone();

                        api_resource.runtime.spawn(async move {
                            match get_profile(&api_base_url, &token).await {
                                Ok(user) => tx
                                    .send(ApiMessage::LoadProfileFulfilled(user))
                                    .await
                                    .unwrap(),
                                Err(_) => {}
                            }
                        });
                    }
                }
            }
            ApiEvent::LoadServers => {
                if !api_resource.servers.loading {
                    api_resource.servers.start();

                    let tx = api_resource.tx.clone();
                    let api_base_url = client_args.api_base_url.clone();

                    api_resource.runtime.spawn(async move {
                        match list_servers(&api_base_url).await {
                            Ok(servers) => {
                                tx.send(ApiMessage::LoadServersFulfilled(servers))
                                    .await
                                    .unwrap();
                            }
                            Err(_) => {}
                        }
                    });
                }
            }
        }
    }
}

fn api_message_handler_system(
    mut api_resource: ResMut<ApiResource>,
    mut events: EventWriter<ApiEvent>,
    mut next_auth_state: ResMut<NextState<AuthState>>,
) {
    match api_resource.rx.try_recv() {
        Ok(message) => match message {
            ApiMessage::AuthenticateFulfilled(token) => {
                api_resource.token.finish(token);
                next_auth_state.set(AuthState::Authenticated);
                events.send(ApiEvent::LoadProfile);
            }
            ApiMessage::RegisterFulfilled(token) => {
                api_resource.token.finish(token);
                next_auth_state.set(AuthState::Authenticated);
                events.send(ApiEvent::LoadProfile);
            }
            ApiMessage::LoadProfileFulfilled(user) => {
                api_resource.profile.finish(user);
            }
            ApiMessage::LoadServersFulfilled(servers) => {
                api_resource.servers.finish(servers);
            }
        },
        Err(_) => {}
    }
}
