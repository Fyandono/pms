use crate::{AppState};
use actix_web::{
    get,
    post,
    put,
    web::{Data, Query, Json},
    HttpResponse, Responder,
};
use serde_json::json;
use sqlx::{self};
use crate::features::user::services::AuthClaims;
use crate::features::vendor::model::{
    VendorValidationDto,
    ProjectQuery, 
    Vendor, 
    VendorDto, 
    Project,
    ProjectDto,
    ProjectPM,
    ProjectPMDto, 
    ProjectValidationDto,
    PMQuery};
use crate::util::page_response_builder::{ page_response_extra_builder};

#[get("/project")]
pub async fn get_list_project_u(
    state: Data<AppState>,
    query_parameter: Query<ProjectQuery>,
    claims: AuthClaims
) -> impl Responder {

    let user_id = claims.0.sub;
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
                CAST(v.created_at AS TEXT) AS created_at,
                CAST(v.updated_at AS TEXT) AS updated_at,
                COUNT(p.id) AS count_project
            FROM vendor v
            LEFT JOIN project p ON (p.vendor_id = v.id)
            LEFT JOIN users_vendor uv ON (uv.vendor_id = v.id)
            WHERE v.id = $1 AND uv.vendor_id = CAST($2 AS UUID)
            GROUP BY v.id
            ORDER BY v.name;",
    )
    .bind(vendor_id)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No vendor found with specified ID and User ID."  }))
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
                    p.vendor_id,
                    p.name,
                    p.description,
                    p.pic_name,
                    p.pic_email,
                    p.pic_number,
                    p.pm_count,
                    CAST(p.created_at AS TEXT) AS created_at,
                    CAST(p.updated_at AS TEXT) AS updated_at,
                    COALESCE(COUNT(pm.id), 0) AS count_pm_uploaded,
                    COALESCE(dpmv.count_pm_verified, 0) AS count_pm_verified,
                    COALESCE(COUNT(pm.id), 0) - COALESCE(dpmv.count_pm_verified, 0) AS count_pm_unverified
                FROM project p
                LEFT JOIN project_pm pm ON (pm.project_id = p.id)
                LEFT JOIN data_pm_verificated dpmv ON (dpmv.project_id = p.id)
                WHERE p.vendor_id = $1 AND p.name ILIKE CONCAT('%', $2, '%')
                GROUP BY p.id, dpmv.count_pm_verified;",
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
pub async fn get_list_pm_u(
    state: Data<AppState>,
    query_parameter: Query<PMQuery>,
    claims: AuthClaims
) -> impl Responder {

    let user_id = claims.0.sub;
    let project_id = query_parameter.project_id;

    // get project
    let project_detail = match sqlx::query_as::<_, ProjectDto>(
        "WITH data_pm_verificated AS (
                    SELECT COUNT(id) AS count_pm_verified, project_id
                    FROM project_pm
                    WHERE is_verified
                    GROUP BY project_id
                )
                SELECT p.id,
                    p.vendor_id,
                    p.name,
                    p.description,
                    p.pic_name,
                    p.pic_email,
                    p.pic_number,
                    p.pm_count,
                    CAST(p.created_at AS TEXT) AS created_at,
                    CAST(p.updated_at AS TEXT) AS updated_at,
                    COALESCE(COUNT(pm.id), 0) AS count_pm_uploaded,
                    COALESCE(dpmv.count_pm_verified, 0) AS count_pm_verified,
                    COALESCE(COUNT(pm.id), 0) - COALESCE(dpmv.count_pm_verified, 0) AS count_pm_unverified
                FROM project p
                LEFT JOIN project_pm pm ON (pm.project_id = p.id)
                LEFT JOIN data_pm_verificated dpmv ON (dpmv.project_id = p.id)
                LEFT JOIN users_vendor uv ON (uv.vendor_id = p.vendor_id)
                WHERE p.id = $1 AND uv.user_id = CAST($2 AS UUID)
                GROUP BY p.id, dpmv.count_pm_verified;",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No project found with specified ID and User ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

    match sqlx::query_as::<_, ProjectPMDto>(
                "SELECT id,
                        project_id,
                        pm_order,
                        pm_description,
                        url_file,
                        is_verified,
                        CAST(verified_at AS TEXT) as verified_at,
                        CAST(created_at AS TEXT) AS created_at
                FROM project_pm
                WHERE project_id = $1
                ORDER BY pm_order
                ",
            )
                .bind(project_id)
                .fetch_all(&state.postgres)
                .await
            {
                    Ok(pms) => {
                        let response = 
                        json!({
                            "project": project_detail,
                            "pm": pms
                        });
                        HttpResponse::Ok().json(response)
                    }
                    Err(error) => {
                        HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
                    }
                }
}

