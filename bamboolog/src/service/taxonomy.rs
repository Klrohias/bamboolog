use std::collections::{HashMap, HashSet};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, sea_query::Expr,
};

use crate::entity::{category, post, post_category, post_tag, tag};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PostTerms {
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxonomyKind {
    Tag,
    Category,
}

impl TaxonomyKind {
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::Tag => "tags",
            Self::Category => "categories",
        }
    }
}

pub struct TaxonomyService;

impl TaxonomyService {
    pub async fn replace_post_terms<C>(
        db: &C,
        post_id: i32,
        tags: Option<Vec<String>>,
        categories: Option<Vec<String>>,
    ) -> Result<(), sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        if let Some(tags) = tags {
            post_tag::Entity::delete_many()
                .filter(post_tag::Column::PostId.eq(post_id))
                .exec(db)
                .await?;
            for name in normalize_terms(tags) {
                let tag = find_or_create_tag(db, name).await?;
                post_tag::ActiveModel {
                    post_id: Set(post_id),
                    tag_id: Set(tag.id),
                }
                .insert(db)
                .await?;
            }
        }

        if let Some(categories) = categories {
            post_category::Entity::delete_many()
                .filter(post_category::Column::PostId.eq(post_id))
                .exec(db)
                .await?;
            for name in normalize_terms(categories) {
                let category = find_or_create_category(db, name).await?;
                post_category::ActiveModel {
                    post_id: Set(post_id),
                    category_id: Set(category.id),
                }
                .insert(db)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_post_terms<C>(db: &C, post_id: i32) -> Result<(), sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        post_tag::Entity::delete_many()
            .filter(post_tag::Column::PostId.eq(post_id))
            .exec(db)
            .await?;
        post_category::Entity::delete_many()
            .filter(post_category::Column::PostId.eq(post_id))
            .exec(db)
            .await?;
        Ok(())
    }

    pub async fn terms_for_posts<C>(
        db: &C,
        post_ids: &[i32],
    ) -> Result<HashMap<i32, PostTerms>, sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        let mut terms = post_ids
            .iter()
            .copied()
            .map(|post_id| (post_id, PostTerms::default()))
            .collect::<HashMap<_, _>>();
        if post_ids.is_empty() {
            return Ok(terms);
        }

        let tags = post::Entity::find()
            .filter(post::Column::Id.is_in(post_ids.iter().copied()))
            .find_with_related(tag::Entity)
            .all(db)
            .await?;
        for (post, tags) in tags {
            if let Some(terms) = terms.get_mut(&post.id) {
                terms.tags.extend(tags.into_iter().map(|tag| tag.name));
            }
        }

        let categories = post::Entity::find()
            .filter(post::Column::Id.is_in(post_ids.iter().copied()))
            .find_with_related(category::Entity)
            .all(db)
            .await?;
        for (post, categories) in categories {
            if let Some(terms) = terms.get_mut(&post.id) {
                terms
                    .categories
                    .extend(categories.into_iter().map(|category| category.name));
            }
        }

        for value in terms.values_mut() {
            value.tags.sort();
            value.categories.sort();
        }
        Ok(terms)
    }

    pub fn visible_posts_for_term(kind: TaxonomyKind, term: &str) -> sea_orm::Select<post::Entity> {
        match kind {
            TaxonomyKind::Tag => visible_posts()
                .inner_join(tag::Entity)
                .filter(tag::Column::Name.eq(term)),
            TaxonomyKind::Category => visible_posts()
                .inner_join(category::Entity)
                .filter(category::Column::Name.eq(term)),
        }
    }

    pub async fn visible_term_counts<C>(
        db: &C,
        kind: TaxonomyKind,
    ) -> Result<Vec<(String, u64)>, sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        let rows = match kind {
            TaxonomyKind::Tag => {
                tag::Entity::find()
                    .select_only()
                    .column(tag::Column::Name)
                    .column_as(Expr::col((post::Entity, post::Column::Id)).count(), "count")
                    .inner_join(post::Entity)
                    .filter(visible_posts_condition())
                    .group_by(tag::Column::Id)
                    .order_by_asc(tag::Column::Name)
                    .into_tuple::<(String, i64)>()
                    .all(db)
                    .await?
            }
            TaxonomyKind::Category => {
                category::Entity::find()
                    .select_only()
                    .column(category::Column::Name)
                    .column_as(Expr::col((post::Entity, post::Column::Id)).count(), "count")
                    .inner_join(post::Entity)
                    .filter(visible_posts_condition())
                    .group_by(category::Column::Id)
                    .order_by_asc(category::Column::Name)
                    .into_tuple::<(String, i64)>()
                    .all(db)
                    .await?
            }
        };

