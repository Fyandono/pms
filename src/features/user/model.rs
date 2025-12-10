use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub username: String,
    pub password_hash: String,
    pub role_id: i32,
    pub role: String,
    pub is_active: bool
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub username: String,
    pub password: String,
    pub role_id: i32,
    pub is_active: bool
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct UserDto {
    pub id: String,
    pub name: String,
    pub username: String,
    pub role_id: i32,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub created_by: Option<String>,
    pub updated_at: Option<String>,
    pub updated_by: Option<String>
}

#[derive(sqlx::FromRow)]
pub struct UserLoginDto {
    pub id: String, // CAST(u.id AS TEXT)
    pub name: String,
    pub username: String,
    pub password_hash: String,
    pub role_id: i32, 
    pub role: String,
    pub is_active: bool,

    // Role Permissions added here:
    pub can_add_role: bool,
    pub can_edit_role: bool,
    pub can_add_user: bool,
    pub can_edit_user: bool,
    pub can_add_vendor: bool,
    pub can_edit_vendor: bool,
    pub can_add_project: bool,
    pub can_edit_project: bool,
    pub can_add_pm: bool,
    pub can_edit_pm: bool,
    pub can_verify_pm: bool,
    pub can_add_unit: bool,
    pub can_edit_unit: bool,

    pub can_get_user: bool,
    pub can_get_unit: bool,
    pub can_get_vendor: bool,
    pub can_get_role: bool,
    pub can_get_project: bool,
    pub can_get_pm: bool,
}
// -----------------------------------------------------------

#[derive(Deserialize)]
pub struct EditUserRequest {
    pub id: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub role_id: Option<i32>,
    pub is_active: Option<bool>, 
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub password: String,
    pub new_password: String
}


#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    // --- Standard Claims ---
    pub sub: String, 
    pub name: String,
    pub username: String,
    pub exp: usize,

    // --- Role and Identity Claims ---
    pub role_id: i32,
    pub role: String, 

    // --- Granular Permission Claims (11 fields) ---
    pub can_add_role: bool,
    pub can_edit_role: bool,
    pub can_add_user: bool,
    pub can_edit_user: bool,

    pub can_add_vendor: bool,
    pub can_edit_vendor: bool,
    pub can_add_project: bool,
    pub can_edit_project: bool,

    pub can_add_pm: bool,
    pub can_edit_pm: bool,
    pub can_verify_pm: bool,

    pub can_add_unit: bool,
    pub can_edit_unit: bool,

    pub can_get_vendor: bool,
    pub can_get_user: bool,
    pub can_get_unit: bool,
    pub can_get_role: bool,
    pub can_get_project: bool,
    pub can_get_pm: bool
    
}

#[derive(Deserialize)]
pub struct UserQuery {
    pub name: Option<String>,
    pub page: i32,
    pub page_size: i32,
}