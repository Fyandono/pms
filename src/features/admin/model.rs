use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Deserialize)]
pub struct VendorQuery {
    pub name: Option<String>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Deserialize)]
pub struct ProjectQuery {
    pub vendor_id: i32,
    pub name: Option<String>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Deserialize)]
pub struct PMQuery {
    pub project_id: i32
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct Vendor {
    pub id: Option<i32>,
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone_number: String,
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct VendorDto {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone_number: String,
    pub created_at: String,
    pub updated_at: String,
    pub count_project: i64
}

// --- Project Structure ---
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct Project {
    pub id: Option<i32>,
    pub vendor_id: i32, 
    pub name: String,
    pub description: String,
    pub pic_name: Option<String>,
    pub pic_email: Option<String>,
    pub pic_number: Option<String>,
    pub pm_count: i32
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct ProjectDto {
    pub id: i32,
    pub vendor_id: i32, 
    pub name: String,
    pub description: String,
    pub pic_name: Option<String>,
    pub pic_email: Option<String>,
    pub pic_number: Option<String>,
    pub pm_count: i32,
    pub created_at: String, 
    pub updated_at: String,
    pub count_pm_uploaded: i64,
    pub count_pm_verified: i64,
    pub count_pm_unverified: i64
}

// --- Project PM (Preventive Maintenance) Structure ---
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct ProjectPMDto {
    pub id: i32,
    pub project_id: i32, 
    pub pm_order: i32,
    pub pm_description: String,
    pub url_file: String,
    pub is_verified: bool,
    pub verified_at: Option<String>, 
    pub created_at: String, 
}

// --- Project PM (Preventive Maintenance) Structure ---
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct VerifyPM {
    pub id: i32,
    pub is_verified: bool,
}