        Ok(rows
            .into_iter()
            .map(|(name, count)| (name, count as u64))
            .collect())
    }
}

fn visible_posts() -> sea_orm::Select<post::Entity> {
    post::Entity::find()
        .filter(visible_posts_condition())
        .order_by_desc(post::Column::CreatedAt)
}

fn visible_posts_condition() -> Condition {
    Condition::any()
        .add(post::Column::Hidden.eq(false))
        .add(post::Column::Hidden.is_null())
}

fn normalize_terms(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && seen.insert(value.clone()))
        .collect()
}

async fn find_or_create_tag<C>(db: &C, name: String) -> Result<tag::Model, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    match tag::Entity::find()
        .filter(tag::Column::Name.eq(&name))
        .one(db)
        .await?
    {
        Some(tag) => Ok(tag),
        None => {
            tag::ActiveModel {
                name: Set(name),
                ..Default::default()
            }
            .insert(db)
            .await
        }
    }
}

async fn find_or_create_category<C>(db: &C, name: String) -> Result<category::Model, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    match category::Entity::find()
        .filter(category::Column::Name.eq(&name))
        .one(db)
        .await?
    {
        Some(category) => Ok(category),
        None => {
            category::ActiveModel {
                name: Set(name),
                ..Default::default()
            }
            .insert(db)
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, DatabaseBackend, Schema, Set};

    use crate::entity::{self, post, user};

    use super::{TaxonomyKind, TaxonomyService};

    async fn database_with_taxonomy_schema() -> sea_orm::DatabaseConnection {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let schema = Schema::new(DatabaseBackend::Sqlite);
        for statement in [
            schema.create_table_from_entity(user::Entity),
            schema.create_table_from_entity(post::Entity),
            schema.create_table_from_entity(entity::tag::Entity),
            schema.create_table_from_entity(entity::category::Entity),
            schema.create_table_from_entity(entity::post_tag::Entity),
            schema.create_table_from_entity(entity::post_category::Entity),
        ] {
            database.execute(&statement).await.unwrap();
        }
        database
    }

    async fn insert_post(
        database: &sea_orm::DatabaseConnection,
        id: i32,
        name: &str,
        hidden: bool,
    ) -> post::Model {
        post::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            title: Set(name.to_string()),
            content: Set(String::new()),
            author: Set(1),
            hidden: Set(Some(hidden)),
            ..Default::default()
        }
        .insert(database)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn filters_and_counts_only_visible_posts_in_the_database() {
        let database = database_with_taxonomy_schema().await;
        user::ActiveModel {
            id: Set(1),
            username: Set("admin".to_string()),
            email: Set("admin@example.test".to_string()),
            nickname: Set("Admin".to_string()),
            password_hash: Set("hash".to_string()),
            ..Default::default()
        }
        .insert(&database)
        .await
        .unwrap();
        let first = insert_post(&database, 1, "first", false).await;
        let second = insert_post(&database, 2, "second", false).await;
        let hidden = insert_post(&database, 3, "hidden", true).await;

        TaxonomyService::replace_post_terms(
            &database,
            first.id,
            Some(vec!["Rust".to_string(), "Web".to_string()]),
            Some(vec!["Engineering".to_string()]),
        )
        .await
        .unwrap();
        TaxonomyService::replace_post_terms(
            &database,
            second.id,
            Some(vec!["Rust".to_string()]),
            Some(vec!["Notes".to_string()]),
        )
        .await
        .unwrap();
        TaxonomyService::replace_post_terms(
            &database,
            hidden.id,
            Some(vec!["Rust".to_string()]),
            None,
        )
        .await
        .unwrap();

        let rust_posts = TaxonomyService::visible_posts_for_term(TaxonomyKind::Tag, "Rust")
            .all(&database)
            .await
            .unwrap();
        assert_eq!(rust_posts.len(), 2);
        assert_eq!(
            TaxonomyService::visible_term_counts(&database, TaxonomyKind::Tag)
                .await
                .unwrap(),
            vec![("Rust".to_string(), 2), ("Web".to_string(), 1)]
        );
        assert_eq!(
            TaxonomyService::visible_term_counts(&database, TaxonomyKind::Category)
                .await
                .unwrap(),
            vec![("Engineering".to_string(), 1), ("Notes".to_string(), 1)]
        );

        TaxonomyService::replace_post_terms(
            &database,
            first.id,
            Some(vec!["Databases".to_string()]),
            None,
        )
        .await
        .unwrap();
        let terms = TaxonomyService::terms_for_posts(&database, &[first.id])
            .await
            .unwrap();
        assert_eq!(terms[&first.id].tags, ["Databases"]);
        assert_eq!(terms[&first.id].categories, ["Engineering"]);
    }
}
