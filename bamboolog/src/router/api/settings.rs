use axum::{Extension, Json, Router, response::Response, routing::get};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::{
    config::{SiteSettings, config_entries},
    service::{
        jwt::JwtClaims,
        reloader::ServiceReloader,
        theme::{ThemeService, ThemeServiceSettings},
    },
    utils::{ApiResponse, HttpFailibleOperationExts},
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(get_settings).post(update_settings))
        .route("/themes", get(list_themes))
}

async fn get_settings(
    Extension(db): Extension<DatabaseConnection>,
    _claims: JwtClaims,
) -> Result<ApiResponse<JsonValue>, Response> {
    let site_settings = config_entries::SITE_SETTINGS
        .get::<SiteSettings>(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
        .unwrap_or_default();

    let theme_settings = config_entries::THEME_SERVICE_SETTINGS
        .get::<ThemeServiceSettings>(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
        .unwrap_or_default();

    Ok(ApiResponse::ok(serde_json::json!({
        "site": site_settings,
        "theme": theme_settings,
    })))
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsPayload {
    site: Option<SiteSettings>,
    theme: Option<ThemeServiceSettings>,
}

async fn update_settings(
    Extension(db): Extension<DatabaseConnection>,
    Extension(reloader): Extension<ServiceReloader>,
    _claims: JwtClaims,
    Json(payload): Json<UpdateSettingsPayload>,
) -> Result<ApiResponse, Response> {
    if let Some(site) = payload.site {
        config_entries::SITE_SETTINGS
            .set(&db, Some(site))
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?;
    }

    if let Some(theme) = payload.theme {
        config_entries::THEME_SERVICE_SETTINGS
            .set(&db, Some(theme))
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?;
    }

    reloader
        .reload()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(()))
}

async fn list_themes(
    Extension(theme_service): Extension<ThemeService>,
    _claims: JwtClaims,
) -> Result<ApiResponse<Vec<String>>, Response> {
    let themes = theme_service
        .list_themes()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(themes))
}
