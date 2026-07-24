use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
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
    pub tags: Option<String>,
    pub categories: Option<String>,
    pub hidden: Option<bool>,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,
    // Nullable so existing SQLite databases can add this column in place.
    pub updated_at: Option<DateTimeUtc>,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::Author",
        to = "super::user::Column::Id"
    )]
    User,
}
