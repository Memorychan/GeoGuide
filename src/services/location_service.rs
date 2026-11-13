use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{
        location::{haversine_distance_meters, BatchLocationInputDto, LocationFix, SingleLocationInputDto, TimelineQueryDto, TimelineResponseDto},
        notification::MovementAlertDto,
    },
    error::{AppError, AppResult},
    repository::{
        location_repo::LocationRepo,
        user_repo::UserRepo,
        watchlist_repo::WatchlistRepo,
    },
    state::AppState,
};

pub struct LocationService;

impl LocationService {
    pub async fn ingest_single(
        state: &AppState,
        user_id: &Uuid,
        dto: SingleLocationInputDto,
    ) -> AppResult<LocationFix> {
        let user = UserRepo::find_by_id(&state.db, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if !user.is_tracking_enabled {
            return Err(AppError::Forbidden("Location tracking is currently disabled for your account".to_string()));
        }

        let now = Utc::now();
        let fix = LocationFix {
            id: None,
            user_id: *user_id,
            latitude: dto.latitude,
            longitude: dto.longitude,
            altitude: dto.altitude,
            accuracy: dto.accuracy,
            speed: dto.speed,
            heading: dto.heading,
            battery_level: dto.battery_level,
            is_mock_location: dto.is_mock_location.unwrap_or(false),
            recorded_at: dto.recorded_at.unwrap_or(now),
            received_at: now,
        };

        // 1. Insert into database
        let inserted_id = LocationRepo::insert_location(&state.db, &fix).await?;
        let mut fix_with_id = fix.clone();
        fix_with_id.id = Some(inserted_id);

        // 2. Broadcast to live real-time stream
        let user_channel = state.get_or_create_user_channel(user_id);
        let _ = user_channel.send(fix_with_id.clone());

        // 3. Process movement detection for Admins watching this user
        Self::detect_and_notify_movement(state, user_id, &user.full_name, &fix).await?;

        Ok(fix_with_id)
    }

    pub async fn ingest_batch(
        state: &AppState,
        user_id: &Uuid,
        dto: BatchLocationInputDto,
    ) -> AppResult<usize> {
        if dto.fixes.is_empty() {
            return Ok(0);
        }

        let user = UserRepo::find_by_id(&state.db, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if !user.is_tracking_enabled {
            return Err(AppError::Forbidden("Location tracking is currently disabled for your account".to_string()));
        }

        let now = Utc::now();
        let mut fixes = Vec::with_capacity(dto.fixes.len());

        for f in dto.fixes {
            fixes.push(LocationFix {
                id: None,
                user_id: *user_id,
                latitude: f.latitude,
                longitude: f.longitude,
                altitude: f.altitude,
                accuracy: f.accuracy,
                speed: f.speed,
                heading: f.heading,
                battery_level: f.battery_level,
                is_mock_location: f.is_mock_location.unwrap_or(false),
                recorded_at: f.recorded_at.unwrap_or(now),
                received_at: now,
            });
        }

        // Sort by recorded_at ascending so movement is processed in chronological order
        fixes.sort_by(|a, b| a.recorded_at.cmp(&b.recorded_at));

        // 1. Insert batch to database
        let count = LocationRepo::insert_batch(&state.db, &fixes).await?;

        // 2. Broadcast latest point to live map stream
        if let Some(latest_fix) = fixes.last() {
            let user_channel = state.get_or_create_user_channel(user_id);
            let _ = user_channel.send(latest_fix.clone());

            // 3. Check movement detection using latest fix
            Self::detect_and_notify_movement(state, user_id, &user.full_name, latest_fix).await?;
        }

        Ok(count)
    }

    pub async fn get_latest(state: &AppState, user_id: &Uuid) -> AppResult<Option<LocationFix>> {
        LocationRepo::get_latest(&state.db, user_id).await
    }

    pub async fn query_timeline(
        state: &AppState,
        user_id: &Uuid,
        query: &TimelineQueryDto,
    ) -> AppResult<TimelineResponseDto> {
        LocationRepo::query_timeline(&state.db, user_id, query).await
    }

    /// Evaluates movement displacement and dispatches real-time alerts ONLY to admins watching this user
    async fn detect_and_notify_movement(
        state: &AppState,
        user_id: &Uuid,
        user_name: &str,
        current_fix: &LocationFix,
    ) -> AppResult<()> {
        let prev_pos = state.last_known_positions.get(user_id).map(|r| *r);

        let distance_moved = match prev_pos {
            Some((prev_lat, prev_lon, _)) => {
                haversine_distance_meters(prev_lat, prev_lon, current_fix.latitude, current_fix.longitude)
            }
            None => {
                // First recorded fix in memory: store and exit without movement trigger
                state.last_known_positions.insert(*user_id, (current_fix.latitude, current_fix.longitude, current_fix.recorded_at));
                return Ok(());
            }
        };

        // Query admins actively watching this user
        let watching_admins = WatchlistRepo::get_admins_watching_user(&state.db, user_id).await?;
        if watching_admins.is_empty() {
            // No admins watching this user: simply update position and exit
            state.last_known_positions.insert(*user_id, (current_fix.latitude, current_fix.longitude, current_fix.recorded_at));
            return Ok(());
        }

        for (admin_id, threshold) in watching_admins {
            if distance_moved >= threshold {
                let alert = MovementAlertDto {
                    user_id: *user_id,
                    user_name: user_name.to_string(),
                    latitude: current_fix.latitude,
                    longitude: current_fix.longitude,
                    speed: current_fix.speed,
                    distance_moved_meters: (distance_moved * 10.0).round() / 10.0,
                    timestamp: current_fix.recorded_at,
                };

                // Dispatch to admin real-time notification WebSocket channel
                let _ = state.admin_notification_tx.send(alert.clone());

                // Persist notification to database for the admin
                let title = format!("Movement detected: {}", user_name);
                let body = format!(
                    "{} moved {:.1}m to ({:.5}, {:.5}) at speed {:.1} km/h",
                    user_name,
                    distance_moved,
                    current_fix.latitude,
                    current_fix.longitude,
                    current_fix.speed.unwrap_or(0.0) * 3.6
                );

                let meta_val = serde_json::to_value(&alert).ok();
                let _ = WatchlistRepo::save_notification(
                    &state.db,
                    &admin_id,
                    Some(user_id),
                    "MOVEMENT_DETECTED",
                    &title,
                    &body,
                    meta_val.as_ref(),
                )
                .await;
            }
        }

        // Update last known position in fast memory cache
        state.last_known_positions.insert(*user_id, (current_fix.latitude, current_fix.longitude, current_fix.recorded_at));
        Ok(())
    }
}

