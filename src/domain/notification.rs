use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminWatchedUser {
    pub id: Uuid,
    pub admin_id: Uuid,
    pub user_id: Uuid,
    pub notify_on_movement: bool,
    pub movement_threshold_meters: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddWatchlistDto {
    pub user_id: Uuid,
    pub movement_threshold_meters: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationEvent {
    pub id: Uuid,
    pub recipient_id: Uuid,
    pub subject_user_id: Option<Uuid>,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub metadata: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MovementAlertDto {
    pub user_id: Uuid,
    pub user_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub speed: Option<f32>,
    pub distance_moved_meters: f64,
    pub timestamp: DateTime<Utc>,
}

