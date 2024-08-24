use axum::{
    routing::{get, post},
    Extension, Router,
};
use clap::Parser;
use dotenvy::dotenv;
use handlers::{
    auth::{authenticate, profile},
    servers::{list_servers, ping_server, register_server},
    users::register_user,
};
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr};
use tower_http::trace::TraceLayer;

mod handlers;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ApiArgs {
    /// The port to run the server on
    #[arg(short, long, default_value = "3000")]
    port: u16,
}

#[tokio::main]
async fn main() {
    dotenv().expect(".env file not found");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL env variable is missing");

    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&database_url)
        .await
        .expect("could not connect to database_url");

    let args = ApiArgs::parse();

    let app = Router::new()
        .route("/auth", get(profile).post(authenticate))
        .route("/servers", get(list_servers).post(register_server))
        .route("/servers/:id/ping", post(ping_server))
        .route("/users", post(register_user))
        .layer(Extension(pool))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
