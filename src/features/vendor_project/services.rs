use crate::{AppState, features::vendor_project::model::{PMReportDto, ReportQuery}};
use actix_web::{
    get,
    post,
    put,
    web::{Data, Query, Json, Path, Bytes},
    HttpResponse, Responder,
};
use actix_multipart::Multipart;
use serde_json::json;
use sqlx::{self};
use crate::features::user::services::{AuthClaims};
use crate::features::vendor_project::model::{ProjectQuery,
    Vendor, VendorDto, VendorQuery, Project, ProjectDto, 
    ProjectPMDto, PMQuery, VerifyPM, VendorDropdownDto,
    ProjectPM, ProjectPMData, FilePathResult, PMDetailQuery, NoteEntry};
use crate::util::page_response_builder::{page_response_builder, page_response_extra_builder};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use std::{borrow::Cow, collections::HashMap};

#[get("/vendor")]
pub async fn get_list_vendor(
    state: Data<AppState>,
    query_parameter: Query<VendorQuery>,
) -> impl Responder {

    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;
    match sqlx::query_as::<_, VendorDto>(
        "SELECT v.id,
                v.name,
                v.address,
                v.email,
                v.phone_number,
                c.username AS created_by,
                CAST(v.created_at AS CHAR) AS created_at,
                u.username AS updated_by,
                CAST(v.updated_at AS CHAR) AS updated_at,
                COUNT(p.id) AS count_project
            FROM vendor v
            LEFT JOIN project p ON (p.vendor_id = v.id)
            LEFT JOIN users c ON (c.id = v.created_by)
            LEFT JOIN users u ON (u.id = v.updated_by)
            WHERE v.name LIKE CONCAT('%',?, '%')
            GROUP BY v.id, c.username, u.username
            ORDER BY v.name;",
    )
    .bind(name_filter)
    .fetch_all(&state.db)
    .await
    {
        Ok(vendors) => {
            let response = page_response_builder(page, page_size, &vendors);
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
        }
    }
}

#[get("/all-vendor")]
pub async fn get_all_vendor(
    state: Data<AppState>,
) -> impl Responder {

    match sqlx::query_as::<_, VendorDropdownDto>(
        "SELECT v.id, v.name
        FROM vendor v
        ORDER BY v.name"
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(vendors) => {
            let response = json!({"data": vendors});
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
        }
    }
}

#[get("/project")]
pub async fn get_list_project(
    state: Data<AppState>,
    query_parameter: Query<ProjectQuery>,
) -> impl Responder {

    let vendor_id = query_parameter.vendor_id;
    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;

    let vendor_detail = match sqlx::query_as::<_, VendorDto>(
        "SELECT v.id,
                v.name,
                v.address,
                v.email,
                v.phone_number,
                c.username AS created_by,
                CAST(v.created_at AS CHAR) AS created_at,
                u.username AS updated_by,
                CAST(v.updated_at AS CHAR) AS updated_at,
                COUNT(p.id) AS count_project
            FROM vendor v
            LEFT JOIN project p ON (p.vendor_id = v.id)
            LEFT JOIN users c ON (c.id = v.created_by)
            LEFT JOIN users u ON (u.id = v.updated_by)
            WHERE v.id = ?
            GROUP BY v.id, c.username, u.username
            ORDER BY v.name;",
    )
    .bind(vendor_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "message": "No vendor found with specified ID." }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
        }
    };

    match sqlx::query_as::<_, ProjectDto>(
        "WITH data_pm_verificated AS (
                    SELECT COUNT(id) AS count_pm_verified, project_id
                    FROM project_pm
                    WHERE is_verified
                    GROUP BY project_id
                )
                SELECT p.id,
                    p.name,
                    v.name AS vendor_name,
                    p.description,
                    p.pic_name,
                    p.pic_email,
                    un.name AS pic_unit,
                    p.pic_unit_id,
                    p.project_type,
                    c.username AS created_by,
                    CAST(p.created_at AS CHAR) AS created_at,
                    u.username AS updated_by,
                    CAST(p.updated_at AS CHAR) AS updated_at,
                    COALESCE(COUNT(pm.id), 0) AS count_pm_uploaded,
                    COALESCE(dpmv.count_pm_verified, 0) AS count_pm_verified,
                    COALESCE(COUNT(pm.id), 0) - COALESCE(dpmv.count_pm_verified, 0) AS count_pm_unverified
                FROM project p
                LEFT JOIN project_pm pm ON (pm.project_id = p.id)
                LEFT JOIN data_pm_verificated dpmv ON (dpmv.project_id = p.id)
                LEFT JOIN users c ON (c.id = p.created_by)
                LEFT JOIN users u ON (u.id = p.updated_by)
                LEFT JOIN unit un ON (un.id = p.pic_unit_id)
                LEFT JOIN vendor v on (v.id = p.vendor_id)
                WHERE p.vendor_id = ? AND p.name LIKE CONCAT('%', ?, '%')
                GROUP BY p.id, dpmv.count_pm_verified, un.name, c.username, u.username, v.name;",
            )
                .bind(vendor_id)
                .bind(name_filter)
                .fetch_all(&state.db)
                .await
            {
                Ok(vendors) => {
                    let response = page_response_extra_builder(page, 
                        page_size, 
                        &vendors, 
                        json!({"vendor": vendor_detail}));
                    HttpResponse::Ok().json(response)
                }
                Err(error) => {
                    HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
                }
            }
}

