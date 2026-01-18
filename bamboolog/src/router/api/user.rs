use crate::{
    entity::user,
    service::jwt::{JwtClaims, JwtService},
    utils::{ApiResponse, HttpFailibleOperationExts},
};
use axum::{
    Extension, Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub fn get_routes() -> Router {
    Router::new()
        .route("/auth", post(login_user))
        .route("/me", get(get_me).post(update_me))
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
    Extension(jwt_service): Extension<JwtService>,
    Json(login_user): Json<LoginUser>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match user::Entity::find()
        .filter(user::Column::Username.eq(login_user.username))
        .one(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
    {
        Some(user) => {
            if bcrypt::verify(&login_user.password, &user.password_hash)
                .traced_and_response(|e| tracing::error!("{}", e))?
            {
                let token = jwt_service
                    .issue(user.clone())
                    .await
                    .traced_and_response(|e| tracing::error!("{}", e))?;
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

// ... (previous imports)

#[instrument(skip(db))]
async fn get_me(
    Extension(db): Extension<DatabaseConnection>,
    claims: JwtClaims,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match user::Entity::find()
        .filter(user::Column::Id.eq(claims.user_id))
        .one(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
    {
        Some(user) => Ok(ApiResponse::ok(UserResponse::from(user)).into_response()),
        None => Err(
            ApiResponse::code_and_message(StatusCode::NOT_FOUND, "User not found").into_response(),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct UpdateUserRequest {
    pub nickname: Option<String>,
    pub old_password: Option<String>,
    pub new_password: Option<String>,
}

#[instrument(skip(db))]
async fn update_me(
    Extension(db): Extension<DatabaseConnection>,
    claims: JwtClaims,
    Json(req): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let mut user = match user::Entity::find()
        .filter(user::Column::Id.eq(claims.user_id))
        .one(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
    {
        Some(u) => u.into_active_model(),
        None => {
            return Err(
                ApiResponse::code_and_message(StatusCode::NOT_FOUND, "User not found")
                    .into_response(),
            );
        }
    };

    if let Some(nickname) = req.nickname {
        user.nickname = sea_orm::ActiveValue::Set(nickname);
    }

    if let (Some(old_password), Some(new_password)) = (&req.old_password, &req.new_password) {
        let current_password_hash = user.password_hash.clone().unwrap();
        if !bcrypt::verify(old_password, &current_password_hash)
            .traced_and_response(|e| tracing::error!("{}", e))?
        {
            return Err(ApiResponse::code_and_message(
                StatusCode::BAD_REQUEST,
                "Invalid old password",
            )
            .into_response());
        }

        let new_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .traced_and_response(|e| tracing::error!("{}", e))?;
        user.password_hash = sea_orm::ActiveValue::Set(new_hash);
    } else if req.new_password.is_some() || req.old_password.is_some() {
        return Err(ApiResponse::code_and_message(
            StatusCode::BAD_REQUEST,
            "Both old and new passwords are required to change password",
        )
        .into_response());
    }

    let updated = user
        .update(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok(ApiResponse::ok(UserResponse::from(updated)).into_response())
}
