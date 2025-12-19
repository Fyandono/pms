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

#[post("/role")]
pub async fn post_create_role(
    state: Data<AppState>,
    body: Json<Role>,
    claims: AuthClaims,
) -> impl Responder {

    let user_id = claims.0.sub;

    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // MySQL INSERT: Use '?' placeholders and remove RETURNING
    match sqlx::query(
        "INSERT INTO role (
                name, can_add_role, can_edit_role, can_add_user, can_edit_user, 
                can_add_vendor, can_edit_vendor, can_add_project, can_edit_project, 
                can_add_pm, can_edit_pm, can_verify_pm, can_add_unit, can_edit_unit, 
                can_get_vendor, can_get_user, can_get_unit, can_get_role, can_get_project, can_get_pm,
                is_active, created_at, created_by
            )
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(), ?
            )"
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
    .bind(body.can_get_vendor)
    .bind(body.can_get_user)
    .bind(body.can_get_unit)
    .bind(body.can_get_role)
    .bind(body.can_get_project)
    .bind(body.can_get_pm)
    .bind(body.is_active)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    {
        Ok(_) => {
            // Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": "Role successfully created.".to_string(),
                })),
                Err(e) => HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to commit transaction: {}", e)
                }))
            }
        }
        Err(error) => {
            // Rollback on failure
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
    body: Json<Role>,
    claims: AuthClaims,
) -> impl Responder {

    let user_id = claims.0.sub;

    let role_id = match body.id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(json!({
                "error": "The role ID is required for editing a role."
            }));
        }
    };

    let mut transaction = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to start transaction: {}", e)
            }))
        }
    };

    // MySQL UPDATE: Use '?' placeholders and remove RETURNING
    match sqlx::query(
        "UPDATE role 
         SET 
            name = ?, 
            can_add_role = ?, 
            can_edit_role = ?, 
            can_add_user = ?, 
            can_edit_user = ?, 
            can_add_vendor = ?, 
            can_edit_vendor = ?, 
            can_add_project = ?, 
            can_edit_project = ?, 
            can_add_pm = ?, 
            can_edit_pm = ?, 
            can_verify_pm = ?, 
            can_add_unit = ?,
            can_edit_unit = ?,
            can_get_vendor = ?,
            can_get_user = ?,
            can_get_unit = ?,
            can_get_role = ?,
            can_get_project = ?,
            can_get_pm = ?,
            is_active = ?,
            updated_at = NOW(),
            updated_by = ?
         WHERE id = ?"
    )
    .bind(&body.name) // 1
    .bind(body.can_add_role) // 2
    .bind(body.can_edit_role) // 3
    .bind(body.can_add_user) // 4
    .bind(body.can_edit_user) // 5
    .bind(body.can_add_vendor) // 6
    .bind(body.can_edit_vendor) // 7
    .bind(body.can_add_project) // 8
    .bind(body.can_edit_project) // 9
    .bind(body.can_add_pm) // 10
    .bind(body.can_edit_pm) // 11
    .bind(body.can_verify_pm) // 12
    .bind(body.can_add_unit) // 13
    .bind(body.can_edit_unit) // 14
    .bind(body.can_get_vendor) // 15
    .bind(body.can_get_user) // 16
    .bind(body.can_get_unit) // 17
    .bind(body.can_get_role) // 18
    .bind(body.can_get_project) // 19
    .bind(body.can_get_pm) // 20
    .bind(body.is_active) // 21
    .bind(user_id) // 22
    .bind(role_id) // 23
    .execute(&mut *transaction)
    .await
    {
        Ok(result) => {
            // Check if any row was affected
            if result.rows_affected() == 0 {
                let _ = transaction.rollback().await;
                return HttpResponse::NotFound().json(json!({
                    "error": format!("Role with ID {} not found.", role_id)
                }));
            }

            // Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": "Role successfully updated.".to_string(),
                })),
                Err(e) => HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to commit transaction: {}", e)
                }))
            }
        }
        Err(error) => {
            // Rollback on failure
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to update role: {}", error)
            }))
        }
    }
}

#[get("/role")]
pub async fn get_role(
    state: Data<AppState>,
    query_parameter: Query<RoleQuery>
) -> impl Responder {

    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;
    let is_active = query_parameter.is_active.clone();
    let is_report = query_parameter.is_report;

    let query_str = "SELECT r.id, r.name, r.can_add_role, r.can_edit_role, r.can_add_user, r.can_edit_user, 
                     r.can_add_vendor, r.can_edit_vendor, r.can_add_project, r.can_edit_project, 
                     r.can_add_pm, r.can_edit_pm, can_add_unit, can_edit_unit, r.can_verify_pm,
                     r.can_get_user, r.can_get_vendor, r.can_get_unit, r.can_get_role, r.can_get_project, r.can_get_pm,
                     r.is_active, CAST(r.created_at AS CHAR) AS created_at, c.username AS created_by, CAST(r.updated_at AS CHAR) AS updated_at, u.username AS updated_by
                     FROM role r
                     LEFT JOIN users c ON (c.id = r.created_by)
                     LEFT JOIN users u ON (u.id = r.updated_by)
                     WHERE r.name LIKE CONCAT('%', ?, '%') 
                       AND (? IS NULL OR r.is_active = ?)";

    // To implement the ? IS NULL OR column = ? pattern, we must bind the is_active value twice.
    let mut query = sqlx::query_as::<_, RoleDto>(query_str);

    // 1. Bind name filter (first ?)
    query = query.bind(name_filter);

    // 2. Bind is_active twice (second and third ?)
    // Binding an Option<bool> directly works for both the NULL check and the comparison in MySQL.
    query = query.bind(is_active.clone()); 
    query = query.bind(is_active); 

    match query
        .fetch_all(&state.db)
        .await
    {
        Ok(roles) => {
             let response = if is_report.unwrap_or(false) {
                    json!({ "data": roles })
                } else {
                    page_response_builder(page, 
                        page_size, 
                        &roles)
                };
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error) }))
        }
    }
}