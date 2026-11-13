use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use crate::{
    domain::user::Role,
    error::AppError,
    security::jwt::verify_access_token,
    state::AppState,
};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: Role,
    #[allow(dead_code)]
    pub session_id: Uuid,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized("Invalid Authorization header format. Expected 'Bearer <token>'".to_string()));
        }

        let token = &auth_header["Bearer ".len()..];
        let claims = verify_access_token(token, &app_state.config.jwt_secret)?;

        Ok(AuthUser {
            id: claims.sub,
            role: claims.role,
            session_id: claims.session_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequireAdmin(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAdmin
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        if auth_user.role != Role::Admin {
            return Err(AppError::Forbidden("Administrator privileges required for this action".to_string()));
        }

        Ok(RequireAdmin(auth_user))
    }
}
