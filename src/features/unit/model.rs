use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, Deserialize, Debug)]
pub struct Unit {
    pub id: Option<i32>,
    pub name: String, 
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct UnitDto {
    pub id: i32,
    pub name: String, 
    pub is_active: bool,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
    pub updated_at: Option<String>,
    pub updated_by: Option<String>
}

#[derive(Deserialize)]
pub struct UnitQuery {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub page: i32,
    pub page_size: i32,
}