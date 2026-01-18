use axum::{
    Extension, Router,
    extract::multipart::MultipartError,
    extract::{Multipart, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    config::ApplicationConfiguration,
    entity::attachment,
    service::{jwt::JwtClaims, storage::StorageService},
    utils::{ApiResponse, HttpFailibleOperationExts},
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", post(upload_attachment))
        .route("/", get(list_attachments))
        .route("/{id}", delete(delete_attachment))
}

#[derive(Deserialize)]
pub struct ListAttachmentsQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub mime: Option<String>,
    pub storage_engine_id: Option<i32>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Serialize)]
pub struct AttachmentList {
    pub items: Vec<attachment::Model>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
    pub total_pages: u64,
}

async fn upload_attachment(
    Extension(db): Extension<DatabaseConnection>,
    Extension(config): Extension<Arc<ApplicationConfiguration>>,
    _user: JwtClaims,
    mut multipart: Multipart,
) -> Result<Response, Response> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut content_type: String = "application/octet-stream".to_string();
    let mut storage_engine_id: Option<i32> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e: MultipartError| {
        ApiResponse::code_and_message(StatusCode::BAD_REQUEST, e.to_string()).into_response()
    })? {
        let name: String = field.name().unwrap_or("").to_string();
        if name == "file" {
            content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let data = field.bytes().await.map_err(|e: MultipartError| {
                ApiResponse::code_and_message(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    .into_response()
            })?;
            file_data = Some(data.to_vec());
        } else if name == "storage_engine_id" {
            if let Ok(text) = field.text().await {
                storage_engine_id = text.parse::<i32>().ok();
            }
        }
    }

    if let Some(data) = file_data {
        let attachment =
            StorageService::upload(&db, &config, &data, content_type, storage_engine_id)
                .await
                .traced_and_response(|e| tracing::error!("{}", e))?;

        return Ok(ApiResponse::ok(attachment).into_response());
    }

    Err(
        ApiResponse::code_and_message(StatusCode::BAD_REQUEST, "No file field 'file' found")
            .into_response(),
    )
}

async fn list_attachments(
    Extension(db): Extension<DatabaseConnection>,
    _user: JwtClaims,
    Query(query): Query<ListAttachmentsQuery>,
) -> Result<Response, Response> {
    let page = query.page.unwrap_or(1);
    let size = query.size.unwrap_or(20);

    let mut select = attachment::Entity::find();

    if let Some(mime) = query.mime {
        if !mime.is_empty() {
            select = select.filter(attachment::Column::Mime.contains(&mime));
        }
    }

    if let Some(storage_engine_id) = query.storage_engine_id {
        select = select.filter(attachment::Column::StorageEngineId.eq(storage_engine_id));
    }

    match (query.sort.as_deref(), query.order.as_deref()) {
        (Some("created_at"), Some("asc")) => {
            select = select.order_by_asc(attachment::Column::CreatedAt);
        }
        (Some("created_at"), Some("desc")) => {
            select = select.order_by_desc(attachment::Column::CreatedAt);
        }
        (Some("id"), Some("asc")) => {
            select = select.order_by_asc(attachment::Column::Id);
        }
        _ => {
            select = select.order_by_desc(attachment::Column::Id);
        }
    }

    let paginator = select.paginate(&db, size);
    let total = paginator
        .num_items()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    let total_pages = paginator
        .num_pages()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    let items = paginator
        .fetch_page(page - 1)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(AttachmentList {
        items,
        total,
        page,
        size,
        total_pages,
    })
    .into_response())
}

async fn delete_attachment(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Extension(config): Extension<Arc<ApplicationConfiguration>>,
    _user: JwtClaims,
) -> Result<Response, Response> {
    StorageService::delete(&db, &config, id)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(ApiResponse::ok(()).into_response())
}
