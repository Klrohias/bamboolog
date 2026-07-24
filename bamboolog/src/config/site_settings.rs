use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub navigation: Vec<SiteNavigationItem>,
    #[serde(default = "default_rss_enabled")]
    pub rss_enabled: bool,
    #[serde(default = "default_sitemap_enabled")]
    pub sitemap_enabled: bool,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: u64,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_posts_per_page() -> u64 {
    10
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
            navigation: Vec::new(),
            rss_enabled: default_rss_enabled(),
            sitemap_enabled: default_sitemap_enabled(),
            posts_per_page: default_posts_per_page(),
        }
    }
}

impl SiteSettings {
    pub fn public_posts_per_page(&self) -> u64 {
        self.posts_per_page.clamp(1, 100)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteNavigationItem {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub target: SiteNavigationTarget,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SiteNavigationTarget {
    #[default]
    Custom,
    Archives,
    Categories,
    Tags,
    Feed,
}

impl SiteNavigationItem {
    pub fn resolved_url(&self) -> &str {
        match self.target {
            SiteNavigationTarget::Custom => &self.url,
            SiteNavigationTarget::Archives => "/archives",
            SiteNavigationTarget::Categories => "/categories",
            SiteNavigationTarget::Tags => "/tags",
            SiteNavigationTarget::Feed => "/index.xml",
        }
    }

    pub fn translation_key(&self) -> Option<&'static str> {
        match self.target {
            SiteNavigationTarget::Custom => None,
            SiteNavigationTarget::Archives => Some("archives"),
            SiteNavigationTarget::Categories => Some("categories"),
            SiteNavigationTarget::Tags => Some("tags"),
            SiteNavigationTarget::Feed => Some("rss_feed"),
        }
    }

    pub fn is_available(&self, rss_enabled: bool) -> bool {
        !matches!(self.target, SiteNavigationTarget::Feed) || rss_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::{SiteNavigationItem, SiteNavigationTarget, SiteSettings};

    #[test]
    fn bounds_the_public_page_size() {
        assert!(SiteSettings::default().rss_enabled);
        assert!(SiteSettings::default().sitemap_enabled);
        assert_eq!(SiteSettings::default().public_posts_per_page(), 10);
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

    #[test]
    fn resolves_system_navigation_targets_without_persisted_urls() {
        let archives = SiteNavigationItem {
            label: String::new(),
            url: "https://example.invalid/ignored".to_string(),
            target: SiteNavigationTarget::Archives,
        };

        assert_eq!(archives.resolved_url(), "/archives");
        assert_eq!(archives.translation_key(), Some("archives"));
    }

    #[test]
    fn deserializes_legacy_navigation_as_a_custom_link() {
        let item: SiteNavigationItem =
            serde_json::from_str(r#"{ "label": "About", "url": "/about" }"#).unwrap();

        assert_eq!(item.target, SiteNavigationTarget::Custom);
        assert_eq!(item.resolved_url(), "/about");
        assert_eq!(item.translation_key(), None);
    }
}
