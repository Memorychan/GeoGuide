use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    domain::user::Role,
    error::{AppError, AppResult},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub role: Role,
    pub session_id: Uuid,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_access_token(
    user_id: Uuid,
    role: Role,
    session_id: Uuid,
    secret: &str,
    expiry_minutes: i64,
) -> AppResult<String> {
    let now = Utc::now();
    let exp = now + Duration::minutes(expiry_minutes);

    let claims = Claims {
        sub: user_id,
        role,
        session_id,
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to generate JWT: {}", e)))
}

pub fn verify_access_token(token: &str, secret: &str) -> AppResult<Claims> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| AppError::Unauthorized(format!("Invalid or expired token: {}", e)))
}

