# GeoGuide Backend

High-performance, secure, asynchronous location-tracking backend written in **Rust** using **Axum**, **Tokio**, and **SQLx**. Built to support second-by-second timeline reconstruction, real-time live map streaming via WebSockets, time-limited peer-to-peer location sharing, and role-based Admin controls with real-time movement alerts for selected users.

---

## Key Features

- **Blazing Fast & Low Memory Footprint**: Powered by Axum 0.7, Tokio work-stealing async scheduler, and SQLx.
- **Continuous 1-Second Timeline Tracking**:
  - Automatically enabled upon user account creation.
  - Supports 1 Hz direct WebSocket ingestion (`/api/v1/ws/location/stream`) and compressed batch REST uploads (`/api/v1/locations/batch`) for battery-saving background sync.
  - High-precision timestamp preservation down to the millisecond/second.
- **Admin Selected Watchlist & Movement Detection**:
  - Admin can select specific users to monitor.
  - Ingestion calculates geodesic displacement using the Haversine formula in real time.
  - Movement alerts are dispatched **only** for watched users via dedicated WebSocket feed (`/api/v1/admin/ws/notifications`) and saved to notifications.
- **Time-Limited Peer-to-Peer Sharing**:
  - Mandatory duration limits on all location shares (15m, 1h, 8h, 24h, custom).
  - Mutual or one-way sharing supported.
  - Full request-approval workflow (`/api/v1/sharing/requests`).
- **Cryptographic Security**:
  - **Argon2id** password hashing.
  - **Stateless JWT** access tokens (15-minute expiry).
  - **Opaque Refresh Tokens** (256-bit CSPRNG) with automatic Token Family reuse detection.
  - **AES-256-GCM** field encryption helpers.
- **Immutable Audit Logging**: Every critical security action (logins, consent modifications, shares, timeline inspections) is persisted to an append-only audit trail.
- **Interactive Swagger UI & OpenAPI Specification**: Auto-generated OpenAPI 3.1 contract accessible at `/swagger-ui` and `/api-docs/openapi.json`.
- **Database Flexibility**: Default embedded SQLite with WAL mode for instantaneous zero-setup local development, with production-ready PostgreSQL 16 + PostGIS migrations provided.

---

## System Architecture

```text
d:\GeoGuide\
├── Cargo.toml                          # Rust 2021 crate manifest & dependencies
├── .env.example                        # Template environment variables
├── .env                                # Local environment settings
├── migrations/
│   └── postgres/
│       └── 001_initial_schema.sql      # Production PostgreSQL 16 + PostGIS migration
├── src/
│   ├── lib.rs                          # Library root exposing all core modules
│   ├── main.rs                         # Application entrypoint & HTTP/WS server
│   ├── config.rs                       # AppConfig environment loader
│   ├── error.rs                        # Centralized AppError and uniform error responses
│   ├── state.rs                        # Shared AppState (DB pool, channel hubs, position cache)
│   ├── common/
│   │   └── response.rs                 # Standardized ApiResponse<T> envelope
│   ├── domain/                         # Core domain entities and DTOs
│   │   ├── user.rs                     # User, Role, RegisterDto, LoginDto
│   │   ├── location.rs                 # LocationFix, BatchDto, Haversine formula
│   │   ├── sharing.rs                  # Time-limited shares, requests, Admin consent
│   │   ├── notification.rs             # Admin watchlist, MovementAlertDto, notifications
│   │   └── audit.rs                    # AuditLog, AuditLogQueryDto
│   ├── security/                       # Cryptographic primitives
│   │   ├── password.rs                 # Argon2id password hashing & verification
│   │   ├── jwt.rs                      # JWT encoding and claims verification
│   │   ├── token.rs                    # 256-bit CSPRNG refresh token generator & SHA-256
│   │   └── encryption.rs               # AES-256-GCM field-level encryption
│   ├── repository/                     # Database access layer
│   │   ├── schema.rs                   # SQLite WAL schema auto-initialization
│   │   ├── user_repo.rs                # User CRUD & refresh token family rotation
│   │   ├── location_repo.rs            # Batch insert & second-resolution timeline query
│   │   ├── sharing_repo.rs             # Consent, active time-limited shares, requests
│   │   ├── watchlist_repo.rs           # Admin watchlist and notifications
│   │   └── audit_repo.rs               # Immutable append-only audit trail
│   ├── services/                       # Business logic & event processing
│   │   ├── auth_service.rs             # Registration, login, token rotation, logout
│   │   ├── location_service.rs         # Ingestion, broadcasting & movement detection
│   │   └── sharing_service.rs          # Share validation & access verification
│   ├── middleware/
│   │   └── auth.rs                     # AuthUser and RequireAdmin Axum extractors
│   └── api/                            # REST handlers & WebSocket routes
│       ├── auth_handlers.rs            # Register, login, refresh, profile, tracking toggle
│       ├── location_handlers.rs        # Ingest single, batch, latest fix
│       ├── timeline_handlers.rs        # Second-by-second personal & peer timeline
│       ├── sharing_handlers.rs         # Admin consent, time-limited shares, requests
│       ├── admin_handlers.rs           # Users list, watchlist, notifications, audit logs
│       ├── ws_handlers.rs              # 1Hz ingestion stream, live view, admin alert WS
│       └── openapi.rs                  # Utoipa OpenAPI 3.1 specification
├── tests/
│   └── backend_tests.rs                # Full unit & integration test suite
└── ANDROID_AI_STUDIO_GUIDE.md          # Turnkey prompts & API reference for Google AI Studio
```

