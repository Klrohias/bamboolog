use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use sea_orm::*;
use tokio::sync::RwLock;

use crate::{
    config::ApplicationConfiguration,
    entity::{attachment, storage_engine},
    storage::create_storage_provider,
};

#[derive(Clone)]
pub struct StorageService {
    config: Arc<ApplicationConfiguration>,
    providers: Arc<RwLock<HashMap<i32, CachedProvider>>>,
}

struct CachedProvider {
    fingerprint: StorageEngineFingerprint,
    provider: Arc<dyn crate::storage::AttachmentStorage>,
}

#[derive(PartialEq, Eq)]
struct StorageEngineFingerprint {
    kind: String,
    config_json: Option<String>,
}

impl StorageService {
    pub fn new(config: Arc<ApplicationConfiguration>) -> Self {
        Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn upload(
        &self,
        db: &DatabaseConnection,
        data: Vec<u8>,
        mime_type: String,
        filename: Option<String>,
        engine_id: Option<i32>,
    ) -> Result<attachment::Model, anyhow::Error> {
        let byte_size = i64::try_from(data.len()).unwrap_or(i64::MAX);
        let hash = format!("{:x}", md5::compute(&data));

        if let Some(existing_attachment) = attachment::Entity::find()
            .filter(attachment::Column::Hash.eq(&hash))
            .one(db)
            .await?
        {
            return Ok(existing_attachment);
        }

        let engine = Self::resolve_engine(db, engine_id).await?;
        let ext = mime_guess::get_mime_extensions_str(&mime_type)
            .and_then(|exts| exts.first())
            .unwrap_or(&"bin");
        let key = format!("attachments/{}/{}.{}", engine.id, hash, ext);

        let provider = self.provider_for(&engine).await?;
        provider
            .put(&key, data, &mime_type)
            .await
            .with_context(|| format!("Failed to store attachment with key {key}"))?;

        let attach_model = attachment::ActiveModel {
            hash: Set(hash),
            storage_engine_id: Set(engine.id),
            object_key: Set(key.clone()),
            filename: Set(filename.unwrap_or_default()),
            mime: Set(mime_type),
            byte_size: Set(byte_size),
            ..Default::default()
        };

        match attach_model.insert(db).await {
            Ok(attachment) => Ok(attachment),
            Err(error) => {
                if let Err(cleanup_error) = provider.delete(&key).await {
                    tracing::error!(
                        "Failed to clean up attachment object after database insert failed: {cleanup_error}"
                    );
                }
                Err(anyhow::anyhow!(error))
            }
        }
    }

    pub async fn serve(
        &self,
        db: &DatabaseConnection,
        hash: &str,
    ) -> Result<Response, anyhow::Error> {
        let attach = attachment::Entity::find()
            .filter(attachment::Column::Hash.eq(hash))
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attachment not found"))?;

        let engine = storage_engine::Entity::find_by_id(attach.storage_engine_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Storage engine not found"))?;

        let provider = self.provider_for(&engine).await?;
        let object = provider.get(&attach.object_key).await?;
        let content_type = object.mime.unwrap_or(attach.mime);

        Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CONTENT_LENGTH, attach.byte_size.to_string()),
            ],
            object.body,
        )
            .into_response())
    }

    pub async fn delete(&self, db: &DatabaseConnection, id: i32) -> Result<(), anyhow::Error> {
        let attach = attachment::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attachment not found"))?;

        let engine = storage_engine::Entity::find_by_id(attach.storage_engine_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Storage engine not found"))?;
        let provider = self.provider_for(&engine).await?;

        provider.delete(&attach.object_key).await?;
        attachment::Entity::delete_by_id(id).exec(db).await?;

        Ok(())
    }

    async fn resolve_engine(
        db: &DatabaseConnection,
        engine_id: Option<i32>,
    ) -> Result<storage_engine::Model, anyhow::Error> {
        if let Some(id) = engine_id {
            let engine = storage_engine::Entity::find_by_id(id)
                .one(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Storage engine not found"))?;
            if !engine.enabled {
                return Err(anyhow::anyhow!("Storage engine is disabled"));
            }
            return Ok(engine);
        }

        storage_engine::Entity::find()
            .filter(storage_engine::Column::Enabled.eq(true))
            .order_by_desc(storage_engine::Column::IsDefault)
            .order_by_asc(storage_engine::Column::Id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No enabled storage engine found"))
    }

    async fn provider_for(
        &self,
        engine: &storage_engine::Model,
    ) -> Result<Arc<dyn crate::storage::AttachmentStorage>, anyhow::Error> {
        let fingerprint = StorageEngineFingerprint {
            kind: engine.kind.clone(),
            config_json: engine.config_json.clone(),
        };

        if let Some(cached) = self.providers.read().await.get(&engine.id)
            && cached.fingerprint == fingerprint
        {
            return Ok(cached.provider.clone());
        }

        let provider = create_storage_provider(self.config.clone(), engine).await?;
        let mut providers = self.providers.write().await;
        providers.insert(
            engine.id,
            CachedProvider {
                fingerprint,
                provider: provider.clone(),
            },
        );
        Ok(provider)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{config::ApplicationConfiguration, entity::storage_engine};

    use super::StorageService;

    fn local_engine(config_json: Option<&str>) -> storage_engine::Model {
        storage_engine::Model {
            id: 1,
            name: "Local storage".to_string(),
            comments: String::new(),
            kind: "local".to_string(),
            config_json: config_json.map(ToOwned::to_owned),
            is_default: true,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn reuses_providers_until_the_engine_configuration_changes() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let service = StorageService::new(Arc::new(ApplicationConfiguration {
            listen_addr: "127.0.0.1:0".to_string(),
            database: "sqlite::memory:".to_string(),
            raw_asset_dir: temporary_directory.path().display().to_string(),
            asset_dir: temporary_directory.path().to_path_buf(),
        }));
        let engine = local_engine(None);

        let first = service.provider_for(&engine).await.unwrap();
        let second = service.provider_for(&engine).await.unwrap();

        assert!(Arc::ptr_eq(&first, &second));

        let changed_engine = local_engine(Some(r#"{"root": "uploads"}"#));
        let replacement = service.provider_for(&changed_engine).await.unwrap();

        assert!(!Arc::ptr_eq(&first, &replacement));
    }
}
