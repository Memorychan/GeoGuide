use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    domain::audit::{AuditLog, AuditLogQueryDto},
    error::{AppError, AppResult},
};

pub struct AuditRepo;

impl AuditRepo {
    pub async fn record(
        pool: &SqlitePool,
        actor_id: Option<&Uuid>,
        target_id: Option<&Uuid>,
        action: &str,
        resource: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        details: Option<&serde_json::Value>,
    ) -> AppResult<i64> {
        let now = Utc::now().to_rfc3339();
        let details_str = details.map(|d| d.to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO audit_logs (actor_id, target_id, action, resource, ip_address, user_agent, details, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(actor_id.map(|u| u.to_string()))
        .bind(target_id.map(|u| u.to_string()))
        .bind(action)
        .bind(resource)
        .bind(ip_address)
        .bind(user_agent)
        .bind(details_str)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn query(pool: &SqlitePool, query: &AuditLogQueryDto) -> AppResult<Vec<AuditLog>> {
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let offset = query.offset.unwrap_or(0).max(0);

        let rows = match (&query.actor_id, &query.action) {
            (Some(actor_id), Some(action)) => {
                sqlx::query(
                    r#"
                    SELECT id, actor_id, target_id, action, resource, ip_address, user_agent, details, created_at
                    FROM audit_logs
                    WHERE actor_id = ? AND action = ?
                    ORDER BY created_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(actor_id.to_string())
                .bind(action)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
            }
            (Some(actor_id), None) => {
                sqlx::query(
                    r#"
                    SELECT id, actor_id, target_id, action, resource, ip_address, user_agent, details, created_at
                    FROM audit_logs
                    WHERE actor_id = ?
                    ORDER BY created_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(actor_id.to_string())
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
            }
            (None, Some(action)) => {
                sqlx::query(
                    r#"
                    SELECT id, actor_id, target_id, action, resource, ip_address, user_agent, details, created_at
                    FROM audit_logs
                    WHERE action = ?
                    ORDER BY created_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(action)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
            }
            (None, None) => {
                sqlx::query(
                    r#"
                    SELECT id, actor_id, target_id, action, resource, ip_address, user_agent, details, created_at
                    FROM audit_logs
                    ORDER BY created_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
            }
        };

        let mut logs = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.get("id");
            let actor_id_str: Option<&str> = row.get("actor_id");
            let actor_id = actor_id_str.and_then(|s| Uuid::parse_str(s).ok());

            let target_id_str: Option<&str> = row.get("target_id");
            let target_id = target_id_str.and_then(|s| Uuid::parse_str(s).ok());

            let details_str: Option<&str> = row.get("details");
            let details = details_str.and_then(|s| serde_json::from_str(s).ok());

            let created_str: &str = row.get("created_at");
            let created_at = DateTime::parse_from_rfc3339(created_str)
                .map_err(|e| AppError::Internal(e.to_string()))?
                .with_timezone(&Utc);

            logs.push(AuditLog {
                id,
                actor_id,
                target_id,
                action: row.get("action"),
                resource: row.get("resource"),
                ip_address: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                details,
                created_at,
            });
        }

        Ok(logs)
    }
}

