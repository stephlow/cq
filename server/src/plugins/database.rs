use crate::TokioServerMessage;
use bevy::prelude::*;
use engine::resources::TokioRuntimeResource;
use sqlx::{migrate::MigrateDatabase, query, Pool, Sqlite, SqlitePool};

pub mod models;

#[derive(Default, Resource)]
pub struct SqliteServer {
    pub pool: Option<Pool<Sqlite>>,
}

#[derive(Default)]
pub struct DatabasePlugin;

impl Plugin for DatabasePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SqliteServer>()
            .add_systems(Startup, init_database);
    }
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

        let pool = SqlitePool::connect(DB_URL).await.unwrap();

        migrate(&pool).await;

        tx.send(TokioServerMessage::InitializePool(pool))
            .await
            .unwrap();
    });
}

async fn migrate(pool: &Pool<Sqlite>) {
    query("CREATE TABLE IF NOT EXISTS users (client_id INTEGER NOT NULL, user_id UUID UNIQUE NOT NULL, last_ping DATETIME NOT NULL);")
        .execute(pool)
        .await
        .unwrap();

    query("CREATE TABLE IF NOT EXISTS messages (user_id UUID NOT NULL, content TEXT NOT NULL, sent_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL);")
        .execute(pool)
        .await.unwrap();
}
