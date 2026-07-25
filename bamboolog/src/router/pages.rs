use std::collections::BTreeMap;

use axum::{
    Extension, Router,
    extract::{Path, Query},
    http::StatusCode,
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

use crate::{
    config::SiteSettings,
    entity::post::{Column as PostColumn, Entity as PostEntity, Model as Post},
    service::{
        site_settings::SiteSettingsService,
        storage::StorageService,
        taxonomy::{PostTerms, TaxonomyKind, TaxonomyService},
        theme::ThemeService,
    },
    utils::{HttpFailibleOperationExts, Pagination, render_markdown},
};

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

const LAYOUT_HOME: &str = "home";
const LAYOUT_ARCHIVE: &str = "archive";
const LAYOUT_TERMS: &str = "terms";
const LAYOUT_TAXONOMY: &str = "taxonomy";
const LAYOUT_POST: &str = "post";
const LAYOUT_NOT_FOUND: &str = "not-found";

struct PostWithTerms {
    post: Post,
    terms: PostTerms,
}

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(display_home))
        .route("/archives", get(display_archives))
        .route("/tags", get(display_tags))
        .route("/tags/{term}", get(display_tag))
        .route("/categories", get(display_categories))
        .route("/categories/{term}", get(display_category))
        .route("/posts/{id_or_name}", get(display_post))
        .route("/static/theme/{*path}", get(serve_theme_static))
        .route("/attachments/{hash}", get(serve_attachment))
}

