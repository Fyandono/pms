use crate::{AppState};
use actix_web::{
    get,
    post,
    put,
    web::{Data, Query, Json, Path, Bytes},
    HttpResponse, Responder,
};
use actix_multipart::Multipart;
use serde_json::json;
use sqlx::{self, types::Json as SqlxJson};
use crate::features::user::services::{AuthClaims};
use crate::features::vendor_project::model::{ProjectQuery,
    Vendor, VendorDto, VendorQuery, Project, ProjectDto, 
    ProjectPMDto, PMQuery, VerifyPM, VendorDropdownDto,
    ProjectPM, ProjectPMData, FilePathResult, PMDetailQuery, NoteEntry};
use crate::util::page_response_builder::{page_response_builder, page_response_extra_builder};
use crate::util::require_role::{require_role};
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
    claims: AuthClaims,
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
                        CAST(v.created_at AS TEXT) AS created_at,
                        u.username AS updated_by,
                        CAST(v.updated_at AS TEXT) AS updated_at,
                        COUNT(p.id) AS count_project
                    FROM vendor v
                    LEFT JOIN project p ON (p.vendor_id = v.id)
                    LEFT JOIN users c ON (c.id = v.created_by)
                    LEFT JOIN users u ON (u.id = v.updated_by)
                    WHERE v.name ILIKE CONCAT('%',$1,'%')
                    GROUP BY v.id, c.username, u.username
                    ORDER BY v.name;",
    )
    .bind(name_filter)
    .fetch_all(&state.postgres)
    .await
    {
        Ok(vendors) => {
            let response = page_response_builder(page, page_size, &vendors);
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    }
}

