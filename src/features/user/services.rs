use crate::AppState;
use crate::features::user::model::{Claims, LoginRequest, RegisterRequest, User};
use actix_web::FromRequest;
use actix_web::dev::Payload;
use actix_web::http::header::AUTHORIZATION;
use actix_web::{
    Error, HttpResponse, Responder, post,
    web::{Data, Json},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use chrono::{Duration, Utc};
use futures::future::{Ready, ready};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use argon2::password_hash::rand_core::OsRng;
use std::env;
use uuid::Uuid;

#[post("/register")]
async fn register(state: Data<AppState>, payload: Json<RegisterRequest>) -> impl Responder {
    let email = payload.email.trim().to_lowercase();
    if email.is_empty() || payload.password.len() < 8 {
        return HttpResponse::BadRequest().body("invalid email or password too short");
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
        "INSERT INTO users (id, email, password_hash, role, is_active) 
         VALUES (CAST($1 AS UUID), $2, $3, 'vendor', true)",
    )
    .bind(&new_id)
    .bind(&email)
    .bind(&password_hash)
    .execute(&state.postgres)
    .await;

    match res {
        Ok(_) => HttpResponse::Ok()
            .json(serde_json::json!({ "id": new_id, "email": email, "role": "vendor" })),
        Err(e) => {
            HttpResponse::InternalServerError().body("could not create user")
        }
    }
}

#[post("/login")]
async fn login(state: Data<AppState>, payload: Json<LoginRequest>) -> impl Responder {
    let email = payload.email.trim().to_lowercase();

    let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&email)
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
        email: user.email.clone(),
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
        Err(e) => {
            HttpResponse::InternalServerError().body("could not create token")
        }
    }
}

// Extractor for Claims from Authorization header
struct AuthClaims(Claims);

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
async fn me(claims: AuthClaims) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "user_id": claims.0.sub,
        "email": claims.0.email,
        "role": claims.0.role,
    }))
}

// Role guard middleware function (can be used inside handler or as wrapper)
fn require_role(claims: &Claims, allowed: &[&str]) -> bool {
    allowed.iter().any(|r| *r == claims.role)
}

// Example admin-only route
async fn admin_only(claims: AuthClaims) -> impl Responder {
    if !require_role(&claims.0, &["admin"]) {
        return HttpResponse::Forbidden().body("forbidden: admin only");
    }

    HttpResponse::Ok().body(format!("Welcome, admin {}!", claims.0.email))
}
