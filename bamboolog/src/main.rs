use bamboolog::{
    self,
    config::ApplicationConfiguration,
    router::get_routes,
    service::{
        jwt::JwtService,
        reloadable::{ReloadableService, ServiceReloader},
        site_settings::SiteSettingsService,
        theme::ThemeService,
    },
};

use axum::{Extension, Router};
use dotenvy::dotenv;
use sea_orm::{Database, DatabaseConnection};
use std::{env::args, net::SocketAddr, sync::Arc};
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
    let result = JwtService::new(database.to_owned());
    result.reload().await;
    result
}

async fn configure_theme_service(
    database: &DatabaseConnection,
    application_configuration: &Arc<ApplicationConfiguration>,
    site_settings_service: &SiteSettingsService,
) -> ThemeService {
    let result = ThemeService::new(
        database.to_owned(),
        application_configuration.to_owned(),
        site_settings_service.to_owned(),
    );
    result.reload().await;
    result
}

async fn configure_site_settings_service(database: &DatabaseConnection) -> SiteSettingsService {
    let result = SiteSettingsService::new(database.to_owned());
    result.reload().await;
    result
}

async fn build_app(config: Arc<ApplicationConfiguration>) -> Router {
    // Configure services
    let database = configure_database(&config).await;
    let jwt_service = configure_jwt_service(&database).await;
    let site_settings_service = configure_site_settings_service(&database).await;
    let theme_service = configure_theme_service(&database, &config, &site_settings_service).await;
    let service_reloader = ServiceReloader::new(vec![
        Box::new(jwt_service.clone()),
        Box::new(site_settings_service.clone()),
        Box::new(theme_service.clone()),
    ]);

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

#[tokio::main]
async fn main() {
    dotenv().ok();
    configure_tracing();

    let config = Arc::new(ApplicationConfiguration::load().expect("Failed to load configuration"));

    if bamboolog::maintenance::action_dispatch(args(), &config).await {
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
