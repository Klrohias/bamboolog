use std::collections::BTreeMap;

use axum::{
    Extension, Router,
    extract::{Path, Query},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::instrument;

use crate::service::storage::StorageService;
use crate::{
    config::SiteSettings,
    entity::post::{Column as PostColumn, Entity as PostEntity, Model as Post},
    service::{site_settings::SiteSettingsService, theme::ThemeService},
    utils::{HttpFailibleOperationExts, Pagination, render_markdown},
};

const PUBLIC_PAGE_SIZE: u64 = 10;

#[derive(Debug, Deserialize)]
struct HomeQuery {
    page: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PageQuery {
    page: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TaxonomyQuery {
    name: Option<String>,
    page: Option<u64>,
}

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(display_home))
        .route("/index.xml", get(display_feed))
        .route("/archives", get(display_archives))
        .route("/tags", get(display_tags))
        .route("/tags/{term}", get(display_tag))
        .route("/categories", get(display_categories))
        .route("/categories/{term}", get(display_category))
        .route("/posts/{id_or_name}", get(display_post))
        .route("/static/theme/{*path}", get(serve_theme_static))
        .route("/attachments/{hash}", get(serve_attachment))
}

async fn display_feed(
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Response, Response> {
    let site = site_settings.read().await.clone();
    let content = theme_service
        .render(
            "feed",
            json!({
                "site": site_context(&site, "/index.xml"),
                "posts": visible_posts(&database).await?.iter().map(post_summary).collect::<Vec<_>>(),
            }),
        )
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        content,
    )
        .into_response())
}

