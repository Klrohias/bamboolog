use bamboolog::{
    self,
    config::{ApplicationConfiguration, SiteSettings, config_entries},
    router::get_routes,
    service::{
        jwt::{JwtService, JwtServiceSettings, JwtServiceState},
        reloader::ServiceReloader,
        site_settings::SiteSettingsService,
        theme::{ThemeService, ThemeServiceSettings, ThemeServiceState},
    },
};

use axum::{Extension, Router};
use dotenvy::dotenv;
use sea_orm::{Database, DatabaseConnection};
use std::{
    env::{Args, args},
    net::SocketAddr,
    sync::Arc,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::instrument;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn configure_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn configure_database(config: &ApplicationConfiguration) -> DatabaseConnection {
    Database::connect(&config.database)
        .await
        .expect("Failed to connect to database")
}

#[instrument(skip_all)]
async fn configure_jwt_service(database: &DatabaseConnection) -> JwtService {
    let settings = match config_entries::JWT_SETTINGS
        .get::<JwtServiceSettings>(database)
        .await
    {
        Err(e) => {
            tracing::warn!(
                "Failed to load settings for jwt service. For security, we will generate a random temporary settings, you should shutdown the application and check it: {}",
                e
            );

            Some(JwtServiceSettings::default())
        }
        Ok(v) => v,
    };

    let settings = match settings {
        None => {
            tracing::warn!(
                "No settings present for jwt service. For security, we will generate a new settings."
            );

            let new_settings = JwtServiceSettings::default();

            if let Err(e) = config_entries::JWT_SETTINGS
                .set(database, Some(&new_settings))
                .await
            {
                tracing::warn!("Failed to save a new jwt settings: {}", e);
            }

            new_settings
        }
        Some(v) => v,
    };

    JwtService::new(JwtServiceState::from(settings))
}

#[instrument(skip_all)]
async fn configure_theme_service(
    database: &DatabaseConnection,
    application_configuration: &Arc<ApplicationConfiguration>,
    site_settings_service: &SiteSettingsService,
) -> ThemeService {
    let settings = match config_entries::THEME_SERVICE_SETTINGS
        .get::<ThemeServiceSettings>(&database)
        .await
    {
        Err(e) => {
            tracing::warn!(
                "Failed to load settings for theme service, and it will use a default settings. Error: {}",
                e
            );

            ThemeServiceSettings::default()
        }
        Ok(None) => {
            tracing::warn!(
                "No settings present for theme service, and it wil use a default settings."
            );
            ThemeServiceSettings::default()
        }
        Ok(Some(v)) => v,
    };

    let mut state = ThemeServiceState::new(
        application_configuration.clone(),
        site_settings_service.clone(),
    );
    if let Err(e) = state.load_settings(&settings) {
        tracing::warn!("Failed to load theme: {}", e);
    }

    ThemeService::new(state)
}

async fn configure_site_settings_service(database: &DatabaseConnection) -> SiteSettingsService {
    let settings = match config_entries::SITE_SETTINGS
        .get::<SiteSettings>(&database)
        .await
    {
        Err(e) => {
            tracing::warn!(
                "Failed to load site settings, and it will use a default settings. Error: {}",
                e
            );

            SiteSettings::default()
        }
        Ok(None) => {
            tracing::warn!("No site settings, and it will use a default settings.");
            SiteSettings::default()
        }
        Ok(Some(v)) => v,
    };

    SiteSettingsService::new(settings)
}

async fn build_app(config: Arc<ApplicationConfiguration>) -> Router {
    // Configure services
    let database = configure_database(&config).await;
    let jwt_service = configure_jwt_service(&database).await;
    let site_settings_service = configure_site_settings_service(&database).await;
    let theme_service = configure_theme_service(&database, &config, &site_settings_service).await;
    let service_reloader = ServiceReloader::new(
        database.clone(),
        config.clone(),
        jwt_service.clone(),
        theme_service.clone(),
        site_settings_service.clone(),
    );

    // Create routes
    get_routes(&config).layer(
        ServiceBuilder::new()
            .layer(Extension(config.clone()))
            .layer(Extension(database))
            .layer(Extension(jwt_service))
            .layer(Extension(theme_service))
            .layer(Extension(service_reloader)),
    )
}

async fn action_dispatch(mut args: Args, config: &Arc<ApplicationConfiguration>) -> bool {
    if args.any(|x| x == "sync-entities-ef") {
        action_sync_entities(config).await;
        return true;
    }

    false
}

async fn action_sync_entities(config: &Arc<ApplicationConfiguration>) {
    tracing::info!("Sync entities (Entity first)");
    let db = configure_database(&config).await;

    db.get_schema_registry("bamboolog::entity::*")
        .sync(&db)
        .await
        .expect("Failed to sync schemas");
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    configure_tracing();

    let config = Arc::new(ApplicationConfiguration::load().expect("Failed to load configuration"));

    if action_dispatch(args(), &config).await {
        return;
    }

    let app = build_app(config.clone()).await;

    let addr: SocketAddr = config
        .listen_addr
        .as_str()
        .parse()
        .expect("Invalid listen_addr");

    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
