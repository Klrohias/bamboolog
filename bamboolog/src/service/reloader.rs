use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tracing::instrument;

use crate::{
    config::{ApplicationConfiguration, SiteSettings, config_entries},
    service::{
        jwt::{JwtService, JwtServiceSettings, JwtServiceState},
        site_settings::SiteSettingsService,
        theme::{ThemeService, ThemeServiceSettings},
    },
    utils::FailibleOperationExts,
};

#[derive(Clone)]
pub struct ServiceReloader {
    pub database: DatabaseConnection,
    pub application_configuration: Arc<ApplicationConfiguration>,
    pub jwt_service: JwtService,
    pub theme_service: ThemeService,
    pub site_settings_service: SiteSettingsService,
}

impl ServiceReloader {
    pub fn new(
        database: DatabaseConnection,
        application_configuration: Arc<ApplicationConfiguration>,
        jwt_service: JwtService,
        theme_service: ThemeService,
        site_settings_service: SiteSettingsService,
    ) -> Self {
        Self {
            database,
            application_configuration,
            jwt_service,
            theme_service,
            site_settings_service,
        }
    }

    pub async fn reload(&self) -> Result<(), anyhow::Error> {
        tokio::try_join!(
            self.reload_jwt_service(),
            self.reload_theme_service(),
            self.reload_site_settings_service()
        )?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_jwt_service(&self) -> Result<(), anyhow::Error> {
        self.jwt_service
            .set_state(JwtServiceState::from(
                config_entries::JWT_SETTINGS
                    .get::<JwtServiceSettings>(&self.database)
                    .await
                    .traced(|e| tracing::error!("{}", e))?
                    .unwrap_or_default(),
            ))
            .await;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_site_settings_service(&self) -> Result<(), anyhow::Error> {
        let settings = config_entries::SITE_SETTINGS
            .get::<SiteSettings>(&self.database)
            .await?
            .ok_or(anyhow::anyhow!("No settings for site"))?;

        let mut state = self.site_settings_service.write().await;
        *state = settings;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_theme_service(&self) -> Result<(), anyhow::Error> {
        let settings = config_entries::THEME_SERVICE_SETTINGS
            .get::<ThemeServiceSettings>(&self.database)
            .await?
            .ok_or(anyhow::anyhow!("No settings for theme service"))?;

        let state = self.theme_service.get_state();
        let mut state = state.write().await;
        state.load_settings(&settings)?;

        Ok(())
    }
}
