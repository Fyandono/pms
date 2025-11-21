use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub role: Option<String>, // optional: allow role assignment for demo
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id as string
    pub email: String,
    pub role: String,
    pub exp: usize,
}
