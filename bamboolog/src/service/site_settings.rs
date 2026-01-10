use std::{ops::Deref, sync::Arc};

use tokio::sync::RwLock;

use crate::config::SiteSettings;

#[derive(Debug, Clone)]
pub struct SiteSettingsService(Arc<RwLock<SiteSettings>>);

impl SiteSettingsService {
    pub fn new(initial: SiteSettings) -> Self {
        Self(Arc::new(RwLock::new(initial)))
    }

    pub async fn update(&self, new: SiteSettings) {
        let mut state = self.0.write().await;
        *state = new;
    }
}

impl Deref for SiteSettingsService {
    type Target = RwLock<SiteSettings>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
