use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use axum_extra::response::FileStream;
use minijinja::{AutoEscape, Environment, Value};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use tokio::fs::File;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::instrument;

use crate::config::{
    ApplicationConfiguration, ThemeConfigError, ThemeConfigField, ThemeManifest, config_entries,
};
use crate::service::reloadable::ReloadableService;
use crate::service::site_settings::SiteSettingsService;
use crate::utils::FailibleOperationExts;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeServiceSettings {
    pub current: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeDetails {
    pub id: String,
    pub active: bool,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub author: Option<String>,
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
    id: String,
    renderer_env: Environment<'static>,
    manifest: ThemeManifest,
    config: JsonMap<String, JsonValue>,
    translations: JsonMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeConfiguration {
    pub theme: ThemeDetails,
    pub schema: Vec<ThemeConfigField>,
    pub values: JsonMap<String, JsonValue>,
}

#[derive(Debug, Default)]
pub struct ThemeServiceState {
    current_theme: Option<LoadedTheme>,
    current_settings: ThemeServiceSettings,
}

struct ThemeLoader<'a> {
    application_configuration: &'a Arc<ApplicationConfiguration>,
    theme_service_settings: &'a ThemeServiceSettings,
    base_url: String,
}

impl<'a> ThemeLoader<'a> {
    fn create_renderer() -> Environment<'static> {
        let mut environment = Environment::new();
        environment.set_auto_escape_callback(|name| {
            if name.ends_with(".html") || name.ends_with(".xml") {
                AutoEscape::Html
            } else {
                AutoEscape::None
            }
        });
        environment
    }

    fn load_templates(layouts_root: PathBuf, renderer_env: &mut Environment<'static>) {
        renderer_env.set_loader(move |path| {
            if !is_safe_relative_path(path) {
                return Ok(None);
            }
            let path = layouts_root.join(path);
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
        base_url: String,
    ) -> Self {
        Self {
            application_configuration,
            theme_service_settings,
            base_url,
        }
    }

    fn get_theme_root(&self) -> Result<PathBuf, ThemeError> {
        let theme_root = self
            .application_configuration
            .asset_dir
            .join(format!("themes/{}", self.theme_service_settings.current));
        if !fs::exists(&theme_root)? {
            return Err(ThemeError::NoTheme(
                self.theme_service_settings.current.to_owned(),
            ));
        }

        Ok(theme_root)
    }

    fn setup_renderer_features(&self, renderer_env: &mut Environment<'static>) {
        let base_url = self.base_url.clone();

        renderer_env.add_filter("theme_static", move |value: String| -> Value {
            Value::from_safe_string(theme_static_url(&base_url, &value))
        });
        let base_url = self.base_url.clone();
        renderer_env.add_filter("absolute_url", move |value: String| -> Value {
            Value::from_safe_string(absolute_url(&base_url, &value))
        });
        renderer_env.add_filter("format_date", format_date);
        renderer_env.add_filter("format_rfc2822", format_rfc2822);
        renderer_env.add_filter("urlencode", url_encode);
    }

    pub fn get_manifest(&self) -> Result<ThemeManifest, ThemeError> {
        // Check definition
        let manifest_file = self.get_theme_root()?.join("manifest.toml");
        if !fs::exists(&manifest_file)? {
            return Err(ThemeError::BrokenTheme(
                self.theme_service_settings.current.to_owned(),
            ));
        }
        let manifest: ThemeManifest = toml::from_str(&fs::read_to_string(manifest_file)?)?;
        manifest.validate_config_schema()?;

        Ok(manifest)
    }

    pub fn get_renderer_env(&self) -> Result<Environment<'static>, ThemeError> {
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

fn is_safe_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
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
            for entry in entries.flatten() {
                if entry.file_type()?.is_dir()
                    && let Some(name) = entry.file_name().to_str()
                {
                    themes.push(name.to_owned());
                }
            }
        }

        Ok(themes)
    }

    pub async fn list_theme_details(&self) -> Result<Vec<ThemeDetails>, io::Error> {
        let current = self.state.read().await.current_settings.current.clone();
        let theme_root = self.dep_app_cfg.asset_dir.join("themes");
        let mut themes = Vec::new();

        if let Ok(entries) = fs::read_dir(theme_root) {
            for entry in entries.flatten() {
                if !entry.file_type()?.is_dir() {
                    continue;
                }
                let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                let manifest_path = entry.path().join("manifest.toml");
                let manifest = match fs::read_to_string(&manifest_path)
                    .ok()
                    .and_then(|contents| toml::from_str::<ThemeManifest>(&contents).ok())
                {
                    Some(manifest) => manifest,
                    None => {
                        tracing::warn!("Skipping theme with an invalid manifest: {}", id);
                        continue;
                    }
                };
                if let Err(error) = manifest.validate_config_schema() {
                    tracing::warn!(
                        "Skipping theme with an invalid configuration schema: {id}: {error}"
                    );
                    continue;
                }

                themes.push(ThemeDetails {
                    active: id == current,
                    id,
                    name: manifest.name,
                    version: manifest.version,
                    description: manifest.description,
                    homepage: manifest.homepage,
                    author: manifest.author,
                });
            }
        }
        themes.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(themes)
    }

    pub async fn active_theme_configuration(&self) -> Result<ThemeConfiguration, ThemeError> {
        let state = self.state.read().await;
        let loaded_theme = state
            .current_theme
            .as_ref()
            .ok_or_else(|| ThemeError::NoTheme(state.current_settings.current.clone()))?;

        Ok(ThemeConfiguration {
            theme: ThemeDetails {
                id: loaded_theme.id.clone(),
                active: true,
                name: loaded_theme.manifest.name.clone(),
                version: loaded_theme.manifest.version.clone(),
                description: loaded_theme.manifest.description.clone(),
                homepage: loaded_theme.manifest.homepage.clone(),
                author: loaded_theme.manifest.author.clone(),
            },
            schema: loaded_theme.manifest.config.clone(),
            values: loaded_theme.config.clone(),
        })
    }

    pub async fn update_active_theme_config(
        &self,
        values: JsonMap<String, JsonValue>,
    ) -> Result<ThemeConfiguration, ThemeError> {
        let (id, manifest) = {
            let state = self.state.read().await;
            let loaded_theme = state
                .current_theme
                .as_ref()
                .ok_or_else(|| ThemeError::NoTheme(state.current_settings.current.clone()))?;
            (loaded_theme.id.clone(), loaded_theme.manifest.clone())
        };
        let values = manifest.resolve_config(&values, true)?;
        write_theme_config_file(
            &self.dep_app_cfg.asset_dir.join("themes").join(&id),
            &values,
        )?;

        Ok(ThemeConfiguration {
            theme: ThemeDetails {
                id,
                active: true,
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                description: manifest.description.clone(),
                homepage: manifest.homepage.clone(),
                author: manifest.author.clone(),
            },
            schema: manifest.config,
            values,
        })
    }

    pub async fn render(
        &self,
        name: impl AsRef<str>,
        ctx: impl Serialize,
    ) -> Result<String, ThemeError> {
        let state = self.state.read().await;
        let loaded_theme = state
            .current_theme
            .as_ref()
            .ok_or_else(|| ThemeError::NoTheme(state.current_settings.current.clone()))?;

        let mapped_file = loaded_theme.manifest.map_layout_file(name.as_ref());
        let template = loaded_theme.renderer_env.get_template(&mapped_file)?;

        let mut ctx = serde_json::to_value(ctx)?;
        let context = ctx
            .as_object_mut()
            .ok_or(ThemeError::InvalidRenderContext)?;
        context.insert(
            "theme".to_string(),
            json!({
                "id": loaded_theme.id,
                "config": loaded_theme.config,
            }),
        );
        let language = context
            .get("site")
            .and_then(JsonValue::as_object)
            .and_then(|site| site.get("language"))
            .and_then(JsonValue::as_str)
            .unwrap_or("en");
        context.insert(
            "i18n".to_string(),
            select_translation(&loaded_theme.translations, language),
        );

        Ok(template.render(ctx)?)
    }

    #[instrument]
    pub async fn serve_static(&self, path: String) -> Result<Response, ThemeError> {
        // Check if the theme is loaded
        let state = self.state.read().await;
        if state.current_theme.is_none() {
            return Err(ThemeError::NoTheme(state.current_settings.current.clone()));
        }

        // Serve static file
        let static_root = self
            .dep_app_cfg
            .asset_dir
            .join(format!("themes/{}/static", state.current_settings.current));

        let content_type = mime_guess::from_path(&path).first_or_octet_stream();

        let path = Path::new(&path);
        if path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ThemeError::NotFound);
        }

        let file = File::open(static_root.join(path))
            .await
            .traced(|e| tracing::error!("{}", e))
            .map_err(|_| ThemeError::NotFound)?;

        let stream = ReaderStream::new(file);
        Ok((
            [(header::CONTENT_TYPE, content_type.essence_str())],
            FileStream::new(stream),
        )
            .into_response())
    }
}

