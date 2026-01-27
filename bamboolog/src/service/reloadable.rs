use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait ReloadableService {
    async fn reload(&self);
}

/// ServiceReloader is responsible for reloading all dynamic services from the database configuration.
#[derive(Clone)]
pub struct ServiceReloader(Arc<Vec<Box<dyn ReloadableService + Sync + Send>>>);

impl ServiceReloader {
    pub fn new(services: Vec<Box<dyn ReloadableService + Sync + Send>>) -> Self {
        Self(Arc::new(services))
    }

    pub async fn reload(&self) {
        for services in self.0.iter() {
            services.reload().await;
        }
    }
}
