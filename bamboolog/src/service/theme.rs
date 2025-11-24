use std::{fs, io, path::PathBuf, sync::Arc};

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

use crate::config::{ApplicationConfiguration, ThemeDefinition};
use crate::utils::FailibleOperationExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeServiceSettings {
    pub current: String,
}

#[derive(Debug)]
struct LoadedTheme {
    name: String,
    definition: ThemeDefinition,
    renderer: Tera,
}

#[derive(Debug)]
pub struct ThemeServiceState {
    current: Option<LoadedTheme>,
    application_configuration: Arc<ApplicationConfiguration>,
}

impl ThemeServiceState {
    pub fn new(application_configuration: Arc<ApplicationConfiguration>) -> Self {
        Self {
            current: None,
            application_configuration,
        }
    }

    #[instrument(skip(self))]
    pub fn load_theme(&mut self, settings: &ThemeServiceSettings) -> Result<(), StateLoadError> {
        let base_dir = &self.application_configuration.asset_dir;

        let mut loaded_theme = LoadedTheme {
            name: settings.current.to_owned(),
            definition: ThemeDefinition::default(),
            renderer: Tera::default(),
        };

        // Check theme root
        let theme_root = base_dir.join(format!("themes/{}", settings.current));
        if !fs::exists(&theme_root)? {
            return Err(StateLoadError::ThemeNotFound(settings.current.to_owned()));
        }

        // Check layouts
        let layouts_root = theme_root.join("layouts");
        if !fs::exists(&layouts_root)? {
            return Err(StateLoadError::BrokenTheme(settings.current.to_owned()));
        }
        Self::load_layouts(&mut loaded_theme, layouts_root)?;

        // Check static
        let static_root = theme_root.join("static");
        if !fs::exists(&static_root)? {
            return Err(StateLoadError::BrokenTheme(settings.current.to_owned()));
        }

        self.current = Some(loaded_theme);
        Ok(())
    }

    fn load_layouts(
        loaded_theme: &mut LoadedTheme,
        layouts_root: PathBuf,
    ) -> Result<(), TeraError> {
        let path = layouts_root.to_string_lossy().to_string();
        loaded_theme.renderer = Tera::new(&format!("{}/**/*.html", path))?;
        Ok(())
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

    pub async fn render(
        &self,
        name: impl AsRef<str>,
        ctx: &Context,
    ) -> Result<String, ThemeRenderError> {
        let state = self.0.read().await;
        let loaded_theme = state
            .current
            .as_ref()
            .ok_or(ThemeRenderError::NoLoadedTheme)?;
        let mapped_file = loaded_theme.definition.map_layout_file(name.as_ref());

        Ok(loaded_theme.renderer.render(&mapped_file, ctx)?)
    }

    #[instrument]
    pub async fn serve_static(&self, path: String) -> Result<Response, StaticServingError> {
        let state = self.0.read().await;
        let static_root = state.application_configuration.asset_dir.join(format!(
            "themes/{}/static",
            state
                .current
                .as_ref()
                .ok_or(StaticServingError::NoLoadedTheme)?
                .name
        ));

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

    #[error("No theme loaded")]
    NoLoadedTheme,
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeRenderError {
    #[error(transparent)]
    TeraError(#[from] TeraError),

    #[error("No theme loaded")]
    NoLoadedTheme,
}
