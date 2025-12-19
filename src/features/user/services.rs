use crate::AppState;
use crate::features::user::model::{Claims, LoginRequest, RegisterRequest, EditUserRequest, UserLoginDto, UserQuery, UserDto, ChangePasswordRequest};
use crate::util::page_response_builder::page_response_builder;
use actix_web::FromRequest;
use actix_web::dev::Payload;
use actix_web::http::header::AUTHORIZATION;
use actix_web::{
    Error, HttpResponse, Responder, post, put, get,
    web::{Data, Json, Query},
};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString, PasswordHash, PasswordVerifier};
use futures::future::{Ready, ready};
use jsonwebtoken::{DecodingKey, Validation, decode};
use argon2::password_hash::rand_core::OsRng;
use std::env;
use uuid::Uuid;
use serde_json::json;
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};

#[get("/user")]
pub async fn get_user(
    state: Data<AppState>,
    query_parameter: Query<UserQuery>
) -> impl Responder {

    let name_filter = query_parameter.name.clone().unwrap_or("".to_string());
    let page = query_parameter.page;
    let page_size = query_parameter.page_size;
    let is_report = query_parameter.is_report;
    
    match sqlx::query_as::<_, UserDto>(
        "SELECT a.id, 
        a.name, 
        a.username,
        r.name AS role, 
        r.id AS role_id,
        a.is_active, 
        CAST(a.created_at AS CHAR) AS created_at, 
        c.username AS created_by,
        CAST(a.updated_at AS CHAR) AS updated_at, 
        u.username AS updated_by
        FROM users a
        LEFT JOIN users c ON (c.id = a.created_by)
        LEFT JOIN users u ON (u.id = a.updated_by)
        LEFT JOIN role r ON (r.id = a.role_id)
        WHERE a.username LIKE CONCAT('%', ?, '%') OR a.name LIKE CONCAT('%', ?, '%')
        ORDER BY a.name, a.is_active DESC",
    )
    .bind(&name_filter)
    .bind(&name_filter)
    .fetch_all(&state.db)
    .await
    {
        Ok(users) => {
            let response = if is_report.unwrap_or(false) {
                    json!({ "data": users })
                } else {
                    page_response_builder(page, 
                        page_size, 
                        &users)
                };
            HttpResponse::Ok().json(response)
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({ "error": format!("{}", error) }))
        }
    }
}

#[post("/register")]
async fn register(state: Data<AppState>, payload: Json<RegisterRequest>, claims: AuthClaims) -> impl Responder {

    let user_id = claims.0.sub.to_string();
    let username = payload.username.trim().to_lowercase();
    let name = payload.name.trim();
    let role_id = payload.role_id;
    let is_active = payload.is_active;

    if name.is_empty() || username.is_empty() || payload.password.len() < 8  {
        return HttpResponse::BadRequest().json(json!({ "message": "invalid name or invalid username or password too short." }))
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map(|ph| ph.to_string());

    if password_hash.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    let password_hash = password_hash.unwrap();

    let new_id = Uuid::new_v4().to_string();

    let res = sqlx::query(
        "INSERT INTO users (id, username, password_hash, name, role_id, is_active, created_at, created_by) 
         VALUES (?, ?, ?, ?, ?, ?, NOW(), ?)",
    )
    .bind(&new_id)
    .bind(&username)
    .bind(&password_hash)
    .bind(&name)
    .bind(&role_id)
    .bind(&is_active)
    .bind(user_id)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok()
            .json(json!({ "message": "User successfully created."})),
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({"error":e.to_string()}));
        }
    }
}

