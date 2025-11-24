use axum::{
    Extension, Router,
    extract::Path,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::json;
use tera::Context;

use crate::{
    entity::post::{Column as PostColumn, Entity as PostEntity},
    service::theme::ThemeService,
    utils::HttpFailibleOperationExt,
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(display_home))
        .route("/posts/{id_or_name}", get(display_post))
        .route("/static/theme/{*path}", get(serve_theme_static))
}

async fn display_home(
    Extension(theme_service): Extension<ThemeService>,
) -> Result<Html<String>, Response> {
    Ok(Html(
        theme_service
            .render("home.html", &Context::default())
            .await
            .traced_and_response()?,
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
            .traced_and_response()?,
        Ok(id) => PostEntity::find_by_id(id)
            .one(&database)
            .await
            .traced_and_response()?,
    };

    // Really found?
    let post = match post {
        None => {
            return Err(theme_service
                .render("not-found.html", &Context::default())
                .await
                .traced_and_response()?
                .into_response());
        }
        Some(v) => v,
    };

    // Render markdown
    let rendered_content = markdown::to_html_with_options(&post.content, &markdown::Options::gfm())
        .traced_and_response()?;

    // Render jinja
    Ok(Html(
        theme_service
            .render(
                "post.html",
                &Context::from_value(json!({
                    "content": rendered_content,
                    "post": post
                }))
                .traced_and_response()?,
            )
            .await
            .traced_and_response()?,
    ))
}

async fn serve_theme_static(
    Path(path): Path<String>,
    Extension(theme_service): Extension<ThemeService>,
) -> Result<Response, Response> {
    Ok(theme_service
        .serve_static(path)
        .await
        .traced_and_response()?)
}
