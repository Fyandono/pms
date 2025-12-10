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

        // Get User ID
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

        // 2. Insert a new Project PM
        match sqlx::query_as::<_, UnitDto>(
            "INSERT INTO unit (name, is_active)
            VALUES ($1, $2, NOW(), CAST($3 AS UUID))
            RETURNING id, name, is_active, CAST(created_at AS TEXT) AS created_at, CAST(created_by AS TEXT) AS created_by, CAST(updated_at AS TEXT) AS updated_at, CAST(updated_by AS TEXT) AS updated_by"
        )
        .bind(&body.name)
        .bind(&body.is_active)
        .bind(user_id)
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

        // Get User ID
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

        // 2. Insert a new Project PM
        match sqlx::query_as::<_, UnitDto>(
            "UPDATE unit 
            SET name = $2,
                is_active = $3,
                updated_at = NOW(),
                updated_by = CAST($4 AS UUID)
            WHERE id = $1
            RETURNING id, name, is_active, CAST(created_at AS TEXT) AS created_at, CAST(created_by AS TEXT) AS created_by, CAST(updated_at AS TEXT) AS updated_at, CAST(updated_by AS TEXT) AS updated_by"
        )
        .bind(&body.id)
        .bind(&body.name)
        .bind(&body.is_active)
        .bind(user_id)
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
        query_parameter: Query<UnitQuery>
    ) -> impl Responder {

        let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
        let page = query_parameter.page;
        let page_size = query_parameter.page_size;
        let is_active = query_parameter.is_active.clone();

        match sqlx::query_as::<_, UnitDto>(
            "SELECT un.id, un.name, un.is_active, c.username AS created_by, CAST(un.created_at AS TEXT) AS created_at, u.username AS updated_by, CAST(un.updated_at AS TEXT) AS updated_at
            FROM unit un
            LEFT JOIN users c ON (c.id = un.created_by)
            LEFT JOIN users u ON (u.id = un.updated_by)
            WHERE un.name ILIKE CONCAT('%', $1, '%') AND ($2 IS NULL OR un.is_active = CAST($2 AS BOOL))
            ORDER BY un.name",
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