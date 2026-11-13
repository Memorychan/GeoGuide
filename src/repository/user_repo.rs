use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    domain::user::{Role, User},
    error::{AppError, AppResult},
};

pub struct UserRepo;

impl UserRepo {
    pub async fn create_user(
        pool: &SqlitePool,
        user: &User,
    ) -> AppResult<()> {
        let role_str = user.role.to_string();
        let now_str = user.created_at.to_rfc3339();
        let updated_str = user.updated_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO users (id, email, phone, password_hash, full_name, role, is_active, is_tracking_enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.id.to_string())
        .bind(&user.email)
        .bind(&user.phone)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(role_str)
        .bind(if user.is_active { 1 } else { 0 })
        .bind(if user.is_tracking_enabled { 1 } else { 0 })
        .bind(now_str)
        .bind(updated_str)
        .execute(pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.message().contains("UNIQUE constraint failed") {
                    return AppError::Conflict("User with this email or phone already exists".to_string());
                }
            }
            AppError::Database(e)
        })?;

        // Initialize default admin tracking consent for the newly registered user
        let consent_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO admin_consents (id, user_id, admin_id, allow_live_tracking, allow_timeline_history, granted_at)
            VALUES (?, ?, NULL, 1, 1, ?)
            "#,
        )
        .bind(consent_id)
        .bind(user.id.to_string())
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_email(pool: &SqlitePool, email: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, email, phone, password_hash, full_name, role, is_active, is_tracking_enabled, created_at, updated_at
            FROM users WHERE email = ?
            "#,
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        row.map(Self::row_to_user).transpose()
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &Uuid) -> AppResult<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, email, phone, password_hash, full_name, role, is_active, is_tracking_enabled, created_at, updated_at
            FROM users WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

        row.map(Self::row_to_user).transpose()
    }

    pub async fn list_users(pool: &SqlitePool) -> AppResult<Vec<User>> {
        let rows = sqlx::query(
            r#"
            SELECT id, email, phone, password_hash, full_name, role, is_active, is_tracking_enabled, created_at, updated_at
            FROM users ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(Self::row_to_user).collect()
    }

    pub async fn update_tracking_status(pool: &SqlitePool, user_id: &Uuid, enabled: bool) -> AppResult<()> {
        let updated_at = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE users SET is_tracking_enabled = ?, updated_at = ? WHERE id = ?
            "#,
        )
        .bind(if enabled { 1 } else { 0 })
        .bind(updated_at)
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

        Ok(())
    }

    // Refresh Token Management with Token Family Reuse Detection
    pub async fn save_refresh_token(
        pool: &SqlitePool,
        id: &Uuid,
        user_id: &Uuid,
        token_hash: &str,
        family_id: &Uuid,
        device_info: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, family_id, is_revoked, device_info, expires_at, created_at)
            VALUES (?, ?, ?, ?, 0, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(token_hash)
        .bind(family_id.to_string())
        .bind(device_info)
        .bind(expires_at.to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn find_refresh_token(
        pool: &SqlitePool,
        token_hash: &str,
    ) -> AppResult<Option<(Uuid, Uuid, Uuid, bool, DateTime<Utc>)>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, family_id, is_revoked, expires_at
            FROM refresh_tokens WHERE token_hash = ?
            "#,
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(r) => {
                let id = Uuid::parse_str(r.get::<&str, _>("id")).map_err(|e| AppError::Internal(e.to_string()))?;
                let user_id = Uuid::parse_str(r.get::<&str, _>("user_id")).map_err(|e| AppError::Internal(e.to_string()))?;
                let family_id = Uuid::parse_str(r.get::<&str, _>("family_id")).map_err(|e| AppError::Internal(e.to_string()))?;
                let is_revoked: i64 = r.get("is_revoked");
                let expires_at_str: &str = r.get("expires_at");
                let expires_at = DateTime::parse_from_rfc3339(expires_at_str)
                    .map_err(|e| AppError::Internal(e.to_string()))?
                    .with_timezone(&Utc);

                Ok(Some((id, user_id, family_id, is_revoked != 0, expires_at)))
            }
            None => Ok(None),
        }
    }

    pub async fn revoke_token(pool: &SqlitePool, token_hash: &str) -> AppResult<()> {
        sqlx::query("UPDATE refresh_tokens SET is_revoked = 1 WHERE token_hash = ?")
            .bind(token_hash)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn revoke_family(pool: &SqlitePool, family_id: &Uuid) -> AppResult<()> {
        sqlx::query("UPDATE refresh_tokens SET is_revoked = 1 WHERE family_id = ?")
            .bind(family_id.to_string())
            .execute(pool)
            .await?;
        Ok(())
    }

    fn row_to_user(row: sqlx::sqlite::SqliteRow) -> AppResult<User> {
        let id_str: &str = row.get("id");
        let role_str: &str = row.get("role");
        let created_str: &str = row.get("created_at");
        let updated_str: &str = row.get("updated_at");

        let id = Uuid::parse_str(id_str).map_err(|e| AppError::Internal(e.to_string()))?;
        let role = Role::from_str(role_str).map_err(AppError::Internal)?;
        let created_at = DateTime::parse_from_rfc3339(created_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(updated_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);

        let is_active: i64 = row.get("is_active");
        let is_tracking_enabled: i64 = row.get("is_tracking_enabled");

        Ok(User {
            id,
            email: row.get("email"),
            phone: row.get("phone"),
            password_hash: row.get("password_hash"),
            full_name: row.get("full_name"),
            role,
            is_active: is_active != 0,
            is_tracking_enabled: is_tracking_enabled != 0,
            created_at,
            updated_at,
        })
    }
}

