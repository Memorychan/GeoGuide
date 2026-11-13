use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Admin,
    User,
}

impl ToString for Role {
    fn to_string(&self) -> String {
        match self {
            Role::Admin => "ADMIN".to_string(),
            Role::User => "USER".to_string(),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ADMIN" => Ok(Role::Admin),
            "USER" => Ok(Role::User),
            other => Err(format!("Unknown role: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub phone: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: String,
    pub role: Role,
    pub is_active: bool,
    pub is_tracking_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegisterDto {
    pub email: String,
    pub phone: Option<String>,
    pub password: String,
    pub full_name: String,
    pub admin_secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
    pub device_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthResponseDto {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in_seconds: i64,
    pub user: UserResponseDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponseDto {
    pub id: Uuid,
    pub email: String,
    pub phone: Option<String>,
    pub full_name: String,
    pub role: Role,
    pub is_active: bool,
    pub is_tracking_enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponseDto {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            phone: u.phone,
            full_name: u.full_name,
            role: u.role,
            is_active: u.is_active,
            is_tracking_enabled: u.is_tracking_enabled,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RefreshTokenDto {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateTrackingStatusDto {
    pub is_tracking_enabled: bool,
}

