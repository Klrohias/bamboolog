use axum::Router;

mod attachments;
mod posts;
mod settings;
mod storage_engines;
mod themes;
mod user;

pub fn get_routes() -> Router {
    Router::new()
        .nest("/posts/", posts::get_routes())
        .nest("/user/", user::get_routes())
        .nest("/settings/", settings::get_routes())
        .nest("/themes/", themes::get_routes())
        .nest("/attachments/", attachments::get_routes())
        .nest("/storage_engines/", storage_engines::get_routes())
}
