use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub site_name: String,
    pub base_url: String,
}
