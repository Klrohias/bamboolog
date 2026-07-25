use axum::Router;

mod attachments;
mod posts;
mod settings;
mod storage_engines;
mod themes;
mod user;

pub fn get_routes() -> Router {
    Router::new()
        .nest("/posts", posts::get_routes())
        .nest("/user", user::get_routes())
        .nest("/settings", settings::get_routes())
        .nest("/themes", themes::get_routes())
        .nest("/attachments", attachments::get_routes())
        .nest("/storage_engines", storage_engines::get_routes())
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::get_routes;

    #[tokio::test]
    async fn collection_routes_match_without_a_trailing_slash() {
        let app = Router::new().nest("/api", get_routes());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/posts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/posts/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
