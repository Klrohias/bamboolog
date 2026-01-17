use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "admin-dist/"]
struct Assets;

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/{*path}", get(static_handler))
}

async fn index_handler() -> impl IntoResponse {
    static_handler(axum::extract::Path("index.html".to_string())).await
}

async fn static_handler(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    let path = if path.is_empty() || path == "/" {
        "index.html".to_string()
    } else {
        path
    };

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => {
            // SPA fallback: if not found, serve index.html
            match Assets::get("index.html") {
                Some(content) => Response::builder()
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(Body::from(content.data))
                    .unwrap(),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}
