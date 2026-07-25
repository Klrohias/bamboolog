use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "post_tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub post_id: i32,
    #[sea_orm(primary_key, auto_increment = false, indexed)]
    pub tag_id: i32,
    #[sea_orm(belongs_to, from = "post_id", to = "id")]
    pub post: BelongsTo<super::post::Entity>,
    #[sea_orm(belongs_to, from = "tag_id", to = "id")]
    pub tag: BelongsTo<super::tag::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
