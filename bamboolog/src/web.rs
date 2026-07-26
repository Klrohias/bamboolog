use crate::{
    config::ApplicationConfiguration,
    router::get_routes,
    service::{
        jwt::JwtService,
        reloadable::{ReloadableService, ServiceReloader},
        site_settings::SiteSettingsService,
        storage::StorageService,
        theme::ThemeService,
    },
};
use axum::{
    Extension, Router,
    http::{HeaderValue, header},
    middleware,
    response::Response,
};
use sea_orm::DatabaseConnection;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::instrument;

const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; base-uri 'self'; form-action 'self'; frame-ancestors 'self'; object-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; media-src 'self'; frame-src 'none'";

async fn set_security_headers(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY),
    );
    response
}

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
    let storage_service = StorageService::new(config.clone());
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
            .layer(Extension(storage_service))
            .layer(Extension(service_reloader))
            .layer(middleware::map_response(set_security_headers)),
    )
}

pub async fn run(config: Arc<ApplicationConfiguration>) {
    let addr: SocketAddr = config.listen_addr.parse().expect("Invalid listen_addr");
    let app = build_app(config).await;

    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use axum::{http::StatusCode, response::IntoResponse};

    use super::{CONTENT_SECURITY_POLICY, set_security_headers};

    #[tokio::test]
    async fn adds_a_content_security_policy_to_responses() {
        let response = set_security_headers(StatusCode::OK.into_response()).await;

        assert_eq!(
            response.headers().get("content-security-policy").unwrap(),
            CONTENT_SECURITY_POLICY
        );
    }
}
