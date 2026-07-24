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
    utils::{ApiResponse, HttpFailibleOperationExts, Pagination},
};

#[derive(Debug, Deserialize)]
pub struct PostCreateRequest {
    pub title: String,
    pub name: String,
    pub content: String,
    pub created_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PostUpdateRequest {
    pub title: Option<String>,
    pub name: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<i64>,
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
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
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

    let pagination = Pagination::new(query.page, query.page_size, 10);

    let paginator = select
        .into_partial_model()
        .paginate(&database, pagination.size());

    let total = paginator
        .num_items()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    let total_pages = pagination.total_pages(total);

    let posts = paginator
        .fetch_page(pagination.offset())
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(PostListResponse {
        posts,
        total,
        page: pagination.page(),
        page_size: pagination.size(),
        total_pages,
    }))
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
        author: ActiveValue::Set(user.id),
        created_at: post_payload
            .created_at
            .map(|x| {
                DateTimeUtc::from_timestamp_secs(x)
                    .map(ActiveValue::Set)
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
    _claims: JwtClaims,
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

#[cfg(test)]
mod tests {
    use axum::{
        Extension, Json,
        body::Body,
        extract::Query,
        http::{Request, StatusCode},
    };
    use sea_orm::{
        ActiveModelTrait, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection,
        EntityTrait, Schema, Set,
    };
    use tower::ServiceExt;

    use crate::{
        entity::{self, user},
        service::user::User,
    };

    use super::{PostListRequest, create_post, get_routes, list_posts};

    async fn database_with_post_schema() -> DatabaseConnection {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let schema = Schema::new(DatabaseBackend::Sqlite);

        for statement in [
            schema.create_table_from_entity(user::Entity),
            schema.create_table_from_entity(entity::post::Entity),
        ] {
            database.execute(&statement).await.unwrap();
        }

        database
    }

    async fn insert_user(database: &DatabaseConnection, id: i32) -> user::Model {
        user::ActiveModel {
            id: Set(id),
            username: Set(format!("user-{id}")),
            email: Set(format!("user-{id}@example.test")),
            nickname: Set(format!("User {id}")),
            password_hash: Set("password-hash".to_string()),
            ..Default::default()
        }
        .insert(database)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn rejects_anonymous_post_edits_before_database_access() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let app = get_routes().layer(Extension(database));
        let request = Request::builder()
            .method("POST")
            .uri("/1")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"changed"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn list_posts_returns_normalized_pagination_metadata() {
        let database = database_with_post_schema().await;
        let user = insert_user(&database, 1).await;

        for name in ["first", "second"] {
            entity::post::ActiveModel {
                name: Set(name.to_string()),
                title: Set(name.to_string()),
                content: Set(String::new()),
                author: Set(user.id),
                ..Default::default()
            }
            .insert(&database)
            .await
            .unwrap();
        }

        let response = list_posts(
            Extension(database),
            Query(PostListRequest {
                page: Some(0),
                page_size: Some(1),
                title: None,
                name: None,
                sort_by: Some("id".to_string()),
                order: Some("asc".to_string()),
            }),
        )
        .await
        .unwrap();
        let data = response.data.unwrap();

        assert_eq!(data.posts.len(), 1);
        assert_eq!(data.total, 2);
        assert_eq!(data.page, 1);
        assert_eq!(data.page_size, 1);
        assert_eq!(data.total_pages, 2);
    }

    #[tokio::test]
    async fn create_post_uses_the_authenticated_user_as_author() {
        let database = database_with_post_schema().await;
        let user = insert_user(&database, 1).await;
        let request = serde_json::from_value(serde_json::json!({
            "title": "Test post",
            "name": "test-post",
            "content": "Content",
            "user": 999,
        }))
        .unwrap();

        create_post(Extension(database.clone()), User(user), Json(request))
            .await
            .unwrap();

        let post = entity::post::Entity::find()
            .one(&database)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(post.author, 1);
    }
}
