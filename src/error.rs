use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    Internal(String),
    Database(sqlx::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal server error: {}", msg),
            AppError::Database(err) => write!(f, "Database error: {}", err),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Database(err) => Some(err),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: ErrorDetail,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An unexpected error occurred".to_string())
            }
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "A database error occurred".to_string())
            }
        };

        let response = ErrorResponse {
            success: false,
            data: None,
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
            timestamp: chrono::Utc::now(),
        };

        (status, Json(response)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
