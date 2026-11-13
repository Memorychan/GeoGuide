use std::net::SocketAddr;
use axum::{
    response::Html,
    routing::{delete, get, post, put},
    Json, Router,
};
use geoguide::{
    api::{
        admin_handlers, auth_handlers, location_handlers, openapi::ApiDoc, sharing_handlers,
        timeline_handlers, ws_handlers,
    },
    common::response::ApiResponse,
    config::AppConfig,
    repository::schema::initialize_database,
    state::AppState,
};
use sqlx::sqlite::SqlitePoolOptions;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;

const SWAGGER_UI_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>GeoGuide Backend API Documentation</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: '/api-docs/openapi.json',
        dom_id: '#swagger-ui',
        deepLinking: true,
        presets: [SwaggerUIBundle.presets.apis],
      });
    };
  </script>
</body>
</html>"#;

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

async fn swagger_ui() -> Html<&'static str> {
    Html(SWAGGER_UI_HTML)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "geoguide=debug,tower_http=info".into()),
        )
        .init();

    tracing::info!("Starting GeoGuide High-Frequency Location Tracking Backend...");

    // 2. Load configuration
    let config = AppConfig::from_env();
    tracing::info!("Database URL: {}", config.database_url);

    // 3. Connect to database & run schema migrations
    let pool = SqlitePoolOptions::new()
        .max_connections(25)
        .connect(&config.database_url)
        .await?;

    initialize_database(&pool).await?;

    // 4. Create shared application state
    let state = AppState::new(pool, config.clone());

    // 5. Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 6. Build Router
    let app = Router::new()
        // Health check
        .route("/health", get(|| async { ApiResponse::ok_msg("GeoGuide Backend is healthy") }))
        // Swagger UI & OpenAPI Specification routes
        .route("/api-docs/openapi.json", get(openapi_json))
        .route("/swagger-ui", get(swagger_ui))
        // Authentication routes
        .route("/api/v1/auth/register", post(auth_handlers::register))
        .route("/api/v1/auth/login", post(auth_handlers::login))
        .route("/api/v1/auth/refresh", post(auth_handlers::refresh_token))
        .route("/api/v1/auth/logout", post(auth_handlers::logout))
        .route("/api/v1/auth/me", get(auth_handlers::me))
        .route("/api/v1/auth/tracking-status", put(auth_handlers::update_tracking_status))
        // Location ingestion routes
        .route("/api/v1/locations/single", post(location_handlers::ingest_single))
        .route("/api/v1/locations/batch", post(location_handlers::ingest_batch))
        .route("/api/v1/locations/latest", get(location_handlers::get_latest))
        // Timeline query routes
        .route("/api/v1/timeline/mine", get(timeline_handlers::get_my_timeline))
        .route("/api/v1/timeline/users/:user_id", get(timeline_handlers::get_peer_timeline))
        // Sharing & consent routes
        .route("/api/v1/sharing/admin-consent", get(sharing_handlers::get_admin_consent))
        .route("/api/v1/sharing/admin-consent", put(sharing_handlers::update_admin_consent))
        .route("/api/v1/sharing/shares", post(sharing_handlers::create_share))
        .route("/api/v1/sharing/shares/active", get(sharing_handlers::list_active_shares))
        .route("/api/v1/sharing/shares/:share_id", delete(sharing_handlers::revoke_share))
        .route("/api/v1/sharing/requests", post(sharing_handlers::create_location_request))
        .route("/api/v1/sharing/requests/inbox", get(sharing_handlers::list_pending_requests))
        .route("/api/v1/sharing/requests/:request_id/respond", put(sharing_handlers::respond_request))
        // Admin control & watchlist routes
        .route("/api/v1/admin/users", get(admin_handlers::list_users))
        .route("/api/v1/admin/users/:user_id/timeline", get(admin_handlers::inspect_user_timeline))
        .route("/api/v1/admin/watchlist", post(admin_handlers::add_to_watchlist))
        .route("/api/v1/admin/watchlist/:user_id", delete(admin_handlers::remove_from_watchlist))
        .route("/api/v1/admin/watchlist", get(admin_handlers::list_watchlist))
        .route("/api/v1/admin/notifications", get(admin_handlers::list_notifications))
        .route("/api/v1/admin/audit-logs", get(admin_handlers::query_audit_logs))
        // WebSocket real-time streams
        .route("/api/v1/ws/location/stream", get(ws_handlers::ws_location_stream_handler))
        .route("/api/v1/ws/live/:user_id", get(ws_handlers::ws_live_view_handler))
        .route("/api/v1/admin/ws/notifications", get(ws_handlers::ws_admin_notifications_handler))
        // Layering
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 7. Bind TCP listener and serve
    let bind_addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("GeoGuide server listening on http://{}", bind_addr);
    tracing::info!("Swagger UI documentation available at: http://{}/swagger-ui", bind_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
