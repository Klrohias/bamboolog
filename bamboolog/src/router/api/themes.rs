use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::{
    config::{ApplicationConfiguration, config_entries},
    service::{
        jwt::JwtClaims,
        reloadable::ServiceReloader,
        theme::{
            ThemeDeleteError, ThemeError, ThemeInstallError, ThemeService, ThemeServiceSettings,
            install_theme_archive,
        },
    },
    utils::{ApiResponse, HttpFailibleOperationExts},
};

const MAX_THEME_UPLOAD_SIZE: usize = 15 * 1024 * 1024;

pub fn get_routes() -> Router {
    Router::new()
        .route(
            "/",
            get(list_themes)
                .post(upload_theme)
                .layer(DefaultBodyLimit::max(MAX_THEME_UPLOAD_SIZE + 1024 * 1024)),
        )
        .route(
            "/active/config",
            get(get_active_theme_config).post(update_active_theme_config),
        )
        .route("/{theme}/activate", post(activate_theme))
        .route("/{theme}", delete(delete_theme))
}

async fn list_themes(
    Extension(theme_service): Extension<ThemeService>,
    _claims: JwtClaims,
) -> Result<ApiResponse<Vec<crate::service::theme::ThemeDetails>>, Response> {
    let themes = theme_service
        .list_theme_details()
        .await
        .traced_and_response(|error| tracing::error!("{error}"))?;
    Ok(ApiResponse::ok(themes))
}

async fn upload_theme(
    Extension(config): Extension<Arc<ApplicationConfiguration>>,
    _claims: JwtClaims,
    mut multipart: Multipart,
) -> Result<Response, Response> {
    let mut archive = None;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|error| bad_request(error.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        if archive.is_some() {
            return Err(message_response(
                StatusCode::BAD_REQUEST,
                "Only one theme archive may be uploaded",
            ));
        }
        if !field
            .file_name()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .ends_with(".zip")
        {
            return Err(message_response(
                StatusCode::BAD_REQUEST,
                "Theme package must be a ZIP archive",
            ));
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|error| bad_request(error.to_string()))?
        {
            if bytes.len() + chunk.len() > MAX_THEME_UPLOAD_SIZE {
                return Err(message_response(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Theme archive must not exceed 15 MB",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        archive = Some(bytes);
    }

    let archive = archive
        .ok_or_else(|| message_response(StatusCode::BAD_REQUEST, "No file field 'file' found"))?;
    let theme =
        install_theme_archive(&config.asset_dir, &archive).map_err(|error| match error {
            ThemeInstallError::InvalidArchive(message) => {
                message_response(StatusCode::BAD_REQUEST, message)
            }
            ThemeInstallError::AlreadyInstalled(theme) => message_response(
                StatusCode::CONFLICT,
                format!("Theme `{theme}` is already installed"),
            ),
            ThemeInstallError::Io(error) => {
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
        .traced_and_response(|error| tracing::error!("{error}"))?;
    if !available_themes.iter().any(|name| name == &theme) {
        return Err(message_response(StatusCode::BAD_REQUEST, "Theme not found"));
    }

    let settings = ThemeServiceSettings { current: theme };
    config_entries::THEME_SERVICE_SETTINGS
        .set(&db, Some(settings.clone()))
        .await
        .traced_and_response(|error| tracing::error!("{error}"))?;
    reloader.reload().await;
    Ok(ApiResponse::ok(settings))
}

async fn delete_theme(
    Path(theme): Path<String>,
    Extension(theme_service): Extension<ThemeService>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    theme_service
        .delete_theme(&theme)
        .await
        .map_err(|error| match error {
            ThemeDeleteError::InvalidThemeId => {
                message_response(StatusCode::BAD_REQUEST, "Invalid theme identifier")
            }
            ThemeDeleteError::ActiveTheme => {
                message_response(StatusCode::CONFLICT, "The active theme cannot be deleted")
            }
            ThemeDeleteError::NotFound => {
                message_response(StatusCode::NOT_FOUND, "Theme not found")
            }
            ThemeDeleteError::Io(error) => {
                tracing::error!("Failed to delete theme `{theme}`: {error}");
                ApiResponse::internal_server_error().into_response()
            }
        })?;
    Ok(ApiResponse::ok(()).into_response())
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
            return Err(message_response(StatusCode::BAD_REQUEST, error.to_string()));
        }
        Err(error) => {
            tracing::error!("Failed to update theme configuration: {error}");
            return Err(ApiResponse::internal_server_error().into_response());
        }
    };
    reloader.reload().await;
    Ok(ApiResponse::ok(configuration))
}

fn bad_request(message: impl Into<String>) -> Response {
    message_response(StatusCode::BAD_REQUEST, message)
}

fn message_response(status: StatusCode, message: impl Into<String>) -> Response {
    ApiResponse::code_and_message(status, message.into()).into_response()
}
