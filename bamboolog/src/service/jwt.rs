use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::{
    extract::TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::instrument;

use crate::{entity::user, utils::ApiResponse};

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
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| AuthError::InvalidToken)?;

        let service = parts
            .extensions
            .get::<JwtService>()
            .expect("JwtService should be configured");

        let token_data = service.decode(bearer.token()).await.map_err(|e| {
            tracing::info!("Failed to decode jwt: {}", e);
            AuthError::InvalidToken
        })?;

        Ok(token_data.claims)
    }
}

#[derive(Serialize, Deserialize)]
pub struct JwtServiceSettings {
    pub secret: String,
    pub audience: String,
    pub expire: u64,
}

impl Default for JwtServiceSettings {
    fn default() -> Self {
        Self {
            secret: "".to_string(),
            audience: "".to_string(),
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

impl From<JwtServiceSettings> for JwtServiceState {
    fn from(value: JwtServiceSettings) -> Self {
        let expire = usize::try_from(value.expire).expect("JWT expire value does not fit in usize");

        let decoding_key = DecodingKey::from_secret(value.secret.as_bytes());
        let encoding_key = EncodingKey::from_secret(value.secret.as_bytes());
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[value.audience.clone()]);

        Self::new(decoding_key, validation, encoding_key, expire)
    }
}

#[derive(Debug, Clone)]
pub struct JwtService(Arc<RwLock<JwtServiceState>>);

impl JwtService {
    pub fn new(state: JwtServiceState) -> Self {
        Self(Arc::new(RwLock::new(state)))
    }

    pub async fn decode(
        &self,
        bearer: impl AsRef<str>,
    ) -> Result<TokenData<JwtClaims>, jsonwebtoken::errors::Error> {
        let state = self.0.read().await;
        decode::<JwtClaims>(bearer.as_ref(), &state.decoding_key, &state.validation)
    }

    pub async fn encode(&self, chaims: &JwtClaims) -> Result<String, jsonwebtoken::errors::Error> {
        let state = self.0.read().await;
        encode(&Header::default(), chaims, &state.encoding_key)
    }

    pub async fn get_state(&self) -> JwtServiceState {
        self.0.read().await.clone()
    }

    pub async fn set_state(&self, new_state: JwtServiceState) {
        *self.0.write().await = new_state;
    }

    pub async fn issue(&self, user: user::Model) -> Result<String, jsonwebtoken::errors::Error> {
        let state = self.0.read().await;
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

#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, ApiResponse::unauthorized()).into_response()
            }
        }
    }
}
