mod api;
mod pages;

use axum::Router;
use tower_http::services::ServeDir;

pub fn get_routes() -> Router {
    Router::new()
        .nest_service("/admin", ServeDir::new("static-admin"))
        .nest("/api", api::get_routes())
        .merge(pages::get_routes())
}
