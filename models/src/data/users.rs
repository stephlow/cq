use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

impl User {
    pub fn verify_password(&self, password: &str) -> bool {
        verify(password, &self.password_hash).unwrap_or(false)
    }
}

pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

impl From<crate::api::users::NewUser> for NewUser {
    fn from(value: crate::api::users::NewUser) -> Self {
        let password_hash = hash(&value.password, DEFAULT_COST).unwrap();

        Self {
            username: value.username,
            email: value.email,
            password_hash,
        }
    }
}