#[get("/dropdown-vendor")]
pub async fn get_dropdown_vendor(
    state: Data<AppState>,
    claims: AuthClaims,
    query_parameter: Query<VendorQuery>
) -> impl Responder {

    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;
    match sqlx::query_as::<_, VendorDropdownDto>(
        "SELECT v.id, v.name
        FROM vendor v
        WHERE v.name ILIKE CONCAT('%', $1, '%')",
    )
    .bind(name_filter)
    .fetch_all(&state.postgres)
    .await
    {
        Ok(vendors) => {
            let response = page_response_builder(page, page_size, &vendors);
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    }
}

#[get("/project")]
pub async fn get_list_project(
    state: Data<AppState>,
    query_parameter: Query<ProjectQuery>,
    claims: AuthClaims
) -> impl Responder {

    let vendor_id = query_parameter.vendor_id;
    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;

    // get vendor
    let vendor_detail = match sqlx::query_as::<_, VendorDto>(
        "SELECT v.id,
                v.name,
                v.address,
                v.email,
                v.phone_number,
                c.username AS created_by,
                CAST(v.created_at AS TEXT) AS created_at,
                u.username AS updated_by,
                CAST(v.updated_at AS TEXT) AS updated_at,
                COUNT(p.id) AS count_project
            FROM vendor v
            LEFT JOIN project p ON (p.vendor_id = v.id)
            LEFT JOIN users c ON (c.id = v.created_by)
            LEFT JOIN users u ON (u.id = v.updated_by)
            WHERE v.id = $1
            GROUP BY v.id, c.username, u.username
            ORDER BY v.name;",
    )
    .bind(vendor_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No vendor found with specified ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

    // get projects
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
                    CAST(p.created_at AS TEXT) AS created_at,
                    u.username AS updated_by,
                    CAST(p.updated_at AS TEXT) AS updated_at,
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
                WHERE p.vendor_id = $1 AND p.name ILIKE CONCAT('%', $2, '%')
                GROUP BY p.id, dpmv.count_pm_verified, un.name, c.username, u.username, v.name;",
            )
                .bind(vendor_id)
                .bind(name_filter)
                .fetch_all(&state.postgres)
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
                        HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
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
    let start_date = query_parameter.start_date.clone();
    let end_date = query_parameter.end_date.clone();
    let pm_type = query_parameter.pm_type.clone();
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;

    // get project
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
                    CAST(p.created_at AS TEXT) AS created_at,
                    u.username AS updated_by,
                    CAST(p.updated_at AS TEXT) AS updated_at,
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
                WHERE p.id = $1
                GROUP BY p.id, dpmv.count_pm_verified, un.name, c.username, u.username, v.name;",
    )
    .bind(project_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No project found with specified ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

    match sqlx::query_as::<_, ProjectPMDto>(
                "SELECT a.id,
                        a.project_id,
                        a.pm_description,
                        a.pm_solution,
                        a.pm_type,
                        CAST(a.pm_project_date AS TEXT) AS pm_project_date,
                        a.url_file,
                        a.is_verified,
                        v.username AS verified_by,
                        CAST(a.verified_at AS TEXT) as verified_at,
                        c.username AS created_by,
                        CAST(a.created_at AS TEXT) AS created_at,
                        up.username AS updated_by,
                        CAST(a.updated_at AS TEXT) AS updated_at,
                        a.pic_name,
                        a.pic_email,
                        a.pic_unit_id,
                        u.name AS pic_unit,
                        CAST(a.pm_completion_date AS TEXT) AS pm_completion_date,
                        a.note
                    FROM project_pm a
                    LEFT JOIN users c ON (c.id = a.created_by)
                    LEFT JOIN users v ON (v.id = a.verified_by)
                    LEFT JOIN users up ON (up.id = a.updated_by)
                    LEFT JOIN unit u ON (u.id = a.pic_unit_id)
                    WHERE a.project_id = $1 
                    AND a.pm_description ILIKE CONCAT('%', $2, '%') 
                    AND ($5 IS NULL OR a.pm_type = $5)
                    AND ($3 IS NULL OR a.pm_project_date >= CAST($3 AS DATE))
                    AND ($4 IS NULL OR a.pm_project_date <= CAST($4 AS DATE))
                    ORDER BY a.created_at DESC;
                ",
            )
                .bind(project_id)
                .bind(description)
                .bind(start_date)
                .bind(end_date)
                .bind(pm_type)
                .fetch_all(&state.postgres)
                .await
            {
                    Ok(pms) => {
                        let response = 
                        page_response_extra_builder(page, page_size, &pms, json!({"project": project_detail}));
                        HttpResponse::Ok().json(response)
                    }
                    Err(error) => {
                        HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
                    }
                }
}

#[post("/vendor")]
pub async fn post_create_vendor(
    state: Data<AppState>,
    body: Json<Vendor>,
    claims: AuthClaims
) -> impl Responder {

    // get user id
    let user_id = claims.0.sub.to_string();

    // 1. Begin a new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to start transaction: {}", e) 
            }))
        }
    };

    // 2. Perform the INSERT query using the transaction
    match sqlx::query_as::<_, Vendor>(
        "INSERT INTO vendor (name, address, email, phone_number, created_by) 
         VALUES ($1, $2, $3, $4, CAST($5 AS UUID))
         RETURNING id, name, address, email, phone_number",
    )
    .bind(&body.name)
    .bind(&body.address)
    .bind(&body.email)
    .bind(&body.phone_number)
    .bind(user_id)
    .fetch_one(&mut *transaction) // <-- Executed within the transaction
    .await
    {
        Ok(vendor) => {
            // 3. Commit the transaction (makes the change permanent)
            match transaction.commit().await {
                Ok(_) => {
                    HttpResponse::Created().json(json!({
                        "message": format!("Vendor '{}' successfully created.", vendor.name),
                        "vendor": vendor,
                    }))
                }
                Err(e) => {
                    // This handles failure during the commit process
                    HttpResponse::InternalServerError().json(json!({ 
                        "error": format!("Failed to commit transaction: {}", e) 
                    }))
                }
            }
        }
        Err(error) => {
            // 4. Rollback the transaction on failure
            let _ = transaction.rollback().await; 
            HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to create vendor: {}", error) 
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

    // get user id
    let user_id = claims.0.sub.to_string();

    // 1. Begin a new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to start transaction: {}", e) 
            }))
        }
    };

    // 2. Perform the UPDATE query using the transaction
    match sqlx::query_as::<_, Vendor>(
        "UPDATE vendor 
         SET name = $2,
             address = $3,
             email = $4,
             phone_number = $5,
             updated_at = NOW(),
             updated_by = CAST($6 AS UUID)
         WHERE id = $1
         RETURNING id, name, address, email, phone_number"
    )
    .bind(&body.id)
    .bind(&body.name)
    .bind(&body.address)
    .bind(&body.email)
    .bind(&body.phone_number)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(vendor) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": format!("Vendor '{}' successfully updated.", vendor.name),
                    "vendor": vendor,
                })),
                Err(e) => HttpResponse::InternalServerError().json(json!({ 
                    "error": format!("Failed to commit transaction: {}", e) 
                })),
            }
        }
        Err(error) => {
            // 4. Rollback on failure
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to update vendor: {}", error) 
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
    // get user id
    let user_id = claims.0.sub.to_string();

    // 1. Begin a new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to start transaction: {}", e) 
            }))
        }
    };
    
    // 2. Insert the new Project within the transaction
    match sqlx::query_as::<_, Project>(
        "INSERT INTO project 
            (vendor_id, name, description, pic_name, pic_email, pic_unit_id, project_type, created_by) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, CAST($8 AS UUID))
         RETURNING id, vendor_id, name, description, pic_name, pic_email, pic_unit_id, project_type, created_by",
    )
    .bind(body.vendor_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.pic_name)
    .bind(&body.pic_email)
    .bind(&body.pic_unit_id)
    .bind(&body.project_type)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(project) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => {
                    HttpResponse::Created().json(json!({
                        "message": format!("Project '{}' successfully created for vendor {}.", project.name, project.vendor_id),
                        "project": project,
                    }))
                }
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({ 
                        "error": format!("Failed to commit transaction: {}", e) 
                    }))
                }
            }
        }
        Err(error) => {
            // 4. Rollback the transaction on failure
            let _ = transaction.rollback().await; 
            HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to create project: {}", error) 
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
    // get user id
    let user_id = claims.0.sub.to_string();

    // 1. Begin a new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // 2. Update the Project within the transaction
    match sqlx::query_as::<_, Project>(
        "UPDATE project
         SET vendor_id   = $2,
             name        = $3,
             description = $4,
             pic_name    = $5,
             pic_email   = $6,
             pic_unit_id = $7,
             project_type = $8,
             updated_at  = NOW(),
             updated_by  = CAST($9 AS UUID)
         WHERE id = $1
         RETURNING id, vendor_id, name, description, pic_name, pic_email, pic_unit_id, project_type"
    )
    .bind(&body.id)
    .bind(&body.vendor_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.pic_name)
    .bind(&body.pic_email)
    .bind(&body.pic_unit_id)
    .bind(&body.project_type)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(project) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => {
                    HttpResponse::Ok().json(json!({
                        "message": format!("Project '{}' successfully updated.", project.name),
                        "project": project,
                    }))
                }
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to commit transaction: {}", e)
                    }))
                }
            }
        }
        Err(error) => {
            // 4. Rollback on failure
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to update project: {}", error)
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
    let pool = &state.postgres;

    // 1. Start Transaction
    let mut transaction = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // --- A. FETCH ROW EXISTENCE (Necessary to distinguish NULL note from missing row) ---
    let row_exists: bool = match sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM project_pm WHERE id = $1)")
        .bind(&body.id)
        .fetch_one(&mut *transaction)
        .await
    {
        Ok(exists) => exists,
        Err(e) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().json(json!({ "error": format!("Database error during row check: {}", e) }));
        }
    };

    if !row_exists {
        let _ = transaction.rollback().await;
        return HttpResponse::NotFound().json(json!({ "message": "Project PM record not found." }));
    }

    // --- B. FETCH existing notes history as Optional String (The requested method) ---
    let notes_json_string: Option<String> = match sqlx::query_scalar::<_, Option<String>>(
        "SELECT note FROM project_pm WHERE id = $1"
    )
    .bind(&body.id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(s)) => s, 
        Ok(None) => None, // Note column was SQL NULL, or row was just fetched by SELECT EXISTS
        Err(e) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to fetch notes history string: {}", e) 
            }));
        }
    };
    
    // 3. Safely parse the fetched string into Vec<NoteEntry>
    let mut notes_history: Vec<NoteEntry> = match notes_json_string {
        Some(s) => {
            // Attempt to parse the string content
            match serde_json::from_str(&s) {
                Ok(notes) => notes,
                Err(e) => {
                    // Log the error and treat it as empty history for resilience
                    eprintln!("Warning: Corrupt JSON data found in note column for ID {}. Error: {}", body.id, e);
                    Vec::new() 
                }
            }
        },
        // If the column was NULL, start with an empty vector
        None => Vec::new(), 
    };

    // 4. Prepare and conditionally append the new note entry
    
    // Check if the input note is Some() AND if the contained string is not empty/whitespace
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
    
    // 5. Serialize the complete history back into a String/Text for the UPDATE query
    let updated_notes_json_string = match serde_json::to_string(&notes_history) {
        Ok(s) => s,
        Err(e) => {
             let _ = transaction.rollback().await;
             return HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to serialize notes history: {}", e) 
            }));
        }
    };


    // --- C. UPDATE the record ---
    match sqlx::query(
        "UPDATE project_pm
         SET is_verified = $2,
             note = $3,
             pm_completion_date = CAST($4 AS DATE),  
             verified_at = NOW(),
             verified_by = CAST($5 AS UUID)
         WHERE id = $1
         RETURNING id" 
    )
    .bind(&body.id)
    .bind(&body.is_verified)
    .bind(updated_notes_json_string) /* Binding the JSON as a raw String */
    .bind(&body.pm_completion_date) 
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(_) => {
            // 6. Commit the transaction
            match transaction.commit().await {
                Ok(_) => {
                    HttpResponse::Ok().json(json!({
                        "message": format!("PM ID '{}' successfully updated.", body.id),
                    }))
                }
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to commit transaction: {}", e)
                    }))
                }
            }
        }
        Err(error) => {
            // 7. Rollback on failure
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to update project: {}", error)
            }))
        }
    }
}

