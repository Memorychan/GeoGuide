use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::{
    common::response::ApiResponse,
    domain::sharing::{
        AdminConsent, CreateLocationRequestDto, CreateShareDto, LocationRequest, LocationShare,
        RespondLocationRequestDto, UpdateAdminConsentDto,
    },
    error::AppResult,
    middleware::auth::AuthUser,
    services::sharing_service::SharingService,
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
    path = "/api/v1/sharing/admin-consent",
    responses(
        (status = 200, description = "Current consent status for Admin tracking", body = ApiResponse<AdminConsent>)
    )
)]
pub async fn get_admin_consent(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<ApiResponse<AdminConsent>> {
    let consent = SharingService::get_admin_consent(&state, &auth_user.id).await?;
    Ok(ApiResponse::ok(consent))
}

#[utoipa::path(
    put,
    path = "/api/v1/sharing/admin-consent",
    request_body = UpdateAdminConsentDto,
    responses(
        (status = 200, description = "Admin tracking consent updated", body = ApiResponse<AdminConsent>)
    )
)]
pub async fn update_admin_consent(
    State(state): State<AppState>,
    auth_user: AuthUser,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<UpdateAdminConsentDto>,
) -> AppResult<ApiResponse<AdminConsent>> {
    let (ip, ua) = extract_client_info(&headers, Some(addr));
    let consent = SharingService::update_admin_consent(
        &state,
        &auth_user.id,
        dto.allow_live_tracking,
        dto.allow_timeline_history,
        ip,
        ua,
    )
    .await?;
    Ok(ApiResponse::ok(consent))
}

#[utoipa::path(
    post,
    path = "/api/v1/sharing/shares",
    request_body = CreateShareDto,
    responses(
        (status = 200, description = "Time-limited location share created", body = ApiResponse<LocationShare>),
        (status = 400, description = "Invalid duration or target user")
    )
)]
pub async fn create_share(
    State(state): State<AppState>,
    auth_user: AuthUser,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateShareDto>,
) -> AppResult<ApiResponse<LocationShare>> {
    let (ip, ua) = extract_client_info(&headers, Some(addr));
    let share = SharingService::create_share(&state, &auth_user.id, dto, ip, ua).await?;
    Ok(ApiResponse::ok(share))
}

#[utoipa::path(
    get,
    path = "/api/v1/sharing/shares/active",
    responses(
        (status = 200, description = "List of all active (unexpired) shares", body = ApiResponse<Vec<LocationShare>>)
    )
)]
pub async fn list_active_shares(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<ApiResponse<Vec<LocationShare>>> {
    let shares = SharingService::list_active_shares(&state, &auth_user.id).await?;
    Ok(ApiResponse::ok(shares))
}

#[utoipa::path(
    delete,
    path = "/api/v1/sharing/shares/{share_id}",
    params(
        ("share_id" = Uuid, Path, description = "Share UUID to revoke")
    ),
    responses(
        (status = 200, description = "Location share revoked")
    )
)]
pub async fn revoke_share(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(share_id): Path<Uuid>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let (ip, ua) = extract_client_info(&headers, Some(addr));
    SharingService::revoke_share(&state, &share_id, &auth_user.id, ip, ua).await?;
    Ok(ApiResponse::ok_msg("Share successfully revoked"))
}

#[utoipa::path(
    post,
    path = "/api/v1/sharing/requests",
    request_body = CreateLocationRequestDto,
    responses(
        (status = 200, description = "Location request sent", body = ApiResponse<LocationRequest>)
    )
)]
pub async fn create_location_request(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<CreateLocationRequestDto>,
) -> AppResult<ApiResponse<LocationRequest>> {
    let req = SharingService::create_request(&state, &auth_user.id, dto).await?;
    Ok(ApiResponse::ok(req))
}

#[utoipa::path(
    get,
    path = "/api/v1/sharing/requests/inbox",
    responses(
        (status = 200, description = "List of incoming pending location requests", body = ApiResponse<Vec<LocationRequest>>)
    )
)]
pub async fn list_pending_requests(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<ApiResponse<Vec<LocationRequest>>> {
    let reqs = SharingService::list_pending_requests(&state, &auth_user.id).await?;
    Ok(ApiResponse::ok(reqs))
}

#[utoipa::path(
    put,
    path = "/api/v1/sharing/requests/{request_id}/respond",
    params(
        ("request_id" = Uuid, Path, description = "Request UUID to approve/reject")
    ),
    request_body = RespondLocationRequestDto,
    responses(
        (status = 200, description = "Response processed")
    )
)]
pub async fn respond_request(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(request_id): Path<Uuid>,
    Json(dto): Json<RespondLocationRequestDto>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    SharingService::respond_to_request(&state, &request_id, &auth_user.id, dto.approve).await?;
    Ok(ApiResponse::ok_msg(if dto.approve {
        "Request approved; location share active"
    } else {
        "Request rejected"
    }))
}

