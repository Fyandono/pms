use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::features::user::model::Claims;

pub async fn validate_jwt(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let token = credentials.token();
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    );

    match decoded {
        Ok(data) => {
            req.extensions_mut().insert(data.claims);
            Ok(req)
        }
        Err(_) => {
            let err = actix_web::error::ErrorUnauthorized("Invalid or expired token");
            Err((err, req))
        }
    }
}