#[put("/vendor")]
pub async fn put_edit_vendor_u(
    state: Data<AppState>,
    body: Json<Vendor>,
    claims: AuthClaims
) -> impl Responder {

    // Validation
    let user_id = claims.0.sub;
    let vendor_id = body.id;

    // Validate vendor
    match sqlx::query_as::<_, VendorValidationDto>(
        "SELECT v.id
            FROM vendor v
            LEFT JOIN users_vendor uv ON (uv.vendor_id = v.id)
            WHERE v.id = $1 AND uv.user_id = CAST($2 AS UUID)",
    )
    .bind(vendor_id)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No vendor found with specified ID and User ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

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
             updated_at = NOW()
         WHERE id = $1
         RETURNING id, name, address, email, phone_number, 
                   CAST(created_at AS TEXT) AS created_at,
                   CAST(updated_at AS TEXT) AS updated_at"
    )
    .bind(&body.id)
    .bind(&body.name)
    .bind(&body.address)
    .bind(&body.email)
    .bind(&body.phone_number)
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
pub async fn post_create_vendor_project_u(
    state: Data<AppState>,
    body: Json<Project>,
    claims: AuthClaims,
) -> impl Responder {

    // Validation
    let user_id = claims.0.sub;
    let vendor_id = body.vendor_id;

    // Validate vendor
    match sqlx::query_as::<_, VendorDto>(
        "SELECT v.id
            FROM vendor v
            LEFT JOIN users_vendor uv ON (uv.vendor_id = v.id)
            WHERE v.id = $1 AND uv.user_id = CAST($2 AS UUID)",
    )
    .bind(vendor_id)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No vendor found with specified ID and User ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

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
            (vendor_id, name, description, pic_name, pic_email, pic_number, pm_count) 
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, vendor_id, name, description, pic_name, pic_email, pic_number, pm_count",
    )
    .bind(body.vendor_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.pic_name)
    .bind(&body.pic_email)
    .bind(&body.pic_number)
    .bind(body.pm_count)
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
pub async fn put_edit_vendor_project_u(
    state: Data<AppState>,
    body: Json<Project>,
    claims: AuthClaims,
) -> impl Responder {

    // Validation
    let user_id = claims.0.sub;
    let vendor_id = body.vendor_id;

    // Validate vendor
    match sqlx::query_as::<_, VendorValidationDto>(
        "SELECT v.id
            FROM vendor v
            LEFT JOIN users_vendor uv ON (uv.vendor_id = v.id)
            WHERE v.id = $1 AND uv.user_id = CAST($2 AS UUID)",
    )
    .bind(vendor_id)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No vendor found with specified ID and User ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

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
             pic_number  = $7,
             pm_count    = $8,
             updated_at  = NOW()
         WHERE id = $1
         RETURNING id, vendor_id, name, description, pic_name, pic_email, pic_number, pm_count"
    )
    .bind(&body.id)
    .bind(&body.vendor_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.pic_name)
    .bind(&body.pic_email)
    .bind(&body.pic_number)
    .bind(&body.pm_count)
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

#[post("/pm")]
pub async fn post_create_project_pm_u(
    state: Data<AppState>,
    body: Json<ProjectPM>,
    claims: AuthClaims,
) -> impl Responder {

    // Validation
    let user_id = claims.0.sub;
    let project_id = body.project_id;

    // Validate vendor
    match sqlx::query_as::<_, ProjectValidationDto>(
        "SELECT p.id
            FROM project p 
            LEFT JOIN users_vendor uv ON (uv.vendor_id = v.id)
            LEFT JOIN vendor v ON (v.id = p.vendor_id)
            WHERE p.id = $1 AND uv.user_id = CAST($2 AS UUID)",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No project found with specified ID and User ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

    // 1. Begin new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // 2. Insert a new Project PM
    match sqlx::query_as::<_, ProjectPM>(
        "INSERT INTO project_pm (project_id, pm_order, pm_description, url_file)
         VALUES ($1, $2, $3, $4)
         RETURNING id, project_id, pm_order, pm_description, url_file"
    )
    .bind(&body.project_id)
    .bind(&body.pm_order)
    .bind(&body.pm_description)
    .bind(&body.url_file)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(pm) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": format!("PM step {} added to project {}.", pm.pm_order, pm.project_id),
                    "project_pm": pm,
                })),
                Err(e) => HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to commit transaction: {}", e)
                }))
            }
        }
        Err(error) => {
            // 4. Rollback on failure
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to add project PM step: {}", error)
            }))
        }
    }
}

#[put("/pm")]
pub async fn put_edit_project_pm_u(
    state: Data<AppState>,
    body: Json<ProjectPM>,
    claims: AuthClaims
) -> impl Responder {

    // Validation
    let user_id = claims.0.sub;
    let project_id = body.project_id;

    // Validate vendor
    match sqlx::query_as::<_, ProjectValidationDto>(
        "SELECT p.id
            FROM project p
            LEFT JOIN users_vendor uv ON (uv.vendor_id = v.id)
            LEFT JOIN vendor v ON (v.id = p.vendor_id)
            WHERE p.id = $1 AND uv.user_id = CAST($2 AS UUID)",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await
    {
        Ok(Some(vendor)) => vendor,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ "error": "No project found with specified ID and User ID."  }))
        }
        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    };

    // Require ID for update
    if body.id.is_none() {
        return HttpResponse::BadRequest().json(json!({
            "error": "PM ID is required for update."
        }));
    }

    // 1. Begin transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // 2. Perform update
    match sqlx::query_as::<_, ProjectPM>(
        "UPDATE project_pm
         SET project_id     = $2,
             pm_order       = $3,
             pm_description = $4,
             url_file       = $5
         WHERE id = $1
         RETURNING id, project_id, pm_order, pm_description, url_file"
    )
    .bind(&body.id)
    .bind(&body.project_id)
    .bind(&body.pm_order)
    .bind(&body.pm_description)
    .bind(&body.url_file)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(pm) => {
            // 3. Commit
            match transaction.commit().await {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": format!("PM ID {} updated successfully.", pm.id.unwrap()),
                    "project_pm": pm,
                })),
                Err(e) => HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to commit transaction: {}", e)
                }))
            }
        }
        Err(error) => {
            // 4. Rollback
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to update project PM: {}", error)
            }))
        }
    }
}
