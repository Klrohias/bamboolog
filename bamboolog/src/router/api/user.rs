use crate::{
    entity::user,
    service::jwt::{JWT_COOKIE_NAME, JwtClaims, JwtService, uses_cookie_authorization},
    utils::{ApiResponse, HttpFailibleOperationExts},
};
use axum::{
    Extension, Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub fn get_routes() -> Router {
    Router::new()
        .route("/auth", post(login_user))
        .route("/logout", post(logout_user))
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
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
    headers: axum::http::HeaderMap,
    Json(login_user): Json<LoginUser>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let Some(user) = user::Entity::find()
        .filter(user::Column::Username.eq(login_user.username))
        .one(&db)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?
    else {
        return Err(invalid_credentials_response());
    };

    let password_matches = bcrypt::verify(&login_user.password, &user.password_hash)
        .traced_and_response(|e| tracing::error!("{}", e))?;
    if !password_matches {
        return Err(invalid_credentials_response());
    }

    let token = jwt_service
        .issue(user.clone())
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    let uses_cookie_session = uses_cookie_authorization(&headers);

    let response = LoginResponse {
        user: UserResponse::from(user),
        token: (!uses_cookie_session).then_some(token.clone()),
    };
    let response = ApiResponse::ok(response);

    Ok(if uses_cookie_session {
        (
            CookieJar::new().add(
                Cookie::build((JWT_COOKIE_NAME, token))
                    .http_only(true)
                    .same_site(SameSite::Strict)
                    .path("/")
                    .build(),
            ),
            response,
        )
            .into_response()
    } else {
        response.into_response()
    })
}

fn invalid_credentials_response() -> axum::response::Response {
    ApiResponse::code_and_message(StatusCode::UNAUTHORIZED, "Invalid credentials").into_response()
}

async fn logout_user() -> impl IntoResponse {
    let cookie = Cookie::build(JWT_COOKIE_NAME).path("/").build();
    (CookieJar::new().remove(cookie), ApiResponse::ok(()))
}

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
        if new_password.is_empty() {
            return Err(ApiResponse::code_and_message(
                StatusCode::BAD_REQUEST,
                "New password cannot be empty",
            )
            .into_response());
        }
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

#[cfg(test)]
mod tests {
    use axum::{
        Extension,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use sea_orm::{
        ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, DatabaseBackend,
        DatabaseConnection, Schema,
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::get_routes;
    use crate::{entity::user, service::jwt::JwtService};

    async fn database_with_user_schema() -> DatabaseConnection {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let schema = Schema::new(DatabaseBackend::Sqlite);
        database
            .execute(&schema.create_table_from_entity(user::Entity))
            .await
            .unwrap();
        database
    }

    async fn insert_user(database: &DatabaseConnection) {
        user::ActiveModel {
            username: Set("admin".to_string()),
            email: Set("admin@example.test".to_string()),
            nickname: Set("Admin".to_string()),
            password_hash: Set(bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap()),
            ..Default::default()
        }
        .insert(database)
        .await
        .unwrap();
    }

    fn login_request(cookie_mode: bool) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/auth")
            .header(header::CONTENT_TYPE, "application/json");
        if cookie_mode {
            builder = builder.header(header::AUTHORIZATION, "cookie");
        }
        builder
            .body(Body::from(r#"{"username":"admin","password":"password"}"#))
            .unwrap()
    }

    #[tokio::test]
    async fn supports_bearer_and_http_only_cookie_login_modes() {
        let database = database_with_user_schema().await;
        insert_user(&database).await;
        let jwt = JwtService::new(database.clone());
        let app = get_routes()
            .layer(Extension(database))
            .layer(Extension(jwt));

        let bearer_login = app.clone().oneshot(login_request(false)).await.unwrap();
        assert_eq!(bearer_login.status(), StatusCode::OK);
        assert!(bearer_login.headers().get(header::SET_COOKIE).is_none());
        let bearer_body = to_bytes(bearer_login.into_body(), usize::MAX)
            .await
            .unwrap();
        let bearer_payload: Value = serde_json::from_slice(&bearer_body).unwrap();
        let token = bearer_payload["data"]["token"].as_str().unwrap();

        let bearer_me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/me")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bearer_me.status(), StatusCode::OK);

        let cookie_login = app.clone().oneshot(login_request(true)).await.unwrap();
        assert_eq!(cookie_login.status(), StatusCode::OK);
        let cookie = cookie_login
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Path=/"));
        let cookie_body = to_bytes(cookie_login.into_body(), usize::MAX)
            .await
            .unwrap();
        let cookie_payload: Value = serde_json::from_slice(&cookie_body).unwrap();
        assert!(cookie_payload["data"].get("token").is_none());

        let cookie_me = app
            .oneshot(
                Request::builder()
                    .uri("/me")
                    .header(header::AUTHORIZATION, "cookie")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cookie_me.status(), StatusCode::OK);
    }
}