async fn display_archives(
    Query(query): Query<PageQuery>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    let pagination = Pagination::new(query.page, Some(PUBLIC_PAGE_SIZE), PUBLIC_PAGE_SIZE);
    let posts = visible_posts(&database).await?;
    let total = posts.len() as u64;
    let mut years = BTreeMap::<String, Vec<Value>>::new();
    for post in page_posts(posts, pagination) {
        years
            .entry(post.created_at.format("%Y").to_string())
            .or_default()
            .push(post_summary(&post));
    }
    let site = site_settings.read().await.clone();

    Ok(Html(
        theme_service
            .render(
                "archive",
                json!({
                    "site": site_context(&site, "/archives"),
                    "page": { "kind": "archives", "title": "Archives", "description": "Archives", "url": "/archives" },
                    "years": years.into_iter().rev().map(|(year, posts)| json!({ "year": year, "posts": posts })).collect::<Vec<_>>(),
                    "pagination": pagination_context(pagination, total, "/archives"),
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    ))
}

async fn display_tags(
    Query(query): Query<TaxonomyQuery>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    display_taxonomy(
        "tags",
        "Tags",
        query,
        database,
        theme_service,
        site_settings,
    )
    .await
}

async fn display_categories(
    Query(query): Query<TaxonomyQuery>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    display_taxonomy(
        "categories",
        "Categories",
        query,
        database,
        theme_service,
        site_settings,
    )
    .await
}

async fn display_tag(
    Path(term): Path<String>,
    Query(query): Query<PageQuery>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    display_taxonomy(
        "tags",
        "Tags",
        TaxonomyQuery {
            name: Some(term),
            page: query.page,
        },
        database,
        theme_service,
        site_settings,
    )
    .await
}

async fn display_category(
    Path(term): Path<String>,
    Query(query): Query<PageQuery>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    display_taxonomy(
        "categories",
        "Categories",
        TaxonomyQuery {
            name: Some(term),
            page: query.page,
        },
        database,
        theme_service,
        site_settings,
    )
    .await
}

async fn display_taxonomy(
    field: &str,
    title: &str,
    query: TaxonomyQuery,
    database: DatabaseConnection,
    theme_service: ThemeService,
    site_settings: SiteSettingsService,
) -> Result<Html<String>, Response> {
    let posts = visible_posts(&database).await?;
    let selected = query.name.filter(|name| !name.trim().is_empty());
    let site = site_settings.read().await.clone();

    if let Some(selected) = selected {
        let path = format!("/{field}/{}", encode_query_component(&selected));
        let posts = posts
            .into_iter()
            .filter(|post| {
                taxonomy_terms(post, field)
                    .iter()
                    .any(|term| term == &selected)
            })
            .collect::<Vec<_>>();
        let pagination = Pagination::new(query.page, Some(PUBLIC_PAGE_SIZE), PUBLIC_PAGE_SIZE);
        let total = posts.len() as u64;
        let posts = page_posts(posts, pagination)
            .iter()
            .map(post_summary)
            .collect::<Vec<_>>();
        return Ok(Html(
            theme_service
                .render(
                    "taxonomy",
                    json!({
                        "site": site_context(&site, &format!("/{field}")),
                        "page": { "kind": field, "title": format!("{title}: {selected}"), "description": format!("{title}: {selected}"), "url": path },
                        "taxonomy": { "kind": field, "name": title, "term": selected },
                        "posts": posts,
                        "pagination": pagination_context(pagination, total, &path),
                    }),
                )
                .await
                .traced_and_response(|e| tracing::error!("{}", e))?,
        ));
    }

    let mut counts = BTreeMap::<String, u64>::new();
    for post in &posts {
        for term in taxonomy_terms(post, field) {
            *counts.entry(term).or_default() += 1;
        }
    }
    Ok(Html(
        theme_service
            .render(
                "terms",
                json!({
                    "site": site_context(&site, &format!("/{field}")),
                    "page": { "kind": field, "title": title, "description": title, "url": format!("/{field}") },
                    "taxonomy": { "kind": field, "name": title, "path": format!("/{field}"), "terms": counts.into_iter().map(|(name, count)| json!({ "name": name, "count": count })).collect::<Vec<_>>() },
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    ))
}

#[instrument(skip_all)]
async fn display_home(
    Query(query): Query<HomeQuery>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    let pagination = Pagination::new(query.page, Some(PUBLIC_PAGE_SIZE), PUBLIC_PAGE_SIZE);
    let posts_query = PostEntity::find()
        .filter(
            Condition::any()
                .add(PostColumn::Hidden.eq(false))
                .add(PostColumn::Hidden.is_null()),
        )
        .order_by_desc(PostColumn::CreatedAt);
    let paginator = posts_query.paginate(&database, pagination.size());
    let total = paginator
        .num_items()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    let posts = paginator
        .fetch_page(pagination.offset())
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    let site = site_settings.read().await.clone();

    Ok(Html(
        theme_service
            .render(
                "home",
                json!({
                    "site": site_context(&site, "/"),
                    "page": { "kind": "home", "title": site.site_name, "description": site.description, "url": "/" },
                    "posts": posts.iter().map(post_summary).collect::<Vec<_>>(),
                    "pagination": pagination_context(pagination, total, "/"),
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    ))
}

async fn display_post(
    Path(id_or_name): Path<String>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    // Is `is_or_name` a number?
    let post = match id_or_name.parse::<i32>() {
        Err(_) => PostEntity::find()
            .filter(PostColumn::Name.eq(id_or_name))
            .one(&database)
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
        Ok(id) => PostEntity::find_by_id(id)
            .one(&database)
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    };

    // Really found?
    let post = match post {
        None => {
            let site = site_settings.read().await.clone();
            let content = theme_service
                .render(
                    "not-found",
                    json!({
                        "site": site_context(&site, ""),
                        "page": { "kind": "not-found", "title": "Not found", "description": "The requested post does not exist.", "url": "" },
                    }),
                )
                .await
                .traced_and_response(|e| tracing::error!("{}", e))?;
            return Err((StatusCode::NOT_FOUND, Html(content)).into_response());
        }
        Some(v) => v,
    };

    if post.hidden.unwrap_or(false) {
        let site = site_settings.read().await.clone();
        let content = theme_service
            .render(
                "not-found",
                json!({
                    "site": site_context(&site, ""),
                    "page": { "kind": "not-found", "title": "Not found", "description": "The requested post does not exist.", "url": "" },
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?;
        return Err((StatusCode::NOT_FOUND, Html(content)).into_response());
    }

    // Render markdown
    let rendered_content = render_markdown(&post.content)
        .traced_and_response(|e| tracing::error!("{}", e))?;

    let site = site_settings.read().await.clone();
    let newer_post = PostEntity::find()
        .filter(PostColumn::Id.ne(post.id))
        .filter(PostColumn::CreatedAt.gt(post.created_at))
        .order_by_asc(PostColumn::CreatedAt)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    let older_post = PostEntity::find()
        .filter(PostColumn::Id.ne(post.id))
        .filter(PostColumn::CreatedAt.lt(post.created_at))
        .order_by_desc(PostColumn::CreatedAt)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    // Render jinja
    Ok(Html(
        theme_service
            .render(
                "post",
                json!({
                    "site": site_context(&site, &post_url(&post)),
                    "page": { "kind": "post", "title": post.title, "description": post.description.clone().unwrap_or_else(|| excerpt(&post.content, 240)), "illustration": post.illustration.clone(), "toc_enabled": post.toc_enabled.unwrap_or(true), "math_enabled": post.math_enabled, "url": post_url(&post) },
                    "content": rendered_content,
                    "post": post_detail(&post),
                    "comments": comment_context(&site.comments, post.comments_enabled.unwrap_or(true)),
                    "newer_post": newer_post.as_ref().map(post_summary),
                    "older_post": older_post.as_ref().map(post_summary),
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    ))
}

fn site_context(site: &SiteSettings, current_url: &str) -> Value {
    let navigation = if site.navigation.is_empty() {
        vec![
            ("Archives", "/archives", Some("archives")),
            ("Categories", "/categories", Some("categories")),
            ("Tags", "/tags", Some("tags")),
            ("RSS Feed", "/index.xml", Some("rss_feed")),
        ]
    } else {
        site.navigation
            .iter()
            .map(|item| (item.label.as_str(), item.url.as_str(), None))
            .collect()
    };
    json!({
        "name": site.site_name,
        "base_url": site.base_url.trim_end_matches('/'),
        "copyright": site.copyright,
        "description": site.description,
        "language": if site.language.trim().is_empty() { "en" } else { site.language.as_str() },
        "favicon_url": site.favicon_url,
        "manifest_url": site.manifest_url,
        "search": {
            "google_cse_id": site.search.google_cse_id,
        },
        "analytics": {
            "google_analytics_id": site.analytics.google_analytics_id,
            "clarity_project_id": site.analytics.clarity_project_id,
            "cloudflare_beacon_token": site.analytics.cloudflare_beacon_token,
        },
        "head_html": site.head_html,
        "home_url": "/",
        "navigation": navigation.into_iter().map(|(label, url, translation_key)| json!({
            "label": label,
            "url": url,
            "translation_key": translation_key,
            "active": current_url == url || (url != "/" && current_url.starts_with(url)),
        })).collect::<Vec<_>>(),
    })
}

fn comment_context(settings: &crate::config::CommentSettings, post_enabled: bool) -> Value {
    let provider = settings.provider.trim().to_ascii_lowercase();
    let enabled = post_enabled
        && matches!(
            provider.as_str(),
            "disqus" | "utterances" | "giscus" | "livere" | "twikoo" | "waline"
        )
        && settings.is_configured();
    json!({
        "enabled": enabled,
        "disabled_for_post": !post_enabled,
        "provider": provider,
        "config": settings.config,
    })
}

fn pagination_context(pagination: Pagination, total: u64, path: &str) -> Value {
    let total_pages = pagination.total_pages(total);
    let page = pagination.page();
    let separator = if path.contains('?') { "&" } else { "?" };
    json!({
        "page": page,
        "page_size": pagination.size(),
        "total": total,
        "total_pages": total_pages,
        "has_previous": page > 1,
        "previous_url": format!("{path}{separator}page={}", page.saturating_sub(1)),
        "has_next": page < total_pages,
        "next_url": format!("{path}{separator}page={}", page + 1),
    })
}

fn page_posts(posts: Vec<Post>, pagination: Pagination) -> Vec<Post> {
    let offset = pagination.offset().saturating_mul(pagination.size());
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    posts
        .into_iter()
        .skip(offset)
        .take(pagination.size() as usize)
        .collect()
}

fn encode_query_component(value: &str) -> String {
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

fn post_url(post: &Post) -> String {
    format!("/posts/{}", post.name)
}

fn post_summary(post: &Post) -> Value {
    json!({
        "id": post.id,
        "name": post.name,
        "title": post.title,
        "created_at": post.created_at,
        "updated_at": post
            .updated_at
            .clone()
            .unwrap_or_else(|| post.created_at.clone()),
        "url": post_url(post),
        "summary": post.description.clone().filter(|description| !description.is_empty()).unwrap_or_else(|| excerpt(&post.content, 240)),
        "reading_minutes": reading_minutes(&post.content),
        "illustration": post.illustration,
        "tags": terms(&post.tags),
        "categories": terms(&post.categories),
        "toc_enabled": post.toc_enabled.unwrap_or(true),
        "math_enabled": post.math_enabled,
        "comments_enabled": post.comments_enabled.unwrap_or(true),
    })
}

fn terms(value: &Option<String>) -> Vec<String> {
    value
        .as_deref()
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_default()
}

fn taxonomy_terms(post: &Post, field: &str) -> Vec<String> {
    match field {
        "tags" => terms(&post.tags),
        "categories" => terms(&post.categories),
        _ => Vec::new(),
    }
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
        .traced_and_response(|e| tracing::error!("{}", e))
}

fn post_detail(post: &Post) -> Value {
    let mut result = post_summary(post);
    result["content"] = Value::String(post.content.clone());
    result
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

fn reading_minutes(markdown: &str) -> u64 {
    let words = markdown.split_whitespace().count() as u64;
    words.div_ceil(220).max(1)
}

async fn serve_theme_static(
    Path(path): Path<String>,
    Extension(theme_service): Extension<ThemeService>,
) -> Result<Response, Response> {
    theme_service
        .serve_static(path)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))
}

async fn serve_attachment(
    Path(hash): Path<String>,
    Extension(db): Extension<DatabaseConnection>,
    Extension(config): Extension<std::sync::Arc<crate::config::ApplicationConfiguration>>,
) -> Result<Response, Response> {
    StorageService::serve(&db, config.clone(), &hash)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::{
        SiteSettings, comment_context, encode_query_component, excerpt, pagination_context,
        reading_minutes, site_context,
    };
    use crate::utils::Pagination;

    #[test]
    fn creates_a_short_plain_text_excerpt() {
        assert_eq!(
            excerpt("# Heading\n\nA paragraph", 240),
            "Heading  A paragraph"
        );
        assert_eq!(excerpt("abcdefgh", 5), "abcde...");
    }

    #[test]
    fn reading_time_is_at_least_one_minute() {
        assert_eq!(reading_minutes("brief"), 1);
        assert_eq!(reading_minutes(&"word ".repeat(221)), 2);
    }

    #[test]
    fn preserves_taxonomy_filters_in_pagination_urls() {
        let context = pagination_context(Pagination::new(Some(2), Some(10), 10), 30, "/tags/Rust");

        assert_eq!(context["previous_url"], "/tags/Rust?page=1");
        assert_eq!(context["next_url"], "/tags/Rust?page=3");
        assert_eq!(encode_query_component("Rust & Web"), "Rust%20%26%20Web");
    }

    #[test]
    fn provides_feed_aware_default_navigation() {
        let context = site_context(&SiteSettings::default(), "/");
        let navigation = context["navigation"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["url"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            navigation,
            ["/archives", "/categories", "/tags", "/index.xml"]
        );
    }

    #[test]
    fn exposes_site_integrations_without_theme_configuration() {
        let context = site_context(
            &SiteSettings {
                search: crate::config::SearchSettings {
                    google_cse_id: "search-id".to_string(),
                },
                analytics: crate::config::AnalyticsSettings {
                    google_analytics_id: "G-123".to_string(),
                    clarity_project_id: "clarity-id".to_string(),
                    cloudflare_beacon_token: "cf-token".to_string(),
                },
                head_html: "<meta name=\"verification\" content=\"token\">".to_string(),
                ..Default::default()
            },
            "/",
        );

        assert_eq!(context["search"]["google_cse_id"], "search-id");
        assert_eq!(context["analytics"]["google_analytics_id"], "G-123");
        assert_eq!(context["head_html"], "<meta name=\"verification\" content=\"token\">");
    }

    #[test]
    fn exposes_configured_comment_providers_only_for_enabled_posts() {
        let settings = crate::config::CommentSettings {
            provider: "Giscus".to_string(),
            config: [
                ("repo".to_string(), "example/blog".to_string()),
                ("repo_id".to_string(), "repo-id".to_string()),
                ("category".to_string(), "General".to_string()),
                ("category_id".to_string(), "category-id".to_string()),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(comment_context(&settings, true)["enabled"], true);
        assert_eq!(comment_context(&settings, false)["enabled"], false);
        assert_eq!(comment_context(&settings, true)["provider"], "giscus");
        let incomplete = crate::config::CommentSettings {
            provider: "giscus".to_string(),
            config: [("repo".to_string(), "example/blog".to_string())]
                .into_iter()
                .collect(),
        };
        assert_eq!(comment_context(&incomplete, true)["enabled"], false);
        let waline = crate::config::CommentSettings {
            provider: "waline".to_string(),
            config: [("server_url".to_string(), "https://comments.example".to_string())]
                .into_iter()
                .collect(),
        };
        assert_eq!(comment_context(&waline, true)["enabled"], true);
    }
}
