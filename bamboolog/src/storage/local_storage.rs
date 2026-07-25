use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use axum::body::Body;
use tokio::fs;
use tokio_util::io::ReaderStream;

use super::{AttachmentStorage, StorageError, StorageResult, StoredObject};

pub struct LocalStorageProvider {
    root: PathBuf,
}

impl LocalStorageProvider {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn object_path(&self, key: &str) -> StorageResult<PathBuf> {
        let key_path = Path::new(key);
        if key_path.is_absolute()
            || key_path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(StorageError::InvalidConfig(format!(
                "unsafe attachment key: {key}"
            )));
        }

        Ok(self.root.join(key_path))
    }
}

#[async_trait]
impl AttachmentStorage for LocalStorageProvider {
    async fn put(&self, key: &str, bytes: Vec<u8>, _mime: &str) -> StorageResult<()> {
        let path = self.object_path(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(path, bytes).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> StorageResult<StoredObject> {
        let path = self.object_path(key)?;
        match fs::File::open(path).await {
            Ok(file) => Ok(StoredObject {
                body: Body::from_stream(ReaderStream::new(file)),
                mime: None,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(StorageError::NotFound)
            }
            Err(error) => Err(error.into()),
        }
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let path = self.object_path(key)?;
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::LocalStorageProvider;
    use crate::storage::AttachmentStorage;

    #[tokio::test]
    async fn stores_reads_and_deletes_an_object() {
        let temp_dir = tempfile::tempdir().unwrap();
        let provider = LocalStorageProvider::new(temp_dir.path().to_path_buf());

        provider
            .put("attachments/1/file.txt", b"content".to_vec(), "text/plain")
            .await
            .unwrap();
        assert_eq!(
            to_bytes(
                provider.get("attachments/1/file.txt").await.unwrap().body,
                usize::MAX,
            )
            .await
            .unwrap(),
            b"content".as_slice()
        );

        provider.delete("attachments/1/file.txt").await.unwrap();
        assert!(provider.get("attachments/1/file.txt").await.is_err());
    }

    #[tokio::test]
    async fn rejects_parent_directory_keys() {
        let temp_dir = tempfile::tempdir().unwrap();
        let provider = LocalStorageProvider::new(temp_dir.path().to_path_buf());

        assert!(
            provider
                .put("../outside", b"content".to_vec(), "text/plain")
                .await
                .is_err()
        );
    }
}
