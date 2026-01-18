use crate::config::ApplicationConfiguration;
use crate::entity::{attachment, storage_engine};
use anyhow::Context;
use sea_orm::*;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct StorageService;

impl StorageService {
    pub async fn upload(
        db: &DatabaseConnection,
        config: &ApplicationConfiguration,
        data: &[u8],
        mime_type: String,
        engine_id: Option<i32>,
    ) -> Result<attachment::Model, anyhow::Error> {
        let digest = md5::compute(data);
        let hash = format!("{:x}", digest);

        // Check if hash exists
        if let Some(_) = attachment::Entity::find()
            .filter(attachment::Column::Hash.eq(&hash))
            .one(db)
            .await?
        {
            return Err(anyhow::anyhow!("File with hash {} already exists", hash));
        }

        // Find storage engine
        let engine = if let Some(id) = engine_id {
            storage_engine::Entity::find_by_id(id)
                .one(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Storage engine not found"))?
        } else {
            storage_engine::Entity::find()
                .filter(storage_engine::Column::Type.eq("internal"))
                .one(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No internal storage engine found"))?
        };

        // Determine extension
        let ext = mime_guess::get_mime_extensions_str(&mime_type)
            .and_then(|exts| exts.first())
            .unwrap_or(&"bin");
        let filename = format!("{}.{}", hash, ext);

        // Construct Path: asset_dir/attachments/{engine_id}/
        let relative_dir = Path::new("attachments").join(engine.id.to_string());
        let abs_dir = config.asset_dir.join(&relative_dir);

        if !abs_dir.exists() {
            fs::create_dir_all(&abs_dir)
                .await
                .context("Failed to create storage directory")?;
        }

        let abs_path = abs_dir.join(&filename);
        fs::write(&abs_path, data)
            .await
            .context("Failed to write file")?;

        // Store path relative to asset_dir
        let stored_path = relative_dir.join(&filename).to_string_lossy().to_string();

        let attach_model = attachment::ActiveModel {
            mime: Set(mime_type),
            hash: Set(hash),
            storage_engine_id: Set(engine.id),
            path: Set(stored_path),
            ..Default::default()
        };

        attach_model
            .insert(db)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn get_attachment_path(
        db: &DatabaseConnection,
        config: &ApplicationConfiguration,
        hash: &str,
    ) -> Result<PathBuf, anyhow::Error> {
        let attach = attachment::Entity::find()
            .filter(attachment::Column::Hash.eq(hash))
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attachment not found"))?;

        let path = config.asset_dir.join(attach.path);
        if !path.exists() {
            return Err(anyhow::anyhow!("File not found on disk"));
        }
        Ok(path)
    }

    pub async fn delete(
        db: &DatabaseConnection,
        config: &ApplicationConfiguration,
        id: i32,
    ) -> Result<(), anyhow::Error> {
        let attach = attachment::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Attachment not found"))?;

        let path = config.asset_dir.join(&attach.path);

        attachment::Entity::delete_by_id(id).exec(db).await?;

        if path.exists() {
            fs::remove_file(path)
                .await
                .context("Failed to delete file")?;
        }
        Ok(())
    }
}
