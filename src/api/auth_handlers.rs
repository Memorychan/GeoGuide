use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    Json,
};
use std::net::SocketAddr;

use crate::{
    common::response::ApiResponse,
    domain::user::{
        AuthResponseDto, LoginDto, RefreshTokenDto, RegisterDto, UpdateTrackingStatusDto, UserResponseDto,
    },
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    repository::user_repo::UserRepo,
    services::auth_service::AuthService,
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
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterDto,
    responses(
        (status = 200, description = "User successfully registered", body = ApiResponse<UserResponseDto>),
        (status = 400, description = "Invalid registration parameters"),
        (status = 409, description = "Email or phone already exists")
    )
)]
pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<RegisterDto>,
) -> AppResult<ApiResponse<UserResponseDto>> {
    let (ip, ua) = extract_client_info(&headers, Some(addr));
    let user = AuthService::register(&state, dto, ip, ua).await?;
    Ok(ApiResponse::ok(user))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginDto,
    responses(
        (status = 200, description = "Successful authentication", body = ApiResponse<AuthResponseDto>),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Account deactivated")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<LoginDto>,
) -> AppResult<ApiResponse<AuthResponseDto>> {
    let (ip, ua) = extract_client_info(&headers, Some(addr));
    let auth = AuthService::login(&state, dto, ip, ua).await?;
    Ok(ApiResponse::ok(auth))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshTokenDto,
    responses(
        (status = 200, description = "Tokens rotated successfully", body = ApiResponse<AuthResponseDto>),
        (status = 401, description = "Invalid or revoked refresh token")
    )
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<RefreshTokenDto>,
) -> AppResult<ApiResponse<AuthResponseDto>> {
    let (ip, ua) = extract_client_info(&headers, Some(addr));
    let auth = AuthService::refresh(&state, &dto.refresh_token, ip, ua).await?;
    Ok(ApiResponse::ok(auth))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    request_body = RefreshTokenDto,
    responses(
        (status = 200, description = "Logged out successfully")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<RefreshTokenDto>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let (ip, ua) = extract_client_info(&headers, Some(addr));
    AuthService::logout(&state, &dto.refresh_token, &auth_user.id, ip, ua).await?;
    Ok(ApiResponse::ok_msg("Successfully logged out"))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current authenticated user profile", body = ApiResponse<UserResponseDto>)
    )
)]
pub async fn me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<ApiResponse<UserResponseDto>> {
    let user = UserRepo::find_by_id(&state.db, &auth_user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    Ok(ApiResponse::ok(user.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/auth/tracking-status",
    request_body = UpdateTrackingStatusDto,
    responses(
        (status = 200, description = "Updated tracking preference")
    )
)]
pub async fn update_tracking_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(dto): Json<UpdateTrackingStatusDto>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    UserRepo::update_tracking_status(&state.db, &auth_user.id, dto.is_tracking_enabled).await?;
    Ok(ApiResponse::ok_msg(if dto.is_tracking_enabled {
        "Continuous tracking enabled"
    } else {
        "Continuous tracking paused"
    }))
}

