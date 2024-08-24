use axum::{response::IntoResponse, routing::get, Router};
use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*, utils::HashMap};
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, QuinnetServer, QuinnetServerPlugin,
        ServerEndpointConfiguration,
    },
    shared::{channels::ChannelsConfiguration, ClientId},
};
use clap::{arg, Parser};
use engine::{
    api_client::{ping_server, register_server},
    models::api::servers::{RegisterServer, Server},
    network::{ClientMessage, ServerMessage},
    resources::TokioRuntimeResource,
};
use sqlx::{migrate::MigrateDatabase, query, query_as, Pool, Sqlite, SqlitePool};
use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
use time::OffsetDateTime;
use tokio::sync::mpsc::channel;
use uuid::Uuid;

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct ServerArgs {
    /// The name of the server
    #[arg(short, long, default_value = "My Server")]
    name: String,

    #[arg(long, default_value = "http://localhost:3000")]
    api_base_url: String,

    #[arg(long, default_value = "127.0.0.1")]
    addr: IpAddr,

    /// The port to run the server on
    #[arg(short, long, default_value = "2525")]
    port: u16,

    /// The port to run the management web server on
    #[arg(short, long, default_value = "3001")]
    web_port: u16,
}

enum TokioServerMessage {
    InitializePool(Pool<Sqlite>),
    PingServer(Server),
    RegisterServer(Server),
}

#[derive(Resource)]
struct ConnectionResource {
    users: HashMap<ClientId, Uuid>,
    server: Option<Server>,
    last_ping_attempt: OffsetDateTime,
}

impl Default for ConnectionResource {
    fn default() -> Self {
        Self {
            users: HashMap::new(),
            server: None,
            last_ping_attempt: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Default, Resource)]
struct WebServerResource {
    started: bool,
}

#[derive(Default, Resource)]
struct DatabaseResource {
    pool: Option<Pool<Sqlite>>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args = ServerArgs::parse();
    let (tx, rx) = channel::<TokioServerMessage>(10);

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .insert_resource(args)
        .insert_resource(WebServerResource::default())
        .add_plugins(QuinnetServerPlugin::default())
        .add_systems(Startup, start_listening)
        .add_systems(Update, handle_client_messages)
        .insert_resource(ConnectionResource::default())
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .add_systems(Update, tokio_receiver_system)
        .insert_resource(DatabaseResource::default())
        .add_systems(Startup, init_database)
        .add_systems(Startup, register_server_system)
        .add_systems(Update, ping_server_system)
        .add_systems(Update, start_webserver)
        .run();
}

fn init_database(tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>) {
    let tx = tokio_runtime_resource.sender.clone();
    tokio_runtime_resource.runtime.spawn(async move {
        const DB_URL: &str = "sqlite://.server/server.db";

        if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
            println!("Creating database {}", DB_URL);
            match Sqlite::create_database(DB_URL).await {
                Ok(_) => println!("Create db success"),
                Err(error) => panic!("error: {}", error),
            }
        } else {
            println!("Database already exists");
        }

        let db = SqlitePool::connect(DB_URL).await.unwrap();
        let result = sqlx::query("CREATE TABLE IF NOT EXISTS users (client_id INTEGER NOT NULL, user_id UUID UNIQUE NOT NULL, last_ping TIMESTAMPZ NOT NULL);").execute(&db).await.unwrap();
        println!("Create user table result: {:?}", result);

        tx.send(TokioServerMessage::InitializePool(db))
            .await
            .unwrap();
    });
}

fn start_listening(server_args: Res<ServerArgs>, mut server: ResMut<QuinnetServer>) {
    server
        .start_endpoint(
            ServerEndpointConfiguration::from_ip(
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                server_args.port,
            ),
            CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "127.0.0.1".to_string(),
            },
            ChannelsConfiguration::default(),
        )
        .unwrap();
}

