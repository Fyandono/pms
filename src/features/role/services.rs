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
use crate::features::user::services::{AuthClaims};
use crate::features::role::model::{Role, RoleDto, RoleQuery};
use crate::util::page_response_builder::{page_response_builder};
use crate::util::require_role::{require_role};

#[post("/role")]
pub async fn post_create_role(
    state: Data<AppState>,
    body: Json<Role>, // Use Role for the incoming body
    claims: AuthClaims,
) -> impl Responder {

    // Get user id
    let user_id = claims.0.sub;

    // 1. Begin new transaction
    let mut transaction = match state.postgres.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // 2. Insert a new Role
    // Using direct access to properties on the body (e.g., body.name)
    match sqlx::query_as::<_, RoleDto>(
        "INSERT INTO role (
            name, can_add_role, can_edit_role, can_add_user, can_edit_user, 
            can_add_vendor, can_edit_vendor, can_add_project, can_edit_project, 
            can_add_pm, can_edit_pm, can_verify_pm, can_add_unit, can_edit_unit, is_active, created_at, created_by
        )
         VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW(), CAST($16 AS UUID)
         )
         RETURNING 
            id, name, can_add_role, can_edit_role, can_add_user, can_edit_user, 
            can_add_vendor, can_edit_vendor, can_add_project, can_edit_project, 
            can_add_pm, can_edit_pm, can_verify_pm, can_add_unit, can_edit_unit, is_active, CAST(created_at AS TEXT) AS created_at, CAST(created_by AS TEXT) AS created_by, CAST(updated_at AS TEXT) AS updated_at, CAST(updated_by AS TEXT) AS updated_by"
    )
    .bind(&body.name)
    .bind(body.can_add_role)
    .bind(body.can_edit_role)
    .bind(body.can_add_user)
    .bind(body.can_edit_user)
    .bind(body.can_add_vendor)
    .bind(body.can_edit_vendor)
    .bind(body.can_add_project)
    .bind(body.can_edit_project)
    .bind(body.can_add_pm)
    .bind(body.can_edit_pm)
    .bind(body.can_verify_pm)
    .bind(body.can_add_unit)
    .bind(body.can_edit_unit)
    .bind(body.is_active)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(role) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": "Role successfully created.".to_string(),
                    "role": role,
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
                "error": format!("Failed to create role: {}", error)
            }))
        }
    }
}

#[put("/role")]
pub async fn put_edit_role(
    state: Data<AppState>,
    body: Json<Role>, // Uses the Role struct for the body
    claims: AuthClaims,
) -> impl Responder {

    // Get user id
    let user_id = claims.0.sub;

    // Ensure the ID is present for an update operation
    let role_id = match body.id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(json!({
                "error": "The role ID is required for editing a role."
            }));
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

    // 2. Execute the UPDATE query for the 'role' table
    match sqlx::query_as::<_, RoleDto>(
        "UPDATE role 
         SET 
            name = $2, 
            can_add_role = $3, 
            can_edit_role = $4, 
            can_add_user = $5, 
            can_edit_user = $6, 
            can_add_vendor = $7, 
            can_edit_vendor = $8, 
            can_add_project = $9, 
            can_edit_project = $10, 
            can_add_pm = $11, 
            can_edit_pm = $12, 
            can_verify_pm = $13, 
            can_add_unit = $14,
            can_edit_unit = $15,
            is_active = $16,
            updated_at = NOW(),
            updated_by = CAST($17 AS UUID)
         WHERE id = $1
         RETURNING 
            id, name, can_add_role, can_edit_role, can_add_user, can_edit_user, 
            can_add_vendor, can_edit_vendor, can_add_project, can_edit_project, 
            can_add_pm, can_edit_pm, can_verify_pm, can_add_unit, can_edit_unit, is_active, CAST(created_at AS TEXT) AS created_at, CAST(created_by AS TEXT) AS created_by, CAST(updated_at AS TEXT) AS updated_at, CAST(updated_by AS TEXT) AS updated_by"
    )
    .bind(role_id) 
    .bind(&body.name)
    .bind(body.can_add_role) 
    .bind(body.can_edit_role)
    .bind(body.can_add_user) 
    .bind(body.can_edit_user) 
    .bind(body.can_add_vendor) 
    .bind(body.can_edit_vendor) 
    .bind(body.can_add_project)
    .bind(body.can_edit_project) 
    .bind(body.can_add_pm)
    .bind(body.can_edit_pm) 
    .bind(body.can_verify_pm) 
    .bind(body.can_add_unit)
    .bind(body.can_edit_unit)
    .bind(body.is_active)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(role) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Ok().json(json!({ // HTTP 200 OK
                    "message": "Role successfully updated.".to_string(),
                    "role": role,
                })),
                Err(e) => HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to commit transaction: {}", e)
                }))
            }
        }
        Err(error) => {
            // 4. Rollback on failure
            let _ = transaction.rollback().await;
            
            // Handle case: Role ID not found
            if let sqlx::Error::RowNotFound = &error {
                 return HttpResponse::NotFound().json(json!({
                    "error": format!("Role with ID {} not found.", role_id)
                }));
            }

            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to update role: {}", error)
            }))
        }
    }
}

#[get("/role")]
pub async fn get_role(
    state: Data<AppState>,
    claims: AuthClaims,
    query_parameter: Query<RoleQuery>
) -> impl Responder {

    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;
    let is_active = query_parameter.is_active.clone();

    match sqlx::query_as::<_, RoleDto>(
        "SELECT  r.id, r.name, r.can_add_role, r.can_edit_role, r.can_add_user, r.can_edit_user, 
            r.can_add_vendor, r.can_edit_vendor, r.can_add_project, r.can_edit_project, 
            r.can_add_pm, r.can_edit_pm, can_add_unit, can_edit_unit, r.can_verify_pm, r.is_active, CAST(r.created_at AS TEXT), c.username AS created_by, CAST(r.updated_at AS TEXT), u.username AS updated_by
        FROM role r
        LEFT JOIN users c ON (c.id = r.created_by)
        LEFT JOIN users u ON (u.id = r.updated_by)
        WHERE r.name ILIKE CONCAT('%', $1, '%') AND ($2 IS NULL OR r.is_active = CAST($2 AS BOOL))",
    )
    .bind(name_filter)
    .bind(is_active)
    .fetch_all(&state.postgres)
    .await
    {
        Ok(roles) => {
            let response = page_response_builder(page, page_size, &roles);
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    }
}