#[get("/pm")]
pub async fn get_list_pm(
    state: Data<AppState>,
    query_parameter: Query<PMQuery>,
) -> impl Responder {

    let project_id = query_parameter.project_id;
    let description = query_parameter.description.clone();
    let project_start_date = query_parameter.project_start_date.clone();
    let project_end_date = query_parameter.project_end_date.clone();
    let completion_start_date = query_parameter.completion_start_date.clone();
    let completion_end_date = query_parameter.completion_end_date.clone();
    let pm_type = query_parameter.pm_type.clone();
    let pm_status = query_parameter.pm_status.clone();
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;

    let project_detail = match sqlx::query_as::<_, ProjectDto>(
        "WITH data_pm_verificated AS (
                    SELECT COUNT(id) AS count_pm_verified, project_id
                    FROM project_pm
                    WHERE is_verified
                    GROUP BY project_id
                )
                SELECT p.id,
                    v.name AS vendor_name,
                    p.name,
                    p.description,
                    p.pic_name,
                    p.pic_email,
                    p.pic_unit_id,
                    un.name AS pic_unit,
                    p.project_type,
                    c.username AS created_by,
                    CAST(p.created_at AS CHAR) AS created_at,
                    u.username AS updated_by,
                    CAST(p.updated_at AS CHAR) AS updated_at,
                    COALESCE(COUNT(pm.id), 0) AS count_pm_uploaded,
                    COALESCE(dpmv.count_pm_verified, 0) AS count_pm_verified,
                    COALESCE(COUNT(pm.id), 0) - COALESCE(dpmv.count_pm_verified, 0) AS count_pm_unverified
                FROM project p
                LEFT JOIN project_pm pm ON (pm.project_id = p.id)
                LEFT JOIN data_pm_verificated dpmv ON (dpmv.project_id = p.id)
                LEFT JOIN users c ON (c.id = p.created_by)
                LEFT JOIN users u ON (u.id = p.updated_by)
                LEFT JOIN unit un ON (un.id = p.pic_unit_id)
                LEFT JOIN vendor v ON (v.id = p.vendor_id)
                WHERE p.id = ?
                GROUP BY p.id, dpmv.count_pm_verified, un.name, c.username, u.username, v.name;",
    )
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "message": "No project found with specified ID." }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
        }
    };

    match sqlx::query_as::<_, ProjectPMDto>(
        "SELECT a.id,
                a.project_id,
                a.pm_description,
                a.pm_solution,
                a.pm_type,
                CAST(a.pm_project_date AS CHAR) AS pm_project_date,
                a.url_file,
                a.is_verified,
                v.username AS verified_by,
                CAST(a.verified_at AS CHAR) as verified_at,
                c.username AS created_by,
                CAST(a.created_at AS CHAR) AS created_at,
                up.username AS updated_by,
                CAST(a.updated_at AS CHAR) AS updated_at,
                a.pic_name,
                a.pic_email,
                a.pic_unit_id,
                u.name AS pic_unit,
                CAST(a.pm_completion_date AS CHAR) AS pm_completion_date,
                a.note
            FROM project_pm a
            LEFT JOIN users c ON (c.id = a.created_by)
            LEFT JOIN users v ON (v.id = a.verified_by)
            LEFT JOIN users up ON (up.id = a.updated_by)
            LEFT JOIN unit u ON (u.id = a.pic_unit_id)
            WHERE a.project_id = ? 
            AND (? IS NULL OR a.pm_description LIKE CONCAT('%', ?, '%'))
            AND (? IS NULL OR a.pm_type = ?)
            AND (? IS NULL OR (
                            (? = 'On Progress' AND a.is_verified IS NULL) OR
                            (? = 'Verified' AND a.is_verified = TRUE) OR
                            (? = 'Need Revision' AND a.is_verified = FALSE)
                        ))
            AND (? IS NULL OR a.pm_project_date >= CAST(? AS DATE))
            AND (? IS NULL OR a.pm_project_date <= CAST(? AS DATE))
            AND (? IS NULL OR a.pm_completion_date IS NULL OR a.pm_completion_date >= CAST(? AS DATE))
            AND (? IS NULL OR a.pm_completion_date IS NULL OR a.pm_completion_date <= CAST(? AS DATE))
            ORDER BY a.created_at DESC;
            ",
        )
            .bind(project_id)
            .bind(description.clone())
            .bind(description)
            .bind(&pm_type)
            .bind(&pm_type)
            .bind(&pm_status)
            .bind(&pm_status)
            .bind(&pm_status)
            .bind(&pm_status)
            .bind(project_start_date.clone())
            .bind(project_start_date)
            .bind(project_end_date.clone())
            .bind(project_end_date)
            .bind(completion_start_date.clone())
            .bind(completion_start_date)
            .bind(completion_end_date.clone())
            .bind(completion_end_date)
            .fetch_all(&state.db)
            .await
        {
            Ok(pms) => {
                let response = 
                page_response_extra_builder(page, page_size, &pms, json!({"project": project_detail}));
                HttpResponse::Ok().json(response)
            }
            Err(error) => {
                HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
            }
        }
}

