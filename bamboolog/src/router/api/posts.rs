use axum::{
    Extension,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait};

use crate::{
    entity::post,
    service::jwt::JwtClaims,
    utils::{ApiResponse, HttpFailibleOperationExt},
};

pub async fn get_post_content(
    Extension(database): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
) -> Result<Response, Response> {
    let post = post::Entity::find_by_id(id)
        .one(&database)
        .await
        .traced_and_response()?;

    Ok(match post {
        None => {
            ApiResponse::code_and_message(StatusCode::NOT_FOUND, "No post found").into_response()
        }
        Some(post) => ApiResponse::ok(post).into_response(),
    })
}

pub async fn delete_post(
    Extension(database): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    let post = post::Entity::find_by_id(id)
        .one(&database)
        .await
        .traced_and_response()?;

    match post {
        None => Err(
            ApiResponse::code_and_message(StatusCode::NOT_FOUND, "No post found").into_response(),
        ),
        Some(post) => {
            post.delete(&database).await.traced_and_response()?;
            Ok(ApiResponse::ok(()).into_response())
        }
    }
}

pub async fn put_post(
    Extension(database): Extension<DatabaseConnection>,
    _claims: JwtClaims,
) -> Result<Response, Response> {
    todo!()
}