// #[put("/verify")]
// pub async fn put_edit_verify_pm(
//     state: Data<AppState>,
//     body: Json<VerifyPM>,
//     claims: AuthClaims
// ) -> impl Responder {

//     // Get user id
//     let user_id = &claims.0.sub;
    
//     // 1. Begin a new transaction
//     let mut transaction = match state.postgres.begin().await {
//         Ok(t) => t,
//         Err(e) => {
//             return HttpResponse::InternalServerError().json(json!({
//                 "error": format!("Failed to start transaction: {}", e)
//             }))
//         }
//     };

//     // 2. Update the Project within the transaction
//     match sqlx::query_as::<_, VerifyPM>(
//         "UPDATE project_pm
//          SET is_verified = $2,
//              note = $3,
//              pm_completion_date = CAST($4 AS DATE),
//              verified_at = NOW(),
//              verified_by = CAST($5 AS UUID)
//          WHERE id = $1
//          RETURNING id, is_verified, CAST(pm_completion_date AS TEXT), note"
//     )
//     .bind(&body.id)
//     .bind(&body.is_verified)
//     .bind(&body.note)
//     .bind(&body.pm_completion_date)
//     .bind(user_id)
//     .fetch_one(&mut *transaction)
//     .await
//     {
//         Ok(project_pm) => {
//             // 3. Commit the transaction
//             match transaction.commit().await {
//                 Ok(_) => {
//                     HttpResponse::Ok().json(json!({
//                         "message": format!("PM ID '{}' successfully updated.", project_pm.id),
//                         "project_pm": project_pm,
//                     }))
//                 }
//                 Err(e) => {
//                     HttpResponse::InternalServerError().json(json!({
//                         "error": format!("Failed to commit transaction: {}", e)
//                     }))
//                 }
//             }
//         }
//         Err(error) => {
//             // 4. Rollback on failure
//             let _ = transaction.rollback().await;
//             HttpResponse::InternalServerError().json(json!({
//                 "error": format!("Failed to update project: {}", error)
//             }))
//         }
//     }
// }

