use std::{env, fs, io, path::PathBuf, sync::Arc};

use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use axum_extra::response::FileStream;
use serde::{Deserialize, Serialize};
use tera::Context;
use tera::Error as TeraError;
use tera::Tera;
use tokio::fs::File;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::instrument;

use crate::utils::FailibleOperationExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeServiceSettings {
    pub current: String,
}

#[derive(Debug)]
pub struct ThemeServiceState {
    current: String,
    renderer: Tera,
}

impl ThemeServiceState {
    #[instrument(skip(self))]
    pub fn load(&mut self, settings: &ThemeServiceSettings) -> Result<(), StateLoadError> {
        let cwd = env::current_dir()?;

        self.current = settings.current.to_owned();

        // Check theme root
        let theme_root = cwd.join(format!("themes/{}", settings.current));
        if !fs::exists(&theme_root)? {
            return Err(StateLoadError::ThemeNotFound(settings.current.to_owned()));
        }

        // Check layouts
        let layouts_root = theme_root.join("layouts");
        if !fs::exists(&layouts_root)? {
            return Err(StateLoadError::BrokenTheme(settings.current.to_owned()));
        }
        self.load_layouts(layouts_root)?;

        // Check static
        let static_root = theme_root.join("static");
        if !fs::exists(&static_root)? {
            return Err(StateLoadError::BrokenTheme(settings.current.to_owned()));
        }

        Ok(())
    }

    fn load_layouts(&mut self, layouts_root: PathBuf) -> Result<(), TeraError> {
        let path = layouts_root.to_string_lossy().to_string();
        self.renderer = Tera::new(&format!("{}/**/*.html", path))?;
        Ok(())
    }
}

impl Default for ThemeServiceState {
    fn default() -> Self {
        Self {
            current: String::new(),
            renderer: Tera::default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StateLoadError {
    #[error("Theme `{0}` not found")]
    ThemeNotFound(String),

    #[error("Theme `{0}` is broken")]
    BrokenTheme(String),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    TeraError(#[from] TeraError),
}

#[derive(Debug, Clone)]
pub struct ThemeService(Arc<RwLock<ThemeServiceState>>);

impl ThemeService {
    pub fn new(state: ThemeServiceState) -> Self {
        Self(Arc::new(RwLock::new(state)))
    }

    pub async fn set_state(&self, state: ThemeServiceState) {
        *self.0.write().await = state
    }

    pub fn get_state(&self) -> Arc<RwLock<ThemeServiceState>> {
        self.0.clone()
    }

    pub async fn render(&self, name: impl AsRef<str>, ctx: &Context) -> Result<String, TeraError> {
        let state = self.0.read().await;
        Ok(state.renderer.render(name.as_ref(), ctx)?)
    }

    #[instrument]
    pub async fn serve_static(&self, path: String) -> Result<Response, StaticServingError> {
        let cwd = env::current_dir()?;

        let state = self.0.read().await;
        let static_root = cwd.join(format!("themes/{}/static", state.current));

        let content_type = mime_guess::from_path(&path)
            .first_raw()
            .unwrap_or_else(|| "application/octet-stream");

        let file = File::open(static_root.join(path))
            .await
            .traced()
            .map_err(|_| StaticServingError::NotFound)?;

        let stream = ReaderStream::new(file);
        Ok((
            [(header::CONTENT_TYPE, content_type)],
            FileStream::new(stream),
        )
            .into_response())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StaticServingError {
    #[error("File not found")]
    NotFound,

    #[error(transparent)]
    IoError(#[from] io::Error),
}
