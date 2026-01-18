use sea_orm::DatabaseConnection;
use tracing::{error, instrument};

use crate::{
    config::{SiteSettings, config_entries},
    service::{
        jwt::{JwtService, JwtServiceSettings, JwtServiceState},
        site_settings::SiteSettingsService,
        theme::{ThemeService, ThemeServiceSettings},
    },
    utils::FailibleOperationExts,
};

/// ServiceReloader is responsible for reloading all dynamic services from the database configuration.
#[derive(Clone)]
pub struct ServiceReloader {
    database: DatabaseConnection,
    jwt_service: JwtService,
    theme_service: ThemeService,
    site_settings_service: SiteSettingsService,
}

impl ServiceReloader {
    pub fn new(
        database: DatabaseConnection,
        jwt_service: JwtService,
        theme_service: ThemeService,
        site_settings_service: SiteSettingsService,
    ) -> Self {
        Self {
            database,
            jwt_service,
            theme_service,
            site_settings_service,
        }
    }

    /// Reloads all services. If some reloads fail, others will still be attempted.
    /// Returns an error if any of the reloads failed.
    pub async fn reload(&self) -> Result<(), anyhow::Error> {
        let (jwt_res, theme_res, site_res) = tokio::join!(
            self.reload_jwt(),
            self.reload_theme(),
            self.reload_site_settings()
        );

        let mut errors = Vec::new();
        if let Err(e) = jwt_res {
            errors.push(format!("JWT: {e}"));
        }
        if let Err(e) = theme_res {
            errors.push(format!("Theme: {e}"));
        }
        if let Err(e) = site_res {
            errors.push(format!("Site Settings: {e}"));
        }

        if !errors.is_empty() {
            let error_msg = errors.join("; ");
            error!("Failed to reload some services: {}", error_msg);
            anyhow::bail!("Reload failed: {}", error_msg);
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_jwt(&self) -> Result<(), anyhow::Error> {
        let settings = config_entries::JWT_SETTINGS
            .get::<JwtServiceSettings>(&self.database)
            .await
            .traced(|e| error!("Failed to load JWT settings: {e}"))?
            .unwrap_or_default();

        self.jwt_service
            .set_state(JwtServiceState::from(settings))
            .await;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_theme(&self) -> Result<(), anyhow::Error> {
        let settings = config_entries::THEME_SERVICE_SETTINGS
            .get::<ThemeServiceSettings>(&self.database)
            .await
            .traced(|e| error!("Failed to load theme settings: {e}"))?
            .unwrap_or_default();

        let state = self.theme_service.get_state();
        let mut state = state.write().await;
        state
            .load_settings(&settings)
            .traced(|e| error!("Failed to reload theme: {e}"))?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn reload_site_settings(&self) -> Result<(), anyhow::Error> {
        let settings = config_entries::SITE_SETTINGS
            .get::<SiteSettings>(&self.database)
            .await
            .traced(|e| error!("Failed to load site settings: {e}"))?
            .unwrap_or_default();

        self.site_settings_service.update(settings).await;
        Ok(())
    }
}
