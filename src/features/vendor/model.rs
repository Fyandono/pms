use serde::{Serialize, Deserialize};

// --- Vendor Structure ---
// #[derive(Serialize, Deserialize, Debug)]
// pub struct Vendor {
//     pub id: u32,
//     pub name: String,
//     pub address: String,
//     pub email: String,
//     pub phone_number: String,
//     pub created_at: String,
// }

// --- Project Structure ---
#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub id: u32,
    pub vendor_id: u32, 
    pub name: String,
    pub description: String,
    pub pic_email: String,
    pub pic_number: String,
    pub pm_count: u32,
    pub created_at: String, 
}

// --- Project PM (Project Management) Structure ---
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectPM {
    pub id: u32,
    pub project_id: u32, 
    pub pm_order: u32,
    pub pm_description: String,
    pub url_file: String,
    pub is_verificated: bool,
    pub verificated_at: Option<String>, 
    pub created_at: String, 
}
