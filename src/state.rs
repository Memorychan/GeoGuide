use std::sync::Arc;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    domain::{location::LocationFix, notification::MovementAlertDto},
};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
    pub live_channel_hub: Arc<DashMap<Uuid, broadcast::Sender<LocationFix>>>,
    pub admin_notification_tx: broadcast::Sender<MovementAlertDto>,
    pub last_known_positions: Arc<DashMap<Uuid, (f64, f64, DateTime<Utc>)>>,
}

impl AppState {
    pub fn new(db: SqlitePool, config: AppConfig) -> Self {
        let (admin_notification_tx, _) = broadcast::channel(1000);

        Self {
            db,
            config,
            live_channel_hub: Arc::new(DashMap::new()),
            admin_notification_tx,
            last_known_positions: Arc::new(DashMap::new()),
        }
    }

    pub fn get_or_create_user_channel(&self, user_id: &Uuid) -> broadcast::Sender<LocationFix> {
        self.live_channel_hub
            .entry(*user_id)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(500);
                tx
            })
            .clone()
    }
}