---

## Quick Start

### 1. Requirements
- Rust toolchain (`rustc`, `cargo` 1.75+ or newer).

### 2. Run the Backend
```powershell
# In d:\GeoGuide
cargo run
```
The server binds to `http://0.0.0.0:8080`.
- **Health Check**: `http://localhost:8080/health`
- **Interactive Swagger UI**: `http://localhost:8080/swagger-ui`
- **OpenAPI JSON Spec**: `http://localhost:8080/api-docs/openapi.json`

### 3. Run the Automated Test Suite
```powershell
cargo test
```
Runs 5 automated unit and integration tests covering:
- Argon2id password hashing and constant-time verification
- JWT claims issuance and validation
- CSPRNG refresh token generation and hashing
- Haversine geodesic distance calculation
- Full end-to-end integration flow: Admin registration, tracked user continuous telemetry, movement detection exceeding threshold, admin notification dispatch, second-by-second timeline queries, and time-limited peer sharing with automatic expiration.

---

## Environment Variables (`.env`)

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `HOST` | `0.0.0.0` | Server bind IP address |
| `PORT` | `8080` | Server listening port |
| `DATABASE_URL` | `sqlite://geoguide.db?mode=rwc` | SQLite or PostgreSQL connection string |
| `JWT_SECRET` | *(Random 64-char string)* | HMAC secret for signing JWT access tokens |
| `JWT_ACCESS_EXPIRY_MINS`| `15` | Expiration time for access tokens in minutes |
| `REFRESH_EXPIRY_DAYS` | `7` | Expiration time for refresh tokens in days |
| `ADMIN_REGISTRATION_SECRET`| `geoguide_admin_secret_key_2026` | Secret key provided during registration to claim `ADMIN` role |
| `DEFAULT_MOVEMENT_THRESHOLD_METERS`| `25.0` | Default distance displacement required to trigger movement alert |

---

## Developing the Android App

Refer to [ANDROID_AI_STUDIO_GUIDE.md](file:///d:/GeoGuide/ANDROID_AI_STUDIO_GUIDE.md) for complete copy-paste prompts designed for **Google AI Studio**, complete with:
- Retrofit networking layer with automatic JWT token refresh.
- 1-second continuous GPS tracking foreground service with Room offline buffering.
- Jetpack Compose Live Map and second-by-second Timeline scrubber.
- Admin Watchlist and movement notification listener.