#[post("/vendor")]
pub async fn post_create_vendor(
    state: Data<AppState>,
    body: Json<Vendor>,
    claims: AuthClaims
) -> impl Responder {

    let user_id = claims.0.sub.to_string();

    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({ 
                "message": format!("Failed to start transaction: {}", e) 
            }))
        }
    };

    let result = sqlx::query(
        "INSERT INTO vendor (name, address, email, phone_number, created_by, created_at) 
         VALUES (?, ?, ?, ?, ?, NOW())",
    )
    .bind(&body.name)
    .bind(&body.address)
    .bind(&body.email)
    .bind(&body.phone_number)
    .bind(user_id)
    .execute(&mut *transaction)
    .await;

    if let Err(error) = result {
        let _ = transaction.rollback().await; 
        return HttpResponse::InternalServerError().json(json!({ 
            "message": format!("Failed to create vendor: {}", error) 
        }));
    }

    let vendor_result = sqlx::query_as::<_, Vendor>(
        "SELECT id, name, address, email, phone_number FROM vendor WHERE name = ? ORDER BY id DESC LIMIT 1"
    )
    .bind(&body.name)
    .fetch_optional(&mut *transaction)
    .await;

    let vendor = match vendor_result {
        Ok(Some(v)) => v,
        _ => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().json(json!({ 
                "message": "Failed to retrieve newly created vendor." 
            }));
        }
    };
    
    match transaction.commit().await {
        Ok(_) => {
            HttpResponse::Created().json(json!({
                "message": format!("Vendor '{}' successfully created.", vendor.name),
                "vendor": vendor,
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(json!({ 
                "message": format!("Failed to commit transaction: {}", e) 
            }))
        }
    }
}

#[put("/vendor")]
pub async fn put_edit_vendor(
    state: Data<AppState>,
    body: Json<Vendor>,
    claims: AuthClaims
) -> impl Responder {

    let user_id = claims.0.sub.to_string();

    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({ 
                "message": format!("Failed to start transaction: {}", e) 
            }))
        }
    };

    let result = sqlx::query(
        "UPDATE vendor 
         SET name = ?,
             address = ?,
             email = ?,
             phone_number = ?,
             updated_at = NOW(),
             updated_by = ?
         WHERE id = ?"
    )
    .bind(&body.name)
    .bind(&body.address)
    .bind(&body.email)
    .bind(&body.phone_number)
    .bind(user_id)
    .bind(&body.id)
    .execute(&mut *transaction)
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                let _ = transaction.rollback().await;
                return HttpResponse::NotFound().json(json!({ 
                    "message": "Vendor not found or no changes were made."
                }));
            }
            
            let vendor_result = sqlx::query_as::<_, Vendor>(
                "SELECT id, name, address, email, phone_number FROM vendor WHERE id = ?"
            )
            .bind(&body.id)
            .fetch_optional(&mut *transaction)
            .await;

            let vendor = match vendor_result {
                Ok(Some(v)) => v,
                _ => {
                    let _ = transaction.rollback().await;
                    return HttpResponse::InternalServerError().json(json!({ 
                        "message": "Failed to retrieve updated vendor record." 
                    }))
                }
            };
            
            match transaction.commit().await {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": format!("Vendor '{}' successfully updated.", vendor.name),
                    "vendor": vendor,
                })),
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({ 
                        "message": format!("Failed to commit transaction: {}", e) 
                    }))
                },
            }
        }
        Err(error) => {
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({ 
                "message": format!("Failed to update vendor: {}", error) 
            }))
        }
    }
}

#[post("/project")]
pub async fn post_create_vendor_project(
    state: Data<AppState>,
    body: Json<Project>,
    claims: AuthClaims
) -> impl Responder {
    let user_id = claims.0.sub.to_string();

    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({ 
                "message": format!("Failed to start transaction: {}", e) 
            }))
        }
    };
    
    let result = sqlx::query(
        "INSERT INTO project 
            (vendor_id, name, description, pic_name, pic_email, pic_unit_id, project_type, created_by, created_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, NOW())",
    )
    .bind(body.vendor_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.pic_name)
    .bind(&body.pic_email)
    .bind(&body.pic_unit_id)
    .bind(&body.project_type)
    .bind(user_id)
    .execute(&mut *transaction)
    .await;

    match result {
        Ok(_) => {
            let project_result = sqlx::query_as::<_, Project>(
                "SELECT id, vendor_id, name, description, pic_name, pic_email, pic_unit_id, project_type, created_by FROM project WHERE name = ? AND vendor_id = ? ORDER BY id DESC LIMIT 1"
            )
            .bind(&body.name)
            .bind(body.vendor_id)
            .fetch_optional(&mut *transaction)
            .await;

            let project = match project_result {
                Ok(Some(p)) => p,
                _ => {
                    let _ = transaction.rollback().await;
                    return HttpResponse::InternalServerError().json(json!({ 
                        "message": "Failed to retrieve newly created project." 
                    }))
                }
            };
            
            match transaction.commit().await {
                Ok(_) => {
                    HttpResponse::Created().json(json!({
                        "message": format!("Project '{}' successfully created for vendor {}.", project.name, project.vendor_id),
                        "project": project,
                    }))
                }
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({ 
                        "message": format!("Failed to commit transaction: {}", e) 
                    }))
                }
            }
        }
        Err(error) => {
            let _ = transaction.rollback().await; 
            HttpResponse::InternalServerError().json(json!({ 
                "message": format!("Failed to create project: {}", error) 
            }))
        }
    }
}

