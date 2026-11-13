use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLog {
    pub id: i64,
    pub actor_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
    pub action: String,
    pub resource: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AuditLogQueryDto {
    pub actor_id: Option<Uuid>,
    pub action: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

