use axum::{
    Extension, Router,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};

mod posts;

use crate::{
    service::{jwt::JwtClaims, reloader::ServiceReloader},
    utils::{ApiResponse, HttpFailibleOperationExt},
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/reload", post(reload))
        .route(
            "/posts/{id}",
            get(posts::get_post_content).delete(posts::delete_post),
        )
        .route("/posts/", put(posts::put_post))
}

async fn reload(
    Extension(reloader): Extension<ServiceReloader>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    reloader.reload().await.traced_and_response()?;

    Ok(ApiResponse::ok(()).into_response())
}