#[put("/project")]
pub async fn put_edit_vendor_project(
    state: Data<AppState>,
    body: Json<Project>,
    claims: AuthClaims
) -> impl Responder {
    let user_id = claims.0.sub.to_string();

    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    let result = sqlx::query(
        "UPDATE project
         SET vendor_id   = ?,
             name        = ?,
             description = ?,
             pic_name    = ?,
             pic_email   = ?,
             pic_unit_id = ?,
             project_type = ?,
             updated_at  = NOW(),
             updated_by  = ?
         WHERE id = ?"
    )
    .bind(&body.vendor_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.pic_name)
    .bind(&body.pic_email)
    .bind(&body.pic_unit_id)
    .bind(&body.project_type)
    .bind(user_id)
    .bind(&body.id)
    .execute(&mut *transaction)
    .await;

    match result {
        Ok(res) => {
             if res.rows_affected() == 0 {
                let _ = transaction.rollback().await;
                return HttpResponse::NotFound().json(json!({ 
                    "message": "Project not found or no changes were made."
                }));
            }
            
            let project_result = sqlx::query_as::<_, Project>(
                "SELECT id, vendor_id, name, description, pic_name, pic_email, pic_unit_id, project_type FROM project WHERE id = ?"
            )
            .bind(&body.id)
            .fetch_optional(&mut *transaction)
            .await;

            let project = match project_result {
                Ok(Some(p)) => p,
                _ => {
                    let _ = transaction.rollback().await;
                    return HttpResponse::InternalServerError().json(json!({ 
                        "message": "Failed to retrieve updated project record." 
                    }))
                }
            };

            match transaction.commit().await {
                Ok(_) => {
                    HttpResponse::Ok().json(json!({
                        "message": format!("Project '{}' successfully updated.", project.name),
                        "project": project,
                    }))
                }
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({ 
                        "message": format!("Failed to commit transaction: {}", e) 
                    }))
                }
            }
        }
        Err(error) => {
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to update project: {}", error)
            }))
        }
    }
}

#[put("/verify")]
pub async fn put_edit_verify_pm(
    state: Data<AppState>,
    body: Json<VerifyPM>,
    claims: AuthClaims
) -> impl Responder {
    
    let user_id = &claims.0.sub;
    let username = &claims.0.username; 
    let pool = &state.db;

    let mut transaction = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    let row_exists: i8 = match sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM project_pm WHERE id = ?)")
        .bind(&body.id)
        .fetch_one(&mut *transaction)
        .await
    {
        Ok(exists) => exists,
        Err(e) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().json(json!({ "message": format!("Database error during row check: {}", e) }));
        }
    };

    if row_exists == 0 {
        let _ = transaction.rollback().await;
        return HttpResponse::NotFound().json(json!({ "message": "Project PM record not found." }));
    }

    let notes_json_string: Option<String> = match sqlx::query_scalar::<_, Option<String>>(
        "SELECT CAST(note AS CHAR) FROM project_pm WHERE id = ?"
    )
    .bind(&body.id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(s) => s.flatten(),
        Err(e) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().json(json!({ 
                "message": format!("Failed to fetch notes history string: {}", e) 
            }));
        }
    };
    
    let mut notes_history: Vec<NoteEntry> = match notes_json_string {
        Some(s) => {
            match serde_json::from_str(&s) {
                Ok(notes) => notes,
                Err(e) => {
                    eprintln!("Warning: Corrupt JSON data found in note column for ID {}. Error: {}", body.id, e);
                    Vec::new() 
                }
            }
        },
        None => Vec::new(), 
    };

    let is_note_valid = body.note.as_ref() 
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false); 
    
    if is_note_valid {
        let current_time = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.f%z")
        .to_string();
    
        let new_entry = NoteEntry {
            timestamp: current_time,
            user: username.clone(),
            note: body.note.as_ref().map(|s| s.trim().to_string()),
        };
        notes_history.push(new_entry);
    }
    
    let updated_notes_json_string = match serde_json::to_string(&notes_history) {
        Ok(s) => s,
        Err(e) => {
             let _ = transaction.rollback().await;
             return HttpResponse::InternalServerError().json(json!({ 
                 "message": format!("Failed to serialize notes history: {}", e) 
             }));
        }
    };

    match sqlx::query(
        "UPDATE project_pm
         SET is_verified = ?,
             note = ?,
             pm_completion_date = ?, 
             verified_at = NOW(),
             verified_by = ?
         WHERE id = ?" 
    )
    .bind(&body.is_verified)
    .bind(updated_notes_json_string) 
    .bind(&body.pm_completion_date) 
    .bind(user_id)
    .bind(&body.id)
    .execute(&mut *transaction)
    .await
    {
        Ok(_) => {
            match transaction.commit().await {
                Ok(_) => {
                    HttpResponse::Ok().json(json!({
                        "message": format!("PM ID '{}' successfully updated.", body.id),
                    }))
                }
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({
                        "message": format!("Failed to commit transaction: {}", e)
                    }))
                }
            }
        }
        Err(error) => {
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to update project: {}", error)
            }))
        }
    }
}

