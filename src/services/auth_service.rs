use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    domain::user::{AuthResponseDto, LoginDto, RegisterDto, Role, User, UserResponseDto},
    error::{AppError, AppResult},
    repository::{audit_repo::AuditRepo, user_repo::UserRepo},
    security::{
        jwt::create_access_token,
        password::{hash_password, verify_password},
        token::{generate_refresh_token, hash_token},
    },
    state::AppState,
};

pub struct AuthService;

impl AuthService {
    pub async fn register(
        state: &AppState,
        dto: RegisterDto,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AppResult<UserResponseDto> {
        if dto.email.trim().is_empty() || !dto.email.contains('@') {
            return Err(AppError::BadRequest("Invalid email address".to_string()));
        }
        if dto.password.len() < 8 {
            return Err(AppError::BadRequest("Password must be at least 8 characters long".to_string()));
        }

        // Determine Role based on admin registration secret
        let role = match dto.admin_secret {
            Some(secret) if secret == state.config.admin_registration_secret => Role::Admin,
            _ => Role::User,
        };

        let password_hash = hash_password(&dto.password)?;
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let user = User {
            id: user_id,
            email: dto.email.to_lowercase().trim().to_string(),
            phone: dto.phone.map(|p| p.trim().to_string()),
            password_hash,
            full_name: dto.full_name.trim().to_string(),
            role,
            is_active: true,
            is_tracking_enabled: true, // Continuous tracking active immediately on registration!
            created_at: now,
            updated_at: now,
        };

        UserRepo::create_user(&state.db, &user).await?;

        // Audit register event
        AuditRepo::record(
            &state.db,
            Some(&user_id),
            Some(&user_id),
            "REGISTER",
            "users",
            ip_address.as_deref(),
            user_agent.as_deref(),
            Some(&serde_json::json!({ "role": role.to_string(), "tracking_enabled": true })),
        )
        .await?;

        Ok(user.into())
    }

    pub async fn login(
        state: &AppState,
        dto: LoginDto,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AppResult<AuthResponseDto> {
        let email = dto.email.to_lowercase().trim().to_string();
        let user = UserRepo::find_by_email(&state.db, &email)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

        if !user.is_active {
            return Err(AppError::Forbidden("Account is deactivated".to_string()));
        }

        let is_valid = verify_password(&dto.password, &user.password_hash)?;
        if !is_valid {
            AuditRepo::record(
                &state.db,
                Some(&user.id),
                Some(&user.id),
                "LOGIN_FAILED",
                "auth",
                ip_address.as_deref(),
                user_agent.as_deref(),
                Some(&serde_json::json!({ "reason": "Invalid password" })),
            )
            .await?;

            return Err(AppError::Unauthorized("Invalid email or password".to_string()));
        }

        let session_id = Uuid::new_v4();
        let access_token = create_access_token(
            user.id,
            user.role,
            session_id,
            &state.config.jwt_secret,
            state.config.jwt_access_expiry_mins,
        )?;

        // Generate opaque refresh token and family
        let raw_refresh_token = generate_refresh_token();
        let refresh_token_hash = hash_token(&raw_refresh_token);
        let family_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::days(state.config.refresh_expiry_days);

        UserRepo::save_refresh_token(
            &state.db,
            &Uuid::new_v4(),
            &user.id,
            &refresh_token_hash,
            &family_id,
            dto.device_info.as_deref(),
            expires_at,
        )
        .await?;

        AuditRepo::record(
            &state.db,
            Some(&user.id),
            Some(&user.id),
            "LOGIN_SUCCESS",
            "auth",
            ip_address.as_deref(),
            user_agent.as_deref(),
            Some(&serde_json::json!({ "session_id": session_id.to_string() })),
        )
        .await?;

        Ok(AuthResponseDto {
            access_token,
            refresh_token: raw_refresh_token,
            token_type: "Bearer".to_string(),
            expires_in_seconds: state.config.jwt_access_expiry_mins * 60,
            user: user.into(),
        })
    }

    pub async fn refresh(
        state: &AppState,
        raw_refresh_token: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AppResult<AuthResponseDto> {
        let token_hash = hash_token(raw_refresh_token);
        let token_record = UserRepo::find_refresh_token(&state.db, &token_hash)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid refresh token".to_string()))?;

        let (_id, user_id, family_id, is_revoked, expires_at) = token_record;

        // Token family reuse detection: If an already-revoked token is used, someone may have stolen it
        if is_revoked {
            tracing::warn!("Revoked refresh token reuse detected for family {}. Revoking all family tokens.", family_id);
            UserRepo::revoke_family(&state.db, &family_id).await?;

            AuditRepo::record(
                &state.db,
                Some(&user_id),
                Some(&user_id),
                "SECURITY_ALERT_TOKEN_REUSE",
                "auth",
                ip_address.as_deref(),
                user_agent.as_deref(),
                Some(&serde_json::json!({ "family_id": family_id.to_string() })),
            )
            .await?;

            return Err(AppError::Unauthorized("Refresh token has been revoked due to security violation".to_string()));
        }

        if Utc::now() > expires_at {
            return Err(AppError::Unauthorized("Refresh token has expired".to_string()));
        }

        // Fetch user
        let user = UserRepo::find_by_id(&state.db, &user_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("User no longer exists".to_string()))?;

        if !user.is_active {
            return Err(AppError::Forbidden("Account is deactivated".to_string()));
        }

        // Invalidate current refresh token
        UserRepo::revoke_token(&state.db, &token_hash).await?;

        // Issue new refresh token within the same family (token family rotation)
        let new_raw_refresh_token = generate_refresh_token();
        let new_refresh_token_hash = hash_token(&new_raw_refresh_token);
        let new_expires_at = Utc::now() + Duration::days(state.config.refresh_expiry_days);

        UserRepo::save_refresh_token(
            &state.db,
            &Uuid::new_v4(),
            &user.id,
            &new_refresh_token_hash,
            &family_id,
            None,
            new_expires_at,
        )
        .await?;

        // Issue new JWT access token
        let session_id = Uuid::new_v4();
        let access_token = create_access_token(
            user.id,
            user.role,
            session_id,
            &state.config.jwt_secret,
            state.config.jwt_access_expiry_mins,
        )?;

        AuditRepo::record(
            &state.db,
            Some(&user.id),
            Some(&user.id),
            "TOKEN_REFRESH",
            "auth",
            ip_address.as_deref(),
            user_agent.as_deref(),
            None,
        )
        .await?;

        Ok(AuthResponseDto {
            access_token,
            refresh_token: new_raw_refresh_token,
            token_type: "Bearer".to_string(),
            expires_in_seconds: state.config.jwt_access_expiry_mins * 60,
            user: user.into(),
        })
    }

    pub async fn logout(
        state: &AppState,
        raw_refresh_token: &str,
        user_id: &Uuid,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AppResult<()> {
        let token_hash = hash_token(raw_refresh_token);
        UserRepo::revoke_token(&state.db, &token_hash).await?;

        AuditRepo::record(
            &state.db,
            Some(user_id),
            Some(user_id),
            "LOGOUT",
            "auth",
            ip_address.as_deref(),
            user_agent.as_deref(),
            None,
        )
        .await?;

        Ok(())
    }
}

