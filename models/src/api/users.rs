use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
}

impl From<crate::data::users::User> for User {
    fn from(value: crate::data::users::User) -> Self {
        Self {
            id: value.id,
            username: value.username,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}