#[post("/pm")]
pub async fn post_create_project_pm(
    state: Data<AppState>,
    mut payload: Multipart,
    claims: AuthClaims,
) -> impl Responder {
    

    // Get user id
    let user_id = claims.0.sub;

    let mut data = ProjectPMData::default();
    let mut file_path: Option<String> = None;

    // 1. Process the multipart fields (data and file)
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Multipart processing error: {}", e)})),
        };

        // FIX 2: Safely extract field name
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            // --- File Handling ---
            
            // FIX 3: Safely extract filename from Content-Disposition
            let filename = field.content_disposition()
                .as_ref()
                .and_then(|cd| cd.get_filename())
                .map(|s| s.to_owned())
                .unwrap_or_else(|| format!("upload-{}", Uuid::new_v4()));

            // Generate unique filename structure: filename_uuid.ext
            let extension = filename.split('.').last().unwrap_or("dat");
            let base_name: String = filename.rsplit_once('.').map(|(base, _)| base.to_owned()).unwrap_or(filename.clone());

            let unique_code = Uuid::new_v4().to_string();
            let new_filename = format!("{}_{}.{}", base_name, unique_code, extension);

            // Define the full path where the file will be saved. Ensure the 'data' directory exists.
            let filepath = format!("./data/{}", new_filename);
            file_path = Some(filepath.clone());
            
            // Create and write the file to the local filesystem
            match File::create(&filepath).await {
                Ok(mut f) => {
                    while let Some(chunk) = field.next().await {
                        if let Ok(chunk) = chunk {
                            if let Err(e) = f.write_all(&chunk).await {
                                let _ = tokio::fs::remove_file(&filepath).await; // Clean up partially written file
                                return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to write file to disk: {}", e)}));
                            }
                        }
                    }
                }
                Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to create file on disk: {}", e)})),
            }
        } else if !field_name.is_empty() {
            // --- Regular Field Handling ---
            let bytes = match field.next().await {
                Some(Ok(b)) => b,
                _ => continue,
            };
            let value = String::from_utf8(bytes.to_vec()).unwrap_or_default();
            // inside the 'else if !field_name.is_empty()' block
            match field_name.as_str() {
                "project_id" => data.project_id = value.parse::<i32>().ok(),
                "pm_description" => data.pm_description = Some(value),
                "pm_solution" => data.pm_solution = Some(value),
                "pm_type" => data.pm_type = Some(value),
                "pm_project_date" => data.pm_project_date = Some(value),
                
                // ⭐ NEW FIELD HANDLING ⭐
                "pic_name" => data.pic_name = Some(value),
                "pic_email" => data.pic_email = Some(value),
                "pic_unit_id" => data.pic_unit_id = value.parse::<i32>().ok(),
                // ----------------------
                _ => {}
            }
        }
    }

    // 2. Input validation and ownership transfer (FIX 4: Avoid "moved value" error)
    
    // Helper function to clean up file if validation fails later
    let cleanup = |path: Option<String>| async move {
        if let Some(p) = path {
            let _ = tokio::fs::remove_file(p).await;
        }
    };

    let project_id = match data.project_id {
        Some(pid) => pid,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing or invalid project_id."}));
        }
    };

    let pm_description = match data.pm_description {
        Some(desc) => desc,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing pm_description."}));
        }
    };

    let pm_solution = match data.pm_solution {
        Some(sol) => sol,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing pm_solution."}));
        }
    };

    let pm_type = match data.pm_type {
        Some(val) => val,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing pm_solution."}));
        }
    };

    
    let pm_project_date = match data.pm_project_date {
        Some(val) => val,
        None => {
            cleanup(file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing pm_project_date."}));
        }
    };

    let url_file = match file_path {
        Some(path) => path,
        None => {
            return HttpResponse::BadRequest().json(json!({"error": "Missing file upload."}));
        }
    };

    let pic_name = data.pic_name;
    let pic_email = data.pic_email;
    let pic_unit_id = data.pic_unit_id;

    // All variables now hold owned, valid data.

    // 3. Begin new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            cleanup(Some(url_file)).await; // Clean up if transaction fails to start
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // 4. Insert a new Project PM
    match sqlx::query_as::<_, ProjectPM>(
        "INSERT INTO project_pm (project_id, pm_description, pm_solution, pm_type, pm_project_date, url_file, pic_name, pic_email, pic_unit_id, created_by)
         VALUES ($1, $2, $3, $4, CAST($5 AS DATE), $6, $7, $8, $9, CAST($10 AS UUID))
         RETURNING id, project_id, pm_description, pm_solution, pm_type, CAST(pm_project_date AS TEXT), url_file, pic_name, pic_email, pic_unit_id"
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
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(pm) => {
            // 5. Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": format!("PM added to project {}.", pm.project_id),
                    "project_pm": pm,
                })),
                Err(e) => {
                    // let _ = transaction.rollback().await;
                    cleanup(Some(url_file)).await; // Clean up on commit failure
                    HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to commit transaction: {}", e)
                    }))
                }
            }
        }
        Err(error) => {
            // 6. Rollback on query failure and delete the file
            let _ = transaction.rollback().await;
            cleanup(Some(url_file)).await; // Clean up on query failure
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to add project PM step: {}", error)
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
    
    // 1. Retrieve the internal file path using query_as! and the new struct
    let result = match sqlx::query_as::<_, FilePathResult>(
        "SELECT url_file FROM project_pm WHERE id = $1"
    )
    .bind(project_pm_id)
    .fetch_optional(&state.postgres)
    .await {
        Ok(Some(r)) => r, // Success: We get the FilePathResult struct
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({"error": format!("Project PM with ID {} not found.", project_pm_id)}))
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({"error": format!("Database error fetching file path: {}", e)}))
        },
    };

    // Extract the file path string from the struct
    let file_path = result.url_file; 

    // 2. Open the file from the filesystem
    let file = match File::open(&file_path).await {
        Ok(f) => f, 
        Err(_) => {
            return HttpResponse::NotFound().json(json!({"error": format!("File not found on server at path: {}", file_path)}))
        },
    };
    
    // 3. Determine MIME type and prepare for streaming
    let mime_type = mime_guess::from_path(&file_path)
        .first_or_text_plain();

    // Create the raw stream of bytes
    let raw_stream = FramedRead::new(file, BytesCodec::new());

    // ⭐ CRITICAL FIX: Map the Ok result (BytesMut) to Bytes using .freeze()
    let stream = raw_stream.map_ok(|bytes_mut| {
        // Convert the BytesMut into the required immutable Bytes type
        Bytes::from(bytes_mut.freeze())
    });
    // -------------------------------------------------------------

    let filename_for_download: Cow<str> = file_path.split('/')
        .last()
        .map_or("download".into(), |s| s.into());

    // 4. Stream the file content back to the client
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
    // Get User ID
    let user_id = &claims.0.sub;

    // This HashMap will store all non-file field updates, including the 'id'
    let mut data_updates: HashMap<String, String> = HashMap::new();
    let mut new_file_path: Option<String> = None;

    // Helper function to clean up the newly saved file if the transaction fails
    let cleanup = |path: Option<String>| async move {
        if let Some(p) = path {
            let _ = tokio::fs::remove_file(p).await;
        }
    };

    // 2. Process the multipart fields (data and file)
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => {
                cleanup(new_file_path.clone()).await;
                return HttpResponse::InternalServerError().json(json!({"error": format!("Multipart processing error: {}", e)}))
            }
        };

        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" {
            // --- File Handling (Only if a new file is uploaded) ---

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

            // Create and write the file to the local filesystem
            match File::create(&filepath).await { // Using tokio::fs::File::create
                Ok(mut f) => {
                    while let Some(chunk) = field.next().await {
                        if let Ok(chunk) = chunk {
                            if let Err(e) = f.write_all(&chunk).await {
                                let _ = cleanup(Some(filepath)).await;
                                return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to write new file to disk: {}", e)}));
                            }
                        }
                    }
                }
                Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to create new file on disk: {}", e)})),
            }
        } else if !field_name.is_empty() {
            // --- Regular Field Handling (Collect all provided fields) ---
            let bytes = match field.next().await {
                Some(Ok(b)) => b,
                _ => continue,
            };
            let value = String::from_utf8(bytes.to_vec()).unwrap_or_default();
            data_updates.insert(field_name, value);
        }
    }
    
    // 3. Extract and validate all MANDATORY fields for replacement/update
    let pm_id = match data_updates.remove("id").and_then(|s| s.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing or invalid mandatory field 'id'."}));
        }
    };
    
    let project_id = match data_updates.remove("project_id").and_then(|s| s.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing or invalid mandatory field 'project_id'."}));
        }
    };
    
    let pm_description = match data_updates.remove("pm_description") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing mandatory field 'pm_description'."}));
        }
    };
    
    let pm_solution = match data_updates.remove("pm_solution") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing mandatory field 'pm_solution'."}));
        }
    };
    
    let pm_type = match data_updates.remove("pm_type") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing mandatory field 'pm_type'."}));
        }
    };

    let pm_project_date = match data_updates.remove("pm_project_date") {
        Some(s) => s,
        None => {
            cleanup(new_file_path).await;
            return HttpResponse::BadRequest().json(json!({"error": "Missing mandatory field 'pm_project_date'."}));
        }
    };
    
    // 4. Extract OPTIONAL fields
    let pic_name_opt = data_updates.remove("pic_name");
    let pic_email_opt = data_updates.remove("pic_email");
    let pic_unit_id_opt: Option<i32> = data_updates.remove("pic_unit_id").and_then(|s| s.parse().ok());


    // 5. Begin new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            cleanup(new_file_path).await;
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // 6. Retrieve the existing record (MANDATORY to get the old file path)
    let old_pm_record = match sqlx::query_as::<_, ProjectPM>("SELECT id, project_id, pm_description, pm_solution, pm_type, CAST(pm_project_date AS TEXT), url_file, pic_name, pic_email, pic_unit_id FROM project_pm WHERE id = $1")
        .bind(pm_id)
        .fetch_optional(&mut *transaction)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => {
            let _ = transaction.rollback().await;
            cleanup(new_file_path).await;
            return HttpResponse::NotFound().json(json!({"error": format!("Project PM with id {} not found.", pm_id)}));
        }
        Err(e) => {
            let _ = transaction.rollback().await;
            cleanup(new_file_path).await;
            return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to fetch existing project PM: {}", e)}));
        }
    };
    
    // Determine the final file path to be saved and the old path to be deleted
    let old_file_path_to_delete = if new_file_path.is_some() {
        Some(old_pm_record.url_file.clone()) // New file uploaded, queue old one for deletion
    } else {
        None // No new file, nothing to delete
    };
    
    // The file path to be stored in the database
    let url_file_to_save = new_file_path.unwrap_or(old_pm_record.url_file);

    // 7. Build the UPDATE query using direct assignment (no COALESCE)
    let query_string = "
        UPDATE project_pm SET 
            project_id = $2,
            pm_description = $3,
            pm_solution = $4,
            pm_type = $5,
            pm_project_date = CAST($6 AS DATE),
            url_file = $7,
            pic_name = $8,
            pic_email = $9,
            pic_unit_id = $10::int,
            updated_by = CAST($11 AS UUID),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, project_id, pm_description, pm_solution, pm_type, CAST(pm_project_date AS TEXT), url_file, pic_name, pic_email, pic_unit_id
    ";

    // 8. Execute the update
    match sqlx::query_as::<_, ProjectPM>(query_string)
        .bind(pm_id)                                  // $1 id (MANDATORY)
        .bind(project_id)                             // $2 project_id (MANDATORY)
        .bind(pm_description)                         // $3 pm_description (MANDATORY)
        .bind(pm_solution)                            // $4 pm_solution (MANDATORY)
        .bind(pm_type)                                // $5 pm_type (MANDATORY)
        .bind(pm_project_date)                        // $6 pm_project_date (MANDATORY)
        .bind(&url_file_to_save)                      // $7 url_file 
        .bind(pic_name_opt)                           // $8 pic_name (OPTIONAL)
        .bind(pic_email_opt)                          // $9 pic_email (OPTIONAL)
        .bind(pic_unit_id_opt)                        // $10 pic_unit_id (OPTIONAL)
        .bind(user_id)                                // $11 updated_by
        .fetch_one(&mut *transaction)
        .await
    {
        Ok(pm) => {
            // 9. Commit the transaction
            match transaction.commit().await {
                Ok(_) => {
                    // 10. Delete the OLD file from disk only AFTER successful commit
                    if let Some(old_path) = old_file_path_to_delete {
                        let _ = cleanup(Some(old_path)).await;
                    }

                    HttpResponse::Ok().json(json!({
                        "message": "Project PM updated successfully. All core fields were replaced.".to_string(),
                        "project_pm": pm,
                    }))
                }
                Err(e) => {
                    // Transaction failed to commit, clean up the NEW file
                    cleanup(Some(url_file_to_save)).await;
                    HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to commit transaction: {}", e)
                    }))
                }
            }
        }
        Err(error) => {
            // 11. Rollback on query failure and delete the NEW file
            let _ = transaction.rollback().await;
            cleanup(Some(url_file_to_save)).await; 
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to update project PM step: {}", error)
            }))
        }
    }
}

