mod api;
mod pages;

use std::sync::Arc;

use axum::Router;
use tower_http::services::ServeDir;

use crate::config::ApplicationConfiguration;

pub fn get_routes(config: &Arc<ApplicationConfiguration>) -> Router {
    Router::new()
        .nest_service(
            "/admin",
            ServeDir::new(config.asset_dir.join("public/admin")),
        )
        .nest("/api", api::get_routes())
        .merge(pages::get_routes())
}
