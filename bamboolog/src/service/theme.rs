use std::{fs, io, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use axum_extra::response::FileStream;
use minijinja::{Environment, Value};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::instrument;

use crate::config::{ApplicationConfiguration, ThemeManifest, config_entries};
use crate::service::reloadable::ReloadableService;
use crate::service::site_settings::SiteSettingsService;
use crate::utils::FailibleOperationExts;

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
struct LoadedTheme {
    renderer_env: Environment<'static>,
    manifest: ThemeManifest,
}

#[derive(Debug, Default)]
pub struct ThemeServiceState {
    current_theme: Option<LoadedTheme>,
    current_settings: ThemeServiceSettings,
}

struct ThemeLoader<'a> {
    application_configuration: &'a Arc<ApplicationConfiguration>,
    theme_service_settings: &'a ThemeServiceSettings,
    site_settings_services: &'a SiteSettingsService,
}

impl<'a> ThemeLoader<'a> {
    fn create_renderer() -> Environment<'static> {
        Environment::new()
    }

    fn load_templates(layouts_root: PathBuf, renderer_env: &mut Environment<'static>) {
        renderer_env.set_loader(move |path| {
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
    }

    pub fn new(
        application_configuration: &'a Arc<ApplicationConfiguration>,
        theme_service_settings: &'a ThemeServiceSettings,
        site_settings_services: &'a SiteSettingsService,
    ) -> Self {
        Self {
            application_configuration,
            theme_service_settings,
            site_settings_services,
        }
    }

    fn get_theme_root(&self) -> Result<PathBuf, ThemeLoadError> {
        let theme_root = self
            .application_configuration
            .asset_dir
            .join(format!("themes/{}", self.theme_service_settings.current));
        if !fs::exists(&theme_root)? {
            return Err(ThemeLoadError::ThemeNotFound(
                self.theme_service_settings.current.to_owned(),
            ));
        }

        return Ok(theme_root);
    }

    fn setup_renderer_features(&self, renderer_env: &mut Environment<'static>) {
        let site_settings_service = self.site_settings_services.to_owned();

        renderer_env.add_filter("fromThemeStatic", move |value: String| -> Value {
            tokio::task::block_in_place(|| {
                Handle::current().block_on(async {
                    let base_url = &site_settings_service.read().await.base_url;
                    return Value::from_safe_string(format!("{}/static/theme/{}", base_url, value));
                })
            })
        });
    }

    pub fn get_manifest(&self) -> Result<ThemeManifest, ThemeLoadError> {
        // Check definition
        let manifest_file = self.get_theme_root()?.join("manifest.toml");
        if !fs::exists(&manifest_file)? {
            return Err(ThemeLoadError::BrokenTheme(
                self.theme_service_settings.current.to_owned(),
            ));
        }
        let manifest = toml::from_str(&fs::read_to_string(manifest_file)?)?;

        return Ok(manifest);
    }

    pub fn get_renderer_env(&self) -> Result<Environment<'static>, ThemeLoadError> {
        let theme_root = self.get_theme_root()?;
        let layouts_root = theme_root.join("layouts");
        let mut result = Self::create_renderer();

        // Load templates
        Self::load_templates(layouts_root, &mut result);

        // Setup filters
        self.setup_renderer_features(&mut result);

        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeLoadError {
    #[error("Theme `{0}` not found")]
    ThemeNotFound(String),

    #[error("Theme `{0}` is broken")]
    BrokenTheme(String),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
}

#[derive(Debug, Clone)]
pub struct ThemeService {
    state: Arc<RwLock<ThemeServiceState>>,
    dep_db: DatabaseConnection,
    dep_app_cfg: Arc<ApplicationConfiguration>,
    dep_site_settings: SiteSettingsService,
}

impl ThemeService {
    pub fn new(
        db: DatabaseConnection,
        app_cfg: Arc<ApplicationConfiguration>,
        site_settings: SiteSettingsService,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(ThemeServiceState::default())),
            dep_db: db,
            dep_app_cfg: app_cfg,
            dep_site_settings: site_settings,
        }
    }

    pub async fn list_themes(&self) -> Result<Vec<String>, io::Error> {
        let theme_root = self.dep_app_cfg.asset_dir.join("themes");
        let mut themes = Vec::new();

        if let Ok(entries) = fs::read_dir(theme_root) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.file_type()?.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            themes.push(name.to_owned());
                        }
                    }
                }
            }
        }

        Ok(themes)
    }

    pub async fn render(
        &self,
        name: impl AsRef<str>,
        ctx: impl Serialize,
    ) -> Result<String, ThemeRenderError> {
        let state = self.state.read().await;
        let loaded_theme = state
            .current_theme
            .as_ref()
            .ok_or_else(|| ThemeRenderError::NoTheme(state.current_settings.current.clone()))?;

        let mapped_file = loaded_theme.manifest.map_layout_file(name.as_ref());
        let template = loaded_theme.renderer_env.get_template(&mapped_file)?;

        Ok(template.render(ctx)?)
    }

    #[instrument]
    pub async fn serve_static(&self, path: String) -> Result<Response, StaticServingError> {
        // Check if the theme is loaded
        let state = self.state.read().await;
        if state.current_theme.is_none() {
            return Err(StaticServingError::NoTheme(
                state.current_settings.current.clone(),
            ));
        }

        // Serve static file
        let static_root = self
            .dep_app_cfg
            .asset_dir
            .join(format!("themes/{}/static", state.current_settings.current));

        let content_type = mime_guess::from_path(&path).first_or_octet_stream();

        let file = File::open(static_root.join(path))
            .await
            .traced(|e| tracing::error!("{}", e))
            .map_err(|_| StaticServingError::NotFound)?;

        let stream = ReaderStream::new(file);
        Ok((
            [(header::CONTENT_TYPE, content_type.essence_str())],
            FileStream::new(stream),
        )
            .into_response())
    }
}

#[async_trait]
impl ReloadableService for ThemeService {
    async fn reload(&self) {
        let settings = match config_entries::THEME_SERVICE_SETTINGS
            .get::<ThemeServiceSettings>(&self.dep_db)
            .await
        {
            Err(e) => {
                tracing::warn!(
                    "Failed to load settings for theme service, and will use a default settings. Error: {}",
                    e
                );

                ThemeServiceSettings::default()
            }
            Ok(None) => {
                tracing::warn!(
                    "No settings present for theme service, and will use a default settings."
                );
                ThemeServiceSettings::default()
            }
            Ok(Some(v)) => v,
        };

        let loader = ThemeLoader::new(&self.dep_app_cfg, &settings, &self.dep_site_settings);
        let manifest = match loader.get_manifest() {
            Err(e) => {
                tracing::error!("Failed to load theme manifest: {e}");
                return;
            }
            Ok(v) => v,
        };
        let renderer_env = match loader.get_renderer_env() {
            Err(e) => {
                tracing::error!("Failed to load theme renderer: {e}");
                return;
            }
            Ok(v) => v,
        };

        {
            let mut state = self.state.write().await;
            let loaded_theme = LoadedTheme {
                manifest,
                renderer_env,
            };
            state.current_settings = settings;
            state.current_theme = Some(loaded_theme);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StaticServingError {
    #[error("Theme `{0}` not found")]
    NoTheme(String),

    #[error("File not found")]
    NotFound,

    #[error(transparent)]
    IoError(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeRenderError {
    #[error("Theme `{0}` not found")]
    NoTheme(String),

    #[error(transparent)]
    JinjaError(#[from] minijinja::Error),
}
