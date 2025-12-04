use crate::AppState;
use crate::features::user::model::{Claims, LoginRequest, RegisterRequest, EditUserRequest, User};
use crate::util::require_role::require_role;
use actix_web::FromRequest;
use actix_web::dev::Payload;
use actix_web::http::header::AUTHORIZATION;
use actix_web::{
    Error, HttpResponse, Responder, post, put,
    web::{Data, Json},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use chrono::{Duration, Utc};
use futures::future::{Ready, ready};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use argon2::password_hash::rand_core::OsRng;
use std::env;
use uuid::Uuid;
use serde_json::json;

#[post("/register")]
async fn register(state: Data<AppState>, payload: Json<RegisterRequest>, claims: AuthClaims) -> impl Responder {
    // Role validator
    if !require_role(&claims.0, &["admin"]) {
        return HttpResponse::Forbidden().json(json!({"message":"You have no access to this feature.\nPlease contact admin for further information."}));
    }

    let user_id = claims.0.sub;
    let username = payload.username.trim().to_lowercase();
    let name = payload.name.trim();
    let role = payload.role.trim();
    let is_active = payload.is_active;

    if name.is_empty() || username.is_empty() || payload.password.len() < 8 || role.is_empty()  {
        return HttpResponse::BadRequest().body("invalid name or invalid username or password too short");
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map(|ph| ph.to_string());
        // .map_err(|e| {
        //     log::error!("hash error: {}", e);
        //     ()
        // });

    if password_hash.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    let password_hash = password_hash.unwrap();

    let new_id = Uuid::new_v4().to_string();

    let res = sqlx::query(
        "INSERT INTO users (id, username, password_hash, name, role, is_active, created_at, created_by) 
         VALUES (CAST($1 AS UUID), $2, $3, $4, $5, $6, NOW(), CAST($7 AS UUID))",
    )
    .bind(&new_id)
    .bind(&username)
    .bind(&password_hash)
    .bind(&name)
    .bind(&role)
    .bind(&is_active)
    .bind(user_id)
    .execute(&state.postgres)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok()
            .json(serde_json::json!({ "id": new_id})),
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

    // Role validator
    if !require_role(&claims.0, &["admin"]) {
        return HttpResponse::Forbidden().json(json!({"message":"You have no access to this feature.\nPlease contact admin for further information."}));
    }

    // Get User ID
    let user_id = claims.0.sub;

    // 1. Validate mandatory user ID
    let user_to_edit_id = payload.id.clone();

    // 2. Prepare optional fields and trim whitespace
    let username = payload.username.as_ref().map(|s| s.trim().to_lowercase());
    let name = payload.name.as_ref().map(|s| s.trim().to_string());
    let role = payload.role.as_ref().map(|s| s.trim().to_string());
    let is_active = payload.is_active;
    
    // 3. Conditional Password Hashing
    let password_hash: Option<String> = match &payload.password {
        Some(pw) if pw.len() >= 8 => {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            
            match argon2.hash_password(pw.as_bytes(), &salt) {
                Ok(ph) => Some(ph.to_string()),
                Err(e) => {
                    eprintln!("Password hashing failed: {}", e);
                    return HttpResponse::InternalServerError().body("Failed to hash password.");
                }
            }
        },
        Some(pw) if pw.len() < 8 => {
            return HttpResponse::BadRequest().body("Password must be at least 8 characters long.");
        }
        _ => None, // Password is None or empty string, do not update.
    };

    // 4. Build the dynamic SQL UPDATE statement
    let res = sqlx::query(
        "
        UPDATE users SET 
            username = COALESCE($2, username),
            name = COALESCE($3, name),
            role = COALESCE($4, role),
            password_hash = COALESCE($5, password_hash),
            is_active = COALESCE($6, is_active),
            updated_at = NOW(),
            updated_by = CAST($7 AS UUID)
        WHERE id = CAST($1 AS UUID)
        RETURNING id
        ",
    )
    .bind(user_to_edit_id)              
    .bind(username.as_deref())        
    .bind(name.as_deref())             
    .bind(role.as_deref())
    .bind(password_hash.as_deref())  
    .bind(is_active)
    .bind(user_id)
    .fetch_optional(&state.postgres)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok()
            .json(serde_json::json!({ "message": "User successfully updated." })),
        Err(e) => {
            eprintln!("Database error during user update: {}", e);
            HttpResponse::InternalServerError().body("Could not update user.")
        }
    }
}

#[post("/login")]
async fn login(state: Data<AppState>, payload: Json<LoginRequest>) -> impl Responder {
    let username = payload.username.trim().to_lowercase();

    let row = sqlx::query_as::<_, User>("SELECT CAST(id AS TEXT), name, username, password_hash, role, is_active FROM users WHERE username = $1")
        .bind(&username)
        .fetch_one(&state.postgres)
        .await;

    let user = match row {
        Ok(u) => u,
        Err(_) => return HttpResponse::Unauthorized().body("invalid credentials"),
    };

    // verify user active
    if !user.is_active {
        return HttpResponse::Forbidden().body("account is not active");
    }

    // verify password
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
        return HttpResponse::Unauthorized().body("invalid credentials");
    }

    // Create JWT
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
        role: user.role.clone(),
        exp: expiration.timestamp() as usize,
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
            HttpResponse::InternalServerError().body("could not create token")
        }
    }
}

// Extractor for Claims from Authorization header
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

// Example protected handler that requires any authenticated user

// #[get("/claim")]
// async fn me(claims: AuthClaims) -> impl Responder {
//     HttpResponse::Ok().json(serde_json::json!({
//         "user_id": claims.0.sub,
//         "email": claims.0.email,
//         "role": claims.0.role,
//     }))
// }

// Example admin-only route
// async fn admin_only(claims: AuthClaims) -> impl Responder {
//     if !require_role(&claims.0, &["admin"]) {
//         return HttpResponse::Forbidden().body("forbidden: admin only");
//     }

//     HttpResponse::Ok().body(format!("Welcome, admin {}!", claims.0.email))
// }
