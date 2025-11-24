use axum::{
    Extension, Router,
    response::{IntoResponse, Response},
    routing::post,
};

mod posts;
mod user;

use crate::{
    service::{jwt::JwtClaims, reloader::ServiceReloader},
    utils::{ApiResponse, HttpFailibleOperationExt},
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/reload", post(reload))
        .nest("/posts/", posts::get_routes())
        .nest("/user/", user::get_routes())
}

async fn reload(
    Extension(reloader): Extension<ServiceReloader>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    reloader.reload().await.traced_and_response()?;

    Ok(ApiResponse::ok(()).into_response())
}