#[put("/edit-user")]
pub async fn update_user(
    state: Data<AppState>,
    payload: Json<EditUserRequest>,
    claims: AuthClaims,
) -> impl Responder {

    let user_id = claims.0.sub.to_string();

    let user_to_edit_id = payload.id.to_string();

    let username = payload.username.as_ref().map(|s| s.trim().to_lowercase());
    let name = payload.name.as_ref().map(|s| s.trim().to_string());
    let role_id = payload.role_id;
    
    let is_active: Option<bool> = payload.is_active;
    
    let password_hash: Option<String> = match &payload.password {
        Some(pw) if pw.len() >= 8 => {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            
            match argon2.hash_password(pw.as_bytes(), &salt) {
                Ok(ph) => Some(ph.to_string()),
                Err(e) => {
                    eprintln!("Password hashing failed: {}", e);
                    return HttpResponse::InternalServerError().json(json!({ "message": "Failed to hash password" }));
                }
            }
        },
        Some(pw) if pw.len() < 8 => {
            return HttpResponse::BadRequest().json(json!({ "message": "Password must be at least 8 characters long." }));
        }
        _ => None,
    };

    let res = sqlx::query(
        "
         UPDATE users SET 
             username = COALESCE(?, username),
             name = COALESCE(?, name),
             role_id = COALESCE(?, role_id),
             password_hash = COALESCE(?, password_hash),
             is_active = COALESCE(?, is_active),
             updated_at = NOW(),
             updated_by = ?
         WHERE id = ?
         ",
    )
    .bind(username.as_deref()) 
    .bind(name.as_deref()) 
    .bind(role_id)
    .bind(password_hash.as_deref()) 
    .bind(is_active)
    .bind(user_id)
    .bind(user_to_edit_id)
    .fetch_optional(&state.db)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok()
            .json(json!({ "message": "User successfully updated." })),
        Err(e) => {
            eprintln!("Database error during user update: {}", e);
            HttpResponse::InternalServerError().json(json!({ "message": "Could not update user." }))
        }
    }
}

#[post("/login")]
async fn login(state: Data<AppState>, payload: Json<LoginRequest>) -> impl Responder {

    let username = payload.username.trim().to_lowercase();

    let row = sqlx::query_as::<_, UserLoginDto>("
    SELECT 
        u.id, 
        u.name, 
        u.username,
        u.password_hash,
        u.role_id, 
        r.name AS role,
        u.is_active,
        r.can_add_role,
        r.can_edit_role,
        r.can_add_user,
        r.can_edit_user,
        r.can_add_vendor,  
        r.can_edit_vendor,
        r.can_add_project, 
        r.can_edit_project,
        r.can_add_pm,
        r.can_edit_pm,
        r.can_verify_pm,
        r.can_add_unit,
        r.can_edit_unit,
        r.can_get_vendor,
        r.can_get_user,
        r.can_get_unit,
        r.can_get_role,
        r.can_get_project,
        r.can_get_pm
    FROM users u 
    LEFT JOIN role r ON (r.id = u.role_id)
    WHERE u.username = ?")
        .bind(&username)
        .fetch_one(&state.db)
        .await;

    let user = match row {
        Ok(u) => u,
        Err(sqlx::Error::RowNotFound) => return HttpResponse::Unauthorized().json(json!({ "message": "Invalid credentials." })),
        Err(_) => return HttpResponse::InternalServerError().json(json!({ "message": "Database error during login." })),
    };

    if !user.is_active {
        return HttpResponse::Forbidden().json(json!({ "message": "Account is not active." }));
    }

    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(ph) => ph,
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    };

    let argon2 = Argon2::default();
    if argon2
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return HttpResponse::Unauthorized().json(json!({ "message": "Invalid credentials." }));
    }

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let exp_seconds: i64 = env::var("JWT_EXP_SECONDS")
        .unwrap_or_else(|_| "3600".to_string())
        .parse()
        .unwrap_or(3600);

    let expiration = Utc::now() + Duration::seconds(exp_seconds);

    let claims = Claims {
        sub: user.id.to_string(),
        name: user.name.to_string(),
        username: user.username.clone(),
        role: user.role,
        role_id: user.role_id.clone(),
        exp: expiration.timestamp() as usize,
        can_add_role: user.can_add_role,
        can_edit_role: user.can_edit_role,
        can_add_user: user.can_add_user,
        can_edit_user: user.can_edit_user,
        can_add_vendor: user.can_add_vendor,
        can_edit_vendor: user.can_edit_vendor,
        can_add_project: user.can_add_project,
        can_edit_project: user.can_edit_project,
        can_add_pm: user.can_add_pm,
        can_edit_pm: user.can_edit_pm,
        can_verify_pm: user.can_verify_pm,
        can_add_unit: user.can_add_unit,
        can_edit_unit: user.can_edit_unit,
        can_get_vendor: user.can_get_vendor,
        can_get_user: user.can_get_user,
        can_get_unit: user.can_get_unit,
        can_get_role: user.can_get_role,
        can_get_project: user.can_get_project,
        can_get_pm: user.can_get_pm
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    );

    match token {
        Ok(t) => HttpResponse::Ok().json(serde_json::json!({
            "access_token": t,
            "token_type": "bearer",
            "expires_in": exp_seconds
        })),
        Err(_) => {
            HttpResponse::InternalServerError().json(json!({ "message": "Could not create token." }))
        }
    }
}


