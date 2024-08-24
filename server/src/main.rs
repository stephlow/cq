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
use plugins::{
    database::{models::UserRow, DatabasePlugin, SqliteServer},
    webserver::WebServerPlugin,
};
use sqlx::{query, query_as, Pool, Sqlite};
use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
use time::OffsetDateTime;
use tokio::sync::mpsc::channel;
use uuid::Uuid;

mod plugins;

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

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args = ServerArgs::parse();
    let web_port = args.web_port.clone();

    let (tx, rx) = channel::<TokioServerMessage>(10);

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .insert_resource(args)
        .add_plugins(QuinnetServerPlugin::default())
        .add_plugins(DatabasePlugin::default())
        .add_systems(Startup, start_listening)
        .add_systems(Update, handle_client_messages)
        .insert_resource(ConnectionResource::default())
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, register_server_system)
        .add_systems(Update, ping_server_system)
        .add_plugins(WebServerPlugin::new(web_port))
        .run();
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
    sqlite_server: Res<SqliteServer>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some((_channel_id, message)) =
            endpoint.try_receive_message_from::<ClientMessage>(client_id)
        {
            match message {
                ClientMessage::Join { user_id } => {
                    if let Some(db) = &sqlite_server.pool {
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
                    if let Some(db) = &sqlite_server.pool {
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
                    if let Some(db) = &sqlite_server.pool {
                        let db = db.clone();
                        // TODO:
                        let db_client_id: i32 =
                            client_id.try_into().expect("error converting client_id");

                        let message = message.clone();
                        tokio_runtime_resource.runtime.spawn(async move {
                            let user: UserRow = query_as("SELECT * FROM users WHERE client_id = $1").bind(db_client_id).fetch_one(&db).await.unwrap();

                            query("INSERT INTO messages (user_id, content) VALUES ($1, $2) RETURNING *;")
                            .bind(user.user_id)
                            .bind(message)
                            .fetch_one(&db)
                            .await
                            .unwrap();
                        });
                    }
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
    mut database_resource: ResMut<SqliteServer>,
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
