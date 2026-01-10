use std::borrow::Borrow;
use std::collections::HashMap;
use std::{fs, io, path::PathBuf, sync::Arc};

use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use axum_extra::response::FileStream;
use minijinja::{Environment, Value};
use serde::{Deserialize, Serialize};
use tera::Error as TeraError;
use tokio::fs::File;
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::instrument;

use crate::config::ApplicationConfiguration;
use crate::service::site_settings::SiteSettingsService;
use crate::utils::FailibleOperationExt;

#[derive(Debug, Default, Deserialize)]
pub struct ThemeManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub author: Option<String>,
    #[serde(default)]
    pub layout_mapping: HashMap<String, String>,
}

impl ThemeManifest {
    pub fn map_layout_file(&self, name: impl Borrow<str>) -> String {
        match self.layout_mapping.get(name.borrow()) {
            Some(value) => value.to_owned(),
            None => name.borrow().to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeServiceSettings {
    pub current: String,
}

impl Default for ThemeServiceSettings {
    fn default() -> Self {
        Self {
            current: "default".to_string(),
        }
    }
}

#[derive(Debug)]
struct ThemeState {
    name: String,
    manifest: ThemeManifest,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            manifest: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct ThemeServiceState {
    current: ThemeState,
    renderer: Environment<'static>,
    application_configuration: Arc<ApplicationConfiguration>,
    site_settings_service: SiteSettingsService,
}

impl ThemeServiceState {
    pub fn new(
        application_configuration: Arc<ApplicationConfiguration>,
        site_settings_service: SiteSettingsService,
    ) -> Self {
        Self {
            current: ThemeState::default(),
            application_configuration,
            site_settings_service,
            renderer: Self::create_renderer(),
        }
    }

    fn create_renderer() -> Environment<'static> {
        Environment::new()
    }

    #[instrument(skip(self))]
    pub fn load_settings(&mut self, settings: &ThemeServiceSettings) -> Result<(), StateLoadError> {
        self.renderer = Self::create_renderer();
        self.load_renderer();
        self.load_theme(settings)?;
        Ok(())
    }

    fn load_renderer(&mut self) {
        self.renderer = Self::create_renderer();
        let site_settings_service = self.site_settings_service.clone();
        self.renderer
            .add_filter("themeStatic", move |value: String| -> Value {
                tokio::task::block_in_place(|| {
                    Handle::current().block_on(async {
                        let base_url = &site_settings_service.read().await.base_url;
                        return Value::from_safe_string(format!(
                            "{}/static/theme/{}",
                            base_url, value
                        ));
                    })
                })
            });
    }

    fn load_theme(&mut self, settings: &ThemeServiceSettings) -> Result<(), StateLoadError> {
        // Check theme root
        let theme_root = self
            .application_configuration
            .asset_dir
            .join(format!("themes/{}", settings.current));
        if !fs::exists(&theme_root)? {
            return Err(StateLoadError::ThemeNotFound(settings.current.to_owned()));
        }

        // Check definition
        let manifest_file = theme_root.join("manifest.toml");
        if !fs::exists(&manifest_file)? {
            return Err(StateLoadError::BrokenTheme(settings.current.to_owned()));
        }
        let manifest = toml::from_str(&fs::read_to_string(manifest_file)?)?;

        // Check layouts
        let layouts_root = theme_root.join("layouts");
        if !fs::exists(&layouts_root)? {
            return Err(StateLoadError::BrokenTheme(settings.current.to_owned()));
        }
        Self::load_layouts(layouts_root, &mut self.renderer)?;

        // Check static
        let static_root = theme_root.join("static");
        if !fs::exists(&static_root)? {
            return Err(StateLoadError::BrokenTheme(settings.current.to_owned()));
        }

        self.current = ThemeState {
            name: settings.current.to_owned(),
            manifest,
        };

        Ok(())
    }

    fn load_layouts(
        layouts_root: PathBuf,
        renderer: &mut Environment<'_>,
    ) -> Result<(), TeraError> {
        renderer.set_loader(move |path| {
            let path = layouts_root.join(path);
            if !path.starts_with(&layouts_root) {
                return Ok(None);
            }

            let content = match fs::read_to_string(&path) {
                Err(e) => {
                    tracing::warn!("Cannot read template {:?}: {}", path, e);

                    None
                }
                Ok(v) => Some(v),
            };

            Ok(content)
        });
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

    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
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
        ctx: impl Serialize,
    ) -> Result<String, ThemeRenderError> {
        let state = self.0.read().await;
        let loaded_theme = &state.current;
        let mapped_file = loaded_theme.manifest.map_layout_file(name.as_ref());
        let template = state.renderer.get_template(&mapped_file)?;

        Ok(template.render(ctx)?)
    }

    #[instrument]
    pub async fn serve_static(&self, path: String) -> Result<Response, StaticServingError> {
        let state = self.0.read().await;
        let static_root = state
            .application_configuration
            .asset_dir
            .join(format!("themes/{}/static", state.current.name));

        let content_type = mime_guess::from_path(&path)
            .first_raw()
            .unwrap_or_else(|| "application/octet-stream");

        let file = File::open(static_root.join(path))
            .await
            .traced(|e| tracing::error!("{}", e))
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

#[derive(Debug, thiserror::Error)]
pub enum ThemeRenderError {
    #[error(transparent)]
    TeraError(#[from] TeraError),

    #[error(transparent)]
    JinjaError(#[from] minijinja::Error),
}
