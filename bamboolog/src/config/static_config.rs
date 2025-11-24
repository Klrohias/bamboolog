use anyhow::anyhow;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationConfiguration {
    pub listen_addr: String,
    pub database: String,
    #[serde(default = "get_default_asset_dir", rename = "asset_dir")]
    pub raw_asset_dir: String,

    #[serde(skip)]
    pub asset_dir: PathBuf,
}

fn get_default_asset_dir() -> String {
    ".".to_string()
}

impl ApplicationConfiguration {
    pub fn load() -> Result<Self, anyhow::Error> {
        let config_path: String =
            env::var("CONFIG_LOCATION").unwrap_or_else(|_| "config.toml".into());
        Self::from_path(PathBuf::from(config_path))
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let path_ref = path.as_ref();
        let relative_root = path_ref
            .parent()
            .ok_or_else(|| anyhow!("Failed to get parent of config file"))?;

        let contents = fs::read_to_string(path_ref)?;
        let mut result: ApplicationConfiguration = toml::from_str(&contents)?;
        result.asset_dir = relative_root.join(&result.raw_asset_dir);

        Ok(result)
    }
}
