use axum::{extract::State, Json};

use crate::{
    common::response::ApiResponse,
    domain::location::{BatchLocationInputDto, LocationFix, SingleLocationInputDto},
    error::AppResult,
    middleware::auth::AuthUser,
    services::location_service::LocationService,
    state::AppState,
};

#[utoipa::path(
    post,
    path = "/api/v1/locations/single",
    request_body = SingleLocationInputDto,
    responses(
        (status = 200, description = "GPS fix successfully ingested", body = ApiResponse<LocationFix>),
        (status = 403, description = "Tracking is disabled")
    )
)]
pub async fn ingest_single(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<SingleLocationInputDto>,
) -> AppResult<ApiResponse<LocationFix>> {
    let fix = LocationService::ingest_single(&state, &auth_user.id, dto).await?;
    Ok(ApiResponse::ok(fix))
}

#[utoipa::path(
    post,
    path = "/api/v1/locations/batch",
    request_body = BatchLocationInputDto,
    responses(
        (status = 200, description = "Batch of second-by-second GPS fixes ingested", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "Tracking is disabled")
    )
)]
pub async fn ingest_batch(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<BatchLocationInputDto>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let count = LocationService::ingest_batch(&state, &auth_user.id, dto).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "message": format!("Successfully ingested {} location fixes", count),
        "count": count
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/locations/latest",
    responses(
        (status = 200, description = "Latest GPS fix", body = ApiResponse<Option<LocationFix>>)
    )
)]
pub async fn get_latest(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<ApiResponse<Option<LocationFix>>> {
    let fix = LocationService::get_latest(&state, &auth_user.id).await?;
    Ok(ApiResponse::ok(fix))
}

