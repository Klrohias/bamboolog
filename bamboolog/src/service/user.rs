use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};
use tracing::instrument;

use crate::{
    entity::user,
    service::jwt::{AuthError, JwtClaims},
    utils::{ApiResponse, FailibleOperationExt},
};

pub struct User(pub user::Model);

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = UserExtractError;

    #[instrument(skip_all)]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = JwtClaims::from_request_parts(parts, state)
            .await
            .traced(|e| tracing::error!("{}", e))?;

        let database = parts
            .extensions
            .get::<DatabaseConnection>()
            .expect("DatabaseConnection should be configured");

        let user = user::Entity::find_by_id(claims.user_id)
            .one(database)
            .await?;

        let user = user.ok_or_else(|| UserExtractError::UserNotFound)?;

        Ok(Self(user))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserExtractError {
    #[error(transparent)]
    AuthError(#[from] AuthError),

    #[error(transparent)]
    DbErr(#[from] DbErr),

    #[error("User not found")]
    UserNotFound,
}

impl IntoResponse for UserExtractError {
    fn into_response(self) -> Response {
        tracing::error!("Failed to get user from header: {}", self);
        (StatusCode::UNAUTHORIZED, ApiResponse::unauthorized()).into_response()
    }
}
