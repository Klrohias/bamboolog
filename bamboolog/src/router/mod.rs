mod admin;
mod api;
mod pages;

use std::sync::Arc;

use axum::Router;

use crate::config::ApplicationConfiguration;

pub fn get_routes(_config: &Arc<ApplicationConfiguration>) -> Router {
    Router::new()
        .nest("/admin", admin::get_routes())
        .nest("/api", api::get_routes())
        .merge(pages::get_routes())
}
