use axum::{
    Extension, Json, Router,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::{
    config::{SiteSettings, config_entries},
    service::{jwt::JwtClaims, reloadable::ServiceReloader},
    utils::{ApiResponse, HttpFailibleOperationExts},
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(get_settings).post(update_settings))
        .route("/reload", post(reload))
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

    Ok(ApiResponse::ok(serde_json::json!({
        "site": site_settings,
    })))
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsPayload {
    site: Option<SiteSettings>,
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

    reloader.reload().await;
    Ok(ApiResponse::ok(()))
}

async fn reload(
    Extension(reloader): Extension<ServiceReloader>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    reloader.reload().await;
    Ok(ApiResponse::ok(()).into_response())
}