#[get("/pm-detail")]
pub async fn get_detail_pm(
    state: Data<AppState>,
    query_parameter: Query<PMDetailQuery>,
    claims: AuthClaims
) -> impl Responder {

    

    let pm_id = query_parameter.pm_id;

    // get project
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
                    CAST(p.created_at AS TEXT) AS created_at,
                    u.username AS updated_by,
                    CAST(p.updated_at AS TEXT) AS updated_at,
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
                WHERE pm.id = $1
                GROUP BY p.id, dpmv.count_pm_verified, un.name, c.username, u.username, v.name;",
    )
    .bind(pm_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No project found with specified ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

    match sqlx::query_as::<_, ProjectPMDto>(
                "SELECT a.id,
                        a.project_id,
                        a.pm_description,
                        a.pm_solution,
                        a.pm_type,
                        CAST(a.pm_project_date AS TEXT) AS pm_project_date,
                        a.url_file,
                        a.is_verified,
                        v.username AS verified_by,
                        CAST(a.verified_at AS TEXT) as verified_at,
                        c.username AS created_by,
                        CAST(a.created_at AS TEXT) AS created_at,
                        up.username AS updated_by,
                        CAST(a.updated_at AS TEXT) AS updated_at,
                        a.pic_name,
                        a.pic_email,
                        a.pic_unit_id,
                        u.name AS pic_unit,
                        CAST(a.pm_completion_date AS TEXT) AS pm_completion_date,
                        a.note
                FROM project_pm a
                LEFT JOIN users c ON (c.id = a.created_by)
                LEFT JOIN users v ON (v.id = a.verified_by)
                LEFT JOIN users up ON (up.id = a.updated_by)
                LEFT JOIN unit u ON (u.id = a.pic_unit_id)
                WHERE a.id = $1
                ",
            )
                .bind(pm_id)
                .fetch_all(&state.postgres)
                .await
            {
                    Ok(pm) => {
                        let response = 
                         json!({"project": project_detail,
                                "project_maintenance": pm.first()});
                        HttpResponse::Ok().json(response)
                    }
                    Err(error) => {
                        HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
                    }
                }
}


