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

#[post("/unit")]
pub async fn post_create_unit(
    state: Data<AppState>,
    body: Json<Unit>,
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

    // MySQL INSERT: Use '?' placeholders, include all columns, and remove RETURNING
    match sqlx::query(
        "INSERT INTO unit (name, is_active, created_at, created_by)
         VALUES (?, ?, NOW(), ?)"
    )
    .bind(&body.name)
    .bind(&body.is_active)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    {
        Ok(_) => {
            // Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Created().json(json!({
                    "message": "Unit successfully created.".to_string(),
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
                "error": format!("Failed to create unit: {}", error)
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

    let user_id = claims.0.sub;

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
        "UPDATE unit 
         SET name = ?,
             is_active = ?,
             updated_at = NOW(),
             updated_by = ?
         WHERE id = ?"
    )
    .bind(&body.name)
    .bind(&body.is_active)
    .bind(user_id)
    .bind(&body.id)
    .execute(&mut *transaction)
    .await
    {
        Ok(result) => {
             if result.rows_affected() == 0 {
                let _ = transaction.rollback().await;
                return HttpResponse::NotFound().json(json!({
                    "error": "Unit not found or no changes were made."
                }));
            }
            // Commit the transaction
            match transaction.commit().await {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": "Unit successfully updated.".to_string(),
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
                "error": format!("Failed to update unit: {}", error)
            }))
        }
    }
}

#[get("/unit")]
pub async fn get_unit(
    state: Data<AppState>,
    query_parameter: Query<UnitQuery>
) -> impl Responder {

    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;
    let is_active = query_parameter.is_active.clone();

    // MySQL SELECT: Use '?' placeholders, LIKE CONCAT for filtering, and the robust IS NULL pattern
    let query_str = "SELECT un.id, un.name, un.is_active, 
                     c.username AS created_by, 
                     CAST(un.created_at AS CHAR) AS created_at, 
                     u.username AS updated_by, 
                     CAST(un.updated_at AS CHAR) AS updated_at
                     FROM unit un
                     LEFT JOIN users c ON (c.id = un.created_by)
                     LEFT JOIN users u ON (u.id = un.updated_by)
                     WHERE un.name LIKE CONCAT('%', ?, '%') 
                     AND (? IS NULL OR un.is_active = ?)
                     ORDER BY un.name";

    let mut query = sqlx::query_as::<_, UnitDto>(query_str);
    
    // Bind the name filter
    query = query.bind(name_filter);
    
    // Bind is_active twice for the (? IS NULL OR un.is_active = ?) pattern
    query = query.bind(is_active.clone()); 
    query = query.bind(is_active); 

    match query
        .fetch_all(&state.db)
        .await
    {
        Ok(units) => {
            let response = page_response_builder(page, page_size, &units);
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error) }))
        }
    }
}