use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DerivePartialModel,
    EntityTrait, IntoActiveModel, ModelTrait, PaginatorTrait, QueryFilter, QueryOrder,
    prelude::DateTimeUtc,
};
use serde::{Deserialize, Serialize};

use crate::{
    entity,
    service::{jwt::JwtClaims, user::User},
    utils::{ApiResponse, HttpFailibleOperationExts, Pagination, render_markdown},
};

#[derive(Debug, Deserialize)]
pub struct PostCreateRequest {
    pub title: String,
    pub name: String,
    pub content: String,
    pub created_at: Option<i64>,
    pub description: Option<String>,
    pub illustration: Option<String>,
    pub tags: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PostUpdateRequest {
    pub title: Option<String>,
    pub name: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<i64>,
    pub description: Option<String>,
    pub illustration: Option<String>,
    pub tags: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub hidden: Option<bool>,
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
    pub updated_at: Option<DateTimeUtc>,
    pub description: Option<String>,
    pub illustration: Option<String>,
    pub hidden: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PostDetailResponse {
    pub id: i32,
    pub title: String,
    pub name: String,
    pub content: String,
    pub author: i32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub description: Option<String>,
    pub illustration: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub hidden: bool,
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
        Some(post) => ApiResponse::ok(post_detail_response(post)).into_response(),
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
            render_markdown(&post.content).traced_and_response(|e| tracing::error!("{}", e))?,
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
    let created_at = post_payload
        .created_at
        .and_then(DateTimeUtc::from_timestamp_secs)
        .unwrap_or_else(Utc::now);
    let active_model = entity::post::ActiveModel {
        id: ActiveValue::NotSet,
        name: ActiveValue::Set(post_payload.name),
        title: ActiveValue::Set(post_payload.title),
        content: ActiveValue::Set(post_payload.content),
        author: ActiveValue::Set(user.id),
        description: ActiveValue::Set(post_payload.description),
        illustration: ActiveValue::Set(post_payload.illustration),
        tags: ActiveValue::Set(Some(serialize_terms(post_payload.tags.unwrap_or_default()))),
        categories: ActiveValue::Set(Some(serialize_terms(
            post_payload.categories.unwrap_or_default(),
        ))),
        hidden: ActiveValue::Set(Some(post_payload.hidden.unwrap_or(false))),
        created_at: ActiveValue::Set(created_at.clone()),
        updated_at: ActiveValue::Set(Some(created_at)),
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

    if let Some(description) = post_payload.description {
        active_model.description = ActiveValue::Set(Some(description));
    }

    if let Some(illustration) = post_payload.illustration {
        active_model.illustration = ActiveValue::Set(Some(illustration));
    }

    if let Some(tags) = post_payload.tags {
        active_model.tags = ActiveValue::Set(Some(serialize_terms(tags)));
    }

    if let Some(categories) = post_payload.categories {
        active_model.categories = ActiveValue::Set(Some(serialize_terms(categories)));
    }

    if let Some(hidden) = post_payload.hidden {
        active_model.hidden = ActiveValue::Set(Some(hidden));
    }

    active_model.updated_at = ActiveValue::Set(Some(Utc::now()));

    active_model
        .update(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(()))
}

fn post_detail_response(post: entity::post::Model) -> PostDetailResponse {
    PostDetailResponse {
        id: post.id,
        title: post.title,
        name: post.name,
        content: post.content,
        author: post.author,
        created_at: post.created_at,
        updated_at: post
            .updated_at
            .clone()
            .unwrap_or_else(|| post.created_at.clone()),
        description: post.description,
        illustration: post.illustration,
        tags: deserialize_terms(post.tags),
        categories: deserialize_terms(post.categories),
        hidden: post.hidden.unwrap_or(false),
    }
}

fn serialize_terms(terms: Vec<String>) -> String {
    let terms = terms
        .into_iter()
        .map(|term| term.trim().to_string())
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    serde_json::to_string(&terms).expect("serializing strings never fails")
}

pub(crate) fn deserialize_terms(value: Option<String>) -> Vec<String> {
    value
        .and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default()
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

    use super::{
        PostListRequest, create_post, deserialize_terms, get_routes, list_posts, serialize_terms,
    };

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
            "description": "A concise description",
            "illustration": "/attachments/cover",
            "tags": ["Rust", "Web"],
            "categories": ["Engineering"],
            "hidden": true,
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
        assert_eq!(post.description.as_deref(), Some("A concise description"));
        assert_eq!(post.illustration.as_deref(), Some("/attachments/cover"));
        assert_eq!(deserialize_terms(post.tags), ["Rust", "Web"]);
        assert_eq!(deserialize_terms(post.categories), ["Engineering"]);
        assert_eq!(post.hidden, Some(true));
        assert!(post.updated_at.is_some());
    }

    #[test]
    fn serializes_terms_as_a_clean_array() {
        assert_eq!(
            serialize_terms(vec![" Rust ".to_string(), String::new(), "Web".to_string()]),
            r#"["Rust","Web"]"#
        );
        assert_eq!(
            deserialize_terms(Some("not json".to_string())),
            Vec::<String>::new()
        );
    }
}
