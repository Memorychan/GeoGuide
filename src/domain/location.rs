use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocationFix {
    pub id: Option<i64>,
    pub user_id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy: Option<f32>,
    pub speed: Option<f32>,
    pub heading: Option<f32>,
    pub battery_level: Option<i16>,
    pub is_mock_location: bool,
    pub recorded_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SingleLocationInputDto {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy: Option<f32>,
    pub speed: Option<f32>,
    pub heading: Option<f32>,
    pub battery_level: Option<i16>,
    pub is_mock_location: Option<bool>,
    pub recorded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchLocationInputDto {
    pub fixes: Vec<SingleLocationInputDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TimelineQueryDto {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TimelineResponseDto {
    pub user_id: Uuid,
    pub total_points: usize,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub total_distance_meters: f64,
    pub fixes: Vec<LocationFix>,
}

/// Computes the great-circle distance between two GPS coordinates in meters using the Haversine formula
pub fn haversine_distance_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_METERS * c
}

