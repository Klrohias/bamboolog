use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteSettings {
    pub site_name: String,
    pub base_url: String,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            site_name: String::default(),
            base_url: String::default(),
        }
    }
}
