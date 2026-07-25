use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique, indexed)]
    pub name: String,
    pub title: String,
    pub content: String,
    pub author: i32,
    pub description: Option<String>,
    pub illustration: Option<String>,
    pub hidden: Option<bool>,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,
    pub updated_at: Option<DateTimeUtc>,
    #[sea_orm(has_many, via = "post_tag")]
    pub tags: HasMany<super::tag::Entity>,
    #[sea_orm(has_many, via = "post_category")]
    pub categories: HasMany<super::category::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
