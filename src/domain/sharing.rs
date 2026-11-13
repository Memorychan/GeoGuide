use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShareType {
    Live,
    Timeline,
    Both,
}

impl ToString for ShareType {
    fn to_string(&self) -> String {
        match self {
            ShareType::Live => "LIVE".to_string(),
            ShareType::Timeline => "TIMELINE".to_string(),
            ShareType::Both => "BOTH".to_string(),
        }
    }
}

impl std::str::FromStr for ShareType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "LIVE" => Ok(ShareType::Live),
            "TIMELINE" => Ok(ShareType::Timeline),
            "BOTH" => Ok(ShareType::Both),
            other => Err(format!("Unknown share type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl ToString for RequestStatus {
    fn to_string(&self) -> String {
        match self {
            RequestStatus::Pending => "PENDING".to_string(),
            RequestStatus::Approved => "APPROVED".to_string(),
            RequestStatus::Rejected => "REJECTED".to_string(),
            RequestStatus::Expired => "EXPIRED".to_string(),
        }
    }
}

impl std::str::FromStr for RequestStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PENDING" => Ok(RequestStatus::Pending),
            "APPROVED" => Ok(RequestStatus::Approved),
            "REJECTED" => Ok(RequestStatus::Rejected),
            "EXPIRED" => Ok(RequestStatus::Expired),
            other => Err(format!("Unknown request status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocationShare {
    pub id: Uuid,
    pub granter_id: Uuid,
    pub grantee_id: Uuid,
    pub share_type: ShareType,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateShareDto {
    pub grantee_id: Uuid,
    pub share_type: ShareType,
    /// Duration in minutes for the location share (Mandatory time limit)
    pub duration_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocationRequest {
    pub id: Uuid,
    pub requester_id: Uuid,
    pub target_id: Uuid,
    pub requested_type: ShareType,
    pub duration_minutes: i64,
    pub status: RequestStatus,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateLocationRequestDto {
    pub target_id: Uuid,
    pub requested_type: ShareType,
    pub duration_minutes: i64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RespondLocationRequestDto {
    pub approve: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminConsent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub admin_id: Option<Uuid>,
    pub allow_live_tracking: bool,
    pub allow_timeline_history: bool,
    pub granted_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateAdminConsentDto {
    pub allow_live_tracking: bool,
    pub allow_timeline_history: bool,
}

