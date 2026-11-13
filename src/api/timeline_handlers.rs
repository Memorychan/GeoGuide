use axum::{
    extract::{Path, Query, State},
};
use uuid::Uuid;

use crate::{
    common::response::ApiResponse,
    domain::{
        location::{TimelineQueryDto, TimelineResponseDto},
        sharing::ShareType,
    },
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    services::{location_service::LocationService, sharing_service::SharingService},
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/api/v1/timeline/mine",
    params(
        ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "Start timestamp (ISO-8601)"),
        ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "End timestamp (ISO-8601)"),
        ("limit" = Option<i64>, Query, description = "Max points to return (default 5000)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset")
    ),
    responses(
        (status = 200, description = "Second-by-second timeline data", body = ApiResponse<TimelineResponseDto>)
    )
)]
pub async fn get_my_timeline(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<TimelineQueryDto>,
) -> AppResult<ApiResponse<TimelineResponseDto>> {
    let timeline = LocationService::query_timeline(&state, &auth_user.id, &query).await?;
    Ok(ApiResponse::ok(timeline))
}

#[utoipa::path(
    get,
    path = "/api/v1/timeline/users/{user_id}",
    params(
        ("user_id" = Uuid, Path, description = "Target peer user ID"),
        ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "Start timestamp"),
        ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "End timestamp"),
        ("limit" = Option<i64>, Query, description = "Max points"),
        ("offset" = Option<i64>, Query, description = "Pagination offset")
    ),
    responses(
        (status = 200, description = "Peer's authorized timeline", body = ApiResponse<TimelineResponseDto>),
        (status = 403, description = "No active share or share has expired")
    )
)]
pub async fn get_peer_timeline(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(target_user_id): Path<Uuid>,
    Query(query): Query<TimelineQueryDto>,
) -> AppResult<ApiResponse<TimelineResponseDto>> {
    // Strictly verify active, unexpired peer share permission
    let has_access = SharingService::verify_peer_access(
        &state,
        &target_user_id,
        &auth_user.id,
        ShareType::Timeline,
    )
    .await?;

    if !has_access {
        return Err(AppError::Forbidden("No active timeline share found or permission has expired".to_string()));
    }

    let timeline = LocationService::query_timeline(&state, &target_user_id, &query).await?;
    Ok(ApiResponse::ok(timeline))
}

