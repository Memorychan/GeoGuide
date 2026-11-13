use sqlx::SqlitePool;
use crate::error::AppResult;

pub async fn initialize_database(pool: &SqlitePool) -> AppResult<()> {
    // Enable WAL mode for high concurrency and performance
    sqlx::query("PRAGMA journal_mode = WAL;").execute(pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON;").execute(pool).await?;
    sqlx::query("PRAGMA synchronous = NORMAL;").execute(pool).await?;

    let schema_sql = r#"
    CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        email TEXT UNIQUE NOT NULL,
        phone TEXT UNIQUE,
        password_hash TEXT NOT NULL,
        full_name TEXT NOT NULL,
        role TEXT NOT NULL,
        is_active INTEGER NOT NULL DEFAULT 1,
        is_tracking_enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS refresh_tokens (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        token_hash TEXT UNIQUE NOT NULL,
        family_id TEXT NOT NULL,
        is_revoked INTEGER NOT NULL DEFAULT 0,
        device_info TEXT,
        expires_at TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS locations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        latitude REAL NOT NULL,
        longitude REAL NOT NULL,
        altitude REAL,
        accuracy REAL,
        speed REAL,
        heading REAL,
        battery_level INTEGER,
        is_mock_location INTEGER NOT NULL DEFAULT 0,
        recorded_at TEXT NOT NULL,
        received_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_locations_user_time ON locations(user_id, recorded_at DESC);
    CREATE INDEX IF NOT EXISTS idx_locations_time ON locations(recorded_at);

    CREATE TABLE IF NOT EXISTS admin_consents (
        id TEXT PRIMARY KEY,
        user_id TEXT UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        admin_id TEXT REFERENCES users(id),
        allow_live_tracking INTEGER NOT NULL DEFAULT 1,
        allow_timeline_history INTEGER NOT NULL DEFAULT 1,
        granted_at TEXT NOT NULL,
        revoked_at TEXT
    );

    CREATE TABLE IF NOT EXISTS location_shares (
        id TEXT PRIMARY KEY,
        granter_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        grantee_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        share_type TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        is_active INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_shares_lookup ON location_shares(granter_id, grantee_id, is_active);

    CREATE TABLE IF NOT EXISTS location_requests (
        id TEXT PRIMARY KEY,
        requester_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        target_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        requested_type TEXT NOT NULL,
        duration_minutes INTEGER NOT NULL,
        status TEXT NOT NULL,
        message TEXT,
        created_at TEXT NOT NULL,
        responded_at TEXT
    );

    CREATE TABLE IF NOT EXISTS admin_watched_users (
        id TEXT PRIMARY KEY,
        admin_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        notify_on_movement INTEGER NOT NULL DEFAULT 1,
        movement_threshold_meters REAL NOT NULL DEFAULT 25.0,
        created_at TEXT NOT NULL,
        UNIQUE(admin_id, user_id)
    );

    CREATE TABLE IF NOT EXISTS notifications (
        id TEXT PRIMARY KEY,
        recipient_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        subject_user_id TEXT REFERENCES users(id),
        notification_type TEXT NOT NULL,
        title TEXT NOT NULL,
        body TEXT NOT NULL,
        metadata TEXT,
        is_read INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_notifications_recipient ON notifications(recipient_id, created_at DESC);

    CREATE TABLE IF NOT EXISTS audit_logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        actor_id TEXT,
        target_id TEXT,
        action TEXT NOT NULL,
        resource TEXT NOT NULL,
        ip_address TEXT,
        user_agent TEXT,
        details TEXT,
        created_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs(actor_id, created_at DESC);
    "#;

    for statement in schema_sql.split(';') {
        let stmt = statement.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(pool).await?;
        }
    }

    tracing::info!("SQLite database schema verified and initialized.");
    Ok(())
}