async fn display_archives(
    Query(query): Query<PageQuery>,
    Extension(database): Extension<DatabaseConnection>,
    Extension(theme_service): Extension<ThemeService>,
    Extension(site_settings): Extension<SiteSettingsService>,
) -> Result<Html<String>, Response> {
    let site = site_settings.read().await.clone();
    let pagination = public_pagination(query.page, &site);
    let (total, posts) = paginated_visible_posts(&database, pagination).await?;
    let posts = posts_with_terms(&database, posts).await?;
    let mut years = BTreeMap::<String, Vec<Value>>::new();
    for post in posts {
        years
            .entry(post.post.created_at.format("%Y").to_string())
            .or_default()
            .push(post_summary(&post));
    }

    Ok(Html(
        theme_service
            .render(
                LAYOUT_ARCHIVE,
                json!({
                    "site": site_context(&site),
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
        TaxonomyKind::Tag,
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
        TaxonomyKind::Category,
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
        TaxonomyKind::Tag,
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
        TaxonomyKind::Category,
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
    kind: TaxonomyKind,
    title: &str,
    query: TaxonomyQuery,
    database: DatabaseConnection,
    theme_service: ThemeService,
    site_settings: SiteSettingsService,
) -> Result<Html<String>, Response> {
    let selected = query.name.filter(|name| !name.trim().is_empty());
    let site = site_settings.read().await.clone();
    let field = kind.path_segment();

    if let Some(selected) = selected {
        let path = format!("/{field}/{}", encode_query_component(&selected));
        let pagination = public_pagination(query.page, &site);
        let paginator = TaxonomyService::visible_posts_for_term(kind, &selected)
            .paginate(&database, pagination.size());
        let total = paginator
            .num_items()
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?;
        let posts = posts_with_terms(
            &database,
            paginator
                .fetch_page(pagination.offset())
                .await
                .traced_and_response(|e| tracing::error!("{}", e))?,
        )
        .await?
        .iter()
        .map(post_summary)
        .collect::<Vec<_>>();
        return Ok(Html(
            theme_service
                .render(
                    LAYOUT_TAXONOMY,
                    json!({
                        "site": site_context(&site),
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

    let counts = TaxonomyService::visible_term_counts(&database, kind)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(Html(
        theme_service
            .render(
                LAYOUT_TERMS,
                json!({
                    "site": site_context(&site),
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
    let site = site_settings.read().await.clone();
    let pagination = public_pagination(query.page, &site);
    let (total, posts) = paginated_visible_posts(&database, pagination).await?;
    let posts = posts_with_terms(&database, posts).await?;

    Ok(Html(
        theme_service
            .render(
                LAYOUT_HOME,
                json!({
                    "site": site_context(&site),
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
                    LAYOUT_NOT_FOUND,
                    json!({
                        "site": site_context(&site),
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
                LAYOUT_NOT_FOUND,
                json!({
                    "site": site_context(&site),
                    "page": { "kind": "not-found", "title": "Not found", "description": "The requested post does not exist.", "url": "" },
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?;
        return Err((StatusCode::NOT_FOUND, Html(content)).into_response());
    }

    let post_with_terms = posts_with_terms(&database, vec![post.clone()])
        .await?
        .pop()
        .expect("a post always has a term context");

    // Render markdown
    let rendered_content =
        render_markdown(&post.content).traced_and_response(|e| tracing::error!("{}", e))?;

    let site = site_settings.read().await.clone();
    let newer_post = visible_posts_query()
        .filter(PostColumn::Id.ne(post.id))
        .filter(PostColumn::CreatedAt.gt(post.created_at))
        .order_by_asc(PostColumn::CreatedAt)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    let older_post = visible_posts_query()
        .filter(PostColumn::Id.ne(post.id))
        .filter(PostColumn::CreatedAt.lt(post.created_at))
        .order_by_desc(PostColumn::CreatedAt)
        .one(&database)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;

    // Render jinja
    let related_posts = posts_with_terms(
        &database,
        [newer_post.clone(), older_post.clone()]
            .into_iter()
            .flatten()
            .collect(),
    )
    .await?;
    let newer_post = related_posts.iter().find(|candidate| {
        newer_post
            .as_ref()
            .is_some_and(|post| candidate.post.id == post.id)
    });
    let older_post = related_posts.iter().find(|candidate| {
        older_post
            .as_ref()
            .is_some_and(|post| candidate.post.id == post.id)
    });

    Ok(Html(
        theme_service
            .render(
                LAYOUT_POST,
                json!({
                    "site": site_context(&site),
                    "page": {
                        "kind": "post", 
                        "title": post.title, 
                        "description": post.description.clone().unwrap_or_else(|| excerpt(&post.content, 240)), 
                        "illustration": post.illustration.clone(), 
                        "url": post_url(&post), 
                        "functions": post.functions.0 
                    },
                    "content": rendered_content,
                    "post": post_detail(&post_with_terms),
                    "newer_post": newer_post.map(post_summary),
                    "older_post": older_post.map(post_summary),
                }),
            )
            .await
            .traced_and_response(|e| tracing::error!("{}", e))?,
    ))
}

fn site_context(site: &SiteSettings) -> Value {
    json!({
        "name": site.site_name,
        "base_url": site.base_url.trim_end_matches('/'),
        "copyright": site.copyright,
        "description": site.description,
        "language": if site.language.trim().is_empty() { "en" } else { site.language.as_str() },
        "favicon_url": site.favicon_url,
        "home_url": "/",
    })
}

fn public_pagination(page: Option<u64>, site: &SiteSettings) -> Pagination {
    let page_size = site.public_posts_per_page();
    Pagination::new(page, Some(page_size), page_size)
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

fn post_summary(context: &PostWithTerms) -> Value {
    let post = &context.post;
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
        "tags": context.terms.tags,
        "categories": context.terms.categories,
        "functions": post.functions.0,
    })
}

fn visible_posts_query() -> sea_orm::Select<PostEntity> {
    PostEntity::find()
        .filter(
            Condition::any()
                .add(PostColumn::Hidden.eq(false))
                .add(PostColumn::Hidden.is_null()),
        )
        .order_by_desc(PostColumn::CreatedAt)
}

async fn paginated_visible_posts(
    database: &DatabaseConnection,
    pagination: Pagination,
) -> Result<(u64, Vec<Post>), Response> {
    let paginator = visible_posts_query().paginate(database, pagination.size());
    let total = paginator
        .num_items()
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    let posts = paginator
        .fetch_page(pagination.offset())
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok((total, posts))
}

fn post_detail(post: &PostWithTerms) -> Value {
    let mut result = post_summary(post);
    result["content"] = Value::String(post.post.content.clone());
    result
}

async fn posts_with_terms(
    database: &DatabaseConnection,
    posts: Vec<Post>,
) -> Result<Vec<PostWithTerms>, Response> {
    let post_ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
    let terms = TaxonomyService::terms_for_posts(database, &post_ids)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))?;
    Ok(posts
        .into_iter()
        .map(|post| PostWithTerms {
            terms: terms.get(&post.id).cloned().unwrap_or_default(),
            post,
        })
        .collect())
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
    Extension(storage): Extension<StorageService>,
) -> Result<Response, Response> {
    storage
        .serve(&db, &hash)
        .await
        .traced_and_response(|e| tracing::error!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::{encode_query_component, excerpt, pagination_context, reading_minutes};
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
}
