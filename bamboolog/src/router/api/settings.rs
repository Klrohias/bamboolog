use axum::{
    Extension, Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::{
    config::{SiteSettings, config_entries},
    service::{
        jwt::JwtClaims,
        reloadable::ServiceReloader,
        theme::{ThemeError, ThemeService, ThemeServiceSettings},
    },
    utils::{ApiResponse, HttpFailibleOperationExts},
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(get_settings).post(update_settings))
        .route("/themes", get(list_themes))
        .route("/themes/{theme}/activate", post(activate_theme))
        .route(
            "/themes/active/config",
            get(get_active_theme_config).post(update_active_theme_config),
        )
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

async fn list_themes(
    Extension(theme_service): Extension<ThemeService>,
    _claims: JwtClaims,
) -> Result<ApiResponse<Vec<crate::service::theme::ThemeDetails>>, Response> {
    let themes = theme_service
        .list_theme_details()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(themes))
}

async fn activate_theme(
    Path(theme): Path<String>,
    Extension(db): Extension<DatabaseConnection>,
    Extension(reloader): Extension<ServiceReloader>,
    Extension(theme_service): Extension<ThemeService>,
    _claims: JwtClaims,
) -> Result<ApiResponse<ThemeServiceSettings>, Response> {
    let available_themes = theme_service
        .list_themes()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    if !available_themes.iter().any(|name| name == &theme) {
        return Err(ApiResponse::code_and_message(
            axum::http::StatusCode::BAD_REQUEST,
            "Theme not found",
        )
        .into_response());
    }

    let settings = ThemeServiceSettings { current: theme };
    config_entries::THEME_SERVICE_SETTINGS
        .set(&db, Some(settings.clone()))
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    reloader.reload().await;

    Ok(ApiResponse::ok(settings))
}

async fn get_active_theme_config(
    Extension(theme_service): Extension<ThemeService>,
    _claims: JwtClaims,
) -> Result<impl IntoResponse, Response> {
    let configuration = theme_service
        .active_theme_configuration()
        .await
        .traced_and_response(|error| tracing::error!("{error}"))?;
    Ok(ApiResponse::ok(configuration))
}

#[derive(Debug, Deserialize)]
struct UpdateThemeConfigPayload {
    values: JsonMap<String, JsonValue>,
}

async fn update_active_theme_config(
    Extension(reloader): Extension<ServiceReloader>,
    Extension(theme_service): Extension<ThemeService>,
    _claims: JwtClaims,
    Json(payload): Json<UpdateThemeConfigPayload>,
) -> Result<impl IntoResponse, Response> {
    let configuration = match theme_service
        .update_active_theme_config(payload.values)
        .await
    {
        Ok(configuration) => configuration,
        Err(ThemeError::ThemeConfigError(error)) => {
            return Err(
                ApiResponse::code_and_message(StatusCode::BAD_REQUEST, error.to_string())
                    .into_response(),
            );
        }
        Err(error) => {
            tracing::error!("Failed to update theme configuration: {error}");
            return Err(ApiResponse::internal_server_error().into_response());
        }
    };
    reloader.reload().await;

    Ok(ApiResponse::ok(configuration))
}

async fn reload(
    Extension(reloader): Extension<ServiceReloader>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    reloader.reload().await;
    Ok(ApiResponse::ok(()).into_response())
}
