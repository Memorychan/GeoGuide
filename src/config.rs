use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry_mins: i64,
    pub refresh_expiry_days: i64,
    pub admin_registration_secret: String,
    pub default_movement_threshold_meters: f64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse::<u16>()
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://geoguide.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "geoguide_super_secure_jwt_secret_key_change_in_production_2026!".to_string()),
            jwt_access_expiry_mins: env::var("JWT_ACCESS_EXPIRY_MINS")
                .unwrap_or_else(|_| "15".to_string())
                .parse::<i64>()
                .unwrap_or(15),
            refresh_expiry_days: env::var("REFRESH_EXPIRY_DAYS")
                .unwrap_or_else(|_| "7".to_string())
                .parse::<i64>()
                .unwrap_or(7),
            admin_registration_secret: env::var("ADMIN_REGISTRATION_SECRET")
                .unwrap_or_else(|_| "geoguide_admin_secret_key_2026".to_string()),
            default_movement_threshold_meters: env::var("DEFAULT_MOVEMENT_THRESHOLD_METERS")
                .unwrap_or_else(|_| "25.0".to_string())
                .parse::<f64>()
                .unwrap_or(25.0),
        }
    }
}

