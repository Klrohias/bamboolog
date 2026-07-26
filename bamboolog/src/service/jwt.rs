use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::{
    extract::TypedHeader,
    extract::cookie::CookieJar,
    headers::{Authorization, authorization::Bearer},
};
use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use rand::{
    RngCore,
    distr::{Alphanumeric, SampleString},
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::instrument;

use crate::{
    config::config_entries,
    entity::user,
    service::reloadable::ReloadableService,
    utils::{ApiResponse, FailibleOperationExts},
};

pub const JWT_COOKIE_NAME: &str = "bamboolog_jwt";

pub fn uses_cookie_authorization(headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("cookie"))
}

#[derive(Serialize, Deserialize)]
pub struct JwtServiceSettings {
    pub secret: String,
    pub audience: String,
    pub expire: u64,
}

impl JwtServiceSettings {
    fn random() -> Self {
        let new_secret = Alphanumeric.sample_string(&mut rand::rng(), 32);
        Self {
            secret: new_secret,
            audience: "RANDOM-JWT".to_string(),
            expire: 3600,
        }
    }
}

impl Default for JwtServiceSettings {
    fn default() -> Self {
        Self {
            secret: "12345678901234567890123456789012".to_string(),
            audience: "DANGER-JWT".to_string(),
            expire: 3600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JwtServiceState {
    pub decoding_key: DecodingKey,
    pub validation: Validation,
    pub encoding_key: EncodingKey,
    pub default_expire: usize,
}

impl JwtServiceState {
    pub fn new(
        decoding_key: DecodingKey,
        validation: Validation,
        encoding_key: EncodingKey,
        default_expire: usize,
    ) -> Self {
        Self {
            decoding_key,
            validation,
            encoding_key,
            default_expire,
        }
    }
}

impl Default for JwtServiceState {
    fn default() -> Self {
        Self::from(JwtServiceSettings::default())
    }
}

impl From<JwtServiceSettings> for JwtServiceState {
    fn from(value: JwtServiceSettings) -> Self {
        let expire = usize::try_from(value.expire).expect("JWT expire value does not fit in usize");

        let decoding_key = DecodingKey::from_secret(value.secret.as_bytes());
        let encoding_key = EncodingKey::from_secret(value.secret.as_bytes());
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(std::slice::from_ref(&value.audience));

        Self::new(decoding_key, validation, encoding_key, expire)
    }
}

#[derive(Debug, Clone)]
pub struct JwtService {
    state: Arc<RwLock<JwtServiceState>>,
    dep_db: DatabaseConnection,
}

impl JwtService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            state: Arc::new(RwLock::new(JwtServiceState::default())),
            dep_db: db,
        }
    }

    pub async fn decode(
        &self,
        bearer: impl AsRef<str>,
    ) -> Result<TokenData<JwtClaims>, jsonwebtoken::errors::Error> {
        let state = self.state.read().await;
        decode::<JwtClaims>(bearer.as_ref(), &state.decoding_key, &state.validation)
    }

    pub async fn encode(&self, chaims: &JwtClaims) -> Result<String, jsonwebtoken::errors::Error> {
        let state = self.state.read().await;
        encode(&Header::default(), chaims, &state.encoding_key)
    }

    pub async fn issue(&self, user: user::Model) -> Result<String, jsonwebtoken::errors::Error> {
        let state = self.state.read().await;
        let mut rng = rand::rng();

        encode(
            &Header::default(),
            &JwtClaims {
                sub: user.username,
                jti: rng.next_u64().to_string(),
                exp: Utc::now().timestamp() as usize + state.default_expire,
                user_id: user.id,
            },
            &state.encoding_key,
        )
    }
}

#[async_trait]
impl ReloadableService for JwtService {
    async fn reload(&self) {
        let settings = match config_entries::JWT_SETTINGS
            .get::<JwtServiceSettings>(&self.dep_db)
            .await
        {
            Err(e) => {
                tracing::warn!(
                    "Failed to load settings for jwt service. \n
For security, we will generate a random temporary settings, you should shutdown the application and check it: {}",
                    e
                );

                Some(JwtServiceSettings::random())
            }
            Ok(v) => v,
        };

        let settings = match settings {
            None => {
                tracing::warn!(
                    "No settings present for jwt service. For security, we will generate a new settings."
                );

                let new_settings = JwtServiceSettings::random();

                if let Err(e) = config_entries::JWT_SETTINGS
                    .set(&self.dep_db, Some(&new_settings))
                    .await
                {
                    tracing::warn!("Failed to save a new jwt settings: {}", e);
                }

                new_settings
            }
            Some(v) => v,
        };

        {
            let mut state = self.state.write().await;
            *state = settings.into();
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid token")]
pub struct AuthError;

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, ApiResponse::unauthorized()).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: usize,
    pub jti: String,
    pub user_id: i32,
}

impl<S> FromRequestParts<S> for JwtClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    #[instrument(skip_all)]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = if uses_cookie_authorization(&parts.headers) {
            CookieJar::from_headers(&parts.headers)
                .get(JWT_COOKIE_NAME)
                .map(|cookie| cookie.value().to_owned())
                .ok_or(AuthError)?
        } else {
            let TypedHeader(Authorization(bearer)) =
                TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                    .await
                    .map_err(|_| AuthError)?;
            bearer.token().to_owned()
        };

        let service = parts
            .extensions
            .get::<JwtService>()
            .expect("JwtService should be configured");

        let token_data = service
            .decode(token)
            .await
            .traced(|e| tracing::error!("{}", e))
            .map_err(|_| AuthError)?;

        Ok(token_data.claims)
    }
}
