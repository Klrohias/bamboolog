use axum::{
    Extension, Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    prelude::DateTimeUtc,
};
use serde::Deserialize;

use crate::{
    entity,
    service::{jwt::JwtClaims, user::User},
    utils::{ApiResponse, HttpFailibleOperationExt},
};

#[derive(Debug, Deserialize)]
pub struct CreatePost {
    pub title: String,
    pub name: String,
    pub content: String,
    pub created_at: Option<i64>,
    pub user: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct EditPost {
    pub title: Option<String>,
    pub name: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<i64>,
    pub user: Option<i32>,
}

pub fn get_routes() -> Router {
    Router::new()
        .route(
            "/{id}",
            get(get_post_content).delete(delete_post).post(edit_post),
        )
        .route("/{id}/rendered", get(get_rendered_post_content))
        .route("/", put(create_post))
}

pub async fn get_post_content(
    Extension(database): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Response, Response> {
    let post = entity::post::Entity::find_by_id(id)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(match post {
        None => {
            ApiResponse::code_and_message(StatusCode::NOT_FOUND, "No post found").into_response()
        }
        Some(post) => ApiResponse::ok(post).into_response(),
    })
}

pub async fn get_rendered_post_content(
    Extension(database): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Response, Response> {
    let post = entity::post::Entity::find_by_id(id)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(match post {
        None => {
            ApiResponse::code_and_message(StatusCode::NOT_FOUND, "No post found").into_response()
        }
        Some(post) => ApiResponse::ok(
            markdown::to_html_with_options(&post.content, &markdown::Options::gfm())
                .traced_and_response(|e| tracing::error!("{}", e))?,
        )
        .into_response(),
    })
}

pub async fn delete_post(
    Extension(database): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    let post = entity::post::Entity::find_by_id(id)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    match post {
        None => Err(
            ApiResponse::code_and_message(StatusCode::NOT_FOUND, "No post found").into_response(),
        ),
        Some(post) => {
            post.delete(&database)
                .await
                .traced_and_response(|e| tracing::error!("{}", e))?;
            Ok(ApiResponse::ok(()).into_response())
        }
    }
}

pub async fn create_post(
    Extension(database): Extension<DatabaseConnection>,
    User(user): User,
    Json(post_payload): Json<CreatePost>,
) -> Result<ApiResponse, Response> {
    let active_model = entity::post::ActiveModel {
        id: ActiveValue::NotSet,
        name: ActiveValue::Set(post_payload.name),
        title: ActiveValue::Set(post_payload.title),
        content: ActiveValue::Set(post_payload.content),
        author: ActiveValue::Set(post_payload.user.unwrap_or(user.id)),
        created_at: post_payload
            .created_at
            .map(|x| {
                DateTimeUtc::from_timestamp_secs(x)
                    .map(|x| ActiveValue::Set(x))
                    .unwrap_or(ActiveValue::NotSet)
            })
            .unwrap_or(ActiveValue::NotSet),
    };

    active_model
        .insert(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(()))
}

pub async fn edit_post(
    Extension(database): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(post_payload): Json<EditPost>,
) -> Result<ApiResponse, Response> {
    let old_post = entity::post::Entity::find_by_id(id)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
        .ok_or_else(|| ApiResponse::code(StatusCode::NOT_FOUND).into_response())?;

    let mut active_model = old_post.into_active_model();
    if let Some(new_content) = post_payload.content {
        active_model.content = ActiveValue::Set(new_content);
    }

    if let Some(new_user) = post_payload.user {
        active_model.author = ActiveValue::Set(new_user);
    }

    if let Some(new_title) = post_payload.title {
        active_model.title = ActiveValue::Set(new_title);
    }

    if let Some(new_created_at) = post_payload.created_at {
        active_model.created_at = ActiveValue::Set(
            DateTimeUtc::from_timestamp_secs(new_created_at).ok_or_else(|| {
                ApiResponse::code_and_message(StatusCode::BAD_REQUEST, "Failed to parse created_at")
                    .into_response()
            })?,
        );
    }

    if let Some(new_name) = post_payload.name {
        active_model.name = ActiveValue::Set(new_name);
    }

    active_model
        .update(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(()))
}