// #[get("/users-vendor")]
// pub async fn get_users_vendor(
//     state: Data<AppState>,
//     claims: AuthClaims,
//     query_parameter: Query<UsersVendorQuery>
// ) -> impl Responder {

//     let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
//     let page = query_parameter.page;
//     let page_size = query_parameter.page_size;
//     match sqlx::query_as::<_, UsersVendorDto>(
//         "SELECT CAST(uv.user_id AS TEXT) AS user_id, 
//         uv.vendor_id, 
//         u.username,
//         v.name AS vendor_name
//         FROM users_vendor uv
//         LEFT JOIN users u ON (CAST(u.id AS UUID) = uv.user_id)
//         LEFT JOIN vendor v ON (v.id = uv.vendor_id)
//         WHERE u.username ILIKE CONCAT('%', $1, '%') OR v.name ILIKE CONCAT('%', $1, '%')
//         ORDER by v.name, u.username",
//     )
//     .bind(name_filter)
//     .fetch_all(&state.postgres)
//     .await
//     {
//         Ok(users_vendor) => {
//             let response = page_response_builder(page, page_size, &users_vendor);
//             HttpResponse::Ok().json(response)
//         }
//         Err(error) => {
//             HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
//         }
//     }
// }

// #[post("/users-vendor")]
// pub async fn post_users_vendor(
//     state: Data<AppState>,
//     claims: AuthClaims,
//     body: Json<UsersVendor>
// ) -> impl Responder {
    
