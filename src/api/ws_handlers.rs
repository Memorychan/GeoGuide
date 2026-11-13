use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::Response,
};
use futures_util::StreamExt;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    domain::{
        location::SingleLocationInputDto,
        sharing::ShareType,
        user::Role,
    },
    error::AppError,
    repository::{sharing_repo::SharingRepo, watchlist_repo::WatchlistRepo},
    security::jwt::verify_access_token,
    services::{location_service::LocationService, sharing_service::SharingService},
    state::AppState,
};

#[derive(Deserialize)]
pub struct WsAuthQuery {
    pub token: String,
}

/// 1. Android Ingestion WebSocket: Client streams 1 Hz GPS fixes directly to backend
pub async fn ws_location_stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsAuthQuery>,
) -> Result<Response, AppError> {
    let claims = verify_access_token(&query.token, &state.config.jwt_secret)?;
    let user_id = claims.sub;

    Ok(ws.on_upgrade(move |socket| handle_ingestion_socket(socket, state, user_id)))
}

async fn handle_ingestion_socket(mut socket: WebSocket, state: AppState, user_id: Uuid) {
    tracing::info!("WebSocket ingestion stream connected for user {}", user_id);

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(dto) = serde_json::from_str::<SingleLocationInputDto>(&text) {
                    if let Err(e) = LocationService::ingest_single(&state, &user_id, dto).await {
                        tracing::warn!("Failed to ingest socket location fix: {:?}", e);
                    }
                }
            }
            Message::Ping(payload) => {
                let _ = socket.send(Message::Pong(payload)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    tracing::info!("WebSocket ingestion stream closed for user {}", user_id);
}

/// 2. Live Map Streaming WebSocket: Authorized Admin or Peer watches a user's real-time coordinates
pub async fn ws_live_view_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(target_user_id): Path<Uuid>,
    Query(query): Query<WsAuthQuery>,
) -> Result<Response, AppError> {
    let claims = verify_access_token(&query.token, &state.config.jwt_secret)?;
    let viewer_id = claims.sub;

    // Check authorization: Admin with consent OR Peer with active time-limited share
    let is_authorized = if claims.role == Role::Admin {
        let consent = SharingRepo::get_admin_consent(&state.db, &target_user_id).await?;
        consent.map(|c| c.allow_live_tracking).unwrap_or(false)
    } else if viewer_id == target_user_id {
        true
    } else {
        SharingService::verify_peer_access(&state, &target_user_id, &viewer_id, ShareType::Live).await?
    };

    if !is_authorized {
        return Err(AppError::Forbidden("Unauthorized: No active live location permission for this user".to_string()));
    }

    Ok(ws.on_upgrade(move |socket| handle_live_view_socket(socket, state, target_user_id)))
}

async fn handle_live_view_socket(mut socket: WebSocket, state: AppState, target_user_id: Uuid) {
    let channel = state.get_or_create_user_channel(&target_user_id);
    let mut rx = channel.subscribe();

    while let Ok(fix) = rx.recv().await {
        if let Ok(json_text) = serde_json::to_string(&fix) {
            if socket.send(Message::Text(json_text)).await.is_err() {
                break;
            }
        }
    }
}

/// 3. Admin Movement Notifications WebSocket: Streams alerts ONLY for users on the admin's watchlist
pub async fn ws_admin_notifications_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsAuthQuery>,
) -> Result<Response, AppError> {
    let claims = verify_access_token(&query.token, &state.config.jwt_secret)?;
    if claims.role != Role::Admin {
        return Err(AppError::Forbidden("Admin privileges required for notification stream".to_string()));
    }

    let admin_id = claims.sub;
    Ok(ws.on_upgrade(move |socket| handle_admin_notifications_socket(socket, state, admin_id)))
}

async fn handle_admin_notifications_socket(mut socket: WebSocket, state: AppState, admin_id: Uuid) {
    let mut rx = state.admin_notification_tx.subscribe();

    while let Ok(alert) = rx.recv().await {
        // Verify this user is on this admin's watchlist
        let is_watched = match WatchlistRepo::list_watchlist(&state.db, &admin_id).await {
            Ok(list) => list.iter().any(|w| w.user_id == alert.user_id && w.notify_on_movement),
            Err(_) => false,
        };

        if is_watched {
            if let Ok(json_text) = serde_json::to_string(&alert) {
                if socket.send(Message::Text(json_text)).await.is_err() {
                    break;
                }
            }
        }
    }
}
