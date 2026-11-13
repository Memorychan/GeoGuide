use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    domain::location::{haversine_distance_meters, LocationFix, TimelineQueryDto, TimelineResponseDto},
    error::{AppError, AppResult},
};

pub struct LocationRepo;

impl LocationRepo {
    pub async fn insert_location(pool: &SqlitePool, fix: &LocationFix) -> AppResult<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO locations (
                user_id, latitude, longitude, altitude, accuracy, speed, heading, battery_level, is_mock_location, recorded_at, received_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(fix.user_id.to_string())
        .bind(fix.latitude)
        .bind(fix.longitude)
        .bind(fix.altitude)
        .bind(fix.accuracy)
        .bind(fix.speed)
        .bind(fix.heading)
        .bind(fix.battery_level)
        .bind(if fix.is_mock_location { 1 } else { 0 })
        .bind(fix.recorded_at.to_rfc3339())
        .bind(fix.received_at.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn insert_batch(pool: &SqlitePool, fixes: &[LocationFix]) -> AppResult<usize> {
        if fixes.is_empty() {
            return Ok(0);
        }

        let mut tx = pool.begin().await?;

        for fix in fixes {
            sqlx::query(
                r#"
                INSERT INTO locations (
                    user_id, latitude, longitude, altitude, accuracy, speed, heading, battery_level, is_mock_location, recorded_at, received_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(fix.user_id.to_string())
            .bind(fix.latitude)
            .bind(fix.longitude)
            .bind(fix.altitude)
            .bind(fix.accuracy)
            .bind(fix.speed)
            .bind(fix.heading)
            .bind(fix.battery_level)
            .bind(if fix.is_mock_location { 1 } else { 0 })
            .bind(fix.recorded_at.to_rfc3339())
            .bind(fix.received_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(fixes.len())
    }

    pub async fn get_latest(pool: &SqlitePool, user_id: &Uuid) -> AppResult<Option<LocationFix>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, latitude, longitude, altitude, accuracy, speed, heading, battery_level, is_mock_location, recorded_at, received_at
            FROM locations
            WHERE user_id = ?
            ORDER BY recorded_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(pool)
        .await?;

        row.map(Self::row_to_fix).transpose()
    }

    pub async fn query_timeline(
        pool: &SqlitePool,
        user_id: &Uuid,
        query: &TimelineQueryDto,
    ) -> AppResult<TimelineResponseDto> {
        let limit = query.limit.unwrap_or(5000).clamp(1, 10000);
        let offset = query.offset.unwrap_or(0).max(0);

        let (sql, rows) = match (&query.from, &query.to) {
            (Some(from), Some(to)) => {
                let s = r#"
                    SELECT id, user_id, latitude, longitude, altitude, accuracy, speed, heading, battery_level, is_mock_location, recorded_at, received_at
                    FROM locations
                    WHERE user_id = ? AND recorded_at >= ? AND recorded_at <= ?
                    ORDER BY recorded_at ASC
                    LIMIT ? OFFSET ?
                "#;
                let r = sqlx::query(s)
                    .bind(user_id.to_string())
                    .bind(from.to_rfc3339())
                    .bind(to.to_rfc3339())
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await?;
                (s, r)
            }
            (Some(from), None) => {
                let s = r#"
                    SELECT id, user_id, latitude, longitude, altitude, accuracy, speed, heading, battery_level, is_mock_location, recorded_at, received_at
                    FROM locations
                    WHERE user_id = ? AND recorded_at >= ?
                    ORDER BY recorded_at ASC
                    LIMIT ? OFFSET ?
                "#;
                let r = sqlx::query(s)
                    .bind(user_id.to_string())
                    .bind(from.to_rfc3339())
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await?;
                (s, r)
            }
            (None, Some(to)) => {
                let s = r#"
                    SELECT id, user_id, latitude, longitude, altitude, accuracy, speed, heading, battery_level, is_mock_location, recorded_at, received_at
                    FROM locations
                    WHERE user_id = ? AND recorded_at <= ?
                    ORDER BY recorded_at ASC
                    LIMIT ? OFFSET ?
                "#;
                let r = sqlx::query(s)
                    .bind(user_id.to_string())
                    .bind(to.to_rfc3339())
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await?;
                (s, r)
            }
            (None, None) => {
                let s = r#"
                    SELECT id, user_id, latitude, longitude, altitude, accuracy, speed, heading, battery_level, is_mock_location, recorded_at, received_at
                    FROM locations
                    WHERE user_id = ?
                    ORDER BY recorded_at ASC
                    LIMIT ? OFFSET ?
                "#;
                let r = sqlx::query(s)
                    .bind(user_id.to_string())
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await?;
                (s, r)
            }
        };

        let _ = sql;
        let mut fixes = Vec::with_capacity(rows.len());
        let mut total_distance_meters = 0.0;
        let mut prev_point: Option<(f64, f64)> = None;

        for row in rows {
            let fix = Self::row_to_fix(row)?;
            if let Some((prev_lat, prev_lon)) = prev_point {
                total_distance_meters += haversine_distance_meters(prev_lat, prev_lon, fix.latitude, fix.longitude);
            }
            prev_point = Some((fix.latitude, fix.longitude));
            fixes.push(fix);
        }

        let total_points = fixes.len();
        Ok(TimelineResponseDto {
            user_id: *user_id,
            total_points,
            from: query.from,
            to: query.to,
            total_distance_meters,
            fixes,
        })
    }

    fn row_to_fix(row: sqlx::sqlite::SqliteRow) -> AppResult<LocationFix> {
        let id: i64 = row.get("id");
        let user_id_str: &str = row.get("user_id");
        let user_id = Uuid::parse_str(user_id_str).map_err(|e| AppError::Internal(e.to_string()))?;

        let recorded_str: &str = row.get("recorded_at");
        let received_str: &str = row.get("received_at");

        let recorded_at = DateTime::parse_from_rfc3339(recorded_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);
        let received_at = DateTime::parse_from_rfc3339(received_str)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .with_timezone(&Utc);

        let is_mock_val: i64 = row.get("is_mock_location");

        Ok(LocationFix {
            id: Some(id),
            user_id,
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
            altitude: row.get("altitude"),
            accuracy: row.get("accuracy"),
            speed: row.get("speed"),
            heading: row.get("heading"),
            battery_level: row.get("battery_level"),
            is_mock_location: is_mock_val != 0,
            recorded_at,
            received_at,
        })
    }
}

