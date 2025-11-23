use sea_orm::DatabaseConnection;
use tracing::instrument;

use crate::{
    config::config_entries,
    service::{
        jwt::{JwtService, JwtServiceSettings, JwtServiceState},
        theme::{ThemeService, ThemeServiceSettings, ThemeServiceState},
    },
    utils::FailibleOperationExt,
};

#[derive(Clone)]
pub struct ServiceReloader {
    pub database: DatabaseConnection,
    pub jwt_service: JwtService,
    pub theme_service: ThemeService,
}

impl ServiceReloader {
    pub fn new(
        database: DatabaseConnection,
        jwt_service: JwtService,
        theme_service: ThemeService,
    ) -> Self {
        Self {
            database,
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

        let mut state = ThemeServiceState::default();
        state.load(&settings)?;
        self.theme_service.set_state(state).await;
        Ok(())
    }
}
