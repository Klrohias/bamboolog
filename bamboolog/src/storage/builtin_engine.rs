use std::path::PathBuf;

use async_trait::async_trait;

use crate::storage::StorageEngine;

pub struct BuiltinStorageEngine {
    name: String,
}

impl BuiltinStorageEngine {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait]
impl StorageEngine for BuiltinStorageEngine {
    async fn upload_attachment(&self, local_path: PathBuf, hash: String) {
        todo!()
    }
}
