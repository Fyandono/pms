use crate::features::user::model::Claims;

// Role guard middleware function (can be used inside handler or as wrapper)
pub fn require_role(claims: &Claims, allowed: &[&str]) -> bool {
    allowed.iter().any(|r| *r == claims.role)
}