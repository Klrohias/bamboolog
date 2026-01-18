use axum::{
    Extension, Router,
    extract::Path,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::json;
use tracing::instrument;

use crate::service::storage::StorageService;
use crate::{
    entity::post::{Column as PostColumn, Entity as PostEntity},
    service::theme::ThemeService,
    utils::HttpFailibleOperationExts,
};
use axum::body::Body;
use axum::http::StatusCode;
use tokio_util::io::ReaderStream;

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(display_home))
        .route("/posts/{id_or_name}", get(display_post))
        .route("/static/theme/{*path}", get(serve_theme_static))
        .route("/attachments/{hash}", get(serve_attachment))
}

#[instrument(skip_all)]
async fn display_home(
    Extension(theme_service): Extension<ThemeService>,
) -> Result<Html<String>, Response> {
    Ok(Html(
        theme_service
            .render("home", json!({}))
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    ))
}

async fn display_post(
    Path(id_or_name): Path<String>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
) -> Result<Html<String>, Response> {
    // Is `is_or_name` a number?
    let post = match id_or_name.parse::<i32>() {
        Err(_) => PostEntity::find()
            .filter(PostColumn::Name.eq(id_or_name))
            .one(&database)
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
        Ok(id) => PostEntity::find_by_id(id)
            .one(&database)
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    };

    // Really found?
    let post = match post {
        None => {
            return Err(theme_service
                .render("not-found", json!({}))
                .await
                .traced_and_response(|e| tracing::error!("{}", e))?
                .into_response());
        }
        Some(v) => v,
    };

    // Render markdown
    let rendered_content = markdown::to_html_with_options(&post.content, &markdown::Options::gfm())
        .traced_and_response(|e| tracing::error!("{}", e))?;

    // Render jinja
    Ok(Html(
        theme_service
            .render(
                "post",
                json!({
                    "content": rendered_content,
                    "post": post
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    ))
}

async fn serve_theme_static(
    Path(path): Path<String>,
    Extension(theme_service): Extension<ThemeService>,
) -> Result<Response, Response> {
    Ok(theme_service
        .serve_static(path)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?)
}

async fn serve_attachment(
    Path(hash): Path<String>,
    Extension(db): Extension<DatabaseConnection>,
    Extension(config): Extension<std::sync::Arc<crate::config::ApplicationConfiguration>>,
) -> Result<impl IntoResponse, Response> {
    let path = StorageService::get_attachment_path(&db, &config, &hash)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    let file = tokio::fs::File::open(path).await.map_err(|e| {
        tracing::error!("{}", e);
        (StatusCode::NOT_FOUND, "File not found").into_response()
    })?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok(body)
}
