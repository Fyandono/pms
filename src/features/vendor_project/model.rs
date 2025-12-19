use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Deserialize)]
pub struct VendorQuery {
    pub name: Option<String>,
    pub page: i32,
    pub page_size: i32,
    pub is_report: Option<bool>
}

#[derive(Deserialize)]
pub struct ProjectQuery {
    pub vendor_id: i32,
    pub name: Option<String>,
    pub page: i32,
    pub page_size: i32,
    pub is_report: Option<bool>
}

#[derive(Deserialize)]
pub struct PMQuery {
    pub project_id: i32,
    pub description: Option<String>,
    pub project_start_date: Option<String>,
    pub project_end_date: Option<String>,
    pub completion_start_date: Option<String>,
    pub completion_end_date: Option<String>,
    pub pm_type: Option<String>,
    pub pm_status: Option<String>,
    pub page: i32,
    pub page_size: i32,
    pub is_report: Option<bool>
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct ReportQuery {
    pub list_vendor_id: Option<String>,
    pub project_start_date: Option<String>,
    pub project_end_date: Option<String>,
    pub completion_start_date: Option<String>,
    pub completion_end_date: Option<String>,
    pub pm_type: Option<String>,
    pub pm_status: Option<String>
}

#[derive(Deserialize)]
pub struct PMDetailQuery {
    pub pm_id: i32
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
    pub created_by: String,
    pub created_at: String, 
    pub updated_by: Option<String>,
    pub updated_at: Option<String>,
    pub count_project: i64
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct VendorDropdownDto {
    pub id: i32,
    pub name: String,
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
    pub pic_unit_id: Option<i32>,
    pub project_type: String,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct ProjectDto {
    pub id: i32,
    pub vendor_name: String, 
    pub name: String,
    pub description: String,
    pub pic_name: Option<String>,
    pub pic_email: Option<String>,
    pub pic_unit: Option<String>,
    pub pic_unit_id: Option<i32>,
    pub project_type: String,
    pub created_by: String,
    pub created_at: String, 
    pub updated_by: Option<String>,
    pub updated_at: Option<String>,
    pub count_pm_uploaded: i64,
    pub count_pm_verified: i64,
    pub count_pm_unverified: i64
}

#[derive(Debug, Default)]
pub struct ProjectPMData {
    pub project_id: Option<i32>,
    pub pm_description: Option<String>,
    pub pm_solution: Option<String>,
    pub pm_type: Option<String>,
    pub pm_project_date: Option<String>,
    pub pic_name: Option<String>,
    pub pic_email: Option<String>,
    pub pic_unit_id: Option<i32>,
}

// --- Project PM (Preventive Maintenance) Structure ---
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct ProjectPMDto {
    pub id: i32,
    pub project_id: i32, 
    pub pm_description: String,
    pub pm_solution: String,
    pub pm_type: String,
    pub pm_project_date: String,
    pub pm_completion_date: String,
    pub url_file: String,
    pub is_verified: Option<bool>,
    pub verified_at: Option<String>, 
    pub verified_by: Option<String>,
    pub note: Option<String>,
    pub pic_name: Option<String>,
    pub pic_email: Option<String>,
    pub pic_unit_id: Option<i32>,
    pub pic_unit: Option<String>,
    pub created_at: String, 
    pub created_by: String,
    pub updated_at: Option<String>,
    pub updated_by: Option<String>
}

// --- Project PM (Preventive Maintenance) Structure ---
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct ProjectPM {
    pub id: Option<i32>,
    pub project_id: i32, 
    pub pm_description: String,
    pub pm_solution: String,
    pub pm_type: String,
    pub pm_project_date: String,
    pub pic_name: Option<String>,
    pub pic_email: Option<String>,
    pub pic_unit_id: Option<i32>,
    pub url_file: String
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct VerifyPM {
    pub id: i32,
    pub is_verified: bool,
    pub pm_completion_date: Option<String>,
    pub note: Option<String>
}

#[derive(Debug, FromRow)]
pub struct FilePathResult {
    pub url_file: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteEntry {
    pub timestamp: String,
    pub user: String,
    pub note: Option<String>,
}

// --- Project PM (Preventive Maintenance) Structure ---
#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct PMReportDto {
    pub vendor_name: String,
    pub project_name: String,
    pub project_type: String,
    pub pm_task: String,
    pub pm_solution: String,
    pub pm_type: String,
    pub pic_name: Option<String>,
    pub pic_email: Option<String>,
    pub pic_unit: Option<String>,
    pub pm_project_date: String,
    pub pm_completion_date: Option<String>,
    pub status: Option<String>,
    pub pm_verified_at: Option<String>, 
    pub pm_verified_by: Option<String>,
    pub note: Option<String>,
    pub pm_created_at: String, 
    pub pm_created_by: String,
    pub pm_updated_at: Option<String>,
    pub pm_updated_by: Option<String>
}

// #[derive(Deserialize)]
// pub struct UsersVendorQuery {
//     pub name: Option<String>,
//     pub page: i32,
//     pub page_size: i32,
// }


// --- Users Vendor Structure ---
// #[derive(Serialize, Deserialize, Debug, FromRow)]
// pub struct UsersVendorDto {
//     pub user_id: String,
//     pub vendor_id: i32,
//     pub username: String,
//     pub vendor_name: String
// }

// #[derive(Serialize, Deserialize, Debug, FromRow)]
// pub struct UsersVendor {
//     pub user_id: String,
//     pub vendor_id: i32
// }