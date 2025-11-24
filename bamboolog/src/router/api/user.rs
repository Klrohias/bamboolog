use std::sync::Arc;

use crate::{
    entity::user,
    service::jwt::{JwtClaims, JwtService},
    utils::{ApiResponse, HttpFailibleOperationExt},
};
use axum::{
    Extension, Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub fn get_routes() -> Router {
    Router::new()
        .route("/auth", post(login_user))
        .route("/me", get(get_me))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub nickname: String,
}

impl From<user::Model> for UserResponse {
    fn from(model: user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
            nickname: model.nickname,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Deserialize)]
struct LoginUser {
    pub username: String,
    pub password: String,
}

#[instrument(skip(db, jwt_service))]
async fn login_user(
    Extension(db): Extension<DatabaseConnection>,
    Extension(jwt_service): Extension<Arc<JwtService>>,
    Json(login_user): Json<LoginUser>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match user::Entity::find()
        .filter(user::Column::Username.eq(login_user.username))
        .one(&db)
        .await
        .traced_and_response()?
    {
        Some(user) => {
            if bcrypt::verify(&login_user.password, &user.password_hash).traced_and_response()? {
                let token = jwt_service
                    .issue(user.clone())
                    .await
                    .traced_and_response()?;
                Ok(ApiResponse::ok(LoginResponse {
                    user: UserResponse::from(user),
                    token,
                })
                .into_response())
            } else {
                Err(
                    ApiResponse::code_and_message(StatusCode::UNAUTHORIZED, "Invalid credentials")
                        .into_response(),
                )
            }
        }
        None => Err(
            ApiResponse::code_and_message(StatusCode::UNAUTHORIZED, "Invalid credentials")
                .into_response(),
        ),
    }
}

#[instrument(skip(db))]
async fn get_me(
    Extension(db): Extension<DatabaseConnection>,
    Extension(claims): Extension<JwtClaims>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match user::Entity::find()
        .filter(user::Column::Id.eq(claims.user_id))
        .one(&db)
        .await
        .traced_and_response()?
    {
        Some(user) => Ok(ApiResponse::ok(UserResponse::from(user)).into_response()),
        None => Err(
            ApiResponse::code_and_message(StatusCode::NOT_FOUND, "User not found").into_response(),
        ),
    }
}
