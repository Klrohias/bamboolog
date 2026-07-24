use crate::{
    entity::{attachment, storage_engine},
    service::jwt::JwtClaims,
    storage::validate_storage_engine_config,
    utils::{ApiResponse, HttpFailibleOperationExts},
};
use axum::{
    Extension, Json, Router,
    extract::Path,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(list_engines))
        .route("/", post(create_engine))
        .route("/{id}", put(update_engine))
        .route("/{id}", delete(delete_engine))
}

async fn list_engines(
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
) -> Result<Response, Response> {
    let engines = storage_engine::Entity::find()
        .all(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(engines).into_response())
}

#[derive(Deserialize)]
struct CreateEngineRequest {
    name: String,
    comments: Option<String>,
    #[serde(alias = "type")]
    kind: String,
    #[serde(alias = "config")]
    config_json: Option<String>,
    is_default: Option<bool>,
    enabled: Option<bool>,
}

async fn create_engine(
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
    Json(payload): Json<CreateEngineRequest>,
) -> Result<Response, Response> {
    validate_engine_config(&payload.kind, payload.config_json.as_deref())?;
    let is_default = payload.is_default.unwrap_or(false);
    let enabled = payload.enabled.unwrap_or(true);
    if is_default && !enabled {
        return Err(ApiResponse::code_and_message(
            axum::http::StatusCode::BAD_REQUEST,
            "Default storage engine must be enabled",
        )
        .into_response());
    }
    let engine = storage_engine::ActiveModel {
        name: Set(payload.name),
        comments: Set(payload.comments.unwrap_or_default()),
        kind: Set(normalize_kind(payload.kind)),
        config_json: Set(payload.config_json),
        is_default: Set(is_default),
        enabled: Set(enabled),
        ..Default::default()
    };

    let transaction = db
        .begin()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    if is_default {
        clear_default_engines(&transaction).await?;
    }
    let res = engine
        .insert(&transaction)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    transaction
        .commit()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(res).into_response())
}

#[derive(Deserialize)]
struct UpdateEngineRequest {
    name: Option<String>,
    comments: Option<String>,
    #[serde(alias = "type")]
    kind: Option<String>,
    #[serde(alias = "config")]
    config_json: Option<String>,
    is_default: Option<bool>,
    enabled: Option<bool>,
}

async fn update_engine(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
    Json(payload): Json<UpdateEngineRequest>,
) -> Result<Response, Response> {
    let current_engine = storage_engine::Entity::find_by_id(id)
        .one(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
        .ok_or_else(|| {
            ApiResponse::code_and_message(axum::http::StatusCode::NOT_FOUND, "Engine not found")
                .into_response()
        })?;

    let attachment_count = attachment::Entity::find()
        .filter(attachment::Column::StorageEngineId.eq(id))
        .count(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    if attachment_count > 0
        && payload
            .kind
            .as_deref()
            .is_some_and(|kind| kind != current_engine.kind)
    {
        return Err(ApiResponse::code_and_message(
            axum::http::StatusCode::BAD_REQUEST,
            "Storage engine kind cannot change while attachments exist",
        )
        .into_response());
    }

    let current_enabled = current_engine.enabled;
    let current_is_default = current_engine.is_default;
    let mut engine = current_engine.into_active_model();

    if let Some(name) = payload.name {
        engine.name = Set(name);
    }
    if let Some(comments) = payload.comments {
        engine.comments = Set(comments);
    }
    let next_kind = payload
        .kind
        .clone()
        .unwrap_or_else(|| engine.kind.clone().unwrap());
    let next_config = payload
        .config_json
        .clone()
        .or_else(|| engine.config_json.clone().unwrap());
    validate_engine_config(&next_kind, next_config.as_deref())?;

    if let Some(kind) = payload.kind {
        engine.kind = Set(normalize_kind(kind));
    }
    if let Some(config_json) = payload.config_json {
        engine.config_json = Set(Some(config_json));
    }
    let next_enabled = payload.enabled.unwrap_or(current_enabled);
    let next_is_default = payload.is_default.unwrap_or(current_is_default);
    if next_is_default && !next_enabled {
        return Err(ApiResponse::code_and_message(
            axum::http::StatusCode::BAD_REQUEST,
            "Default storage engine must be enabled",
        )
        .into_response());
    }

    if let Some(is_default) = payload.is_default {
        engine.is_default = Set(is_default);
    }
    if let Some(enabled) = payload.enabled {
        engine.enabled = Set(enabled);
    }

    let transaction = db
        .begin()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    if next_is_default && !current_is_default {
        clear_default_engines(&transaction).await?;
    }
    let res = engine
        .update(&transaction)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    transaction
        .commit()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(res).into_response())
}

async fn delete_engine(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
) -> Result<Response, Response> {
    let attachment_count = attachment::Entity::find()
        .filter(attachment::Column::StorageEngineId.eq(id))
        .count(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    if attachment_count > 0 {
        return Err(ApiResponse::code_and_message(
            axum::http::StatusCode::CONFLICT,
            "Storage engine cannot be deleted while attachments exist",
        )
        .into_response());
    }

    storage_engine::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(()).into_response())
}

#[allow(
    clippy::result_large_err,
    reason = "Axum handlers use Response as their established rejection type."
)]
fn validate_engine_config(kind: &str, config_json: Option<&str>) -> Result<(), Response> {
    validate_storage_engine_config(kind, config_json).map_err(|error| {
        tracing::warn!("Rejected invalid storage engine configuration: {error}");
        ApiResponse::code_and_message(axum::http::StatusCode::BAD_REQUEST, error.to_string())
            .into_response()
    })
}

async fn clear_default_engines<C>(db: &C) -> Result<(), Response>
where
    C: ConnectionTrait,
{
    let engines = storage_engine::Entity::find()
        .filter(storage_engine::Column::IsDefault.eq(true))
        .all(db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    for engine in engines {
        let mut active = engine.into_active_model();
        active.is_default = Set(false);
        active
            .update(db)
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?;
    }

    Ok(())
}

fn normalize_kind(kind: String) -> String {
    if kind == "internal" {
        "local".to_string()
    } else {
        kind
    }
}
