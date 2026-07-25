use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path},
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
        theme::{ThemeError, ThemeService, ThemeServiceSettings, install_theme_archive},
    },
    utils::{ApiResponse, HttpFailibleOperationExts},
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(get_settings).post(update_settings))
        .route(
            "/themes",
            get(list_themes)
                .post(upload_theme)
                .layer(DefaultBodyLimit::max(MAX_THEME_UPLOAD_SIZE + 1024 * 1024)),
        )
        .route("/themes/{theme}/activate", post(activate_theme))
        .route(
            "/themes/active/config",
            get(get_active_theme_config).post(update_active_theme_config),
        )
        .route("/reload", post(reload))
}

const MAX_THEME_UPLOAD_SIZE: usize = 15 * 1024 * 1024;

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

async fn upload_theme(
    Extension(config): Extension<std::sync::Arc<crate::config::ApplicationConfiguration>>,
    _claims: JwtClaims,
    mut multipart: Multipart,
) -> Result<Response, Response> {
    let mut archive = None;

    while let Some(mut field) = multipart.next_field().await.map_err(|error| {
        ApiResponse::code_and_message(StatusCode::BAD_REQUEST, error.to_string()).into_response()
    })? {
        if field.name() != Some("file") {
            continue;
        }
        if archive.is_some() {
            return Err(ApiResponse::code_and_message(
                StatusCode::BAD_REQUEST,
                "Only one theme archive may be uploaded",
            )
            .into_response());
        }
        let filename = field.file_name().unwrap_or_default();
        if !filename.to_ascii_lowercase().ends_with(".zip") {
            return Err(ApiResponse::code_and_message(
                StatusCode::BAD_REQUEST,
                "Theme package must be a ZIP archive",
            )
            .into_response());
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = field.chunk().await.map_err(|error| {
            ApiResponse::code_and_message(StatusCode::BAD_REQUEST, error.to_string())
                .into_response()
        })? {
            if bytes.len() + chunk.len() > MAX_THEME_UPLOAD_SIZE {
                return Err(ApiResponse::code_and_message(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Theme archive must not exceed 15 MB",
                )
                .into_response());
            }
            bytes.extend_from_slice(&chunk);
        }
        archive = Some(bytes);
    }

    let archive = archive.ok_or_else(|| {
        ApiResponse::code_and_message(StatusCode::BAD_REQUEST, "No file field 'file' found")
            .into_response()
    })?;
    let theme =
        install_theme_archive(&config.asset_dir, &archive).map_err(|error| match error {
            crate::service::theme::ThemeInstallError::InvalidArchive(message) => {
                ApiResponse::code_and_message(StatusCode::BAD_REQUEST, message).into_response()
            }
            crate::service::theme::ThemeInstallError::AlreadyInstalled(message) => {
                ApiResponse::code_and_message(
                    StatusCode::CONFLICT,
                    format!("Theme `{message}` is already installed"),
                )
                .into_response()
            }
            crate::service::theme::ThemeInstallError::Io(error) => {
                tracing::error!("Failed to install uploaded theme: {error}");
                ApiResponse::internal_server_error().into_response()
            }
        })?;

    Ok(ApiResponse::ok(theme).into_response())
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
