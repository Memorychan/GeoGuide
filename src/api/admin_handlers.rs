use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    Json,
};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::{
    common::response::ApiResponse,
    domain::{
        audit::{AuditLog, AuditLogQueryDto},
        location::{TimelineQueryDto, TimelineResponseDto},
        notification::{AddWatchlistDto, AdminWatchedUser, NotificationEvent},
        user::UserResponseDto,
    },
    error::{AppError, AppResult},
    middleware::auth::RequireAdmin,
    repository::{
        audit_repo::AuditRepo,
        location_repo::LocationRepo,
        sharing_repo::SharingRepo,
        user_repo::UserRepo,
        watchlist_repo::WatchlistRepo,
    },
    state::AppState,
};

fn extract_client_info(headers: &HeaderMap, addr: Option<SocketAddr>) -> (Option<String>, Option<String>) {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| addr.map(|a| a.ip().to_string()));

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    (ip, user_agent)
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    responses(
        (status = 200, description = "All registered users", body = ApiResponse<Vec<UserResponseDto>>)
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> AppResult<ApiResponse<Vec<UserResponseDto>>> {
    let users = UserRepo::list_users(&state.db).await?;
    let dtos: Vec<UserResponseDto> = users.into_iter().map(Into::into).collect();
    Ok(ApiResponse::ok(dtos))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/users/{user_id}/timeline",
    params(
        ("user_id" = Uuid, Path, description = "User UUID to inspect"),
        ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "Start timestamp"),
        ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "End timestamp"),
        ("limit" = Option<i64>, Query, description = "Limit (default 5000)"),
        ("offset" = Option<i64>, Query, description = "Offset")
    ),
    responses(
        (status = 200, description = "Second-by-second timeline of consenting user", body = ApiResponse<TimelineResponseDto>),
        (status = 403, description = "User has not permitted admin timeline tracking")
    )
)]
pub async fn inspect_user_timeline(
    State(state): State<AppState>,
    RequireAdmin(admin): RequireAdmin,
    Path(user_id): Path<Uuid>,
    Query(query): Query<TimelineQueryDto>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> AppResult<ApiResponse<TimelineResponseDto>> {
    // Verify Admin Tracking Consent
    let consent = SharingRepo::get_admin_consent(&state.db, &user_id).await?;
    let is_allowed = consent.map(|c| c.allow_timeline_history).unwrap_or(false);

    if !is_allowed {
        return Err(AppError::Forbidden(
            "User has not granted permission for Admin timeline tracking".to_string(),
        ));
    }

    let (ip, ua) = extract_client_info(&headers, Some(addr));
    AuditRepo::record(
        &state.db,
        Some(&admin.id),
        Some(&user_id),
        "TIMELINE_INSPECTED",
        "locations",
        ip.as_deref(),
        ua.as_deref(),
        Some(&serde_json::json!({
            "from": query.from.map(|d| d.to_rfc3339()),
            "to": query.to.map(|d| d.to_rfc3339()),
        })),
    )
    .await?;

    let timeline = LocationRepo::query_timeline(&state.db, &user_id, &query).await?;
    Ok(ApiResponse::ok(timeline))
}

// Watchlist (Selected users for Admin movement notifications)
#[utoipa::path(
    post,
    path = "/api/v1/admin/watchlist",
    request_body = AddWatchlistDto,
    responses(
        (status = 200, description = "User added to Admin movement notification watchlist", body = ApiResponse<AdminWatchedUser>)
    )
)]
pub async fn add_to_watchlist(
    State(state): State<AppState>,
    RequireAdmin(admin): RequireAdmin,
    Json(dto): Json<AddWatchlistDto>,
) -> AppResult<ApiResponse<AdminWatchedUser>> {
    let threshold = dto
        .movement_threshold_meters
        .unwrap_or(state.config.default_movement_threshold_meters);

    let watched = WatchlistRepo::add_to_watchlist(&state.db, &admin.id, &dto.user_id, threshold).await?;
    Ok(ApiResponse::ok(watched))
}

#[utoipa::path(
    delete,
    path = "/api/v1/admin/watchlist/{user_id}",
    params(
        ("user_id" = Uuid, Path, description = "User UUID to remove from watchlist")
    ),
    responses(
        (status = 200, description = "User removed from watchlist")
    )
)]
pub async fn remove_from_watchlist(
    State(state): State<AppState>,
    RequireAdmin(admin): RequireAdmin,
    Path(user_id): Path<Uuid>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    WatchlistRepo::remove_from_watchlist(&state.db, &admin.id, &user_id).await?;
    Ok(ApiResponse::ok_msg("User removed from movement notification watchlist"))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/watchlist",
    responses(
        (status = 200, description = "Currently watched users by Admin", body = ApiResponse<Vec<AdminWatchedUser>>)
    )
)]
pub async fn list_watchlist(
    State(state): State<AppState>,
    RequireAdmin(admin): RequireAdmin,
) -> AppResult<ApiResponse<Vec<AdminWatchedUser>>> {
    let list = WatchlistRepo::list_watchlist(&state.db, &admin.id).await?;
    Ok(ApiResponse::ok(list))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/notifications",
    params(
        ("limit" = Option<i64>, Query, description = "Limit"),
        ("offset" = Option<i64>, Query, description = "Offset")
    ),
    responses(
        (status = 200, description = "Movement alert notifications", body = ApiResponse<Vec<NotificationEvent>>)
    )
)]
pub async fn list_notifications(
    State(state): State<AppState>,
    RequireAdmin(admin): RequireAdmin,
    Query(query): Query<AuditLogQueryDto>,
) -> AppResult<ApiResponse<Vec<NotificationEvent>>> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let notifs = WatchlistRepo::list_notifications(&state.db, &admin.id, limit, offset).await?;
    Ok(ApiResponse::ok(notifs))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/audit-logs",
    params(
        ("actor_id" = Option<Uuid>, Query, description = "Filter by actor"),
        ("action" = Option<String>, Query, description = "Filter by action"),
        ("limit" = Option<i64>, Query, description = "Limit"),
        ("offset" = Option<i64>, Query, description = "Offset")
    ),
    responses(
        (status = 200, description = "Audit trail logs", body = ApiResponse<Vec<AuditLog>>)
    )
)]
pub async fn query_audit_logs(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Query(query): Query<AuditLogQueryDto>,
) -> AppResult<ApiResponse<Vec<AuditLog>>> {
    let logs = AuditRepo::query(&state.db, &query).await?;
    Ok(ApiResponse::ok(logs))
}

