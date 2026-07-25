use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "post_categories")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub post_id: i32,
    #[sea_orm(primary_key, auto_increment = false, indexed)]
    pub category_id: i32,
    #[sea_orm(belongs_to, from = "post_id", to = "id")]
    pub post: BelongsTo<super::post::Entity>,
    #[sea_orm(belongs_to, from = "category_id", to = "id")]
    pub category: BelongsTo<super::category::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