#[post("/pm")]
pub async fn post_create_project_pm(
    state: Data<AppState>,
    mut payload: Multipart,
    claims: AuthClaims,
) -> impl Responder {
    
    let user_id = claims.0.sub.to_string();

    let mut data = ProjectPMData::default();
    let mut file_path: Option<String> = None;

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => return HttpResponse::InternalServerError().json(json!({"message": format!("Multipart processing error: {}", e)})),
        };

        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            
            let filename = field.content_disposition()
                .as_ref()
                .and_then(|cd| cd.get_filename())
                .map(|s| s.to_owned())
                .unwrap_or_else(|| format!("upload-{}", Uuid::new_v4()));

            let extension = filename.split('.').last().unwrap_or("dat");
            let base_name: String = filename.rsplit_once('.').map(|(base, _)| base.to_owned()).unwrap_or(filename.clone());

            let unique_code = Uuid::new_v4().to_string();
            let new_filename = format!("{}_{}.{}", base_name, unique_code, extension);

            let filepath = format!("./data/{}", new_filename);
            file_path = Some(filepath.clone());
            
            
            match File::create(&filepath).await {
                Ok(mut f) => {
                    while let Some(chunk) = field.next().await {
                        if let Ok(chunk) = chunk {
                            if let Err(e) = f.write_all(&chunk).await {
                                let _ = tokio::fs::remove_file(&filepath).await;
                                return HttpResponse::InternalServerError().json(json!({"message": format!("Failed to write file to disk: {}", e)}));
                            }
                        }
                    }
                }
                Err(e) => return HttpResponse::InternalServerError().json(json!({"message": format!("Failed to create file on disk: {}", e)})),
            }
        } else if !field_name.is_empty() {
            
            let bytes = match field.next().await {
                Some(Ok(b)) => b,
                _ => continue,
            };
            let value = String::from_utf8(bytes.to_vec()).unwrap_or_default();
            
            match field_name.as_str() {
                "project_id" => data.project_id = value.parse::<i32>().ok(),
                "pm_description" => data.pm_description = Some(value),
                "pm_solution" => data.pm_solution = Some(value),
                "pm_type" => data.pm_type = Some(value),
                "pm_project_date" => data.pm_project_date = Some(value),
                
                "pic_name" => data.pic_name = Some(value),
                "pic_email" => data.pic_email = Some(value),
                "pic_unit_id" => data.pic_unit_id = value.parse::<i32>().ok(),
                _ => {}
            }
        }
    }

    
    
    let cleanup = |path: Option<String>| async move {
        if let Some(p) = path {
            let _ = tokio::fs::remove_file(p).await;
        }
    };

    let project_id = match data.project_id {
        Some(pid) => pid,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing or invalid project_id."}));
        }
    };

    let pm_description = match data.pm_description {
        Some(desc) => desc,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing pm_description."}));
        }
    };

    let pm_solution = match data.pm_solution {
        Some(sol) => sol,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing pm_solution."}));
        }
    };

    let pm_type = match data.pm_type {
        Some(val) => val,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing pm_solution."}));
        }
    };

    
    let pm_project_date = match data.pm_project_date {
        Some(val) => val,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing pm_project_date."}));
        }
    };

    let url_file = match file_path {
        Some(path) => path,
        None => {
            return HttpResponse::BadRequest().json(json!({"message": "Missing file upload."}));
        }
    };

    let pic_name = data.pic_name;
    let pic_email = data.pic_email;
    let pic_unit_id = data.pic_unit_id;

    

    
    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            cleanup(Some(url_file.clone())).await;
            return HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    
    let result = sqlx::query(
        "INSERT INTO project_pm (project_id, pm_description, pm_solution, pm_type, pm_project_date, url_file, pic_name, pic_email, pic_unit_id, created_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW())"
    )
    .bind(&project_id)
    .bind(&pm_description)
    .bind(&pm_solution)
    .bind(&pm_type)
    .bind(&pm_project_date)
    .bind(&url_file)
    .bind(&pic_name)
    .bind(&pic_email)
    .bind(&pic_unit_id)
    .bind(user_id)
    .execute(&mut *transaction)
    .await;

    match result {
        Ok(_) => {
            
            let pm_result = sqlx::query_as::<_, ProjectPM>(
                "SELECT id, project_id, pm_description, pm_solution, pm_type, CAST(pm_project_date AS CHAR) AS pm_project_date, url_file, pic_name, pic_email, pic_unit_id FROM project_pm WHERE project_id = ? ORDER BY id DESC LIMIT 1"
            )
            .bind(project_id)
            .fetch_optional(&mut *transaction)
            .await;

            let pm = match pm_result {
                Ok(Some(p)) => p,
                _ => {
                    let _ = transaction.rollback().await;
                    cleanup(Some(url_file)).await;
                    return HttpResponse::InternalServerError().json(json!({ 
                        "message": "Failed to retrieve newly created PM record." 
                    }))
                }
            };

            
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": format!("PM added to project {}.", pm.project_id),
                    "project_pm": pm,
                })),
                Err(e) => {
                    cleanup(Some(url_file)).await;
                    HttpResponse::InternalServerError().json(json!({
                        "message": format!("Failed to commit transaction: {}", e)
                    }))
                }
            }
        }
        Err(error) => {
            
            let _ = transaction.rollback().await;
            cleanup(Some(url_file)).await;
            HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to add project PM step: {}", error)
            }))
        }
    }
}

