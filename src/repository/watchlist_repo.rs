use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    domain::notification::{AdminWatchedUser, NotificationEvent},
    error::{AppError, AppResult},
};

pub struct WatchlistRepo;

impl WatchlistRepo {
    pub async fn add_to_watchlist(
        pool: &SqlitePool,
        admin_id: &Uuid,
        user_id: &Uuid,
        movement_threshold_meters: f64,
    ) -> AppResult<AdminWatchedUser> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO admin_watched_users (id, admin_id, user_id, notify_on_movement, movement_threshold_meters, created_at)
            VALUES (?, ?, ?, 1, ?, ?)
            ON CONFLICT(admin_id, user_id) DO UPDATE SET
                movement_threshold_meters = excluded.movement_threshold_meters,
                notify_on_movement = 1
            "#,
        )
        .bind(id.to_string())
        .bind(admin_id.to_string())
        .bind(user_id.to_string())
        .bind(movement_threshold_meters)
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(AdminWatchedUser {
            id,
            admin_id: *admin_id,
            user_id: *user_id,
            notify_on_movement: true,
            movement_threshold_meters,
            created_at: now,
        })
    }

    pub async fn remove_from_watchlist(
        pool: &SqlitePool,
        admin_id: &Uuid,
        user_id: &Uuid,
    ) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM admin_watched_users WHERE admin_id = ? AND user_id = ?")
            .bind(admin_id.to_string())
            .bind(user_id.to_string())
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Watched user record not found in watchlist".to_string()));
        }
        Ok(())
    }

    pub async fn list_watchlist(pool: &SqlitePool, admin_id: &Uuid) -> AppResult<Vec<AdminWatchedUser>> {
        let rows = sqlx::query(
            r#"
            SELECT id, admin_id, user_id, notify_on_movement, movement_threshold_meters, created_at
            FROM admin_watched_users
            WHERE admin_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(admin_id.to_string())
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(Self::row_to_watched).collect()
    }

    pub async fn get_admins_watching_user(
        pool: &SqlitePool,
        user_id: &Uuid,
    ) -> AppResult<Vec<(Uuid, f64)>> {
        let rows = sqlx::query(
            r#"
            SELECT admin_id, movement_threshold_meters
            FROM admin_watched_users
            WHERE user_id = ? AND notify_on_movement = 1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let admin_id_str: &str = row.get("admin_id");
            let admin_id = Uuid::parse_str(admin_id_str).map_err(|e| AppError::Internal(e.to_string()))?;
            let threshold: f64 = row.get("movement_threshold_meters");
            results.push((admin_id, threshold));
        }

        Ok(results)
    }

    pub async fn save_notification(
        pool: &SqlitePool,
        recipient_id: &Uuid,
        subject_user_id: Option<&Uuid>,
        notification_type: &str,
        title: &str,
        body: &str,
        metadata: Option<&serde_json::Value>,
    ) -> AppResult<NotificationEvent> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let meta_str = metadata.map(|m| m.to_string());

        sqlx::query(
            r#"
            INSERT INTO notifications (id, recipient_id, subject_user_id, notification_type, title, body, metadata, is_read, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(recipient_id.to_string())
        .bind(subject_user_id.map(|u| u.to_string()))
        .bind(notification_type)
        .bind(title)
        .bind(body)
        .bind(meta_str)
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(NotificationEvent {
            id,
            recipient_id: *recipient_id,
            subject_user_id: subject_user_id.cloned(),
            notification_type: notification_type.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            metadata: metadata.cloned(),
            is_read: false,
            created_at: now,
        })
    }

    pub async fn list_notifications(
        pool: &SqlitePool,
        recipient_id: &Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<NotificationEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, recipient_id, subject_user_id, notification_type, title, body, metadata, is_read, created_at
            FROM notifications
            WHERE recipient_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(recipient_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id = Uuid::parse_str(row.get("id")).map_err(|e| AppError::Internal(e.to_string()))?;
            let rec_id = Uuid::parse_str(row.get("recipient_id")).map_err(|e| AppError::Internal(e.to_string()))?;
            let sub_id: Option<&str> = row.get("subject_user_id");
            let sub_uuid = sub_id.and_then(|s| Uuid::parse_str(s).ok());

            let meta_str: Option<&str> = row.get("metadata");
            let metadata = meta_str.and_then(|s| serde_json::from_str(s).ok());

            let is_read: i64 = row.get("is_read");
            let created_str: &str = row.get("created_at");
            let created_at = DateTime::parse_from_rfc3339(created_str)
                .map_err(|e| AppError::Internal(e.to_string()))?
                .with_timezone(&Utc);

            results.push(NotificationEvent {
                id,
                recipient_id: rec_id,
                subject_user_id: sub_uuid,
                notification_type: row.get("notification_type"),
                title: row.get("title"),
                body: row.get("body"),
                metadata,
                is_read: is_read != 0,
                created_at,
            });
        }

        Ok(results)
    }

    fn row_to_watched(row: sqlx::sqlite::SqliteRow) -> AppResult<AdminWatchedUser> {
        let id = Uuid::parse_str(row.get("id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let admin_id = Uuid::parse_str(row.get("admin_id")).map_err(|e| AppError::Internal(e.to_string()))?;
        let user_id = Uuid::parse_str(row.get("user_id")).map_err(|e| AppError::Internal(e.to_string()))?;

        let notify: i64 = row.get("notify_on_movement");
        let threshold: f64 = row.get("movement_threshold_meters");
        let created_str: &str = row.get("created_at");
        let created_at = DateTime::parse_from_rfc3339(created_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);

        Ok(AdminWatchedUser {
            id,
            admin_id,
            user_id,
            notify_on_movement: notify != 0,
            movement_threshold_meters: threshold,
            created_at,
        })
    }
}