//     // 1. Begin a new transaction
//     let mut transaction = match state.postgres.begin().await {
//         Ok(t) => t,
//         Err(e) => {
//             return HttpResponse::InternalServerError().json(json!({ 
//                 "error": format!("Failed to start transaction: {}", e) 
//             }))
//         }
//     };
    
//     // 2. Insert the new Project within the transaction
//     match sqlx::query_as::<_, UsersVendor>(
//         "INSERT INTO users_vendor 
//             (user_id, vendor_id) 
//          VALUES (CAST($1 AS UUID), $2)
//          RETURNING CAST(user_id AS TEXT) AS user_id, vendor_id",
//     )
//     .bind(&body.user_id)
//     .bind(body.vendor_id)
//     .fetch_one(&mut *transaction)
//     .await
//     {
//         Ok(user_vendor) => {
//             // 3. Commit the transaction
//             match transaction.commit().await {
//                 Ok(_) => {
//                     HttpResponse::Created().json(json!({
//                         "message": format!("Project '{}' successfully created for user vendor {}.", user_vendor.user_id, user_vendor.vendor_id),
//                         "user_vendor": user_vendor,
//                     }))
//                 }
//                 Err(e) => {
//                     HttpResponse::InternalServerError().json(json!({ 
//                         "error": format!("Failed to commit transaction: {}", e) 
//                     }))
//                 }
//             }
//         }
//         Err(error) => {
//             // 4. Rollback the transaction on failure
//             let _ = transaction.rollback().await; 
//             HttpResponse::InternalServerError().json(json!({ 
//                 "error": format!("Failed to create project: {}", error) 
//             }))
//         }
//     }
// }