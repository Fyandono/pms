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
use crate::features::unit::model::{Unit, UnitDto, UnitQuery};
use crate::util::page_response_builder::{page_response_builder};
use crate::util::require_role::{require_role};

#[post("/unit")]
pub async fn post_create_unit(
    state: Data<AppState>,
    body: Json<Unit>,
    claims: AuthClaims,
) -> impl Responder {

    // Role validator
    if !require_role(&claims.0, &["admin"]) {
        return HttpResponse::Forbidden().json(json!({"message":"You have no access to this feature.\nPlease contact admin for further information."}));
    }

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
    match sqlx::query_as::<_, UnitDto>(
        "INSERT INTO unit (name, is_active)
         VALUES ($1, $2)
         RETURNING id, name, is_active"
    )
    .bind(&body.name)
    .bind(&body.is_active)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(unit) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": "Unit successfully created.".to_string(),
                    "unit": unit,
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


#[put("/unit")]
pub async fn put_edit_unit(
    state: Data<AppState>,
    body: Json<Unit>,
    claims: AuthClaims,
) -> impl Responder {

    // Role validator
    if !require_role(&claims.0, &["admin"]) {
        return HttpResponse::Forbidden().json(json!({"message":"You have no access to this feature.\nPlease contact admin for further information."}));
    }

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
    match sqlx::query_as::<_, UnitDto>(
        "UPDATE unit 
         SET name = $2,
             is_active = $3
         WHERE id = $1
         RETURNING id, name, is_active"
    )
    .bind(&body.id)
    .bind(&body.name)
    .bind(&body.is_active)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(unit) => {
            // 3. Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": "Unit successfully created.".to_string(),
                    "unit": unit,
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

#[get("/unit")]
pub async fn get_unit(
    state: Data<AppState>,
    claims: AuthClaims,
    query_parameter: Query<UnitQuery>
) -> impl Responder {

    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;
    let is_active = query_parameter.is_active.clone();

    match sqlx::query_as::<_, UnitDto>(
        "SELECT id, name, is_active
        FROM unit
        WHERE name ILIKE CONCAT('%', $1, '%') AND ($2 IS NULL OR is_active = CAST($2 AS BOOL))",
    )
    .bind(name_filter)
    .bind(is_active)
    .fetch_all(&state.postgres)
    .await
    {
        Ok(units) => {
            let response = page_response_builder(page, page_size, &units);
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error)  }))
        }
    }
}