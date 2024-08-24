use time::OffsetDateTime;
use uuid::Uuid;

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct MessageRow {
    pub user_id: Uuid,
    pub content: String,
    #[serde(with = "time::serde::rfc3339")]
    pub sent_at: OffsetDateTime,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct UserRow {
    pub client_id: i32,
    pub user_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub last_ping: OffsetDateTime,
}
