use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DerivePartialModel,
    EntityTrait, IntoActiveModel, ModelTrait, PaginatorTrait, QueryFilter, QueryOrder,
    prelude::DateTimeUtc,
};
use serde::{Deserialize, Serialize};

use crate::{
    entity,
    service::{jwt::JwtClaims, user::User},
    utils::{ApiResponse, HttpFailibleOperationExts},
};

#[derive(Debug, Deserialize)]
pub struct PostCreateRequest {
    pub title: String,
    pub name: String,
    pub content: String,
    pub created_at: Option<i64>,
    pub user: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PostUpdateRequest {
    pub title: Option<String>,
    pub name: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<i64>,
    pub user: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PostListRequest {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Serialize, DerivePartialModel)]
#[sea_orm(entity = "entity::post::Entity")]
pub struct PostListItem {
    pub id: i32,
    pub title: String,
    pub name: String,
    pub author: i32,
    pub created_at: DateTimeUtc,
}

#[derive(Debug, Serialize)]
pub struct PostListResponse {
    pub posts: Vec<PostListItem>,
    pub total: u64,
}

pub fn get_routes() -> Router {
    Router::new()
        .route(
            "/{id}",
            get(get_post_content).delete(delete_post).post(edit_post),
        )
        .route("/{id}/rendered", get(get_rendered_post_content))
        .route("/", get(list_posts).put(create_post))
}

pub async fn list_posts(
    Extension(database): Extension<DatabaseConnection>,
    Query(query): Query<PostListRequest>,
) -> Result<ApiResponse<PostListResponse>, Response> {
    let mut select = entity::post::Entity::find();

    if let Some(title) = query.title {
        select = select.filter(entity::post::Column::Title.contains(&title));
    }

    if let Some(name) = query.name {
        select = select.filter(entity::post::Column::Name.contains(&name));
    }

    let sort_by = query.sort_by.unwrap_or_else(|| "id".to_string());
    let order_by = query.order.unwrap_or_else(|| "desc".to_string());

    let column = match sort_by.as_str() {
        "id" => entity::post::Column::Id,
        "title" => entity::post::Column::Title,
        "name" => entity::post::Column::Name,
        "created_at" => entity::post::Column::CreatedAt,
        _ => entity::post::Column::Id,
    };

    select = if order_by.to_lowercase() == "asc" {
        select.order_by_asc(column)
    } else {
        select.order_by_desc(column)
    };

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let paginator = select.into_partial_model().paginate(&database, page_size);

    let total = paginator.num_items().await.map_err(|e| {
        tracing::error!("{}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiResponse::internal_server_error(),
        )
            .into_response()
    })?;

    let posts = paginator.fetch_page(page - 1).await.map_err(|e| {
        tracing::error!("{}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiResponse::internal_server_error(),
        )
            .into_response()
    })?;

    Ok(ApiResponse::ok(PostListResponse { posts, total }))
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
    Json(post_payload): Json<PostCreateRequest>,
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
    Json(post_payload): Json<PostUpdateRequest>,
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