#[get("/files/{project_pm_id}")]
pub async fn get_project_pm_file(
    state: Data<AppState>,
    path: Path<i32>,
) -> impl Responder {
    let project_pm_id = path.into_inner();
    
    
    let result = match sqlx::query_as::<_, FilePathResult>(
        "SELECT url_file FROM project_pm WHERE id = ?"
    )
    .bind(project_pm_id)
    .fetch_optional(&state.db)
    .await {
        Ok(Some(r)) => r, 
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({"message": format!("Project PM with ID {} not found.", project_pm_id)}))
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({"message": format!("Database error fetching file path: {}", e)}))
        },
    };

    
    let file_path = result.url_file; 

    
    let file = match File::open(&file_path).await {
        Ok(f) => f, 
        Err(_) => {
            return HttpResponse::NotFound().json(json!({"message": format!("File not found on server at path: {}", file_path)}))
        },
    };
    
    
    let mime_type = mime_guess::from_path(&file_path)
        .first_or_text_plain();

    
    let raw_stream = FramedRead::new(file, BytesCodec::new());

    
    let stream = raw_stream.map_ok(|bytes_mut| {
        Bytes::from(bytes_mut.freeze())
    });

    let filename_for_download: Cow<str> = file_path.split('/')
        .last()
        .map_or("download".into(), |s| s.into());

    
    HttpResponse::Ok()
        .content_type(mime_type.as_ref())
        .append_header(
            actix_web::http::header::ContentDisposition::attachment(filename_for_download),
        )
        .streaming(stream)
}

#[put("/pm")] pub async fn put_edit_project_pm(
    state: Data<AppState>,
    mut payload: Multipart,
    claims: AuthClaims,
) -> impl Responder {
    
    let user_id = &claims.0.sub;

    let mut data_updates: HashMap<String, String> = HashMap::new();
    let mut new_file_path: Option<String> = None;

    let cleanup = |path: Option<String>| async move {
        if let Some(p) = path {
            let _ = tokio::fs::remove_file(p).await;
        }
    };

    
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => {
                cleanup(new_file_path.clone()).await;
                return HttpResponse::InternalServerError().json(json!({"message": format!("Multipart processing error: {}", e)}))
            }
        };

        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" {
            

            let filename = field.content_disposition()
                .as_ref()
                .and_then(|cd| cd.get_filename())
                .map(|s| s.to_owned())
                .unwrap_or_else(|| format!("upload-{}", Uuid::new_v4()));

            let extension = filename.split('.').last().unwrap_or("dat");
            let base_name: String = filename.rsplit_once('.').map(|(base, _)| base.to_owned()).unwrap_or(filename.clone());

            let unique_code = Uuid::new_v4().to_string();
            let new_filename = format!("{}_{}.{}", base_name, unique_code, extension);

            let filepath = format!("./data/{}", new_filename);
            new_file_path = Some(filepath.clone());

            
            match File::create(&filepath).await {
                Ok(mut f) => {
                    while let Some(chunk) = field.next().await {
                        if let Ok(chunk) = chunk {
                            if let Err(e) = f.write_all(&chunk).await {
                                let _ = cleanup(Some(filepath)).await;
                                return HttpResponse::InternalServerError().json(json!({"message": format!("Failed to write new file to disk: {}", e)}));
                            }
                        }
                    }
                }
                Err(e) => return HttpResponse::InternalServerError().json(json!({"message": format!("Failed to create new file on disk: {}", e)})),
            }
        } else if !field_name.is_empty() {
            
            let bytes = match field.next().await {
                Some(Ok(b)) => b,
                _ => continue,
            };
            let value = String::from_utf8(bytes.to_vec()).unwrap_or_default();
            data_updates.insert(field_name, value);
        }
    }
    
    
    let pm_id = match data_updates.remove("id").and_then(|s| s.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing or invalid mandatory field 'id'."}));
        }
    };
    
    let project_id = match data_updates.remove("project_id").and_then(|s| s.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing or invalid mandatory field 'project_id'."}));
        }
    };
    
    let pm_description = match data_updates.remove("pm_description") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing mandatory field 'pm_description'."}));
        }
    };
    
    let pm_solution = match data_updates.remove("pm_solution") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing mandatory field 'pm_solution'."}));
        }
    };
    
    let pm_type = match data_updates.remove("pm_type") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing mandatory field 'pm_type'."}));
        }
    };

    let pm_project_date = match data_updates.remove("pm_project_date") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"message": "Missing mandatory field 'pm_project_date'."}));
        }
    };
    
    
    let pic_name_opt = data_updates.remove("pic_name");
    let pic_email_opt = data_updates.remove("pic_email");
    let pic_unit_id_opt: Option<i32> = data_updates.remove("pic_unit_id").and_then(|s| s.parse().ok());


    
    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            cleanup(new_file_path).await;
            return HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    
    let old_pm_record = match sqlx::query_as::<_, ProjectPM>("SELECT id, project_id, pm_description, pm_solution, pm_type, CAST(pm_project_date AS CHAR) AS pm_project_date, url_file, pic_name, pic_email, pic_unit_id FROM project_pm WHERE id = ?")
        .bind(pm_id)
        .fetch_optional(&mut *transaction)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => {
            let _ = transaction.rollback().await;
            cleanup(new_file_path).await;
            return HttpResponse::NotFound().json(json!({"message": format!("Project PM with id {} not found.", pm_id)}));
        }
        Err(e) => {
            let _ = transaction.rollback().await;
            cleanup(new_file_path).await;
            return HttpResponse::InternalServerError().json(json!({"message": format!("Failed to fetch existing project PM: {}", e)}));
        }
    };
    
    
    let old_file_path_to_delete = if new_file_path.is_some() {
        Some(old_pm_record.url_file.clone()) 
    } else {
        None 
    };
    
    
    let url_file_to_save = new_file_path.unwrap_or(old_pm_record.url_file);

    
    let query_string = "
        UPDATE project_pm SET 
            project_id = ?,
            pm_description = ?,
            pm_solution = ?,
            pm_type = ?,
            pm_project_date = ?,
            url_file = ?,
            pic_name = ?,
            pic_email = ?,
            pic_unit_id = ?,
            updated_by = ?,
            updated_at = NOW()
        WHERE id = ?
    ";

    
    let result = sqlx::query(query_string)
        .bind(project_id) 
        .bind(pm_description) 
        .bind(pm_solution) 
        .bind(pm_type) 
        .bind(pm_project_date) 
        .bind(&url_file_to_save) 
        .bind(pic_name_opt) 
        .bind(pic_email_opt) 
        .bind(pic_unit_id_opt) 
        .bind(user_id)
        .bind(pm_id) 
        .execute(&mut *transaction)
        .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                let _ = transaction.rollback().await;
                cleanup(Some(url_file_to_save.clone())).await;
                return HttpResponse::NotFound().json(json!({ 
                    "message": "Project PM not found or no changes were made."
                }));
            }

            
            let pm_result = sqlx::query_as::<_, ProjectPM>("SELECT id, project_id, pm_description, pm_solution, pm_type, CAST(pm_project_date AS CHAR) AS pm_project_date, url_file, pic_name, pic_email, pic_unit_id FROM project_pm WHERE id = ?")
                .bind(pm_id)
                .fetch_one(&mut *transaction)
                .await;

            let pm = match pm_result {
                Ok(p) => p,
                Err(e) => {
                    let _ = transaction.rollback().await;
                    cleanup(Some(url_file_to_save)).await;
                    return HttpResponse::InternalServerError().json(json!({"message": format!("Failed to fetch updated record: {}", e)}));
                }
            };
            
            
            match transaction.commit().await {
                Ok(_) => {
                    
                    if let Some(old_path) = old_file_path_to_delete {
                        let _ = cleanup(Some(old_path)).await;
                    }

                    HttpResponse::Ok().json(json!({
                        "message": "Project PM updated successfully.".to_string(),
                        "project_pm": pm,
                    }))
                }
                Err(e) => {
                    
                    cleanup(Some(url_file_to_save)).await;
                    HttpResponse::InternalServerError().json(json!({
                        "message": format!("Failed to commit transaction: {}", e)
                    }))
                }
            }
        }
        Err(error) => {
            
            let _ = transaction.rollback().await;
            cleanup(Some(url_file_to_save)).await; 
            HttpResponse::InternalServerError().json(json!({
                "message": format!("Failed to update project PM step: {}", error)
            }))
        }
    }
}

