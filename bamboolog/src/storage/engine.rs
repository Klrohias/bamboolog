use std::path::PathBuf;

use async_trait::async_trait;

use crate::storage::builtin_engine::BuiltinStorageEngine;

#[async_trait]
pub trait StorageEngine {
    async fn upload_attachment(&self, local_path: PathBuf, hash: String);
}

pub fn create_storage_engine(
    name: String,
    storage_type: String,
    _config: String,
) -> Result<Box<dyn StorageEngine>, EngineCreationError> {
    if storage_type == "builtin" {
        return Ok(Box::new(BuiltinStorageEngine::new(name)));
    }

    Err(EngineCreationError::UnsupportedType(storage_type))
}

#[derive(thiserror::Error, Debug)]
pub enum EngineCreationError {
    #[error("Unsupported storage engine type `{0}`")]
    UnsupportedType(String),
}
