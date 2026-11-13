use utoipa::OpenApi;

use crate::{
    api::{admin_handlers, auth_handlers, location_handlers, sharing_handlers, timeline_handlers},
    common::response::ApiResponse,
    domain::{
        audit::{AuditLog, AuditLogQueryDto},
        location::{
            BatchLocationInputDto, LocationFix, SingleLocationInputDto, TimelineQueryDto,
            TimelineResponseDto,
        },
        notification::{AddWatchlistDto, AdminWatchedUser, MovementAlertDto, NotificationEvent},
        sharing::{
            AdminConsent, CreateLocationRequestDto, CreateShareDto, LocationRequest, LocationShare,
            RequestStatus, RespondLocationRequestDto, ShareType, UpdateAdminConsentDto,
        },
        user::{
            AuthResponseDto, LoginDto, RefreshTokenDto, RegisterDto, Role, UpdateTrackingStatusDto,
            UserResponseDto,
        },
    },
    error::{ErrorDetail, ErrorResponse},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth_handlers::register,
        auth_handlers::login,
        auth_handlers::refresh_token,
        auth_handlers::logout,
        auth_handlers::me,
        auth_handlers::update_tracking_status,
        location_handlers::ingest_single,
        location_handlers::ingest_batch,
        location_handlers::get_latest,
        timeline_handlers::get_my_timeline,
        timeline_handlers::get_peer_timeline,
        sharing_handlers::get_admin_consent,
        sharing_handlers::update_admin_consent,
        sharing_handlers::create_share,
        sharing_handlers::list_active_shares,
        sharing_handlers::revoke_share,
        sharing_handlers::create_location_request,
        sharing_handlers::list_pending_requests,
        sharing_handlers::respond_request,
        admin_handlers::list_users,
        admin_handlers::inspect_user_timeline,
        admin_handlers::add_to_watchlist,
        admin_handlers::remove_from_watchlist,
        admin_handlers::list_watchlist,
        admin_handlers::list_notifications,
        admin_handlers::query_audit_logs,
    ),
    components(
        schemas(
            Role,
            RegisterDto,
            LoginDto,
            AuthResponseDto,
            UserResponseDto,
            RefreshTokenDto,
            UpdateTrackingStatusDto,
            LocationFix,
            SingleLocationInputDto,
            BatchLocationInputDto,
            TimelineQueryDto,
            TimelineResponseDto,
            ShareType,
            RequestStatus,
            LocationShare,
            CreateShareDto,
            LocationRequest,
            CreateLocationRequestDto,
            RespondLocationRequestDto,
            AdminConsent,
            UpdateAdminConsentDto,
            AdminWatchedUser,
            AddWatchlistDto,
            NotificationEvent,
            MovementAlertDto,
            AuditLog,
            AuditLogQueryDto,
            ErrorDetail,
            ErrorResponse,
            ApiResponse<UserResponseDto>,
            ApiResponse<AuthResponseDto>,
            ApiResponse<LocationFix>,
            ApiResponse<Option<LocationFix>>,
            ApiResponse<TimelineResponseDto>,
            ApiResponse<LocationShare>,
            ApiResponse<Vec<LocationShare>>,
            ApiResponse<LocationRequest>,
            ApiResponse<Vec<LocationRequest>>,
            ApiResponse<AdminConsent>,
            ApiResponse<AdminWatchedUser>,
            ApiResponse<Vec<AdminWatchedUser>>,
            ApiResponse<Vec<NotificationEvent>>,
            ApiResponse<Vec<AuditLog>>,
            ApiResponse<serde_json::Value>
        )
    ),
    tags(
        (name = "Authentication", description = "User registration, Argon2id login, JWT & Refresh token rotation"),
        (name = "Locations", description = "High-frequency 1-second single & batch GPS ingestion"),
        (name = "Timeline", description = "Second-by-second historical coordinate queries and statistics"),
        (name = "Sharing & Consents", description = "Time-limited peer-to-peer sharing and Admin tracking consent"),
        (name = "Admin Controls", description = "User management, watchlist alerts, timeline audits, and logs")
    )
)]
pub struct ApiDoc;