#[get("/pm-detail")]
pub async fn get_detail_pm(
    state: Data<AppState>,
    query_parameter: Query<PMDetailQuery>,
) -> impl Responder {

    let pm_id = query_parameter.pm_id;

    
    let project_detail = match sqlx::query_as::<_, ProjectDto>(
        "WITH data_pm_verificated AS (
                    SELECT COUNT(id) AS count_pm_verified, project_id
                    FROM project_pm
                    WHERE is_verified
                    GROUP BY project_id
                )
                SELECT p.id,
                    v.name AS vendor_name,
                    p.name,
                    p.description,
                    p.pic_name,
                    p.pic_email,
                    p.pic_unit_id,
                    un.name AS pic_unit,
                    p.project_type,
                    c.username AS created_by,
                    CAST(p.created_at AS CHAR) AS created_at,
                    u.username AS updated_by,
                    CAST(p.updated_at AS CHAR) AS updated_at,
                    COALESCE(COUNT(pm.id), 0) AS count_pm_uploaded,
                    COALESCE(dpmv.count_pm_verified, 0) AS count_pm_verified,
                    COALESCE(COUNT(pm.id), 0) - COALESCE(dpmv.count_pm_verified, 0) AS count_pm_unverified
                FROM project p
                LEFT JOIN project_pm pm ON (pm.project_id = p.id)
                LEFT JOIN data_pm_verificated dpmv ON (dpmv.project_id = p.id)
                LEFT JOIN users c ON (c.id = p.created_by)
                LEFT JOIN users u ON (u.id = p.updated_by)
                LEFT JOIN unit un ON (un.id = p.pic_unit_id)
                LEFT JOIN vendor v ON (v.id = p.vendor_id)
                WHERE pm.id = ?
                GROUP BY p.id, dpmv.count_pm_verified, un.name, c.username, u.username, v.name;",
    )
    .bind(pm_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "message": "No project found with specified PM ID." }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
        }
    };

    match sqlx::query_as::<_, ProjectPMDto>(
        "SELECT a.id,
                a.project_id,
                a.pm_description,
                a.pm_solution,
                a.pm_type,
                CAST(a.pm_project_date AS CHAR) AS pm_project_date,
                a.url_file,
                a.is_verified,
                v.username AS verified_by,
                CAST(a.verified_at AS CHAR) as verified_at,
                c.username AS created_by,
                CAST(a.created_at AS CHAR) AS created_at,
                up.username AS updated_by,
                CAST(a.updated_at AS CHAR) AS updated_at,
                a.pic_name,
                a.pic_email,
                a.pic_unit_id,
                u.name AS pic_unit,
                CAST(a.pm_completion_date AS CHAR) AS pm_completion_date,
                a.note
            FROM project_pm a
            LEFT JOIN users c ON (c.id = a.created_by)
            LEFT JOIN users v ON (v.id = a.verified_by)
            LEFT JOIN users up ON (up.id = a.updated_by)
            LEFT JOIN unit u ON (u.id = a.pic_unit_id)
            WHERE a.id = ?
            ",
        )
            .bind(pm_id)
            .fetch_all(&state.db)
            .await
        {
            Ok(pms) => {
                let response = 
                 json!({"project": project_detail,
                         "project_maintenance": pms.first()});
                HttpResponse::Ok().json(response)
            }
            Err(error) => {
                HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
            }
        }
}

