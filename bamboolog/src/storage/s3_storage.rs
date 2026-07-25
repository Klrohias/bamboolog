use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    Client,
    config::{Builder as S3ConfigBuilder, Region},
    primitives::ByteStream,
};
use axum::body::Body;
use tokio_util::io::ReaderStream;

use super::{AttachmentStorage, S3StorageConfig, StorageError, StorageResult, StoredObject};

pub struct S3StorageProvider {
    client: Client,
    bucket: String,
    prefix: Option<String>,
}

impl S3StorageProvider {
    pub async fn new(config: S3StorageConfig) -> StorageResult<Self> {
        config.validate()?;

        let region = config.region.clone().unwrap_or_else(|| "auto".to_string());
        let mut loader =
            aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region));

        if let Some(endpoint_url) = config.endpoint_url.as_deref()
            && !endpoint_url.trim().is_empty()
        {
            loader = loader.endpoint_url(endpoint_url.to_string());
        }

        if let (Some(access_key_id), Some(secret_access_key)) =
            (&config.access_key_id, &config.secret_access_key)
        {
            loader = loader.credentials_provider(Credentials::new(
                access_key_id.clone(),
                secret_access_key.clone(),
                config.session_token.clone(),
                None,
                "bamboolog-storage-engine",
            ));
        }

        let shared_config = loader.load().await;
        let s3_config = S3ConfigBuilder::from(&shared_config)
            .force_path_style(config.force_path_style.unwrap_or(false))
            .build();

        Ok(Self {
            client: Client::from_conf(s3_config),
            bucket: config.bucket,
            prefix: config.prefix.and_then(|value| {
                let normalized = value.trim_matches('/').to_string();
                (!normalized.is_empty()).then_some(normalized)
            }),
        })
    }

    fn object_key(&self, key: &str) -> String {
        let clean_key = key.trim_start_matches('/');
        match self.prefix.as_deref() {
            Some(prefix) => {
                let clean_prefix = prefix.trim_matches('/');
                format!("{clean_prefix}/{clean_key}")
            }
            None => clean_key.to_string(),
        }
    }
}

#[async_trait]
impl AttachmentStorage for S3StorageProvider {
    async fn put(&self, key: &str, bytes: Vec<u8>, mime: &str) -> StorageResult<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(self.object_key(key))
            .content_type(mime)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| StorageError::Other(anyhow::anyhow!(e)))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> StorageResult<StoredObject> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.object_key(key))
            .send()
            .await
            .map_err(|e| StorageError::Other(anyhow::anyhow!(e)))?;

        let mime = output.content_type().map(ToOwned::to_owned);
        Ok(StoredObject {
            body: Body::from_stream(ReaderStream::new(output.body.into_async_read())),
            mime,
        })
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(self.object_key(key))
            .send()
            .await
            .map_err(|e| StorageError::Other(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
