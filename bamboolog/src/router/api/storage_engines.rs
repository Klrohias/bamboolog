use crate::{
    entity::storage_engine,
    service::jwt::JwtClaims,
    utils::{ApiResponse, HttpFailibleOperationExts},
};
use axum::{
    Extension, Json, Router,
    extract::Path,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set};
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
    r#type: String,
    config: Option<String>,
}

async fn create_engine(
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
    Json(payload): Json<CreateEngineRequest>,
) -> Result<Response, Response> {
    let engine = storage_engine::ActiveModel {
        name: Set(payload.name),
        comments: Set(payload.comments.unwrap_or_default()),
        r#type: Set(payload.r#type),
        config: Set(payload.config),
        ..Default::default()
    };

    let res = engine
        .insert(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(res).into_response())
}

#[derive(Deserialize)]
struct UpdateEngineRequest {
    name: Option<String>,
    comments: Option<String>,
    r#type: Option<String>,
    config: Option<String>,
}

async fn update_engine(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
    Json(payload): Json<UpdateEngineRequest>,
) -> Result<Response, Response> {
    let mut engine = storage_engine::Entity::find_by_id(id)
        .one(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
        .ok_or_else(|| {
            ApiResponse::code_and_message(axum::http::StatusCode::NOT_FOUND, "Engine not found")
                .into_response()
        })?
        .into_active_model();

    if let Some(name) = payload.name {
        engine.name = Set(name);
    }
    if let Some(comments) = payload.comments {
        engine.comments = Set(comments);
    }
    if let Some(t) = payload.r#type {
        engine.r#type = Set(t);
    }
    if let Some(config) = payload.config {
        engine.config = Set(Some(config));
    }

    let res = engine
        .update(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(res).into_response())
}

async fn delete_engine(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
) -> Result<Response, Response> {
    storage_engine::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(()).into_response())
}
