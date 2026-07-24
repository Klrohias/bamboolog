use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SiteSettings {
    pub site_name: String,
    pub base_url: String,
    #[serde(default)]
    pub copyright: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub favicon_url: String,
    #[serde(default)]
    pub manifest_url: String,
    #[serde(default)]
    pub navigation: Vec<SiteNavigationItem>,
    #[serde(default)]
    pub comments: CommentSettings,
    #[serde(default)]
    pub search: SearchSettings,
    #[serde(default)]
    pub analytics: AnalyticsSettings,
    /// Trusted markup inserted near the end of the public document head.
    #[serde(default)]
    pub head_html: String,
}

fn default_language() -> String {
    "en".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteNavigationItem {
    pub label: String,
    pub url: String,
}

/// Public configuration for an external comment provider.
///
/// The values are rendered on public pages, so this must not contain secrets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommentSettings {
    /// `disabled`, `disqus`, `utterances`, or `giscus` for the bundled Journal theme.
    #[serde(default = "default_comment_provider")]
    pub provider: String,
    /// Provider-specific public settings, such as a Giscus repository or category ID.
    #[serde(default)]
    pub config: BTreeMap<String, String>,
}

fn default_comment_provider() -> String {
    "disabled".to_string()
}

impl CommentSettings {
    pub fn is_configured(&self) -> bool {
        let has = |key: &str| self.config.get(key).is_some_and(|value| !value.trim().is_empty());
        match self.provider.trim().to_ascii_lowercase().as_str() {
            "disqus" => has("shortname"),
            "utterances" => has("repo"),
            "giscus" => has("repo") && has("repo_id") && has("category") && has("category_id"),
            "livere" => has("uid"),
            "twikoo" => has("env_id"),
            "waline" => has("server_url"),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchSettings {
    /// Google Programmable Search Engine ID. This is public and only rendered on the home page.
    #[serde(default)]
    pub google_cse_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsSettings {
    /// Google Analytics measurement ID, for example `G-XXXXXXXXXX`.
    #[serde(default)]
    pub google_analytics_id: String,
    /// Microsoft Clarity project ID.
    #[serde(default)]
    pub clarity_project_id: String,
    /// Cloudflare Web Analytics beacon token.
    #[serde(default)]
    pub cloudflare_beacon_token: String,
}
