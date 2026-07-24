use crate::{
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
use sea_orm::DatabaseConnection;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::instrument;

#[instrument(skip_all)]
async fn configure_jwt_service(database: &DatabaseConnection) -> JwtService {
    let service = JwtService::new(database.to_owned());
    service.reload().await;
    service
}

async fn configure_site_settings_service(database: &DatabaseConnection) -> SiteSettingsService {
    let service = SiteSettingsService::new(database.to_owned());
    service.reload().await;
    service
}

async fn configure_theme_service(
    database: &DatabaseConnection,
    config: &Arc<ApplicationConfiguration>,
    site_settings_service: &SiteSettingsService,
) -> ThemeService {
    let service = ThemeService::new(
        database.to_owned(),
        config.to_owned(),
        site_settings_service.to_owned(),
    );
    service.reload().await;
    service
}

async fn build_app(config: Arc<ApplicationConfiguration>) -> Router {
    let database = config
        .connect_database()
        .await
        .expect("Failed to connect to database");
    let jwt_service = configure_jwt_service(&database).await;
    let site_settings_service = configure_site_settings_service(&database).await;
    let theme_service = configure_theme_service(&database, &config, &site_settings_service).await;
    let service_reloader = ServiceReloader::new(vec![
        Box::new(jwt_service.clone()),
        Box::new(site_settings_service.clone()),
        Box::new(theme_service.clone()),
    ]);

    get_routes(&config).layer(
        ServiceBuilder::new()
            .layer(Extension(config))
            .layer(Extension(database))
            .layer(Extension(jwt_service))
            .layer(Extension(site_settings_service))
            .layer(Extension(theme_service))
            .layer(Extension(service_reloader)),
    )
}

pub async fn run(config: Arc<ApplicationConfiguration>) {
    let addr: SocketAddr = config.listen_addr.parse().expect("Invalid listen_addr");
    let app = build_app(config).await;

    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
