use chrono::{DateTime, Duration, Utc};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    domain::sharing::{AdminConsent, LocationRequest, LocationShare, RequestStatus, ShareType},
    error::{AppError, AppResult},
};

pub struct SharingRepo;

impl SharingRepo {
    // Admin Consent
    pub async fn get_admin_consent(pool: &SqlitePool, user_id: &Uuid) -> AppResult<Option<AdminConsent>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, admin_id, allow_live_tracking, allow_timeline_history, granted_at, revoked_at
            FROM admin_consents WHERE user_id = ?
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await?;

        row.map(Self::row_to_consent).transpose()
    }

    pub async fn update_admin_consent(
        pool: &SqlitePool,
        user_id: &Uuid,
        allow_live: bool,
        allow_timeline: bool,
    ) -> AppResult<AdminConsent> {
        let now = Utc::now();
        let revoked_at = if !allow_live && !allow_timeline {
            Some(now.to_rfc3339())
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO admin_consents (id, user_id, admin_id, allow_live_tracking, allow_timeline_history, granted_at, revoked_at)
            VALUES (?, ?, NULL, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                allow_live_tracking = excluded.allow_live_tracking,
                allow_timeline_history = excluded.allow_timeline_history,
                revoked_at = excluded.revoked_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id.to_string())
        .bind(if allow_live { 1 } else { 0 })
        .bind(if allow_timeline { 1 } else { 0 })
        .bind(now.to_rfc3339())
        .bind(revoked_at)
        .execute(pool)
        .await?;

        Self::get_admin_consent(pool, user_id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to retrieve updated consent".to_string()))
    }

    // Peer-to-Peer Time-Limited Sharing
    pub async fn create_share(
        pool: &SqlitePool,
        granter_id: &Uuid,
        grantee_id: &Uuid,
        share_type: ShareType,
        duration_minutes: i64,
    ) -> AppResult<LocationShare> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::minutes(duration_minutes);

        sqlx::query(
            r#"
            INSERT INTO location_shares (id, granter_id, grantee_id, share_type, expires_at, is_active, created_at)
            VALUES (?, ?, ?, ?, ?, 1, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(granter_id.to_string())
        .bind(grantee_id.to_string())
        .bind(share_type.to_string())
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(LocationShare {
            id,
            granter_id: *granter_id,
            grantee_id: *grantee_id,
            share_type,
            expires_at,
            is_active: true,
            created_at: now,
        })
    }

    pub async fn find_active_share(
        pool: &SqlitePool,
        granter_id: &Uuid,
        grantee_id: &Uuid,
    ) -> AppResult<Option<LocationShare>> {
        let now = Utc::now().to_rfc3339();
        let row = sqlx::query(
            r#"
            SELECT id, granter_id, grantee_id, share_type, expires_at, is_active, created_at
            FROM location_shares
            WHERE granter_id = ? AND grantee_id = ? AND is_active = 1 AND expires_at > ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(granter_id.to_string())
        .bind(grantee_id.to_string())
        .bind(now)
        .fetch_optional(pool)
        .await?;

        row.map(Self::row_to_share).transpose()
    }

    pub async fn list_active_shares_for_user(
        pool: &SqlitePool,
        user_id: &Uuid,
    ) -> AppResult<Vec<LocationShare>> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            r#"
            SELECT id, granter_id, grantee_id, share_type, expires_at, is_active, created_at
            FROM location_shares
            WHERE (granter_id = ? OR grantee_id = ?) AND is_active = 1 AND expires_at > ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .bind(now)
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(Self::row_to_share).collect()
    }

    pub async fn revoke_share(pool: &SqlitePool, share_id: &Uuid, user_id: &Uuid) -> AppResult<()> {
        let result = sqlx::query(
            "UPDATE location_shares SET is_active = 0 WHERE id = ? AND (granter_id = ? OR grantee_id = ?)",
        )
        .bind(share_id.to_string())
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Active share not found or access denied".to_string()));
        }
        Ok(())
    }

    // Location Request Workflows
    pub async fn create_request(
        pool: &SqlitePool,
        requester_id: &Uuid,
        target_id: &Uuid,
        requested_type: ShareType,
        duration_minutes: i64,
        message: Option<&str>,
    ) -> AppResult<LocationRequest> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO location_requests (id, requester_id, target_id, requested_type, duration_minutes, status, message, created_at)
            VALUES (?, ?, ?, ?, ?, 'PENDING', ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(requester_id.to_string())
        .bind(target_id.to_string())
        .bind(requested_type.to_string())
        .bind(duration_minutes)
        .bind(message)
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(LocationRequest {
            id,
            requester_id: *requester_id,
            target_id: *target_id,
            requested_type,
            duration_minutes,
            status: RequestStatus::Pending,
            message: message.map(String::from),
            created_at: now,
            responded_at: None,
        })
    }

    pub async fn find_request_by_id(pool: &SqlitePool, request_id: &Uuid) -> AppResult<Option<LocationRequest>> {
        let row = sqlx::query(
            r#"
            SELECT id, requester_id, target_id, requested_type, duration_minutes, status, message, created_at, responded_at
            FROM location_requests WHERE id = ?
            "#,
        )
        .bind(request_id.to_string())
        .fetch_optional(pool)
        .await?;

        row.map(Self::row_to_request).transpose()
    }

    pub async fn list_pending_requests(pool: &SqlitePool, target_id: &Uuid) -> AppResult<Vec<LocationRequest>> {
        let rows = sqlx::query(
            r#"
            SELECT id, requester_id, target_id, requested_type, duration_minutes, status, message, created_at, responded_at
            FROM location_requests WHERE target_id = ? AND status = 'PENDING'
            ORDER BY created_at DESC
            "#,
        )
        .bind(target_id.to_string())
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(Self::row_to_request).collect()
    }

    pub async fn respond_request(
        pool: &SqlitePool,
        request_id: &Uuid,
        status: RequestStatus,
    ) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE location_requests SET status = ?, responded_at = ? WHERE id = ?",
        )
        .bind(status.to_string())
        .bind(now)
        .bind(request_id.to_string())
        .execute(pool)
        .await?;

        Ok(())
    }

    fn row_to_consent(row: sqlx::sqlite::SqliteRow) -> AppResult<AdminConsent> {
        let id = Uuid::parse_str(row.get("id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let user_id = Uuid::parse_str(row.get("user_id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let admin_id: Option<&str> = row.get("admin_id");
        let admin_uuid = admin_id.and_then(|a| Uuid::parse_str(a).ok());

        let allow_live: i64 = row.get("allow_live_tracking");
        let allow_history: i64 = row.get("allow_timeline_history");

        let granted_str: &str = row.get("granted_at");
        let granted_at = DateTime::parse_from_rfc3339(granted_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);

        let revoked_at = row
            .get::<Option<&str>, _>("revoked_at")
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));

        Ok(AdminConsent {
            id,
            user_id,
            admin_id: admin_uuid,
            allow_live_tracking: allow_live != 0,
            allow_timeline_history: allow_history != 0,
            granted_at,
            revoked_at,
        })
    }

    fn row_to_share(row: sqlx::sqlite::SqliteRow) -> AppResult<LocationShare> {
        let id = Uuid::parse_str(row.get("id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let granter_id = Uuid::parse_str(row.get("granter_id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let grantee_id = Uuid::parse_str(row.get("grantee_id")).map_err(|e| AppError::Internal(e.to_string()))?;

        let share_type_str: &str = row.get("share_type");
        let share_type = ShareType::from_str(share_type_str).map_err(AppError::Internal)?;

        let expires_str: &str = row.get("expires_at");
        let created_str: &str = row.get("created_at");

        let expires_at = DateTime::parse_from_rfc3339(expires_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);
        let created_at = DateTime::parse_from_rfc3339(created_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);

        let is_active: i64 = row.get("is_active");

        Ok(LocationShare {
            id,
            granter_id,
            grantee_id,
            share_type,
            expires_at,
            is_active: is_active != 0,
            created_at,
        })
    }

    fn row_to_request(row: sqlx::sqlite::SqliteRow) -> AppResult<LocationRequest> {
        let id = Uuid::parse_str(row.get("id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let requester_id = Uuid::parse_str(row.get("requester_id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let target_id = Uuid::parse_str(row.get("target_id")).map_err(|e| AppError::Internal(e.to_string()))?;

        let type_str: &str = row.get("requested_type");
        let requested_type = ShareType::from_str(type_str).map_err(AppError::Internal)?;

        let status_str: &str = row.get("status");
        let status = RequestStatus::from_str(status_str).map_err(AppError::Internal)?;

        let duration_minutes: i64 = row.get("duration_minutes");

        let created_str: &str = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(created_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);

        let responded_at = row
            .get::<Option<&str>, _>("responded_at")
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));

        Ok(LocationRequest {
            id,
            requester_id,
            target_id,
            requested_type,
            duration_minutes,
            status,
            message: row.get("message"),
            created_at,
            responded_at,
        })
    }
}

