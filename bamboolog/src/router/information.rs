use std::fmt::Write;

use axum::{
    Extension, Router,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::{
    config::SiteSettings,
    entity::post::{Column as PostColumn, Entity as PostEntity, Model as Post},
    service::site_settings::SiteSettingsService,
    utils::HttpFailibleOperationExts,
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/index.xml", get(display_rss))
        .route("/sitemap.xml", get(display_sitemap))
}

async fn display_rss(
    Extension(database): Extension<DatabaseConnection>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Response, Response> {
    let site = site_settings.read().await.clone();
    if !site.rss_enabled {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    let posts = visible_posts(&database).await?;

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        render_rss(&site, &posts),
    )
        .into_response())
}

async fn display_sitemap(
    Extension(database): Extension<DatabaseConnection>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Response, Response> {
    let site = site_settings.read().await.clone();
    if !site.sitemap_enabled {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    let posts = visible_posts(&database).await?;

    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        render_sitemap(&site, &posts),
    )
        .into_response())
}

async fn visible_posts(database: &DatabaseConnection) -> Result<Vec<Post>, Response> {
    PostEntity::find()
        .filter(
            Condition::any()
                .add(PostColumn::Hidden.eq(false))
                .add(PostColumn::Hidden.is_null()),
        )
        .order_by_desc(PostColumn::CreatedAt)
        .all(database)
        .await
        .traced_and_response(|error| tracing::error!("{error}"))
}

fn render_rss(site: &SiteSettings, posts: &[Post]) -> String {
    let mut result = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom"><channel>"#,
    );
    let channel_url = site_url(site, "/");
    let feed_url = site_url(site, "/index.xml");
    write!(
        result,
        "<title>{}</title><link>{}</link><description>{}</description><language>{}</language><atom:link href=\"{}\" rel=\"self\" type=\"application/rss+xml\" />",
        xml_escape(&site.site_name),
        xml_escape(&channel_url),
        xml_escape(&site.description),
        xml_escape(&site.language),
        xml_escape(&feed_url),
    )
    .expect("writing to a String cannot fail");

    for post in posts {
        let url = site_url(site, &post_url(post));
        let description = post
            .description
            .as_deref()
            .filter(|description| !description.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| excerpt(&post.content, 240));
        write!(
            result,
            "<item><title>{}</title><link>{}</link><guid isPermaLink=\"true\">{}</guid><pubDate>{}</pubDate><description>{}</description></item>",
            xml_escape(&post.title),
            xml_escape(&url),
            xml_escape(&url),
            xml_escape(&post.created_at.to_rfc2822()),
            xml_escape(&description),
        )
        .expect("writing to a String cannot fail");
    }

    result.push_str("</channel></rss>");
    result
}

fn render_sitemap(site: &SiteSettings, posts: &[Post]) -> String {
    let mut result = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for path in ["/", "/archives", "/categories", "/tags"] {
        write!(
            result,
            "<url><loc>{}</loc></url>",
            xml_escape(&site_url(site, path))
        )
        .expect("writing to a String cannot fail");
    }
    for post in posts {
        let url = site_url(site, &post_url(post));
        let last_modified = post
            .updated_at
            .as_ref()
            .unwrap_or(&post.created_at)
            .to_rfc3339();
        write!(
            result,
            "<url><loc>{}</loc><lastmod>{}</lastmod></url>",
            xml_escape(&url),
            xml_escape(&last_modified),
        )
        .expect("writing to a String cannot fail");
    }
    result.push_str("</urlset>");
    result
}

fn post_url(post: &Post) -> String {
    format!("/posts/{}", post.name)
}

fn site_url(site: &SiteSettings, path: &str) -> String {
    format!(
        "{}/{}",
        site.base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn excerpt(markdown: &str, max_characters: usize) -> String {
    let text = markdown
        .lines()
        .map(|line| line.trim_start_matches(['#', '>', '-', '*', ' ']))
        .collect::<Vec<_>>()
        .join(" ");
    let mut result = text.chars().take(max_characters).collect::<String>();
    if text.chars().count() > max_characters {
        result.push_str("...");
    }
    result
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{render_rss, render_sitemap, xml_escape};
    use crate::{config::SiteSettings, entity::post::Model as Post};
    use chrono::Utc;

    fn post() -> Post {
        Post {
            id: 1,
            name: "first-post".to_string(),
            title: "Fish & Chips".to_string(),
            content: "unused".to_string(),
            author: 1,
            description: Some("A <description>".to_string()),
            illustration: None,
            tags: None,
            categories: None,
            hidden: Some(false),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    fn site() -> SiteSettings {
        SiteSettings {
            site_name: "Bamboo & Blog".to_string(),
            base_url: "https://example.com/".to_string(),
            description: "A <journal>".to_string(),
            language: "en".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn renders_a_system_rss_feed_without_theme_data() {
        let rss = render_rss(&site(), &[post()]);

        assert!(rss.starts_with("<?xml version=\"1.0\""));
        assert!(rss.contains("Bamboo &amp; Blog"));
        assert!(rss.contains("https://example.com/index.xml"));
        assert!(rss.contains("Fish &amp; Chips"));
        assert!(rss.contains("A &lt;description&gt;"));
    }

    #[test]
    fn renders_a_sitemap_for_system_pages_and_posts() {
        let sitemap = render_sitemap(&site(), &[post()]);

        assert!(sitemap.starts_with("<?xml version=\"1.0\""));
        assert!(sitemap.contains("https://example.com/archives"));
        assert!(sitemap.contains("https://example.com/posts/first-post"));
        assert!(sitemap.contains("<lastmod>"));
    }

    #[test]
    fn escapes_xml_control_characters() {
        assert_eq!(xml_escape("<&>\"'"), "&lt;&amp;&gt;&quot;&apos;");
    }
}
