use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tracing::instrument;

use crate::{
    config::{ApplicationConfiguration, config_entries},
    service::{
        jwt::{JwtService, JwtServiceSettings, JwtServiceState},
        theme::{ThemeService, ThemeServiceSettings},
    },
    utils::FailibleOperationExt,
};

#[derive(Clone)]
pub struct ServiceReloader {
    pub database: DatabaseConnection,
    pub application_configuration: Arc<ApplicationConfiguration>,
    pub jwt_service: JwtService,
    pub theme_service: ThemeService,
}

impl ServiceReloader {
    pub fn new(
        database: DatabaseConnection,
        application_configuration: Arc<ApplicationConfiguration>,
        jwt_service: JwtService,
        theme_service: ThemeService,
    ) -> Self {
        Self {
            database,
            application_configuration,
            jwt_service,
            theme_service,
        }
    }

    pub async fn reload(&self) -> Result<(), anyhow::Error> {
        tokio::try_join!(self.reload_jwt_service(), self.reload_theme_service())?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_jwt_service(&self) -> Result<(), anyhow::Error> {
        self.jwt_service
            .set_state(JwtServiceState::from(
                config_entries::JWT_CONFIG
                    .get::<JwtServiceSettings>(&self.database)
                    .await
                    .traced()?
                    .unwrap_or_default(),
            ))
            .await;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_theme_service(&self) -> Result<(), anyhow::Error> {
        let settings = config_entries::THEME_SERVICE_CONFIG
            .get::<ThemeServiceSettings>(&self.database)
            .await?
            .ok_or(anyhow::anyhow!("No config for theme service"))?;

        let state = self.theme_service.get_state();
        let mut state = state.write().await;
        state.load_theme(&settings)?;

        Ok(())
    }
}
