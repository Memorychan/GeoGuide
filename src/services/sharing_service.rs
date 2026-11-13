use uuid::Uuid;

use crate::{
    domain::sharing::{AdminConsent, CreateLocationRequestDto, CreateShareDto, LocationRequest, LocationShare, RequestStatus, ShareType},
    error::{AppError, AppResult},
    repository::{
        audit_repo::AuditRepo,
        sharing_repo::SharingRepo,
        user_repo::UserRepo,
    },
    state::AppState,
};

pub struct SharingService;

impl SharingService {
    // Admin Consent
    pub async fn get_admin_consent(state: &AppState, user_id: &Uuid) -> AppResult<AdminConsent> {
        SharingRepo::get_admin_consent(&state.db, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Admin consent configuration not found".to_string()))
    }

    pub async fn update_admin_consent(
        state: &AppState,
        user_id: &Uuid,
        allow_live: bool,
        allow_timeline: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AppResult<AdminConsent> {
        let consent = SharingRepo::update_admin_consent(&state.db, user_id, allow_live, allow_timeline).await?;

        AuditRepo::record(
            &state.db,
            Some(user_id),
            Some(user_id),
            "CONSENT_UPDATE",
            "admin_consents",
            ip_address.as_deref(),
            user_agent.as_deref(),
            Some(&serde_json::json!({
                "allow_live_tracking": allow_live,
                "allow_timeline_history": allow_timeline,
            })),
        )
        .await?;

        Ok(consent)
    }

    // Direct Time-Limited Sharing
    pub async fn create_share(
        state: &AppState,
        granter_id: &Uuid,
        dto: CreateShareDto,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AppResult<LocationShare> {
        if *granter_id == dto.grantee_id {
            return Err(AppError::BadRequest("Cannot share location with yourself".to_string()));
        }
        if dto.duration_minutes <= 0 {
            return Err(AppError::BadRequest("Duration must be greater than 0 minutes".to_string()));
        }
        if dto.duration_minutes > 10080 { // Max 7 days
            return Err(AppError::BadRequest("Maximum share duration is 7 days (10080 minutes)".to_string()));
        }

        // Verify grantee exists
        let grantee = UserRepo::find_by_id(&state.db, &dto.grantee_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Recipient user not found".to_string()))?;

        if !grantee.is_active {
            return Err(AppError::BadRequest("Recipient user account is inactive".to_string()));
        }

        let share = SharingRepo::create_share(
            &state.db,
            granter_id,
            &dto.grantee_id,
            dto.share_type,
            dto.duration_minutes,
        )
        .await?;

        AuditRepo::record(
            &state.db,
            Some(granter_id),
            Some(&dto.grantee_id),
            "SHARE_CREATED",
            "location_shares",
            ip_address.as_deref(),
            user_agent.as_deref(),
            Some(&serde_json::json!({
                "share_type": dto.share_type.to_string(),
                "duration_minutes": dto.duration_minutes,
                "expires_at": share.expires_at.to_rfc3339(),
            })),
        )
        .await?;

        Ok(share)
    }

    pub async fn list_active_shares(state: &AppState, user_id: &Uuid) -> AppResult<Vec<LocationShare>> {
        SharingRepo::list_active_shares_for_user(&state.db, user_id).await
    }

    pub async fn revoke_share(
        state: &AppState,
        share_id: &Uuid,
        user_id: &Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AppResult<()> {
        SharingRepo::revoke_share(&state.db, share_id, user_id).await?;

        AuditRepo::record(
            &state.db,
            Some(user_id),
            None,
            "SHARE_REVOKED",
            "location_shares",
            ip_address.as_deref(),
            user_agent.as_deref(),
            Some(&serde_json::json!({ "share_id": share_id.to_string() })),
        )
        .await?;

        Ok(())
    }

    // Request & Approval Flow
    pub async fn create_request(
        state: &AppState,
        requester_id: &Uuid,
        dto: CreateLocationRequestDto,
    ) -> AppResult<LocationRequest> {
        if *requester_id == dto.target_id {
            return Err(AppError::BadRequest("Cannot request location from yourself".to_string()));
        }
        if dto.duration_minutes <= 0 {
            return Err(AppError::BadRequest("Duration must be greater than 0 minutes".to_string()));
        }

        // Verify target user exists
        let target = UserRepo::find_by_id(&state.db, &dto.target_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Target user not found".to_string()))?;

        if !target.is_active {
            return Err(AppError::BadRequest("Target user account is inactive".to_string()));
        }

        SharingRepo::create_request(
            &state.db,
            requester_id,
            &dto.target_id,
            dto.requested_type,
            dto.duration_minutes,
            dto.message.as_deref(),
        )
        .await
    }

    pub async fn list_pending_requests(state: &AppState, target_id: &Uuid) -> AppResult<Vec<LocationRequest>> {
        SharingRepo::list_pending_requests(&state.db, target_id).await
    }

    pub async fn respond_to_request(
        state: &AppState,
        request_id: &Uuid,
        user_id: &Uuid,
        approve: bool,
    ) -> AppResult<()> {
        let request = SharingRepo::find_request_by_id(&state.db, request_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Location request not found".to_string()))?;

        if request.target_id != *user_id {
            return Err(AppError::Forbidden("You are not authorized to respond to this request".to_string()));
        }
        if request.status != RequestStatus::Pending {
            return Err(AppError::BadRequest(format!("Request is already {:?}", request.status)));
        }

        let new_status = if approve {
            // Automatically establish the active LocationShare with requested time limit
            SharingRepo::create_share(
                &state.db,
                user_id,
                &request.requester_id,
                request.requested_type,
                request.duration_minutes,
            )
            .await?;

            RequestStatus::Approved
        } else {
            RequestStatus::Rejected
        };

        SharingRepo::respond_request(&state.db, request_id, new_status).await
    }

    /// Validates if requester has permission to view target's live or timeline data
    pub async fn verify_peer_access(
        state: &AppState,
        granter_id: &Uuid,
        grantee_id: &Uuid,
        required_type: ShareType,
    ) -> AppResult<bool> {
        let share = SharingRepo::find_active_share(&state.db, granter_id, grantee_id).await?;
        match share {
            Some(s) => match (s.share_type, required_type) {
                (ShareType::Both, _) => Ok(true),
                (ShareType::Live, ShareType::Live) => Ok(true),
                (ShareType::Timeline, ShareType::Timeline) => Ok(true),
                _ => Ok(false),
            },
            None => Ok(false),
        }
    }
}