fn handle_client_messages(
    mut connection_resource: ResMut<ConnectionResource>,
    mut server: ResMut<QuinnetServer>,
    database_resource: Res<DatabaseResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some((_channel_id, message)) =
            endpoint.try_receive_message_from::<ClientMessage>(client_id)
        {
            match message {
                ClientMessage::Join { user_id } => {
                    if let Some(db) = &database_resource.pool {
                        let db = db.clone();
                        // TODO:
                        let db_client_id: i32 =
                            client_id.try_into().expect("error converting client_id");
                        tokio_runtime_resource.runtime.spawn(async move {
                            query("INSERT INTO users (client_id, user_id, last_ping) VALUES ($1, $2, datetime('now')) RETURNING *;")
                            .bind(db_client_id)
                            .bind(user_id)
                            .fetch_one(&db)
                            .await
                            .unwrap();
                        });
                    }
                    endpoint
                        .broadcast_message(ServerMessage::ClientConnected { client_id, user_id })
                        .unwrap();
                    connection_resource.users.insert(client_id, user_id);

                    for (user_client_id, user_id) in connection_resource.users.iter() {
                        endpoint
                            .send_message(
                                client_id,
                                ServerMessage::ClientConnected {
                                    client_id: *user_client_id,
                                    user_id: *user_id,
                                },
                            )
                            .unwrap();
                    }
                }
                ClientMessage::Disconnect {} => {
                    if let Some(db) = &database_resource.pool {
                        let db = db.clone();
                        // TODO:
                        let db_client_id: i32 =
                            client_id.try_into().expect("error converting client_id");
                        tokio_runtime_resource.runtime.spawn(async move {
                            query("DELETE FROM users WHERE client_id = $1;")
                                .bind(db_client_id)
                                .execute(&db)
                                .await
                        });
                    }
                    connection_resource.users.remove(&client_id);
                    endpoint
                        .broadcast_message(ServerMessage::ClientDisconnected { client_id })
                        .unwrap();

                    endpoint.disconnect_client(client_id).unwrap();
                }
                ClientMessage::ChatMessage { message } => {
                    endpoint
                        .broadcast_message(ServerMessage::ChatMessage { client_id, message })
                        .unwrap();
                }
            }
        }
    }
}

fn tokio_receiver_system(
    mut connection_resource: ResMut<ConnectionResource>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<TokioServerMessage>>,
    mut database_resource: ResMut<DatabaseResource>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() {
        match message {
            TokioServerMessage::RegisterServer(server) => connection_resource.server = Some(server),
            TokioServerMessage::PingServer(server) => connection_resource.server = Some(server),
            TokioServerMessage::InitializePool(pool) => database_resource.pool = Some(pool),
        }
    }
}

fn register_server_system(
    server_args: Res<ServerArgs>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
) {
    let tx = tokio_runtime_resource.sender.clone();
    let api_base_url = server_args.api_base_url.clone();
    let addr = server_args.addr;
    let port = server_args.port;
    let name = server_args.name.clone();

    tokio_runtime_resource.runtime.spawn(async move {
        let result = register_server(&api_base_url, &RegisterServer { addr, port, name }).await;

        match result {
            Ok(server) => tx
                .send(TokioServerMessage::RegisterServer(server))
                .await
                .unwrap(),
            Err(error) => error!(error = ?error, "Create"),
        }
    });
}

fn ping_server_system(
    server_args: Res<ServerArgs>,
    mut connection_resource: ResMut<ConnectionResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
) {
    if let Some(server) = &connection_resource.server {
        let now = OffsetDateTime::now_utc();
        let timeout = Duration::from_secs(60);

        if now - server.last_ping >= timeout
            && now - connection_resource.last_ping_attempt >= timeout
        {
            let tx = tokio_runtime_resource.sender.clone();

            let api_base_url = server_args.api_base_url.clone();
            let id = server.id;

            tokio_runtime_resource.runtime.spawn(async move {
                let result = ping_server(&api_base_url, &id).await;

                match result {
                    Ok(server) => tx
                        .send(TokioServerMessage::PingServer(server))
                        .await
                        .unwrap(),
                    Err(error) => error!(error = ?error, "Ping"),
                }
            });

            connection_resource.last_ping_attempt = OffsetDateTime::now_utc();
        }
    }
}

#[derive(Clone)]
struct ApiState {
    pub pool: Option<Pool<Sqlite>>,
}

fn start_webserver(
    server_args: Res<ServerArgs>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
    database_resource: Res<DatabaseResource>,
    mut web_resource: ResMut<WebServerResource>,
) {
    if database_resource.pool.is_some() && !web_resource.started {
        web_resource.started = true;
        let web_port = server_args.web_port;
        let api_state = ApiState {
            pool: database_resource.pool.clone(),
        };

        tokio_runtime_resource.runtime.spawn(async move {
            let app = Router::new()
                .route("/", get(get_root))
                .with_state(api_state);

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", web_port))
                .await
                .unwrap();
            axum::serve(listener, app).await
        });
    }
}

#[derive(serde::Serialize, sqlx::FromRow)]
struct UserRow {
    client_id: i32,
    user_id: Uuid,
    last_ping: OffsetDateTime,
}

#[axum::debug_handler]
async fn get_root(
    axum::extract::State(api_state): axum::extract::State<ApiState>,
) -> impl IntoResponse {
    let users: Vec<UserRow> = query_as("SELECT * FROM users;")
        .fetch_all(&api_state.pool.unwrap())
        .await
        .unwrap();

    axum::Json(users)
}