#[put("/change-password")]
async fn change_password(state: Data<AppState>, payload: Json<ChangePasswordRequest>, claims: AuthClaims) -> impl Responder {

    let user_id = claims.0.sub.to_string();

    let row = sqlx::query_as::<_, UserLoginDto>("
    SELECT 
        u.id, 
        u.name, 
        u.username,
        u.password_hash,
        u.role_id, 
        r.name AS role,
        u.is_active,
        r.can_add_role,
        r.can_edit_role,
        r.can_add_user,
        r.can_edit_user,
        r.can_add_vendor,
        r.can_edit_vendor,
        r.can_add_project,
        r.can_edit_project,
        r.can_add_pm,
        r.can_edit_pm,
        r.can_verify_pm,
        r.can_add_unit,
        r.can_edit_unit,
        r.can_get_vendor,
        r.can_get_user,
        r.can_get_unit,
        r.can_get_role,
        r.can_get_project,
        r.can_get_pm
    FROM users u 
    LEFT JOIN role r ON (r.id = u.role_id)
    WHERE u.id = ?")
        .bind(&user_id)
        .fetch_one(&state.db)
        .await;

    let user = match row {
        Ok(u) => u,
        Err(sqlx::Error::RowNotFound) => return HttpResponse::Unauthorized().json(json!({ "message": "Invalid credentials." })),
        Err(_) => return HttpResponse::InternalServerError().json(json!({ "message": "Database error during login." })),
    };

    if !user.is_active {
        return HttpResponse::Forbidden().json(json!({ "message": "Account is not active." }));
    }

    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(ph) => ph,
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    };

    let argon2 = Argon2::default();
    if argon2
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return HttpResponse::Unauthorized().json(json!({ "message": "Invalid credentials." }));
    }

    if payload.new_password == payload.password {
    return HttpResponse::BadRequest().json(json!({ 
        "message": "New password cannot be the same as old password." 
    }));
}
    let password_hash: Option<String> = match &payload.new_password {
        pw if pw.len() >= 8 => {
            let salt = SaltString::generate(&mut OsRng);
            match argon2.hash_password(pw.as_bytes(), &salt) {
                Ok(ph) => Some(ph.to_string()),
                Err(e) => {
                    eprintln!("Password hashing failed: {}", e);
                    return HttpResponse::InternalServerError().json(json!({ "message": "Failed to hash password" }));
                }
            }
        },
        pw if pw.len() < 8 => {
            return HttpResponse::BadRequest().json(json!({ "message": "Password must be at least 8 characters long." }));
        }
        _ => None,
    };

    let res = sqlx::query(
        "
         UPDATE users SET 
             password_hash = ?
         WHERE id = ?
         ",
    )
    .bind(password_hash.as_deref())
    .bind(user_id)
    .fetch_optional(&state.db)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok()
            .json(json!({ "message": "Password successfully updated." })),
        Err(e) => {
            eprintln!("Database error during user update: {}", e);
            HttpResponse::InternalServerError().json(json!({ "message": "Could not update user." }))
        }
    }
}

pub struct AuthClaims(pub(crate) Claims);

impl FromRequest for AuthClaims {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Read header
        let header = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());
        if header.is_none() {
            return ready(Err(actix_web::error::ErrorUnauthorized(
                "Missing Authorization header",
            )));
        }
        let header = header.unwrap();
        if !header.starts_with("Bearer ") {
            return ready(Err(actix_web::error::ErrorUnauthorized(
                "Invalid Authorization header",
            )));
        }
        let token = header.trim_start_matches("Bearer ").trim();

        let secret = match env::var("JWT_SECRET") {
            Ok(s) => s,
            Err(_) => return ready(Err(actix_web::error::ErrorUnauthorized("No JWT secret"))),
        };

        let mut validation = Validation::default();
        validation.validate_exp = true;

        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        ) {
            Ok(token_data) => ready(Ok(AuthClaims(token_data.claims))),
            Err(_) => {
                ready(Err(actix_web::error::ErrorUnauthorized("Invalid token")))
            }
        }
    }
}