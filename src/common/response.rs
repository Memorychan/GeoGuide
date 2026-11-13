use axum::{response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    #[allow(dead_code)]
    pub fn into_json(self) -> Json<Self> {
        Json(self)
    }
}

impl ApiResponse<serde_json::Value> {
    pub fn ok_msg(msg: &str) -> Self {
        Self {
            success: true,
            data: Some(serde_json::json!({ "message": msg })),
            error: None,
            timestamp: Utc::now(),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
