use chrono::{Duration, Utc};
use geoguide::{
    config::AppConfig,
    domain::{
        location::{haversine_distance_meters, SingleLocationInputDto, TimelineQueryDto},
        sharing::ShareType,
        user::{RegisterDto, Role},
    },
    repository::{
        schema::initialize_database,
        watchlist_repo::WatchlistRepo,
    },
    security::{
        jwt::{create_access_token, verify_access_token},
        password::{hash_password, verify_password},
        token::{generate_refresh_token, hash_token},
    },
    services::{
        auth_service::AuthService,
        location_service::LocationService,
        sharing_service::SharingService,
    },
    state::AppState,
};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

#[test]
fn test_password_hashing() {
    let password = "SuperSecretPassword123!";
    let hashed = hash_password(password).expect("Hashing failed");
    assert_ne!(password, hashed);
    assert!(verify_password(password, &hashed).expect("Verification failed"));
    assert!(!verify_password("WrongPassword!", &hashed).expect("Verification failed"));
}

#[test]
fn test_jwt_generation_and_validation() {
    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let secret = "test_jwt_secret_key_1234567890_test";

    let token = create_access_token(user_id, Role::User, session_id, secret, 15)
        .expect("JWT creation failed");

    let claims = verify_access_token(&token, secret).expect("JWT validation failed");
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.role, Role::User);
    assert_eq!(claims.session_id, session_id);
}

#[test]
fn test_haversine_distance_calculation() {
    // Distance between London (51.5074, -0.1278) and Paris (48.8566, 2.3522) is ~343 km
    let dist = haversine_distance_meters(51.5074, -0.1278, 48.8566, 2.3522);
    assert!(dist > 340_000.0 && dist < 345_000.0);

    // Minor displacement: ~111 meters for 0.001 deg latitude
    let local_dist = haversine_distance_meters(12.9716, 77.5946, 12.9726, 77.5946);
    assert!(local_dist > 110.0 && local_dist < 112.0);
}

#[test]
fn test_refresh_token_generation() {
    let token1 = generate_refresh_token();
    let token2 = generate_refresh_token();
    assert_ne!(token1, token2);
    assert_eq!(token1.len(), 64);

    let hash1 = hash_token(&token1);
    let hash2 = hash_token(&token1);
    assert_eq!(hash1, hash2);
}

#[tokio::test]
async fn test_full_database_and_services_workflow() {
    // Setup in-memory SQLite pool for isolated testing
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to connect to memory SQLite");

    initialize_database(&pool).await.expect("Schema init failed");

    let config = AppConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "test_secret_for_workflow_suite_2026".to_string(),
        jwt_access_expiry_mins: 15,
        refresh_expiry_days: 7,
        admin_registration_secret: "admin_master_key".to_string(),
        default_movement_threshold_meters: 20.0,
    };

    let state = AppState::new(pool.clone(), config);

    // 1. Register Admin User
    let admin_dto = RegisterDto {
        email: "admin@geoguide.com".to_string(),
        phone: Some("+10000000000".to_string()),
        password: "AdminPassword123!".to_string(),
        full_name: "System Admin".to_string(),
        admin_secret: Some("admin_master_key".to_string()),
    };
    let admin_user = AuthService::register(&state, admin_dto, None, None).await.expect("Admin reg failed");
    assert_eq!(admin_user.role, Role::Admin);

    // 2. Register Tracked User (Continuous tracking enabled by default!)
    let user_dto = RegisterDto {
        email: "runner@geoguide.com".to_string(),
        phone: Some("+19999999999".to_string()),
        password: "UserPassword123!".to_string(),
        full_name: "Active Runner".to_string(),
        admin_secret: None,
    };
    let user = AuthService::register(&state, user_dto, None, None).await.expect("User reg failed");
    assert_eq!(user.role, Role::User);
    assert!(user.is_tracking_enabled, "Continuous tracking must be enabled by default upon registration");

    // 3. Admin selects user to watch for movement alerts
    let watched = WatchlistRepo::add_to_watchlist(&state.db, &admin_user.id, &user.id, 15.0)
        .await
        .expect("Add watchlist failed");
    assert_eq!(watched.user_id, user.id);
    assert_eq!(watched.movement_threshold_meters, 15.0);

    // 4. Ingest 1-second GPS fixes (Simulating high-frequency Android background tracking)
    let base_time = Utc::now();
    let fix1 = LocationService::ingest_single(
        &state,
        &user.id,
        SingleLocationInputDto {
            latitude: 12.9716,
            longitude: 77.5946,
            altitude: Some(920.0),
            accuracy: Some(3.0),
            speed: Some(1.2),
            heading: Some(90.0),
            battery_level: Some(85),
            is_mock_location: Some(false),
            recorded_at: Some(base_time),
        },
    )
    .await
    .expect("Fix 1 ingestion failed");
    assert!(fix1.id.is_some());

    // Fix 2: Moved ~111 meters (greater than the 15m admin watch threshold!)
    let _fix2 = LocationService::ingest_single(
        &state,
        &user.id,
        SingleLocationInputDto {
            latitude: 12.9726, // Moved north
            longitude: 77.5946,
            altitude: Some(920.5),
            accuracy: Some(2.5),
            speed: Some(3.5),
            heading: Some(0.0),
            battery_level: Some(84),
            is_mock_location: Some(false),
            recorded_at: Some(base_time + Duration::seconds(1)),
        },
    )
    .await
    .expect("Fix 2 ingestion failed");

    // 5. Verify Admin received movement alert notification
    let notifs = WatchlistRepo::list_notifications(&state.db, &admin_user.id, 10, 0)
        .await
        .expect("List notifs failed");
    assert!(!notifs.is_empty(), "Admin must receive movement alert for watched user");
    assert_eq!(notifs[0].notification_type, "MOVEMENT_DETECTED");

    // 6. Test Second-by-Second Timeline Query
    let timeline = LocationService::query_timeline(
        &state,
        &user.id,
        &TimelineQueryDto {
            from: Some(base_time - Duration::seconds(5)),
            to: Some(base_time + Duration::seconds(10)),
            limit: Some(100),
            offset: Some(0),
        },
    )
    .await
    .expect("Timeline query failed");
    assert_eq!(timeline.total_points, 2);
    assert!(timeline.total_distance_meters > 100.0);

    // 7. Test Time-Limited Peer-to-Peer Sharing
    let peer_dto = RegisterDto {
        email: "friend@geoguide.com".to_string(),
        phone: None,
        password: "FriendPassword123!".to_string(),
        full_name: "Friend User".to_string(),
        admin_secret: None,
    };
    let friend = AuthService::register(&state, peer_dto, None, None).await.expect("Friend reg failed");

    // Grant 60-minute time-limited share
    let share = SharingService::create_share(
        &state,
        &user.id,
        geoguide::domain::sharing::CreateShareDto {
            grantee_id: friend.id,
            share_type: ShareType::Both,
            duration_minutes: 60,
        },
        None,
        None,
    )
    .await
    .expect("Create share failed");
    assert!(share.expires_at > Utc::now());

    // Verify friend has access
    let has_access = SharingService::verify_peer_access(&state, &user.id, &friend.id, ShareType::Timeline)
        .await
        .expect("Verify access failed");
    assert!(has_access);

    // Revoke share
    SharingService::revoke_share(&state, &share.id, &user.id, None, None).await.expect("Revoke failed");
    let has_access_after_revoke = SharingService::verify_peer_access(&state, &user.id, &friend.id, ShareType::Timeline)
        .await
        .expect("Verify access failed");
    assert!(!has_access_after_revoke, "Revoked share must not grant access");
}

