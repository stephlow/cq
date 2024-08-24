use axum::{
    extract::{ConnectInfo, Path},
    Extension, Json,
};
use engine::models;
use sqlx::{query_as, PgPool};
use std::net::SocketAddr;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[axum::debug_handler]
pub async fn list_servers(
    Extension(pool): Extension<PgPool>,
) -> Json<Vec<models::api::servers::Server>> {
    let now = OffsetDateTime::now_utc();
    let timeout = Duration::minutes(30);

    let servers: Vec<models::data::servers::Server> = query_as("SELECT * FROM servers;")
        .fetch_all(&pool)
        .await
        .unwrap();

    let servers = servers
        .into_iter()
        // TODO: Move to query / drop servers
        .filter(|server| now - server.last_ping <= timeout)
        .map(Into::into)
        .collect();

    Json(servers)
}

#[axum::debug_handler]
pub async fn register_server(
    Extension(pool): Extension<PgPool>,
    // TODO: Verify addr
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<models::api::servers::RegisterServer>,
) -> Json<models::api::servers::Server> {
    let port: i32 = payload.port.try_into().expect("Invalid port");

    let server: models::data::servers::Server = query_as(
        "INSERT INTO servers (name, addr, port, last_ping) VALUES ($1, $2, $3, now()) RETURNING *;",
    )
    .bind(payload.name)
    .bind(payload.addr)
    .bind(port)
    .fetch_one(&pool)
    .await
    .unwrap();

    Json(server.into())
}

#[axum::debug_handler]
pub async fn ping_server(
    Extension(pool): Extension<PgPool>,
    Path(id): Path<Uuid>,
    // TODO: Verify addr
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
) -> Json<models::api::servers::Server> {
    let server: models::data::servers::Server =
        query_as("UPDATE servers SET last_ping = now() WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();

    Json(server.into())
}
