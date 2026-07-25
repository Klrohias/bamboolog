use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique, indexed)]
    pub name: String,
    #[sea_orm(has_many, via = "post_tag")]
    pub posts: HasMany<super::post::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
