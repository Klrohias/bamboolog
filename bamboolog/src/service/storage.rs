use std::sync::Arc;

use anyhow::Context;
use axum::{
    body::Body,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use sea_orm::*;

use crate::{
    config::ApplicationConfiguration,
    entity::{attachment, storage_engine},
    storage::create_storage_provider,
};

pub struct StorageService;

impl StorageService {
    pub async fn upload(
        db: &DatabaseConnection,
        config: Arc<ApplicationConfiguration>,
        data: &[u8],
        mime_type: String,
        filename: Option<String>,
        engine_id: Option<i32>,
    ) -> Result<attachment::Model, anyhow::Error> {
        let hash = format!("{:x}", md5::compute(data));

        if attachment::Entity::find()
            .filter(attachment::Column::Hash.eq(&hash))
            .one(db)
            .await?
            .is_some()
        {
            return Err(anyhow::anyhow!("File with hash {hash} already exists"));
        }

        let engine = Self::resolve_engine(db, engine_id).await?;
        let ext = mime_guess::get_mime_extensions_str(&mime_type)
            .and_then(|exts| exts.first())
            .unwrap_or(&"bin");
        let key = format!("attachments/{}/{}.{}", engine.id, hash, ext);

        let provider = create_storage_provider(config, &engine).await?;
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
            byte_size: Set(i64::try_from(data.len()).unwrap_or(i64::MAX)),
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
        db: &DatabaseConnection,
        config: Arc<ApplicationConfiguration>,
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

        let provider = create_storage_provider(config, &engine).await?;
        let object = provider.get(&attach.object_key).await?;
        let content_type = object.mime.unwrap_or(attach.mime);

        Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, content_type)],
            Body::from(object.bytes),
        )
            .into_response())
    }

    pub async fn delete(
        db: &DatabaseConnection,
        config: Arc<ApplicationConfiguration>,
        id: i32,
    ) -> Result<(), anyhow::Error> {
        let attach = attachment::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attachment not found"))?;

        let engine = storage_engine::Entity::find_by_id(attach.storage_engine_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Storage engine not found"))?;
        let provider = create_storage_provider(config, &engine).await?;

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
}
