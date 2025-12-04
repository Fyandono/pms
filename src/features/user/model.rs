use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub username: String,
    pub password: String,
    pub role: String,
    pub is_active: bool
}

#[derive(Deserialize)]
pub struct EditUserRequest {
    pub id: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>, 
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id as string
    pub name: String,
    pub username: String,
    pub role: String,
    pub exp: usize,
}
