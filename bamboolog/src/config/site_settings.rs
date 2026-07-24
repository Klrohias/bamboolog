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
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: u64,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_posts_per_page() -> u64 {
    10
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
    pub label: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::SiteSettings;

    #[test]
    fn bounds_the_public_page_size() {
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
}
