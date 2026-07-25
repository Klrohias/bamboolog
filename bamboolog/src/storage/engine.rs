use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use axum::body::Body;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{config::ApplicationConfiguration, entity::storage_engine};

use super::{local_storage::LocalStorageProvider, s3_storage::S3StorageProvider};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("unsupported storage engine type `{0}`")]
    UnsupportedType(String),
    #[error("invalid storage config: {0}")]
    InvalidConfig(String),
    #[error("file not found")]
    NotFound,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type StorageResult<T> = Result<T, StorageError>;

pub struct StoredObject {
    pub body: Body,
    pub mime: Option<String>,
}

#[async_trait]
pub trait AttachmentStorage: Send + Sync {
    async fn put(&self, key: &str, bytes: Vec<u8>, mime: &str) -> StorageResult<()>;
    async fn get(&self, key: &str) -> StorageResult<StoredObject>;
    async fn delete(&self, key: &str) -> StorageResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    pub root: Option<String>,
}

impl LocalStorageConfig {
    pub fn root_path(&self, app_config: &ApplicationConfiguration) -> PathBuf {
        match self.root.as_deref() {
            Some(root) if !root.trim().is_empty() => {
                let path = PathBuf::from(root);
                if path.is_absolute() {
                    path
                } else {
                    app_config.asset_dir.join(path)
                }
            }
            _ => app_config.asset_dir.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3StorageConfig {
    pub bucket: String,
    pub region: Option<String>,
    pub endpoint_url: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub force_path_style: Option<bool>,
    pub prefix: Option<String>,
}

impl S3StorageConfig {
    pub fn validate(&self) -> StorageResult<()> {
        if self.bucket.trim().is_empty() {
            return Err(StorageError::InvalidConfig(
                "bucket is required".to_string(),
            ));
        }

        match (&self.access_key_id, &self.secret_access_key) {
            (Some(access_key), Some(secret_key))
                if !access_key.trim().is_empty() && !secret_key.trim().is_empty() =>
            {
                Ok(())
            }
            (None, None) => Ok(()),
            _ => Err(StorageError::InvalidConfig(
                "access_key_id and secret_access_key must be provided together".to_string(),
            )),
        }
    }
}

pub fn parse_local_config(config: Option<&str>) -> StorageResult<LocalStorageConfig> {
    match config {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| StorageError::InvalidConfig(format!("local config JSON: {e}"))),
        _ => Ok(LocalStorageConfig { root: None }),
    }
}

pub fn parse_s3_config(config: Option<&str>) -> StorageResult<S3StorageConfig> {
    let raw = config
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| StorageError::InvalidConfig("S3 config JSON is required".to_string()))?;
    let parsed: S3StorageConfig = serde_json::from_str(raw)
        .map_err(|e| StorageError::InvalidConfig(format!("S3 config JSON: {e}")))?;
    parsed.validate()?;
    Ok(parsed)
}

pub fn validate_storage_engine_config(
    storage_kind: &str,
    config: Option<&str>,
) -> StorageResult<()> {
    match storage_kind {
        "local" | "internal" => {
            parse_local_config(config)?;
            Ok(())
        }
        "s3" => {
            parse_s3_config(config)?;
            Ok(())
        }
        other => Err(StorageError::UnsupportedType(other.to_string())),
    }
}

pub async fn create_storage_provider(
    app_config: Arc<ApplicationConfiguration>,
    engine: &storage_engine::Model,
) -> StorageResult<Arc<dyn AttachmentStorage>> {
    match engine.kind.as_str() {
        "local" | "internal" => {
            let config = parse_local_config(engine.config_json.as_deref())?;
            Ok(Arc::new(LocalStorageProvider::new(
                config.root_path(&app_config),
            )))
        }
        "s3" => {
            let config = parse_s3_config(engine.config_json.as_deref())?;
            Ok(Arc::new(S3StorageProvider::new(config).await?))
        }
        other => Err(StorageError::UnsupportedType(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_s3_config, validate_storage_engine_config};

    #[test]
    fn validates_s3_configuration() {
        let config = r#"{
            "bucket": "media",
            "endpoint_url": "https://s3.example.test",
            "access_key_id": "access",
            "secret_access_key": "secret"
        }"#;

        assert!(parse_s3_config(Some(config)).is_ok());
        assert!(parse_s3_config(Some(r#"{"bucket": ""}"#)).is_err());
        assert!(
            parse_s3_config(Some(r#"{"bucket": "media", "access_key_id": "access"}"#)).is_err()
        );
    }

    #[test]
    fn accepts_empty_local_configuration() {
        assert!(validate_storage_engine_config("local", None).is_ok());
        assert!(validate_storage_engine_config("unknown", None).is_err());
    }
}
