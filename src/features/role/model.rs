use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, Deserialize, Debug)]
pub struct Role {
    pub id: Option<i32>,
    pub name: String, 
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

    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct RoleDto {
    pub id: i32,
    pub name: String, 
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

    pub is_active: bool,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
    pub updated_at: Option<String>,
    pub updated_by: Option<String>
}

#[derive(Deserialize)]
pub struct RoleQuery {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub page: i32,
    pub page_size: i32,
}