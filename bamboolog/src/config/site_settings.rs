use serde::{Deserialize, Serialize};

pub const DEFAULT_ATTACHMENT_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_rss_enabled")]
    pub rss_enabled: bool,
    #[serde(default = "default_sitemap_enabled")]
    pub sitemap_enabled: bool,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: u64,
    #[serde(default = "default_attachment_cache_control")]
    pub attachment_cache_control: String,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_posts_per_page() -> u64 {
    10
}

fn default_attachment_cache_control() -> String {
    DEFAULT_ATTACHMENT_CACHE_CONTROL.to_string()
}

fn default_rss_enabled() -> bool {
    true
}

fn default_sitemap_enabled() -> bool {
    true
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            site_name: String::new(),
            base_url: String::new(),
            copyright: String::new(),
            description: String::new(),
            language: default_language(),
            favicon_url: String::new(),
            rss_enabled: default_rss_enabled(),
            sitemap_enabled: default_sitemap_enabled(),
            posts_per_page: default_posts_per_page(),
            attachment_cache_control: default_attachment_cache_control(),
        }
    }
}

impl SiteSettings {
    pub fn public_posts_per_page(&self) -> u64 {
        self.posts_per_page.clamp(1, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_ATTACHMENT_CACHE_CONTROL, SiteSettings};

    #[test]
    fn defaults_attachment_cache_control_for_existing_settings() {
        let settings: SiteSettings = serde_json::from_value(serde_json::json!({
            "site_name": "BambooLog",
            "base_url": "https://example.com",
        }))
        .unwrap();

        assert_eq!(
            settings.attachment_cache_control,
            DEFAULT_ATTACHMENT_CACHE_CONTROL
        );
    }

    #[test]
    fn bounds_the_public_page_size() {
        assert!(SiteSettings::default().rss_enabled);
        assert!(SiteSettings::default().sitemap_enabled);
        assert_eq!(SiteSettings::default().public_posts_per_page(), 10);
        assert_eq!(
            SiteSettings::default().attachment_cache_control,
            DEFAULT_ATTACHMENT_CACHE_CONTROL
        );
        assert_eq!(
            SiteSettings {
                posts_per_page: 0,
                ..Default::default()
            }
            .public_posts_per_page(),
            1
        );
        assert_eq!(
            SiteSettings {
                posts_per_page: 500,
                ..Default::default()
            }
            .public_posts_per_page(),
            100
        );
    }
}
