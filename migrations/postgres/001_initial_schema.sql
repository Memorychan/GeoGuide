-- PostgreSQL 16 + PostGIS Migration for GeoGuide
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "postgis";

-- 1. Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    phone VARCHAR(32) UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    full_name VARCHAR(128) NOT NULL,
    role VARCHAR(16) NOT NULL CHECK (role IN ('ADMIN', 'USER')),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_tracking_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Refresh Tokens with Token Family Rotation
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(64) UNIQUE NOT NULL,
    family_id UUID NOT NULL,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
    device_info TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_hash ON refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family ON refresh_tokens(family_id);

-- 3. High-Frequency Locations (Time-series table)
CREATE TABLE IF NOT EXISTS locations (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    altitude DOUBLE PRECISION,
    accuracy REAL,
    speed REAL,
    heading REAL,
    battery_level SMALLINT,
    is_mock_location BOOLEAN NOT NULL DEFAULT FALSE,
    recorded_at TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_locations_user_recorded ON locations(user_id, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_locations_recorded ON locations(recorded_at DESC);

-- 4. Admin Consents
CREATE TABLE IF NOT EXISTS admin_consents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    admin_id UUID REFERENCES users(id),
    allow_live_tracking BOOLEAN NOT NULL DEFAULT TRUE,
    allow_timeline_history BOOLEAN NOT NULL DEFAULT TRUE,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);

-- 5. Time-Limited Location Shares (User-to-User)
CREATE TABLE IF NOT EXISTS location_shares (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    granter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    grantee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    share_type VARCHAR(16) NOT NULL CHECK (share_type IN ('LIVE', 'TIMELINE', 'BOTH')),
    expires_at TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shares_lookup ON location_shares(granter_id, grantee_id, is_active, expires_at);

-- 6. Location Requests
CREATE TABLE IF NOT EXISTS location_requests (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    requester_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requested_type VARCHAR(16) NOT NULL CHECK (requested_type IN ('LIVE', 'TIMELINE', 'BOTH')),
    duration_minutes BIGINT NOT NULL,
    status VARCHAR(16) NOT NULL CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED', 'EXPIRED')),
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    responded_at TIMESTAMPTZ
);

-- 7. Admin Watched Users (Movement Alerts)
CREATE TABLE IF NOT EXISTS admin_watched_users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    admin_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    notify_on_movement BOOLEAN NOT NULL DEFAULT TRUE,
    movement_threshold_meters DOUBLE PRECISION NOT NULL DEFAULT 25.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(admin_id, user_id)
);

-- 8. Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject_user_id UUID REFERENCES users(id),
    notification_type VARCHAR(32) NOT NULL,
    title VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    metadata JSONB,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_recipient ON notifications(recipient_id, created_at DESC);

-- 9. Immutable Audit Logs
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    actor_id UUID,
    target_id UUID,
    action VARCHAR(64) NOT NULL,
    resource VARCHAR(64) NOT NULL,
    ip_address VARCHAR(45),
    user_agent TEXT,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs(actor_id, created_at DESC);

