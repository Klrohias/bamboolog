use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;

use crate::{
    config::{SiteSettings, config_entries},
    service::reloadable::ReloadableService,
};

#[derive(Debug, Clone)]
pub struct SiteSettingsService {
    state: Arc<RwLock<SiteSettings>>,
    dep_db: DatabaseConnection,
}

impl SiteSettingsService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            state: Arc::new(RwLock::new(SiteSettings::default())),
            dep_db: db,
        }
    }
}

impl Deref for SiteSettingsService {
    type Target = RwLock<SiteSettings>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[async_trait]
impl ReloadableService for SiteSettingsService {
    async fn reload(&self) {
        let settings = match config_entries::SITE_SETTINGS
            .get::<SiteSettings>(&self.dep_db)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => {
                tracing::warn!("There is no site settings, and will use a default one");
                SiteSettings::default()
            }
            Err(e) => {
                tracing::error!("Failed to load site settings: {e}");
                return;
            }
        };

        {
            let mut state = self.state.write().await;
            *state = settings;
        }
    }
}