#[get("/report")]
pub async fn get_report(
    state: Data<AppState>,
    query_parameter: Query<ReportQuery>,
) -> impl Responder {
    let list_vendor_id = query_parameter.list_vendor_id.clone();
    let project_start_date = query_parameter.project_start_date.clone();
    let project_end_date = query_parameter.project_end_date.clone();
    let completion_start_date = query_parameter.completion_start_date.clone();
    let completion_end_date = query_parameter.completion_end_date.clone();
    let pm_type = query_parameter.pm_type.clone();
    let pm_status = query_parameter.pm_status.clone();

    match sqlx::query_as::<_, PMReportDto>(
        "SELECT ve.id AS vendor_id,
                ve.name AS vendor_name,
                pr.id AS project_id,
                pr.name AS project_name,
                a.id AS project_id,
                a.id AS pm_id,
                a.pm_description AS pm_task,
                a.pm_solution,
                a.pm_type,
                CAST(a.pm_project_date AS CHAR) AS pm_project_date,
                CAST(a.pm_completion_date AS CHAR) AS pm_completion_date,
                a.pic_name,
                a.pic_email,
                u.name AS pic_unit,
               (CASE 
                    WHEN a.is_verified IS NULL THEN 'On Progress' 
                    WHEN a.is_verified IS TRUE THEN 'Verified' 
                    WHEN a.is_verified IS FALSE THEN 'Need Revise' 
                END) AS status,
                v.username AS pm_verified_by,
                CAST(a.verified_at AS CHAR) AS pm_verified_at,
                c.username AS pm_created_by,
                CAST(a.created_at AS CHAR) AS pm_created_at,
                up.username AS pm_updated_by,
                CAST(a.updated_at AS CHAR) AS pm_updated_at,
                a.note
                FROM project_pm a
                LEFT JOIN project pr ON (pr.id = a.project_id)
                LEFT JOIN vendor ve ON (ve.id = pr.vendor_id)
                LEFT JOIN users c ON (c.id = a.created_by)
                LEFT JOIN users v ON (v.id = a.verified_by)
                LEFT JOIN users up ON (up.id = a.updated_by)
                LEFT JOIN unit u ON (u.id = a.pic_unit_id)
                WHERE (? IS NULL OR FIND_IN_SET(ve.id,  ?))
                    AND (? IS NULL OR a.pm_type = ?)
                    AND (? IS NULL OR (
                        (? = 'On Progress' AND a.is_verified IS NULL) OR
                        (? = 'Verified' AND a.is_verified = TRUE) OR
                        (? = 'Need Revision' AND a.is_verified = FALSE)
                    ))
                    AND (? IS NULL OR a.pm_project_date >= ?)
                    AND (? IS NULL OR a.pm_project_date <= ?)
                    AND (? IS NULL OR a.pm_completion_date IS NULL OR a.pm_completion_date >= ?)
                    AND (? IS NULL OR a.pm_completion_date IS NULL OR a.pm_completion_date <= ?)
                ORDER BY a.created_at DESC;
            ",
        )
            .bind(&list_vendor_id)
            .bind(&list_vendor_id)
            .bind(&pm_type)
            .bind(&pm_type)
            .bind(&pm_status)
            .bind(&pm_status)
            .bind(&pm_status)
            .bind(&pm_status)
            .bind(project_start_date.clone())
            .bind(project_start_date)
            .bind(project_end_date.clone())
            .bind(project_end_date)
            .bind(completion_start_date.clone())
            .bind(completion_start_date)
            .bind(completion_end_date.clone())
            .bind(completion_end_date)
            .fetch_all(&state.db)
            .await
        {
            Ok(pms) => {
                let response = json!({
                    "data": pms,
                });

                HttpResponse::Ok().json(response)
            }
            Err(error) => {
                HttpResponse::InternalServerError().json(json!({ "message": format!("{}", error) }))
            }
        }
}