fn theme_static_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/static/theme/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn absolute_url(base_url: &str, value: &str) -> String {
    if value.starts_with("http://") || value.starts_with("https://") || value.starts_with("//") {
        return value.to_string();
    }

    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        value.trim_start_matches('/')
    )
}

fn format_date(value: String, format: Option<String>) -> String {
    let format = format.as_deref().unwrap_or("%Y-%m-%d");
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|date| date.format(format).to_string())
        .unwrap_or(value)
}

fn format_rfc2822(value: String) -> String {
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|date| date.to_rfc2822())
        .unwrap_or(value)
}

fn url_encode(value: String) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn read_theme_config_file(theme_root: &Path) -> Result<JsonMap<String, JsonValue>, ThemeError> {
    let path = theme_root.join("config.json");
    if !path.exists() {
        return Ok(JsonMap::new());
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn read_theme_translations(theme_root: &Path) -> JsonMap<String, JsonValue> {
    let mut translations = JsonMap::new();
    let directory = theme_root.join("i18n");
    let Ok(entries) = fs::read_dir(directory) else {
        return translations;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Some(language) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Ok(contents) = fs::read_to_string(&path) else {
            tracing::warn!(?path, "Cannot read theme translation file");
            continue;
        };
        let Ok(value) = serde_json::from_str::<JsonValue>(&contents) else {
            tracing::warn!(?path, "Cannot parse theme translation file");
            continue;
        };
        if value.is_object() {
            translations.insert(language.replace('_', "-").to_ascii_lowercase(), value);
        }
    }

    translations
}

fn select_translation(translations: &JsonMap<String, JsonValue>, language: &str) -> JsonValue {
    let language = language.replace('_', "-").to_ascii_lowercase();
    let primary_language = language.split('-').next().unwrap_or(&language);
    translations
        .get(&language)
        .or_else(|| translations.get(primary_language))
        .or_else(|| translations.get("en"))
        .cloned()
        .unwrap_or_else(|| JsonValue::Object(JsonMap::new()))
}

fn write_theme_config_file(
    theme_root: &Path,
    values: &JsonMap<String, JsonValue>,
) -> Result<(), ThemeError> {
    let path = theme_root.join("config.json");
    let temporary_path = theme_root.join("config.json.tmp");
    fs::write(&temporary_path, serde_json::to_string_pretty(values)?)?;
    fs::rename(temporary_path, path)?;
    Ok(())
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

        let base_url = self.dep_site_settings.read().await.base_url.clone();
        let loader = ThemeLoader::new(&self.dep_app_cfg, &settings, base_url);
        let manifest = match loader.get_manifest() {
            Err(e) => {
                tracing::error!("Failed to load theme manifest: {e}");
                return;
            }
            Ok(v) => v,
        };
        let theme_root = self
            .dep_app_cfg
            .asset_dir
            .join("themes")
            .join(&settings.current);
        let stored_config = match read_theme_config_file(&theme_root) {
            Ok(values) => values,
            Err(error) => {
                tracing::error!("Failed to load theme configuration: {error}");
                return;
            }
        };
        let config = match manifest.resolve_config(&stored_config, false) {
            Ok(config) => config,
            Err(error) => {
                tracing::error!("Failed to validate theme configuration: {error}");
                return;
            }
        };
        let translations = read_theme_translations(&theme_root);
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
                id: settings.current.clone(),
                manifest,
                renderer_env,
                config,
                translations,
            };
            state.current_settings = settings;
            state.current_theme = Some(loaded_theme);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("Theme `{0}` not found")]
    NoTheme(String),

    #[error("File not found")]
    NotFound,

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    JinjaError(#[from] minijinja::Error),

    #[error("Theme `{0}` is broken")]
    BrokenTheme(String),

    #[error(transparent)]
    TomlError(#[from] toml::de::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ThemeConfigError(#[from] ThemeConfigError),

    #[error("Theme render contexts must be JSON objects")]
    InvalidRenderContext,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sea_orm::Database;
    use serde_json::json;

    use crate::config::ApplicationConfiguration;
    use crate::service::site_settings::SiteSettingsService;

    use super::{
        ThemeLoader, ThemeService, ThemeServiceSettings, absolute_url, format_date, format_rfc2822,
        is_safe_relative_path, read_theme_config_file, read_theme_translations, select_translation,
        theme_static_url, url_encode, write_theme_config_file,
    };

    #[test]
    fn builds_static_urls_without_duplicate_slashes() {
        assert_eq!(
            theme_static_url("https://example.test/", "/css/journal.css"),
            "https://example.test/static/theme/css/journal.css"
        );
    }

    #[test]
    fn resolves_relative_urls_without_rewriting_external_urls() {
        assert_eq!(
            absolute_url("https://example.test/", "/posts/first"),
            "https://example.test/posts/first"
        );
        assert_eq!(
            absolute_url("https://example.test/", "https://cdn.example/image.png"),
            "https://cdn.example/image.png"
        );
    }

    #[test]
    fn formats_rfc3339_dates_and_preserves_unknown_values() {
        assert_eq!(
            format_date(
                "2026-07-25T10:20:30+00:00".to_string(),
                Some("%Y".to_string())
            ),
            "2026"
        );
        assert_eq!(format_date("not-a-date".to_string(), None), "not-a-date");
        assert_eq!(
            format_rfc2822("2026-07-25T10:20:30+00:00".to_string()),
            "Sat, 25 Jul 2026 10:20:30 +0000"
        );
    }

    #[test]
    fn url_encodes_unicode_and_reserved_characters() {
        assert_eq!(
            url_encode("Rust & 中文".to_string()),
            "Rust%20%26%20%E4%B8%AD%E6%96%87"
        );
    }

    #[test]
    fn html_templates_escape_untrusted_context_values() {
        let mut environment = ThemeLoader::create_renderer();
        environment
            .add_template("test.html", "{{ title }}")
            .unwrap();

        assert_eq!(
            environment
                .get_template("test.html")
                .unwrap()
                .render(json!({ "title": "<script>" }))
                .unwrap(),
            "&lt;script&gt;"
        );
    }

    #[test]
    fn xml_templates_escape_untrusted_context_values() {
        let mut environment = ThemeLoader::create_renderer();
        environment
            .add_template("test.xml", "<title>{{ title }}</title>")
            .unwrap();

        assert_eq!(
            environment
                .get_template("test.xml")
                .unwrap()
                .render(json!({ "title": "<script>" }))
                .unwrap(),
            "<title>&lt;script&gt;</title>"
        );
    }

    #[test]
    fn writes_and_reads_theme_config_json() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let values = serde_json::from_value(json!({
            "subtitle": "Personal notes",
            "show_reading_time": false,
            "posts_per_page": 12
        }))
        .unwrap();

        write_theme_config_file(temporary_directory.path(), &values).unwrap();

        assert_eq!(
            read_theme_config_file(temporary_directory.path()).unwrap(),
            values
        );
        assert!(temporary_directory.path().join("config.json").is_file());
    }

    #[test]
    fn loads_translations_from_the_active_theme_directory() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let translations_directory = temporary_directory.path().join("i18n");
        std::fs::create_dir(&translations_directory).unwrap();
        std::fs::write(
            translations_directory.join("en.json"),
            r#"{ "archives": "Archives" }"#,
        )
        .unwrap();
        std::fs::write(
            translations_directory.join("zh-Hant.json"),
            r#"{ "archives": "封存" }"#,
        )
        .unwrap();
        let translations = read_theme_translations(temporary_directory.path());
        assert_eq!(
            select_translation(&translations, "zh-Hant")["archives"],
            "封存"
        );
        assert_eq!(
            select_translation(&translations, "zh_Hant")["archives"],
            "封存"
        );
        assert_eq!(
            select_translation(&translations, "unknown")["archives"],
            "Archives"
        );
    }

    #[test]
    fn rejects_template_paths_outside_the_layout_directory() {
        assert!(is_safe_relative_path("partials/navigation.html"));
        assert!(!is_safe_relative_path("../secret.html"));
        assert!(!is_safe_relative_path("/etc/passwd"));
        assert!(!is_safe_relative_path(""));
    }

    #[tokio::test]
    async fn lists_manifest_details_and_marks_the_active_theme() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let asset_dir = temporary_directory.path().to_path_buf();
        let themes_directory = asset_dir.join("themes");
        let journal_directory = themes_directory.join("journal");
        std::fs::create_dir_all(&journal_directory).unwrap();
        std::fs::write(
            journal_directory.join("manifest.toml"),
            "name = 'Journal'\nversion = '0.1.0'\n",
        )
        .unwrap();
        let notes_directory = themes_directory.join("notes");
        std::fs::create_dir_all(&notes_directory).unwrap();
        std::fs::write(notes_directory.join("manifest.toml"), "name = 'Notes'\n").unwrap();
        let config = Arc::new(ApplicationConfiguration {
            listen_addr: "127.0.0.1:0".to_string(),
            database: "sqlite::memory:".to_string(),
            raw_asset_dir: asset_dir.to_string_lossy().to_string(),
            asset_dir,
        });
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let service =
            ThemeService::new(database.clone(), config, SiteSettingsService::new(database));
        service.state.write().await.current_settings = ThemeServiceSettings {
            current: "journal".to_string(),
        };

        let themes = service.list_theme_details().await.unwrap();
        let journal = themes.iter().find(|theme| theme.id == "journal").unwrap();

        assert!(journal.active);
        assert_eq!(journal.name.as_deref(), Some("Journal"));
        assert_eq!(journal.version.as_deref(), Some("0.1.0"));
    }
}
