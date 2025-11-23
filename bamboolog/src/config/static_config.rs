use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationConfiguration {
    pub listen_addr: String,
    pub database: String,
}

impl ApplicationConfiguration {
    pub fn load() -> Result<Self, anyhow::Error> {
        let config_path: String =
            env::var("CONFIG_LOCATION").unwrap_or_else(|_| "config.toml".into());
        Self::from_path(PathBuf::from(config_path))
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let path_ref = path.as_ref();
        let contents = fs::read_to_string(path_ref)?;
        Ok(toml::from_str(&contents)?)
    }
}
